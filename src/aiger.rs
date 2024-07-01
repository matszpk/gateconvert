use flussab::DeferredWriter;
use flussab_aiger::aig::*;
use flussab_aiger::*;
use gatesim::*;

use std::collections::HashMap;
use std::io::{Read, Write};

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
    // in circuit - states are first.
    // in AIGER - states are next after inputs.
    let mut wires2lits = (0..state_len)
        .map(|x| 2 * (input_len - state_len + x) + 2)
        .chain((0..input_len - state_len).map(|x| 2 * x + 2))
        .collect::<Vec<_>>();
    let mut var_index = input_len;
    let mut and_gate_count = 0;
    for g in circuit.gates() {
        let (count, v) = if g.func == GateFunc::Xor {
            (3, var_index + 2)
        } else {
            (1, var_index)
        };
        wires2lits.push(2 * v + 2);
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
                initialization: Some(false),
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

#[derive(thiserror::Error, Debug)]
pub enum AIGERError {
    #[error("AIGER parse error {0}")]
    ParseError(#[from] ParseError),
    #[error("Cycles in AIGER")]
    CyclesInAIGER,
}

fn from_aiger_int(
    aig: &Aig<usize>,
) -> Result<(Circuit<usize>, Vec<Option<usize>>, Vec<Option<usize>>), AIGERError> {
    use gategen::boolvar::*;
    let state_len = aig.latches.len();
    let only_input_len = aig.inputs.len() + aig.latches.len();
    // wires - input and latches initialized, rest is empty - initialized by false
    let mut wires = (0..only_input_len)
        .map(|_| BoolVarSys::var())
        .chain((only_input_len..aig.max_var_index).map(|_| BoolVarSys::from(false)))
        .collect::<Vec<_>>();
    let wire_map = HashMap::<usize, usize>::from_iter(
        aig.latches
            .iter()
            .enumerate()
            .map(|(i, l)| ((l.state >> 1).checked_sub(1).unwrap(), i))
            .chain(
                aig.inputs
                    .iter()
                    .enumerate()
                    .map(|(i, input)| ((input >> 1).checked_sub(1).unwrap(), i + state_len)),
            ),
    );
    #[derive(Clone)]
    struct StackEntry {
        way: usize,
        node: usize,
    };
    let mut visited = vec![false; aig.max_var_index];
    let mut path_visited = vec![false; aig.max_var_index];
    // stage 1 - check cycles
    // stage 2 - main stage to convert AIGER to circuits.
    // XOR subpart gates will be skipped - if they are part of other path then included
    // automatically. Any negation propagation, constant assignments will be done
    // automatically gategen.
    Ok((Circuit::new(0, [], []).unwrap(), vec![], vec![]))
}

// return: circuit, map for input, map for AIGER variables
pub fn from_aiger(
    input: &mut impl Read,
    binmode: bool,
) -> Result<(Circuit<usize>, Vec<Option<usize>>, Vec<Option<usize>>), AIGERError> {
    use gategen::boolvar::*;
    let aig = if binmode {
        let mut parser = ascii::Parser::<usize>::from_read(input, ascii::Config::default())?;
        parser.parse()?
    } else {
        let mut parser = binary::Parser::<usize>::from_read(input, binary::Config::default())?;
        parser.parse()?.into()
    };
    callsys(|| from_aiger_int(&aig))
}
