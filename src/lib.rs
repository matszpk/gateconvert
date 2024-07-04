#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VNegs {
    NoNegs,
    NegInput1, // second input in gate
    NegOutput,
}

pub mod aiger;
pub mod blif;
pub mod btor2;
pub mod cnf;
pub mod vbinopcircuit;
pub mod vcircuit;
pub mod verilog;
pub mod vhdl;

pub fn map_to_string<T: ToString>(map: &[Option<T>]) -> String {
    let mut out = String::new();
    for t in map {
        if let Some(t) = t {
            out += &t.to_string();
        } else {
            out += "-";
        }
        out.push('\n');
    }
    out
}
