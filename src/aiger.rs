// aiger.rs - AIGER conversion module

#![cfg_attr(docsrs, feature(doc_cfg))]
//! Module to conversion between Gate circuit and AIGER logic format.

use crate::gatesim::*;
use flussab::DeferredWriter;
use flussab_aiger::aig::*;
use flussab_aiger::*;

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{self, Read, Write};

use crate::AssignEntry;

/// Converts circuit to AIGER format.
///
/// Function writes Gate circuit logic in AIGER format to `out`. A `circuit` is circuit
/// to convert. A `state_len` is state length that represents in AIGER as latches.
///
/// The circuit inputs are organized in form: `[state,inputs]`.
/// The circuit outputs are organized in form: `[state,outputs]`.
///
/// A `binmode` sets mode used while writing to AIGER mode - if true then use binary mode,
/// otherwise textual mode.
pub fn to_aiger(
    circuit: &Circuit<usize>,
    state_len: usize,
    out: impl Write,
    binmode: bool,
) -> io::Result<()> {
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

/// AIGER error enumeration.
#[derive(thiserror::Error, Debug)]
pub enum AIGERError {
    /// If parse error.
    #[error("AIGER parse error {0}")]
    ParseError(#[from] ParseError),
    /// If encountered cycle in AIGER logic.
    #[error("Cycles in AIGER")]
    CyclesInAIGER,
    /// If gate have wrong output.
    #[error("AndGate bad output")]
    AndGateBadOutput,
    /// If latch have bad state.
    #[error("Latch bad state")]
    LatchBadState,
    /// If bad input.
    #[error("Bad input")]
    BadInput,
}

fn from_aiger_int(
    aig: &Aig<usize>,
) -> Result<(Circuit<usize>, Vec<(usize, AssignEntry)>), AIGERError> {
    use gategen::boolvar::*;
    use gategen::dynintvar::*;
    let state_len = aig.latches.len();
    let all_input_len = aig.inputs.len() + aig.latches.len();
    // exprs - input and latches initialized, rest is empty - initialized by false
    let mut exprs = (0..all_input_len)
        .map(|_| BoolVarSys::var())
        .chain((0..aig.and_gates.len()).map(|_| BoolVarSys::from(false)))
        .collect::<Vec<_>>();
    let mut expr_map = HashMap::<usize, usize>::new();
    for (i, l) in aig.latches.iter().enumerate() {
        let lo = l.state;
        if (lo & 1) == 0 && !expr_map.contains_key(&lo) {
            expr_map.insert(lo, i);
        } else {
            return Err(AIGERError::LatchBadState);
        }
    }
    for (i, input) in aig.inputs.iter().enumerate() {
        let ino = *input;
        if (ino & 1) == 0 && !expr_map.contains_key(&ino) {
            expr_map.insert(ino, i + state_len);
        } else {
            return Err(AIGERError::BadInput);
        }
    }
    for (i, g) in aig.and_gates.iter().enumerate() {
        let go = g.output;
        if (go & 1) == 0 && !expr_map.contains_key(&go) {
            expr_map.insert(go, all_input_len + i);
        } else {
            return Err(AIGERError::AndGateBadOutput);
        }
    }
    let and_resolve = |l: usize| {
        let lpos = l & !1;
        if l < 2 {
            None
        } else if let Some(x) = expr_map.get(&lpos) {
            if *x >= all_input_len {
                Some((*x - all_input_len, &aig.and_gates[*x - all_input_len]))
            } else {
                None
            }
        } else {
            panic!("Unexpected literal");
        }
    };
    #[derive(Clone)]
    struct StackEntry {
        way: usize,
        lit: usize,
    }
    // visited nodes in graph
    let mut visited = vec![false; aig.max_var_index];
    // path_visited - to detect cycles
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
    for ol in &outputs {
        stack.push(StackEntry { way: 0, lit: *ol });
        while !stack.is_empty() {
            let top = stack.last_mut().unwrap();
            let and_gate = and_resolve(top.lit);
            let avar = top.lit >> 1;

            if let Some((and_idx, and_gate)) = and_gate {
                // check if XOR or Equal
                let gi0and = and_resolve(and_gate.inputs[0]);
                let gi1and = and_resolve(and_gate.inputs[1]);
                // and_data - default data for AND gate
                let and_data = (and_gate.inputs[0], and_gate.inputs[1], false);
                // check and resolve XOR construction:
                // xor(a,b) = and(!and(a,b), !and(!a, !b))
                let (gi0l, gi1l, is_xor) =
                    if (and_gate.inputs[0] & 1) != 0 && (and_gate.inputs[1] & 1) != 0 {
                        if let Some((_, gi0and)) = gi0and {
                            if let Some((_, gi1and)) = gi1and {
                                // compare results
                                if (gi0and.inputs[0] == (gi1and.inputs[0] ^ 1)
                                    && gi0and.inputs[1] == (gi1and.inputs[1] ^ 1))
                                    || (gi0and.inputs[0] == (gi1and.inputs[1] ^ 1)
                                        && gi0and.inputs[1] == (gi1and.inputs[0] ^ 1))
                                {
                                    // if XOR
                                    (gi0and.inputs[0], gi0and.inputs[1], true)
                                } else {
                                    and_data
                                }
                            } else {
                                and_data
                            }
                        } else {
                            and_data
                        }
                    } else {
                        and_data
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
                    // first argument
                    top.way += 1;
                    stack.push(StackEntry { way: 0, lit: gi0l });
                } else if way == 1 {
                    // second argument
                    top.way += 1;
                    stack.push(StackEntry { way: 0, lit: gi1l });
                } else {
                    // get expressions for gate arguments
                    let gi0expr = if gi0l < 2 {
                        BoolVarSys::from(gi0l == 1)
                    } else if let Some(x) = expr_map.get(&(gi0l & !1)) {
                        &exprs[*x] ^ ((gi0l & 1) != 0)
                    } else {
                        panic!("Unexpected literal");
                    };
                    let gi1expr = if gi1l < 2 {
                        BoolVarSys::from(gi1l == 1)
                    } else if let Some(x) = expr_map.get(&(gi1l & !1)) {
                        &exprs[*x] ^ ((gi1l & 1) != 0)
                    } else {
                        panic!("Unexpected literal");
                    };
                    // set expression to exprs
                    exprs[all_input_len + and_idx] = if is_xor {
                        gi0expr ^ gi1expr
                    } else {
                        gi0expr & gi1expr
                    };
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
    // map: entry: (literal, Some(value), circuit_output_count)
    let mut ocount = 0;
    let aiger_out_map = outputs
        .iter()
        .map(|l| {
            let lpos = *l & !1;
            let expr = if *l < 2 {
                BoolVarSys::from(*l == 1)
            } else if let Some(x) = expr_map.get(&lpos) {
                &exprs[*x] ^ ((l & 1) != 0)
            } else {
                panic!("Unexpected literal");
            };
            if let Some(v) = expr.value() {
                (l, Some(v), ocount)
            } else {
                let old_ocount = ocount;
                ocount += 1;
                (l, None, old_ocount)
            }
        })
        .collect::<Vec<_>>();
    let filtered_outputs = outputs
        .iter()
        .filter_map(|l| {
            let lpos = *l & !1;
            let expr = if *l < 2 {
                BoolVarSys::from(*l == 1)
            } else if let Some(x) = expr_map.get(&lpos) {
                &exprs[*x] ^ ((l & 1) != 0)
            } else {
                panic!("Unexpected literal");
            };
            // just choose only not constant expressions
            if expr.value().is_none() {
                Some(expr)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let outint = if !filtered_outputs.is_empty() {
        UDynVarSys::from_iter(filtered_outputs.into_iter())
    } else {
        UDynVarSys::var(0)
    };

    // generate circuit with assign map
    let (circuit, assign_map) =
        outint.to_translated_circuit_with_map(exprs.iter().take(all_input_len).cloned());
    // collect for aiger_map: first are AIGER latches and AIGER inputs
    let aiger_map = aig
        .latches
        .iter()
        .map(|latch| latch.state)
        .chain(aig.inputs.iter().copied())
        .enumerate()
        .map(|(i, l)| {
            // map AIGER latches and inputs
            (
                l,
                if let Some(newidx) = assign_map[i] {
                    AssignEntry::Var(newidx, false)
                } else {
                    AssignEntry::NoMap
                },
            )
        })
        .chain(
            // main next states of latches and outputs
            aiger_out_map.into_iter().map(|(l, cb, circ_out_idx)| {
                if let Some(c) = cb {
                    // constant
                    (*l, AssignEntry::Value(c))
                } else {
                    let (o, n) = circuit.outputs()[circ_out_idx];
                    (*l, AssignEntry::Var(o, n))
                }
            }),
        )
        .collect::<Vec<_>>();
    Ok((circuit, aiger_map))
}

// return: circuit, map for AIGER variables (input, latches and output)
// format of AIGER map: (AIGER literal, AIGER Entry)

/// Converts AIGER logic to Gate circuit.
///
/// An `input` is read stream with AIGER logic. Function returns Gate circuit with its mapping.
/// Mapping in form: key - original variable in AIGER logic, value - assignment in circuit.
///
/// A `binmode` sets mode used while writing to AIGER mode - if true then use binary mode,
/// otherwise textual mode.
pub fn from_aiger(
    input: impl Read,
    binmode: bool,
) -> Result<(Circuit<usize>, Vec<(usize, AssignEntry)>), AIGERError> {
    use gategen::boolvar::*;
    let aig = if binmode {
        let parser = binary::Parser::<usize>::from_read(input, binary::Config::default())?;
        parser.parse()?.into()
    } else {
        let parser = ascii::Parser::<usize>::from_read(input, ascii::Config::default())?;
        parser.parse()?
    };
    callsys(|| from_aiger_int(&aig))
}
