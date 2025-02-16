#![cfg_attr(docsrs, feature(doc_cfg))]
//! Module to conversion between Gate circuit and DIMACS CNF (Conjuctive Normal Form) format.

use crate::gatesim::*;
use cnfgen::writer::{CNFError, CNFWriter};
use flussab_cnf::cnf;
use std::io::{Read, Write};

fn to_cnf_int(circuit: &Circuit<usize>, out: &mut impl Write) -> Result<(), CNFError> {
    use cnfgen::boolvar::*;
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

/// Converts Gate circuit to DIMACS CNF (Conjuctive Normal Form) format.
///
/// `circuit` is circuit to convert. `out` is an output stream.
pub fn to_cnf(circuit: &Circuit<usize>, mut out: impl Write) -> Result<(), CNFError> {
    use cnfgen::boolvar::*;
    callsys(|| to_cnf_int(circuit, &mut out))
}

fn from_cnf_int(
    parser: &mut cnf::Parser<isize>,
) -> Result<(Circuit<usize>, Vec<Option<usize>>), flussab_cnf::ParseError> {
    use gategen::boolvar::*;
    let hdr = parser.header().unwrap();
    let vars = (0..hdr.var_count)
        .map(|_| BoolVarSys::var())
        .collect::<Vec<_>>();
    let mut clauses = BoolVarSys::from(true);
    loop {
        match parser.next_clause() {
            Ok(Some(clause)) => {
                let clause = clause.into_iter().fold(BoolVarSys::from(false), |a, l| {
                    let l = if *l > 0 {
                        vars[usize::try_from(*l).unwrap() - 1].clone()
                    } else if *l < 0 {
                        !&vars[usize::try_from(-*l).unwrap() - 1]
                    } else {
                        panic!("Unexpected 0");
                    };
                    a | l
                });
                clauses &= clause;
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(clauses.to_translated_circuit_with_map(vars.into_iter()))
}

/// Converts DIMACS CNF (Conjuctive Normal Form) logic to Gate circuit.
///
/// `input` is stream with logic in DIMACS CNF format. Function returns Gate circuit with
/// its mapping. Mapping in form: index - original variable in CNF logic (starts from 0),
/// value - circuit wire index.
pub fn from_cnf(
    input: impl Read,
) -> Result<(Circuit<usize>, Vec<Option<usize>>), flussab_cnf::ParseError> {
    use gategen::boolvar::*;
    let mut parser = cnf::Parser::<isize>::from_read(input, cnf::Config::default())?;
    callsys(|| from_cnf_int(&mut parser))
}
