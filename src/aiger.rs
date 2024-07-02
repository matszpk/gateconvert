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
    #[error("AndGate bad output")]
    AndGateBadOutput,
    #[error("Latch bad state")]
    LatchBadState,
    #[error("Bad input")]
    BadInput,
}

fn from_aiger_int(
    aig: &Aig<usize>,
) -> Result<(Circuit<usize>, Vec<Option<usize>>, Vec<Option<usize>>), AIGERError> {
    use gategen::boolvar::*;
    let state_len = aig.latches.len();
    let all_input_len = aig.inputs.len() + aig.latches.len();
    // exprs - input and latches initialized, rest is empty - initialized by false
    let mut exprs = (0..all_input_len)
        .map(|_| BoolVarSys::var())
        .chain((0..aig.and_gates.len()).map(|_| BoolVarSys::from(false)))
        .collect::<Vec<_>>();
    let mut expr_map = HashMap::<usize, usize>::new();
    for (i, l) in aig.latches.iter().enumerate() {
        let lo = (l.state >> 1).checked_sub(1).unwrap();
        if !expr_map.contains_key(&lo) {
            expr_map.insert(lo, i);
        } else {
            return Err(AIGERError::LatchBadState);
        }
    }
    for (i, input) in aig.inputs.iter().enumerate() {
        let ino = (input >> 1).checked_sub(1).unwrap();
        if !expr_map.contains_key(&ino) {
            expr_map.insert(ino, i + state_len);
        } else {
            return Err(AIGERError::BadInput);
        }
    }
    for (i, g) in aig.and_gates.iter().enumerate() {
        let go = (g.output >> 1).checked_sub(1).unwrap();
        if !expr_map.contains_key(&go) {
            expr_map.insert(go, all_input_len + i);
        } else {
            return Err(AIGERError::AndGateBadOutput);
        }
    }
    // expression resolve: (expr, and_gate)
    let mut expr_resolve = |l| {
        let lpos = l & !1;
        if l < 2 {
            (BoolVarSys::from(l == 1), None)
        } else if let Some(x) = expr_map.get(&lpos) {
            let and_gate = if *x >= all_input_len {
                Some(aig.and_gates[*x - all_input_len])
            } else {
                None
            };
            (&exprs[expr_map[x]] ^ ((l & 1) != 0), and_gate)
        } else {
            panic!("Unexpected literal");
        }
    };
    #[derive(Clone)]
    struct StackEntry {
        way: usize,
        lit: usize,
    }
    let mut visited = vec![false; aig.max_var_index];
    let mut path_visited = vec![false; aig.max_var_index];
    let mut stack = vec![];
    // XOR subpart gates will be skipped - if they are part of other path then included
    // automatically. Any negation propagation, constant assignments will be done
    // automatically gategen.
    let outputs = aig
        .latches
        .iter()
        .map(|latch| latch.next_state)
        .chain(aig.outputs.iter().copied())
        .collect::<Vec<_>>();
    for (i, ol) in outputs.iter().enumerate() {
        stack.push(StackEntry { way: 0, lit: *ol });
        while !stack.is_empty() {
            let mut top = stack.last_mut().unwrap();
            let (expr, and_gate) = expr_resolve(top.lit);
            let avar = top.lit >> 1;

            if let Some(and_gate) = and_gate {
                // check if XOR or Equal
                let (gi0expr, gi0and) = expr_resolve(and_gate.inputs[0]);
                let (gi1expr, gi1and) = expr_resolve(and_gate.inputs[1]);
                let (expr, gate, is_xor) =
                    if (and_gate.inputs[0] & 1) != 0 && (and_gate.inputs[1] & 1) != 0 {
                        if let Some(gi0and) = gi0and {
                            if let Some(gi1and) = gi1and {
                                // compare results
                                if (gi0and.inputs[0] == (gi1and.inputs[0] ^ 1)
                                    && gi0and.inputs[1] == (gi1and.inputs[1] ^ 1))
                                    || (gi0and.inputs[0] == (gi1and.inputs[1] ^ 1)
                                        && gi0and.inputs[1] == (gi1and.inputs[0] ^ 1))
                                {
                                    // if XOR
                                    ((gi0expr ^ gi1expr), and_gate, true)
                                } else {
                                    (expr, and_gate, false)
                                }
                            } else {
                                (expr, and_gate, false)
                            }
                        } else {
                            (expr, and_gate, false)
                        }
                    } else {
                        (expr, and_gate, false)
                    };

                let way = top.way;
                if way == 0 {
                    if !path_visited[avar - 1] {
                        path_visited[avar - 1] = true;
                    } else {
                        return Err(AIGERError::CyclesInAIGER);
                    }
                    if !visited[avar - 1] {
                        visited[avar - 1] = true;
                    } else {
                        path_visited[avar - 1] = false;
                        stack.pop();
                        continue;
                    }
                } else if way == 1 {
                } else {
                    path_visited[avar - 1] = false;
                    stack.pop();
                }
            } else {
                // if constant
                if avar >= 1 {
                    path_visited[avar - 1] = false;
                }
                stack.pop();
            }
        }
    }
    Ok((Circuit::new(0, [], []).unwrap(), vec![], vec![]))
}

// return: circuit, map for input (with values),
// map for AIGER variables (input, latches and output)
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
