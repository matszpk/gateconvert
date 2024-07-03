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
    for i in 0..output_len {
        writeln!(out, ".outputs o{}", i)?;
    }
    for i in 0..state_len {
        writeln!(out, ".latch o{0} i{0}", i)?;
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
    for (i, (o, n)) in circuit.outputs().iter().enumerate() {
        if *n {
            write!(out, ".names i{} o{}\n0 1\n", o, i)?;
        } else {
            write!(out, ".names i{} o{}\n1 1\n", o, i)?;
        }
    }
    out.write(b".end\n")?;
    Ok(())
}
