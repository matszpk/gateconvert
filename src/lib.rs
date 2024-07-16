#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VNegs {
    NoNegs,
    NegInput1, // second input in gate
    NegOutput,
}

pub mod aiger;
pub mod blif;
mod blif_pla;
pub mod btor2;
pub mod cnf;
pub mod vbinopcircuit;
pub mod vcircuit;
pub mod verilog;
pub mod vhdl;
mod xor_table;

use std::fmt::{self, Debug, Display};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignEntry {
    NoMap,            // no mapping
    Value(bool),      // boolean value
    Var(usize, bool), // (circuit wire index, negation)
}

impl Display for AssignEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssignEntry::NoMap => write!(f, "-"),
            AssignEntry::Value(v) => write!(f, "{}", v),
            AssignEntry::Var(v, n) => write!(f, "{}{}", if *n { "!" } else { "" }, v),
        }
    }
}

pub fn assign_map_to_string(map: &[(usize, AssignEntry)]) -> String {
    let mut out = String::new();
    for (i, t) in map {
        out += &i.to_string();
        out.push(' ');
        out += &t.to_string();
        out.push('\n');
    }
    out
}

pub fn string_assign_map_to_string(map: &[(String, AssignEntry)]) -> String {
    let mut out = String::new();
    for (k, t) in map {
        out += &k;
        out.push(' ');
        out += &t.to_string();
        out.push('\n');
    }
    out
}
