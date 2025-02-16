#![cfg_attr(docsrs, feature(doc_cfg))]
//! Module to conversion between Gate circuit and the VHDL language.

use crate::gatesim::*;

use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};

use crate::vbinopcircuit::*;
use crate::vcircuit::VGateFunc;
use crate::VNegs::*;

/// Converts circuit to Verilog language source.
///
/// Function writes Gate circuit logic in Verilog language to `out`. `circuit` is circuit
/// to convert. `entity_name` is name of top entity. `arch_name` is architecture name,
/// `optimize_negs` determines whether optimize negations while conversion (if true)
/// or not (if false).
pub fn to_vhdl(
    circuit: Circuit<usize>,
    entity_name: &str,
    arch_name: &str,
    optimize_negs: bool,
    out: impl Write,
) -> io::Result<()> {
    let input_len = circuit.input_len();
    let output_len = circuit.outputs().len();

    let circuit = {
        let mut circuit = VBinOpCircuit::from(circuit);
        if optimize_negs {
            circuit.optimize_negs();
        }
        circuit
    };

    let mut out = BufWriter::new(out);
    let mut wire_out_map = BTreeMap::new();
    let mut dup_map = vec![];
    for (oi, (o, n)) in circuit.outputs.iter().enumerate() {
        if let Some((old_oi, _)) = wire_out_map.get(&(*o, *n)) {
            // resolve duplicate
            dup_map.push((oi, *old_oi));
        } else {
            wire_out_map.insert((*o, *n), (oi, *n));
        }
    }
    let resolve_name = |i| {
        if let Some((oi, _)) = wire_out_map.get(&(i, false)) {
            format!("o{}", oi)
        } else {
            format!("i{}", i)
        }
    };
    out.write(b"library ieee;\nuse ieee.std_logic_1164.all;\n")?;
    // module declaration
    writeln!(out, "entity {} is", entity_name)?;
    out.write(b"    port(\n")?;
    // input and output definitions
    for i in 0..input_len + output_len {
        if i < input_len {
            write!(out, "        i{} : in std_logic", i)?;
        } else {
            write!(out, "        o{} : out std_logic", i - input_len)?;
        }
        if i + 1 < input_len + output_len {
            out.write(b";")?;
        }
        out.write(b"\n")?;
    }
    out.write(b"    );\n")?;
    writeln!(out, "end {};", entity_name)?;
    // architecture definition
    writeln!(out, "architecture {} of {} is", arch_name, entity_name)?;
    // wires definitions
    for i in 0..circuit.gates.len() {
        let wi = input_len + i;
        if wire_out_map.contains_key(&(wi, false)) {
            continue;
        }
        writeln!(out, "    signal i{} : std_logic;", wi)?;
    }
    out.write(b"begin\n")?;
    // gates assignments
    for (i, (g, n)) in circuit.gates.iter().enumerate() {
        let op = match g.func {
            VGateFunc::And => {
                if *n == NegOutput {
                    "nand"
                } else {
                    "and"
                }
            }
            VGateFunc::Or => {
                if *n == NegOutput {
                    "nor"
                } else {
                    "or"
                }
            }
            VGateFunc::Xor => {
                if *n == NegOutput {
                    "xnor"
                } else {
                    "xor"
                }
            }
            _ => {
                panic!("Unexpected!");
            }
        };
        writeln!(
            out,
            "    {} <= {} {} {}{};",
            resolve_name(i + input_len),
            resolve_name(g.i0),
            op,
            if *n == NegInput1 { "not " } else { "" },
            resolve_name(g.i1),
        )?;
    }
    // generate negations
    for ((o, _), (oi, n)) in &wire_out_map {
        if *n {
            writeln!(out, "    o{} <= not {};", *oi, resolve_name(*o))?;
        }
    }
    // generate output duplicates
    for (oi, old_oi) in dup_map {
        writeln!(out, "    o{} <= o{};", oi, old_oi)?;
    }
    writeln!(out, "end {};", arch_name)?;
    Ok(())
}
