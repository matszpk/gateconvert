// lib.rs - main code

#![cfg_attr(docsrs, feature(doc_cfg))]
//! The library allows to easily convert Gate circuit from/to one of few foreign formats.
//! This library is used by `gateconvert_exec` program that allow conversion by command line
//! interface.
//!
//! A conversion to foreign logic format writes result data into output (by `Write` trait).
//! A conversion from foreign logic format returns Gate circuit object and sometimes
//! additional mapping. Any functions that make conversion returns Result to allow handle
//! various errors.

/// Utility to mark negation
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VNegs {
    /// If no negations.
    NoNegs,
    /// Second (starting from 0) argument will be negated.
    // second input in gate
    NegInput1,
    /// Output will be negated.
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

pub use gategen;
pub use gateutil;
pub use gateutil::gatesim;

use std::fmt::{self, Debug, Display};

/// Generate output string from mapping. The `T` must be convertible to string.
///
/// This function simplify generation of map file. Mapping in form:
/// index - original variable index, value - index of circuit inputs.
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

/// Entry of assignment for mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignEntry {
    /// No mapping.
    NoMap,
    /// Boolean value set for variable.
    Value(bool),
    /// Circuit wire index and its negation.
    Var(usize, bool),
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

/// Generate output string from mapping.
///
/// This function simplify generation of map file. Mapping in form:
/// key - original variable index, value - asisgnment to circuit.
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

/// Generate output string from mapping.
///
/// This function simplify generation of map file. Mapping in form:
/// key - original variable name, value - asisgnment to circuit.
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
