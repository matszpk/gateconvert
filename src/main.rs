//mod verilog;
use gateconvert::cnf;

use clap::{Parser, Subcommand};
use gatesim::*;

use std::fs::{self, File};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Parser)]
struct FromCNF {
    #[clap(help = "Set CNF filename")]
    cnf: PathBuf,
}

#[derive(Parser)]
struct ToCNF {
    #[clap(help = "Set circuit filename")]
    circuit: PathBuf,
    cnf: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Convert from CNF")]
    FromCNF(FromCNF),
    #[clap(about = "Convert to CNF")]
    ToCNF(ToCNF),
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::FromCNF(from_cnf) => {}
        Commands::ToCNF(to_cnf) => {
            let circuit =
                Circuit::<usize>::from_str(&fs::read_to_string(to_cnf.circuit).unwrap()).unwrap();
            let mut file = File::create(to_cnf.cnf).unwrap();
            cnf::to_cnf(&circuit, &mut file).unwrap();
        }
    }
}
