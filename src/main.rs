//mod verilog;
use gateconvert::*;

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
    #[clap(help = "Set output circuit filename")]
    circuit: PathBuf,
    #[clap(help = "Set output assign map filename")]
    assign_map: Option<PathBuf>,
}

#[derive(Parser)]
struct ToCNF {
    #[clap(help = "Set circuit filename")]
    circuit: PathBuf,
    #[clap(help = "Set output CNF filename")]
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
        Commands::FromCNF(from_cnf) => {
            let (circuit, map) = {
                let mut cnf_file = File::open(from_cnf.cnf).unwrap();
                cnf::from_cnf(&mut cnf_file).unwrap()
            };
            fs::write(
                from_cnf.circuit,
                FmtLiner::new(&circuit, 4, 8).to_string().as_bytes(),
            )
            .unwrap();
            if let Some(map_name) = from_cnf.assign_map {
                fs::write(map_name, map_to_string(&map)).unwrap();
            }
        }
        Commands::ToCNF(to_cnf) => {
            let circuit =
                Circuit::<usize>::from_str(&fs::read_to_string(to_cnf.circuit).unwrap()).unwrap();
            let mut file = File::create(to_cnf.cnf).unwrap();
            cnf::to_cnf(&circuit, &mut file).unwrap();
        }
    }
}
