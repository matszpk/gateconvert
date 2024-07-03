//mod verilog;
use gateconvert::*;

use clap::{Parser, Subcommand};
use gatesim::*;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
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

#[derive(Parser)]
struct FromAIGER {
    #[clap(help = "Set AIGER filename")]
    aiger: PathBuf,
    #[clap(help = "Set output circuit filename")]
    circuit: PathBuf,
    #[clap(help = "Set output AIGER map filename")]
    aiger_map: Option<PathBuf>,
    #[clap(short, long, help = "Set binary mode if no proper file extension")]
    binary: bool,
}

#[derive(Parser)]
struct ToAIGER {
    #[clap(help = "Set circuit filename")]
    circuit: PathBuf,
    #[clap(help = "Set output AIGER filename")]
    aiger: PathBuf,
    #[clap(help = "Set state length (number of latches)")]
    state_len: Option<usize>,
    #[clap(short, long, help = "Set binary mode if no proper file extension")]
    binary: bool,
}

#[derive(Parser)]
struct ToBTOR2 {
    #[clap(help = "Set circuit filename")]
    circuit: PathBuf,
    #[clap(help = "Set output BTOR2 filename")]
    btor2: PathBuf,
    #[clap(help = "Set state length (number of latches)")]
    state_len: Option<usize>,
}

#[derive(Parser)]
struct ToBLIF {
    #[clap(help = "Set circuit filename")]
    circuit: PathBuf,
    #[clap(help = "Set output BLIF filename")]
    blif: PathBuf,
    #[clap(help = "Set state length (number of latches)")]
    state_len: Option<usize>,
    #[clap(help = "Set model name")]
    model_name: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Convert from CNF")]
    FromCNF(FromCNF),
    #[clap(about = "Convert to CNF")]
    ToCNF(ToCNF),
    #[clap(about = "Convert from AIGER")]
    FromAIGER(FromAIGER),
    #[clap(about = "Convert to AIGER")]
    ToAIGER(ToAIGER),
    #[clap(about = "Convert to BTOR2")]
    ToBTOR2(ToBTOR2),
    #[clap(about = "Convert to BLIF")]
    ToBLIF(ToBLIF),
}

fn aiger_file_ext_binary_mode(name: impl AsRef<Path>, binary: bool) -> bool {
    if let Some(ext) = name.as_ref().extension() {
        if ext == "aag" {
            false
        } else if ext == "aig" {
            true
        } else {
            binary
        }
    } else {
        binary
    }
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
        Commands::FromAIGER(from_aig) => {
            let binary = aiger_file_ext_binary_mode(&from_aig.aiger, from_aig.binary);
            let (circuit, map) = {
                let mut cnf_file = File::open(from_aig.aiger).unwrap();
                aiger::from_aiger(&mut cnf_file, binary).unwrap()
            };
            fs::write(
                from_aig.circuit,
                FmtLiner::new(&circuit, 4, 8).to_string().as_bytes(),
            )
            .unwrap();
            if let Some(map_name) = from_aig.aiger_map {
                fs::write(map_name, aiger::aiger_map_to_string(&map)).unwrap();
            }
        }
        Commands::ToAIGER(to_aig) => {
            let binary = aiger_file_ext_binary_mode(&to_aig.aiger, to_aig.binary);
            let circuit =
                Circuit::<usize>::from_str(&fs::read_to_string(to_aig.circuit).unwrap()).unwrap();
            let mut file = File::create(to_aig.aiger).unwrap();
            aiger::to_aiger(
                &circuit,
                to_aig.state_len.unwrap_or_default(),
                &mut file,
                binary,
            )
            .unwrap();
        }
        Commands::ToBTOR2(to_btor2) => {
            let circuit =
                Circuit::<usize>::from_str(&fs::read_to_string(to_btor2.circuit).unwrap()).unwrap();
            let mut file = File::create(to_btor2.btor2).unwrap();
            btor2::to_btor2(&circuit, to_btor2.state_len.unwrap_or_default(), &mut file).unwrap();
        }
        Commands::ToBLIF(to_blif) => {
            let circuit =
                Circuit::<usize>::from_str(&fs::read_to_string(to_blif.circuit).unwrap()).unwrap();
            let mut file = File::create(to_blif.blif).unwrap();
            blif::to_blif(
                &circuit,
                to_blif.state_len.unwrap_or_default(),
                &to_blif.model_name.unwrap_or("top".to_string()),
                &mut file,
            )
            .unwrap();
        }
    }
}
