//mod verilog;
use gateconvert::vcircuit;
use gateconvert::VNegs;

use clap::{Parser, Subcommand};

use std::path::PathBuf;

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
        Commands::FromCNF(from_cnf) => {
        }
        Commands::ToCNF(to_cnf) => {
        }
    }
}
