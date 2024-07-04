use gatesim::*;

use std::collections::HashMap;
use std::io::{BufWriter, Write};

use crate::vcircuit::*;

pub fn to_btor2(
    circuit: &Circuit<usize>,
    state_len: usize,
    out: &mut impl Write,
) -> Result<(), std::io::Error> {
    let input_len = circuit.input_len();
    let output_len = circuit.outputs().len();
    assert!(state_len <= input_len);
    assert!(state_len <= output_len);
    let circuit = VCircuit::to_op_and_ximpl_circuit(circuit.clone(), false);
    let outputs = &circuit.outputs;

    let mut out = BufWriter::new(out);
    out.write(b"1 sort bitvec 1\n")?;
    // write states
    for i in 0..state_len {
        writeln!(out, "{} state 1", i + 2)?;
    }
    for i in state_len..input_len {
        writeln!(out, "{} input 1", i + 2)?;
    }
    // write gates
    let gate_num = circuit.gates.len();
    for (i, g) in circuit.gates.iter().enumerate() {
        let op = match g.func {
            VGateFunc::And => "and",
            VGateFunc::Nand => "nand",
            VGateFunc::Or => "or",
            VGateFunc::Nor => "nor",
            VGateFunc::Impl => "implies",
            VGateFunc::Xor => "xor",
            _ => {
                panic!("Unsupported gate function");
            }
        };
        let index = input_len + 2 + i;
        writeln!(out, "{} {} 1 {} {}", index, op, g.i0 + 2, g.i1 + 2)?;
    }
    // to collect negations
    let mut neg_map = HashMap::new();
    // write nexts
    let mut index = input_len + 2 + gate_num;
    for (i, (o, n)) in circuit.outputs[0..state_len].iter().enumerate() {
        if *n {
            let neg = if let Some(neg_nid) = neg_map.get(&o) {
                *neg_nid
            } else {
                writeln!(out, "{} not 1 {}", index, o + 2)?;
                neg_map.insert(o, index);
                index += 1;
                neg_map[o]
            };
            writeln!(out, "{} next 1 {} {}", index, i + 2, neg)?;
        } else {
            writeln!(out, "{} next 1 {} {}", index, i + 2, o + 2)?;
        }
        index += 1;
    }
    // write outputs
    for (o, n) in &circuit.outputs[state_len..] {
        if *n {
            let neg = if let Some(neg_nid) = neg_map.get(&o) {
                *neg_nid
            } else {
                writeln!(out, "{} not 1 {}", index, o + 2)?;
                neg_map.insert(o, index);
                index += 1;
                neg_map[o]
            };
            writeln!(out, "{} output {}", index, neg)?;
        } else {
            writeln!(out, "{} output {}", index, o + 2)?;
        }
        index += 1;
    }
    Ok(())
}
