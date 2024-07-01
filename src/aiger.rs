use flussab::DeferredWriter;
use flussab_aiger::aig::*;
use flussab_aiger::*;
use gatesim::*;

use std::io::Write;

pub fn to_aiger(
    circuit: &Circuit<usize>,
    state_len: usize,
    out: &mut impl Write,
    binmode: bool,
) -> Result<(), std::io::Error> {
    let input_len = circuit.input_len();
    let output_len = circuit.outputs().len();
    let outputs = circuit.outputs();
    assert!(state_len <= input_len);
    assert!(state_len <= output_len);
    // convert to OrderedAig
    let mut wires2lits = (0..state_len)
        .map(|x| 2 * (input_len - state_len + x) + 2)
        .chain((0..input_len - state_len).map(|x| 2 * x + 2))
        .collect::<Vec<_>>();
    let mut var_index = 2 * input_len + 2;
    let mut and_gate_count = 0;
    for g in circuit.gates() {
        wires2lits.push(2 * var_index);
        let count = if g.func == GateFunc::Xor { 3 } else { 1 };
        and_gate_count += count;
        var_index += count;
    }
    let mut and_gates = Vec::with_capacity(and_gate_count);
    for (i, g) in circuit.gates().iter().enumerate() {
        let wire = i + input_len;
        let olit = wires2lits[wire];
        match g.func {
            GateFunc::And => {
                and_gates.push(OrderedAndGate {
                    inputs: [wires2lits[g.i0], wires2lits[g.i1]],
                });
            }
            GateFunc::Nor => {
                and_gates.push(OrderedAndGate {
                    inputs: [wires2lits[g.i0] + 1, wires2lits[g.i1] + 1],
                });
            }
            GateFunc::Nimpl => {
                and_gates.push(OrderedAndGate {
                    inputs: [wires2lits[g.i0], wires2lits[g.i1] + 1],
                });
            }
            GateFunc::Xor => {
                // xor(a,b) = and(!and(a,b), !and(!a, !b))
                let prev0 = wires2lits[wire] - 4;
                let prev1 = prev0 + 2;
                and_gates.push(OrderedAndGate {
                    inputs: [wires2lits[g.i0], wires2lits[g.i1]],
                });
                and_gates.push(OrderedAndGate {
                    inputs: [wires2lits[g.i0] + 1, wires2lits[g.i1] + 1],
                });
                and_gates.push(OrderedAndGate {
                    inputs: [prev0 + 1, prev1 + 1],
                });
            }
        }
    }
    let ord_aig = OrderedAig {
        max_var_index: var_index,
        input_count: input_len - state_len,
        latches: (0..state_len)
            .map(|i| OrderedLatch {
                next_state: wires2lits[outputs[i].0] + usize::from(outputs[i].1),
                initialization: None,
            })
            .collect::<Vec<_>>(),
        outputs: (state_len..output_len)
            .map(|i| wires2lits[outputs[i].0] + usize::from(outputs[i].1))
            .collect::<Vec<_>>(),
        bad_state_properties: vec![],
        invariant_constraints: vec![],
        justice_properties: vec![],
        fairness_constraints: vec![],
        and_gates,
        symbols: vec![],
        comment: None,
    };
    let mut dwriter = DeferredWriter::from_write(out);
    if binmode {
        let mut writer = binary::Writer::new(dwriter);
        writer.write_ordered_aig(&ord_aig);
        writer.check_io_error()
    } else {
        let writer = ascii::Writer::new(&mut dwriter);
        writer.write_ordered_aig(&ord_aig);
        writer.check_io_error()
    }
}
