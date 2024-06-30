use cnfgen::writer::{CNFError, CNFWriter};
use gatesim::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn to_cnf_int(circuit: &Circuit<usize>, out: &mut impl Write) -> Result<(), CNFError> {
    use cnfgen::boolvar::*;
    use cnfgen::dynintvar::*;
    assert_eq!(circuit.outputs().len(), 1);
    let mut out_exprs = (0..circuit.input_len())
        .map(|_| BoolVarSys::var())
        .collect::<Vec<_>>();
    for g in circuit.gates() {
        let i0 = usize::try_from(g.i0).unwrap();
        let i1 = usize::try_from(g.i1).unwrap();
        out_exprs.push(match g.func {
            GateFunc::And => &out_exprs[i0] & &out_exprs[i1],
            GateFunc::Nor => !&out_exprs[i0] & !&out_exprs[i1],
            GateFunc::Nimpl => &out_exprs[i0] & !&out_exprs[i1],
            GateFunc::Xor => &out_exprs[i0] ^ &out_exprs[i1],
        });
    }
    let formula = circuit
        .outputs()
        .into_iter()
        .map(|(i, n)| {
            let out = out_exprs[usize::try_from(*i).unwrap()].clone();
            if *n {
                !out
            } else {
                out
            }
        })
        .next()
        .unwrap();

    formula.write(&mut CNFWriter::new(out))
}

pub fn to_cnf(circuit: &Circuit<usize>, out: &mut impl Write) -> Result<(), CNFError> {
    use cnfgen::boolvar::*;
    callsys(|| to_cnf_int(circuit, out))
}
