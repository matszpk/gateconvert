use gatesim::*;

use std::io::{BufWriter, Write};

use crate::vcircuit::*;

pub fn to_blif(
    circuit: &Circuit<usize>,
    state_len: usize,
    model_name: &str,
    out: &mut impl Write,
) -> Result<(), std::io::Error> {
    let input_len = circuit.input_len();
    let output_len = circuit.outputs().len();
    assert!(state_len <= input_len);
    assert!(state_len <= output_len);

    let mut out = BufWriter::new(out);
    writeln!(out, ".model {}", model_name)?;
    for i in 0..input_len {
        writeln!(out, ".inputs i{}", i)?;
    }
    for (i, (o, n)) in circuit.outputs().iter().enumerate() {
        writeln!(out, ".outputs {}{}", if *n { "n" } else { "i" }, o)?;
    }
    for (i, (o, n)) in circuit.outputs()[0..state_len].iter().enumerate() {
        writeln!(out, ".latch {}{} i{}", if *n { "n" } else { "i" }, o, i)?;
    }
    for (i, g) in circuit.gates().iter().enumerate() {
        writeln!(out, ".names i{} i{} i{}", g.i0, g.i1, i + input_len)?;
        let pla_tbl = match g.func {
            GateFunc::And => b"11 1\n".as_slice(),
            GateFunc::Nor => b"00 1\n".as_slice(),
            GateFunc::Nimpl => b"10 1\n".as_slice(),
            GateFunc::Xor => b"10 1\n01 1\n".as_slice(),
        };
        out.write(pla_tbl)?;
    }
    for (o, n) in circuit.outputs() {
        if *n {
            write!(out, ".names i{0} n{0}\n0 1\n", o)?;
        }
    }
    out.write(b".end\n")?;
    Ok(())
}
