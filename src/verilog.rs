use crate::gatesim::*;

use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};

use crate::vbinopcircuit::*;
use crate::vcircuit::VGateFunc;
use crate::VNegs::*;

/// Converts circuit to Verilog language source.
///
/// Function writes Gate circuit logic in Verilog language to `out`. `circuit` is circuit
/// to convert. `module_name` is name of top module. `optimize_negs` determines whether
/// optimize negations while conversion (if true) or not (if false).
pub fn to_verilog(
    circuit: Circuit<usize>,
    module_name: &str,
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
    // module declaration
    writeln!(out, "module {} (", module_name)?;
    for i in 0..input_len + output_len {
        if i < input_len {
            write!(out, "    i{}", i)?;
        } else {
            write!(out, "    o{}", i - input_len)?;
        }
        if i + 1 < input_len + output_len {
            out.write(b",")?;
        } else {
            out.write(b");")?;
        }
        out.write(b"\n")?;
    }
    if input_len + output_len == 0 {
        out.write(b"    );\n")?;
    }
    // input and output definitions
    for i in 0..input_len {
        writeln!(out, "    input i{};", i)?;
    }
    for i in 0..output_len {
        writeln!(out, "    output o{};", i)?;
    }
    // wires definitions
    for i in 0..circuit.gates.len() {
        let wi = input_len + i;
        if wire_out_map.contains_key(&(wi, false)) {
            continue;
        }
        writeln!(out, "    wire i{};", wi)?;
    }
    // gates assignments
    for (i, (g, n)) in circuit.gates.iter().enumerate() {
        let op = match g.func {
            VGateFunc::And => "&",
            VGateFunc::Or => "|",
            VGateFunc::Xor => "^",
            _ => {
                panic!("Unexpected!");
            }
        };
        writeln!(
            out,
            "    assign {} = {}({} {} {}{});",
            resolve_name(i + input_len),
            if *n == NegOutput { "~" } else { "" },
            resolve_name(g.i0),
            op,
            if *n == NegInput1 { "~" } else { "" },
            resolve_name(g.i1)
        )?;
    }
    // generate negations
    for ((o, _), (oi, n)) in &wire_out_map {
        if *n {
            writeln!(out, "    assign o{} = ~{};", *oi, resolve_name(*o))?;
        }
    }
    // generate output duplicates
    for (oi, old_oi) in dup_map {
        writeln!(out, "    assign o{} = o{};", oi, old_oi)?;
    }
    out.write(b"endmodule\n")?;
    Ok(())
}
