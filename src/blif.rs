use crate::AssignEntry;
use gatesim::*;

use crate::blif_pla::*;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

pub fn to_blif(
    circuit: &Circuit<usize>,
    state_len: usize,
    clock_num: usize,
    model_name: &str,
    out: impl Write,
) -> io::Result<()> {
    let input_len = circuit.input_len();
    let output_len = circuit.outputs().len();
    assert!(state_len + clock_num <= input_len);
    assert!(state_len <= output_len);

    let mut out = BufWriter::new(out);
    let mut wire_out_map = BTreeMap::new();
    let mut dup_map = vec![];
    for (oi, (o, n)) in circuit.outputs().iter().enumerate() {
        if let Some((old_oi, _)) = wire_out_map.get(&(*o, *n)) {
            // resolve duplicate
            dup_map.push((oi, *old_oi));
        } else {
            wire_out_map.insert((*o, *n), (oi, *n));
        }
    }
    writeln!(out, ".model {}", model_name)?;
    for i in 0..state_len {
        writeln!(out, ".inputs i{}", i)?;
    }
    for i in state_len..state_len + clock_num {
        writeln!(out, ".clocks i{}", i)?;
    }
    for i in state_len + clock_num..input_len {
        writeln!(out, ".inputs i{}", i)?;
    }
    for i in 0..output_len {
        writeln!(out, ".outputs o{}", i)?;
    }
    for i in 0..state_len {
        writeln!(out, ".latch o{0} i{0}", i)?;
    }
    let resolve_name = |i| {
        if let Some((oi, _)) = wire_out_map.get(&(i, false)) {
            format!("o{}", oi)
        } else {
            format!("i{}", i)
        }
    };
    for (i, g) in circuit.gates().iter().enumerate() {
        writeln!(
            out,
            ".names {} {} {}",
            resolve_name(g.i0),
            resolve_name(g.i1),
            resolve_name(i + input_len)
        )?;
        let pla_tbl = match g.func {
            GateFunc::And => b"11 1\n".as_slice(),
            GateFunc::Nor => b"00 1\n".as_slice(),
            GateFunc::Nimpl => b"10 1\n".as_slice(),
            GateFunc::Xor => b"10 1\n01 1\n".as_slice(),
        };
        out.write(pla_tbl)?;
    }
    // generate negations
    for ((o, _), (oi, n)) in &wire_out_map {
        if *n {
            write!(out, ".names {} o{}\n0 1\n", resolve_name(*o), *oi)?;
        }
    }
    // generate output duplicates
    for (oi, old_oi) in dup_map {
        write!(out, ".names o{} o{}\n1 1\n", old_oi, oi)?;
    }
    out.write(b".end\n")?;
    Ok(())
}

// HINT to optimize PLA tables: if number 0 or 1 (not -) in PLA is greater than (2**inputs)/4
// then try to optimize table by circuit DB and XOR-table.
// if lines are: 'xxxxx1xx0xxxx' and 'xxxxx0xx1xxxx' then use XOR.

// Read lines. Concatenate lines, remove comments and trim lines.
struct BLIFTokensReader<R: Read> {
    br: BufReader<R>,
    line_no: usize,
    last: Option<(usize, Vec<String>)>,
    prev: bool,
}

impl<R: Read> BLIFTokensReader<R> {
    fn new(r: R) -> Self {
        Self {
            br: BufReader::new(r),
            line_no: 1,
            last: None,
            prev: false,
        }
    }

    fn unread_tokens(&mut self) {
        assert!(self.last.is_some());
        self.prev = true;
    }

    // returns line number and tokens
    fn read_tokens(&mut self) -> io::Result<Option<(usize, Vec<String>)>> {
        if self.prev {
            self.prev = false;
            if let Some(l) = self.last.take() {
                return Ok(Some(l));
            } else {
                panic!("No previous result!");
            }
        }
        let mut line = String::new();
        let mut current_line_no;
        loop {
            current_line_no = self.line_no;
            let mut empty = true;
            while self.br.read_line(&mut line)? != 0 {
                empty = false;
                self.line_no += 1;
                // remove line delimiter
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                if let Some(p) = line.bytes().position(|x| x == b'#') {
                    // remove comment
                    line.truncate(p);
                    break;
                } else if line.ends_with('\\') {
                    line.pop(); // remove '\\'
                } else {
                    break;
                }
            }
            // trim line
            if empty {
                // if empty - end of file
                return Ok(None);
            }
            line = line.trim().to_string();
            if !line.is_empty() {
                // if line is not empty
                break;
            }
        }
        self.last = Some((
            current_line_no,
            line.split_whitespace().map(|x| x.to_string()).collect(),
        ));
        Ok(self.last.clone())
    }
}

// error
#[derive(thiserror::Error, Debug)]
enum BLIFError {
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),
    #[error("{0}:{1}: Expected .model")]
    NoModel(String, usize),
    #[error("{0}:{1}: Expected model name")]
    NoModelName(String, usize),
    #[error("{0}:{1}: Expected .end")]
    NoEnd(String, usize),
    #[error("{0}:{1}: Model declarations in model commands")]
    ModelDeclsInCommands(String, usize),
    #[error("{0}:{1}: Name {2} of model already used")]
    ModelNameUsed(String, usize, String),
    #[error("{0}:{1}: Model with name {2} is undefned")]
    UnknownModel(String, usize, String),
    #[error("{0}:{1}: Parameters to model {2} doesn't match")]
    ModelParamMatch(String, usize, String),
    #[error("{0}:{1}: Too few parameters")]
    TooFewParameters(String, usize),
    #[error("{0}:{1}: Unsupported latch input and output")]
    UnsupportedLatch(String, usize),
    #[error("{0}:{1}: Unsupported External Don't Care")]
    UnsupportedEXDC(String, usize),
    #[error("{0}:{1}: Unsupported FSM definition")]
    UnsupportedFSM(String, usize),
    #[error("{0}:{1}: Unsupported library gate")]
    UnsupportedGate(String, usize),
}

// structures of BLIF

#[derive(Clone, Debug)]
struct Gate<'a> {
    params: Vec<String>,
    output: String,
    circuit: &'a TableCircuit,
}

#[derive(Clone, Debug)]
struct Subcircuit {
    model: String,
    mappings: Vec<String>,
    // data for error handling
    filename: String,
    line_no: usize,
}

// Circuit mapping - values are circuit input or circuit output
// if values >= circuit.input_len then is circuit output index: value - circuit.input_len.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CircuitMapping {
    Input(usize),
    Output(usize),
    Latch(usize, usize),
    Clock(usize),
}

#[derive(Clone, Debug)]
struct Model<'a> {
    inputs: Vec<String>,
    outputs: Vec<String>,
    latches: Vec<(String, String)>,
    clocks: Vec<String>,
    gates: Vec<Gate<'a>>,
    subcircuits: Vec<Subcircuit>,
    // circuit: format:
    // first element - table circuit - same circuit,
    // second element - circuit mapping: in form:
    //     value - (name of wire, mapping to circuit)
    circuit: Option<(TableCircuit, Vec<(String, CircuitMapping)>)>,
}

#[derive(Clone, Debug)]
struct MappingKey {
    model: String,
    wire: String,
}

pub fn blif_assign_map_to_string(map: &[(MappingKey, AssignEntry)]) -> String {
    let mut out = String::new();
    for (k, t) in map {
        out += &k.model;
        out.push(':');
        out += &k.wire;
        out.push(' ');
        out += &t.to_string();
        out.push('\n');
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GateCacheKey {
    var_num: usize,
    cells: Vec<PLACell>,
    set_value: bool,
}

type GateCache = HashMap<GateCacheKey, TableCircuit>;
type ModelMap<'a> = HashMap<String, Model<'a>>;

fn gen_model_circuit<'a>(model_name: String, model_map: &mut ModelMap<'a>) {
    let model = model_map.get(&model_name).unwrap();
    // all subcircuit must be resolved and they must have generated circuits.
    assert!(model
        .subcircuits
        .iter()
        .all(|sc| model_map.get(&sc.model).unwrap().circuit.is_some()));
}

fn resolve_model<'a>(top: String, model_map: &mut ModelMap<'a>) {}

fn parse_model<'a, R: Read>(
    filename: &str,
    reader: &mut BLIFTokensReader<R>,
    circuit_cache: &mut CircuitCache,
    gate_cache: &'a mut GateCache,
    model_map: &mut ModelMap<'a>,
) -> Result<(), BLIFError> {
    // get model name
    let mut model_name = String::new();
    while let Some((line_no, line)) = reader.read_tokens()? {
        if line[0] == ".model" {
            if let Some(name) = line.get(1) {
                model_name = name.clone();
                break;
            } else {
                return Err(BLIFError::NoModelName(filename.to_string(), line_no));
            }
        } else if line[0] == ".exdc" {
            return Err(BLIFError::UnsupportedEXDC(filename.to_string(), line_no));
        } else {
            eprintln!(
                "Warning: {}:{}: Unknown directive {}",
                filename, line_no, line[0]
            );
        }
    }

    let mut model = Model {
        inputs: vec![],
        outputs: vec![],
        latches: vec![],
        clocks: vec![],
        gates: vec![],
        subcircuits: vec![],
        circuit: None,
    };
    let mut model_input_set = HashSet::new();
    let mut model_output_set = HashSet::new();
    let mut after_model_decls = false;
    let mut all_names = HashSet::new();
    while let Some((line_no, line)) = reader.read_tokens()? {
        match line[0].as_str() {
            ".names" => {
                // gate
                after_model_decls = true;
                reader.unread_tokens(); // undo last read
                if line.len() < 2 {
                    return Err(BLIFError::TooFewParameters(filename.to_string(), line_no));
                }
            }
            ".inputs" => {
                if after_model_decls {
                    return Err(BLIFError::ModelDeclsInCommands(
                        filename.to_string(),
                        line_no,
                    ));
                }
                model.inputs.extend(line[1..].iter().cloned());
                model_input_set.extend(line[1..].iter().cloned());
                all_names.extend(line[1..].iter().cloned());
            }
            ".outputs" => {
                if after_model_decls {
                    return Err(BLIFError::ModelDeclsInCommands(
                        filename.to_string(),
                        line_no,
                    ));
                }
                model.outputs.extend(line[1..].iter().cloned());
                model_output_set.extend(line[1..].iter().cloned());
                all_names.extend(line[1..].iter().cloned());
            }
            ".clocks" => {
                if after_model_decls {
                    return Err(BLIFError::ModelDeclsInCommands(
                        filename.to_string(),
                        line_no,
                    ));
                }
                model.clocks.extend(line[1..].iter().cloned());
                all_names.extend(line[1..].iter().cloned());
            }
            ".latch" => {
                after_model_decls = true;
                if line.len() < 3 {
                    return Err(BLIFError::TooFewParameters(filename.to_string(), line_no));
                }
                if !model_output_set.contains(&line[1]) {
                    return Err(BLIFError::UnsupportedLatch(filename.to_string(), line_no));
                }
                if !model_input_set.contains(&line[2]) {
                    return Err(BLIFError::UnsupportedLatch(filename.to_string(), line_no));
                }
                model.latches.push((line[1].clone(), line[2].clone()));
            }
            ".subckt" => {
                after_model_decls = true;
            }
            ".start_kiss" => {
                after_model_decls = true;
                return Err(BLIFError::UnsupportedFSM(filename.to_string(), line_no));
            }
            ".gate" | ".mlatch" => {
                after_model_decls = true;
                return Err(BLIFError::UnsupportedGate(filename.to_string(), line_no));
            }
            _ => {
                after_model_decls = true;
                eprintln!(
                    "Warning: {}:{}: Unknown directive {}",
                    filename, line_no, line[0]
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blif_reader_helper(text: &str) -> Vec<(usize, Vec<String>)> {
        let mut reader = BLIFTokensReader::new(text.as_bytes());
        let mut lines = vec![];
        while let Ok(Some(line)) = reader.read_tokens() {
            lines.push(line);
        }
        lines
    }

    fn tokens_to_vectors<'a>(
        lines: impl IntoIterator<Item = (usize, impl IntoIterator<Item = &'a str>)>,
    ) -> Vec<(usize, Vec<String>)> {
        lines
            .into_iter()
            .map(|(ln, tokens)| {
                (
                    ln,
                    tokens
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    #[test]
    fn test_blif_tokens_reader() {
        assert_eq!(
            tokens_to_vectors([(1, vec!["ala", "bum", "bm"]), (2, vec!["beta", "xx"])]),
            blif_reader_helper(" ala bum bm  \n  beta xx \n")
        );
        assert_eq!(
            tokens_to_vectors([(1, vec!["ala", "bum", "bm"]), (2, vec!["beta", "xx"])]),
            blif_reader_helper(" ala bum bm  \n  beta xx \n\n#  \n")
        );
        assert_eq!(
            tokens_to_vectors([(1, vec!["ala", "bum"]), (2, vec!["beta", "xx"])]),
            blif_reader_helper(" ala bum # bm  \n  beta xx # yyy \n")
        );
        assert_eq!(
            tokens_to_vectors([(1, vec!["ala", "bum", "bm", "beta", "xx"])]),
            blif_reader_helper(" ala bum bm  \\\n  beta xx \n")
        );
        assert_eq!(
            tokens_to_vectors([(1, vec!["ala", "bum", "bm"]), (2, vec!["beta", "xx"])]),
            blif_reader_helper(" ala bum bm # comment \\\n  beta xx \n")
        );
        assert_eq!(
            tokens_to_vectors([
                (1, vec![".model", "simple"]),
                (2, vec![".inputs", "a", "b"]),
                (3, vec![".outputs", "c"]),
                (4, vec![".names", "a", "b", "c"]),
                (5, vec!["11", "1"]),
                (6, vec![".end"]),
                (9, vec![".names", "a", "b", "c"]),
                (11, vec!["11", "1"])
            ]),
            blif_reader_helper(
                r##".model simple
.inputs a b
.outputs c
.names a b c      # .names described later
11 1
.end

# unnamed model
.names a b \
c   # ‘\’ here only to demonstrate its use
11 1
"##
            )
        );
    }
}
