use crate::gatesim::*;
use crate::AssignEntry;
use gategen::boolvar::*;
use gategen::dynintvar::*;
use gateutil::{reverse_trans, translate_inputs, translate_outputs};

use crate::blif_pla::*;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

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
    for i in state_len + clock_num..input_len {
        writeln!(out, ".inputs i{}", i)?;
    }
    for i in 0..output_len {
        writeln!(out, ".outputs o{}", i)?;
    }
    for i in state_len..state_len + clock_num {
        writeln!(out, ".clock i{}", i)?;
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
pub enum BLIFError {
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),
    #[error("No models in BLIF")]
    NoModels,
    #[error("Too big depth of '.search'")]
    TooBigSearchDepth,
    #[error("{0}:{1}: Expected .model")]
    NoModel(String, usize),
    #[error("{0}: Expected .end")]
    NoModelEnd(String),
    #[error("{0}:{1}: Expected model name")]
    NoModelName(String, usize),
    #[error("Model {0} without outputs")]
    ModelWithoutOutputs(String),
    #[error("{0}:{1}: Model declarations in model commands")]
    ModelDeclsInCommands(String, usize),
    #[error("Name {0} of model already used")]
    ModelNameUsed(String),
    #[error("{0}:{1}: Model with name {2} is undefined")]
    UnknownModel(String, usize, String),
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
    #[error("{0}:{1}: Bad gate PLA table")]
    BadGateTable(String, usize),
    #[error("{0}:{1}: Bad subcircuit {2} mapping")]
    BadSubcircuitMapping(String, usize, String),
    #[error("{0}:{1}: Duplicate {3} in subcircuit {2} mapping")]
    DuplicateInSubcircuitMapping(String, usize, String, String),
    #[error("{0}:{1}: Already defined as output {2}")]
    AlreadyDefinedAsOutput(String, usize, String),
    #[error("{0}:{1}: Model input duplicate {2}")]
    ModelInputDuplicate(String, usize, String),
    #[error("{0}:{1}: Model clock duplicate {2}")]
    ModelClockDuplicate(String, usize, String),
    #[error("{0}:{1}: Model output duplicate {2}")]
    ModelOutputDuplicate(String, usize, String),
    #[error("{0}:{1}: Defined as model input {2}")]
    DefinedAsModelInput(String, usize, String),
    #[error("{0}:{1}: Defined as model clock {2}")]
    DefinedAsModelClock(String, usize, String),
    #[error("{0}:{1}: Model input defined as input and clock {2}")]
    ModelInputAndClockBoth(String, usize, String),
    #[error("{0}:{1}: Defined as model output {2}")]
    DefinedAsModelOutput(String, usize, String),
    #[error("Wire {1} in model {0} is undefined")]
    UndefinedWire(String, String),
    #[error("Already defined as output in {0}:{1}")]
    AlreadyDefinedAsOutput2(String, String),
    #[error("{0}:{1}: Model have latches")]
    ModelHaveLatches(String, usize),
    #[error("{0}:{1}: Model have clocks")]
    ModelHaveClocks(String, usize),
    #[error("Cycle in model {0} caused by {1}")]
    CycleInModel(String, String),
    #[error("Cycle in model hierarchy caused by {0}")]
    CycleInModelHierarchy(String),
}

// structures of BLIF

#[derive(Clone, Debug, PartialEq, Eq)]
struct Gate {
    params: Vec<String>,
    output: String,
    circuit: TableCircuit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Subcircuit {
    model: String,
    mappings: Vec<(String, String)>,
    // data for error handling
    filename: String,
    line_no: usize,
}

// Circuit mapping - values are circuit input or circuit output
// if values >= circuit.input_len then is circuit output index: value - circuit.input_len.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CircuitMapping {
    NoMapping,
    Value(bool),  // value
    Input(bool),  // if state
    Output(bool), // if state
    Clock,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Model {
    inputs: Vec<String>,
    outputs: Vec<String>,
    latches: Vec<(String, String)>,
    clocks: Vec<String>,
    gates: Vec<Gate>,
    subcircuits: Vec<Subcircuit>,
    // circuit: format:
    // first element - table circuit - same circuit,
    // second element - circuit mapping: in form:
    //     index - in order: [model inputs, model clocks, model outputs]
    //     value - (name of model, name of wire, mapping to circuit)
    circuit: Option<(Circuit<usize>, Vec<CircuitMapping>)>,
}

impl Model {
    fn top_mapping(self) -> (Circuit<usize>, Vec<(String, AssignEntry)>) {
        let (circuit, mapping) = self.circuit.as_ref().unwrap();
        let circ_input_len = circuit.input_len();
        let circ_outputs = circuit.outputs();
        let model_input_len = self.inputs.len();
        let model_clock_len = self.clocks.len();
        let model_output_len = self.outputs.len();
        // circuit_mapping_indexes: index - mapping index,
        //    value - for input or clock is circuit input index or for output is circuit output
        let circuit_mapping_names = self
            .inputs
            .iter()
            .cloned()
            .chain(self.clocks.iter().cloned())
            .chain(self.outputs.iter().cloned())
            .collect::<Vec<_>>();
        let circuit_mapping_indexes = {
            let mut circuit_mapping_indexes =
                vec![None; model_input_len + model_clock_len + model_output_len];
            let mut input_count = 0;
            for (i, cm) in mapping[0..model_input_len + model_clock_len]
                .iter()
                .enumerate()
            {
                if !matches!(cm, CircuitMapping::NoMapping | CircuitMapping::Value(_)) {
                    circuit_mapping_indexes[i] = Some(input_count);
                    input_count += 1;
                }
            }
            let mut output_count = 0;
            for (i, cm) in mapping
                .iter()
                .enumerate()
                .skip(model_input_len + model_clock_len)
            {
                if !matches!(cm, CircuitMapping::NoMapping | CircuitMapping::Value(_)) {
                    circuit_mapping_indexes[i] = Some(output_count);
                    output_count += 1;
                }
            }
            circuit_mapping_indexes
        };
        let model_input_map = HashMap::<String, usize>::from_iter(
            self.inputs.iter().enumerate().map(|(i, x)| (x.clone(), i)),
        );
        let model_output_map = HashMap::<String, usize>::from_iter(
            self.outputs.iter().enumerate().map(|(i, x)| (x.clone(), i)),
        );
        let state_mapping = self
            .latches
            .iter()
            .map(|(model_out, model_in)| {
                (
                    model_input_len + model_clock_len + model_output_map[model_out],
                    model_input_map[model_in],
                )
            })
            .collect::<Vec<_>>();
        // circuit_input_trans_rev: index - new index, value - old index
        // first input states
        let circuit_input_trans_rev = state_mapping
            .iter()
            .filter_map(|(_, model_input_idx)| circuit_mapping_indexes[*model_input_idx])
            .chain(
                // clocks
                circuit_mapping_indexes[model_input_len..model_input_len + model_clock_len]
                    .iter()
                    .copied()
                    .filter_map(|x| x),
            )
            .chain(
                // inputs (not states)
                circuit_mapping_indexes[0..model_input_len]
                    .iter()
                    .zip(mapping[0..model_input_len].iter())
                    .filter_map(|(idx, cm)| {
                        if matches!(cm, CircuitMapping::Input(false)) {
                            *idx
                        } else {
                            None
                        }
                    }),
            )
            .collect::<Vec<_>>();
        // circuit_output_trans_rev: index - new index, value - old index
        let circuit_output_trans_rev = state_mapping
            .iter()
            .filter_map(|(model_output_idx, _)| circuit_mapping_indexes[*model_output_idx])
            .chain(
                // outputs (not states)
                circuit_mapping_indexes[model_input_len + model_clock_len..]
                    .iter()
                    .zip(mapping[model_input_len + model_clock_len..].iter())
                    .filter_map(|(idx, cm)| {
                        if matches!(cm, CircuitMapping::Output(false)) {
                            *idx
                        } else {
                            None
                        }
                    }),
            )
            .collect::<Vec<_>>();
        let circuit_input_trans = reverse_trans(circuit_input_trans_rev);
        // mapping
        let assign_mapping = mapping
            .into_iter()
            .zip(circuit_mapping_indexes.into_iter())
            .zip(circuit_mapping_names.into_iter())
            .map(|((cm, ci), cn)| {
                match cm {
                    CircuitMapping::NoMapping => (cn.clone(), AssignEntry::NoMap),
                    CircuitMapping::Value(v) => (cn.clone(), AssignEntry::Value(*v)),
                    CircuitMapping::Input(_) | CircuitMapping::Clock =>
                    // translate old circuit input into new circuit input
                    {
                        (
                            cn.clone(),
                            AssignEntry::Var(circuit_input_trans[ci.unwrap()], false),
                        )
                    }
                    CircuitMapping::Output(_) => {
                        let ci = ci.unwrap();
                        // translate old circuit input into new circuit input
                        let cwi = if circ_outputs[ci].0 < circ_input_len {
                            circuit_input_trans[circ_outputs[ci].0]
                        } else {
                            circ_outputs[ci].0
                        };
                        (cn.clone(), AssignEntry::Var(cwi, circ_outputs[ci].1))
                    }
                }
            })
            .collect::<Vec<_>>();
        // translating circuit
        let circuit = translate_inputs(circuit.clone(), &circuit_input_trans);
        let circuit = translate_outputs(circuit, &circuit_output_trans_rev);
        (circuit, assign_mapping)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GateCacheKey {
    var_num: usize,
    cells: Vec<PLACell>,
    set_value: bool,
}

impl GateCacheKey {
    fn new(var_num: usize, table: &[(Vec<PLACell>, bool, usize)], set_value: bool) -> Self {
        Self {
            var_num,
            cells: table
                .iter()
                .map(|(e, _, _)| e)
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
            set_value,
        }
    }
}

type GateCache = HashMap<GateCacheKey, TableCircuit>;
type ModelMap = HashMap<String, Model>;

fn parse_model<R: Read>(
    filename: &str,
    reader: &mut BLIFTokensReader<R>,
    circuit_cache: &mut CircuitCache,
    gate_cache: &mut GateCache,
) -> Result<(String, Model), BLIFError> {
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
            return Err(BLIFError::NoModel(filename.to_string(), line_no));
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
    let mut model_clock_set = HashSet::new();
    let mut model_output_set = HashSet::new();
    let mut after_model_decls = false;
    let mut all_outputs = HashSet::new();
    let mut have_end = false;
    while let Some((line_no, line)) = reader.read_tokens()? {
        match line[0].as_str() {
            ".names" => {
                // gate
                after_model_decls = true;
                if line.len() < 2 {
                    return Err(BLIFError::TooFewParameters(filename.to_string(), line_no));
                }
                let mut pla_table = vec![];
                let mut last_set_value = true;
                let var_num = line.len() - 2;

                // check whether output is not in inputs of model
                if model_input_set.contains(line.last().unwrap()) {
                    return Err(BLIFError::DefinedAsModelInput(
                        filename.to_string(),
                        line_no,
                        line.last().unwrap().clone(),
                    ));
                }
                // check whether output is not in clocks of model
                if model_clock_set.contains(line.last().unwrap()) {
                    return Err(BLIFError::DefinedAsModelClock(
                        filename.to_string(),
                        line_no,
                        line.last().unwrap().clone(),
                    ));
                }
                if !all_outputs.insert(line.last().unwrap().clone()) {
                    // if not already newly inserted
                    return Err(BLIFError::AlreadyDefinedAsOutput(
                        filename.to_string(),
                        line_no,
                        line.last().unwrap().clone(),
                    ));
                }

                while let Some((line_no, line)) = reader.read_tokens()? {
                    if let Some((entry, set_value, line_no)) =
                        pla_entry_from_tokens(var_num, line_no, &line)
                    {
                        pla_table.push((entry, set_value, line_no));
                        last_set_value = set_value;
                    } else {
                        if !line[0].starts_with('.') {
                            return Err(BLIFError::BadGateTable(filename.to_string(), line_no));
                        }
                        break;
                    }
                }

                pla_table.sort_by(|(entry1, _, line_no1), (entry2, _, line_no2)| {
                    (entry1, *line_no1).cmp(&(entry2, *line_no2))
                });
                // remove all entries with different set value
                pla_table.retain(|(_, cur_set_value, _)| last_set_value == *cur_set_value);
                pla_table.dedup_by(|(entry1, _, _), (entry2, _, _)| entry1 == entry2);

                // use gate cache only if pla_table have 64 literals and
                // non-zero variables - avoid problem with PLA with same set value.
                let tbl_circuit = if var_num != 0 && var_num * pla_table.len() < 64 {
                    // create key - to find gate circuit.
                    let gc_key = GateCacheKey::new(var_num, pla_table.as_slice(), last_set_value);
                    // check wether is in gate cache
                    if let Some(tbl_circuit) = gate_cache.get(&gc_key) {
                        tbl_circuit.clone()
                    } else {
                        let tbl_circuit = gen_pla_circuit_with_two_methods(
                            circuit_cache,
                            var_num,
                            last_set_value,
                            &pla_table,
                        );
                        gate_cache.insert(gc_key, tbl_circuit.clone());
                        tbl_circuit
                    }
                } else {
                    gen_pla_circuit_with_two_methods(
                        circuit_cache,
                        var_num,
                        last_set_value,
                        &pla_table,
                    )
                };
                model.gates.push(Gate {
                    params: line[1..var_num + 1].to_vec(),
                    output: line.last().unwrap().clone(),
                    circuit: tbl_circuit,
                });
                reader.unread_tokens(); // undo last read
            }
            ".input" | ".inputs" => {
                if after_model_decls {
                    return Err(BLIFError::ModelDeclsInCommands(
                        filename.to_string(),
                        line_no,
                    ));
                }
                for input in &line[1..] {
                    if model_clock_set.contains(input) {
                        return Err(BLIFError::ModelInputAndClockBoth(
                            filename.to_string(),
                            line_no,
                            input.clone(),
                        ));
                    }
                }
                for input in &line[1..] {
                    if model_input_set.contains(input) {
                        return Err(BLIFError::ModelInputDuplicate(
                            filename.to_string(),
                            line_no,
                            input.clone(),
                        ));
                    }
                    model_input_set.insert(input.clone());
                }
                model.inputs.extend(line[1..].iter().cloned());
            }
            ".output" | ".outputs" => {
                if after_model_decls {
                    return Err(BLIFError::ModelDeclsInCommands(
                        filename.to_string(),
                        line_no,
                    ));
                }
                for output in &line[1..] {
                    if model_output_set.contains(output) {
                        return Err(BLIFError::ModelOutputDuplicate(
                            filename.to_string(),
                            line_no,
                            output.clone(),
                        ));
                    }
                    model_output_set.insert(output.clone());
                }
                model.outputs.extend(line[1..].iter().cloned());
            }
            ".clock" => {
                if after_model_decls {
                    return Err(BLIFError::ModelDeclsInCommands(
                        filename.to_string(),
                        line_no,
                    ));
                }
                for clock in &line[1..] {
                    if model_clock_set.contains(clock) {
                        return Err(BLIFError::ModelClockDuplicate(
                            filename.to_string(),
                            line_no,
                            clock.clone(),
                        ));
                    }
                    model_clock_set.insert(clock.clone());
                }
                for clock in &line[1..] {
                    if model_input_set.contains(clock) {
                        return Err(BLIFError::ModelInputAndClockBoth(
                            filename.to_string(),
                            line_no,
                            clock.clone(),
                        ));
                    }
                }
                model.clocks.extend(line[1..].iter().cloned());
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
                if line.len() < 3 {
                    return Err(BLIFError::TooFewParameters(filename.to_string(), line_no));
                }
                // check whether all parameters in form 'A=C': if in parameter
                // '=' is not last and not first.
                if !line[2..].iter().all(|s| {
                    s.find('=')
                        .map(|p| p != 0 && p != s.len() - 1)
                        .unwrap_or(false)
                }) {
                    return Err(BLIFError::BadSubcircuitMapping(
                        filename.to_string(),
                        line_no,
                        line[1].clone(),
                    ));
                }
                let mut sc_wire_set = HashSet::new();
                let mut mappings = vec![];
                for s in &line[2..] {
                    let (s1, s2) = s.split_at(s.find('=').unwrap());
                    if !sc_wire_set.insert(s1) {
                        return Err(BLIFError::DuplicateInSubcircuitMapping(
                            filename.to_string(),
                            line_no,
                            line[1].clone(),
                            s1.to_string(),
                        ));
                    }
                    mappings.push((s1.to_string(), s2[1..].to_string()));
                }

                model.subcircuits.push(Subcircuit {
                    model: line[1].clone(),
                    mappings,
                    // data for error handling
                    filename: filename.to_string(),
                    line_no,
                });
            }
            ".start_kiss" => {
                return Err(BLIFError::UnsupportedFSM(filename.to_string(), line_no));
            }
            ".gate" | ".mlatch" => {
                return Err(BLIFError::UnsupportedGate(filename.to_string(), line_no));
            }
            ".end" => {
                have_end = true;
                break;
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
    if !have_end {
        return Err(BLIFError::NoModelEnd(filename.to_string()));
    }
    if model.outputs.is_empty() {
        return Err(BLIFError::ModelWithoutOutputs(model_name.clone()));
    }
    // next phase - checking graph of gates and subcircuits - check whether graph have cycles.
    // next phase will be done while resolving graph of models.
    Ok((model_name, model))
}

fn gen_model_circuit(model_name: &str, model_map: &mut ModelMap) -> Result<(), BLIFError> {
    let model = model_map.get(model_name).unwrap();
    // all subcircuit must be resolved and they must have generated circuits.
    assert!(model
        .subcircuits
        .iter()
        .all(|sc| model_map.get(&sc.model).unwrap().circuit.is_some()));
    #[derive(Clone, Debug)]
    enum InputNode {
        ModelInput(usize),
        ModelClock(usize),
        Gate,
        Subcircuit,
    }
    #[derive(Clone, Debug)]
    enum OutputNode {
        Gate(usize),              // gate index
        Subcircuit(usize, usize), // subcircuit index, output index
    }
    #[derive(Clone)]
    enum Node {
        ModelInput(usize),
        ModelClock(usize),
        Gate(usize),
        Subcircuit(usize, usize),
    }
    #[derive(Clone)]
    struct SubcircuitMapping {
        inputs: Vec<Option<String>>,
        outputs: Vec<Option<String>>,
    }
    #[derive(Clone)]
    struct StackEntry {
        name: String,
        node: Node,
        way: usize,
    }

    let model_output_set = HashSet::<String>::from_iter(model.outputs.iter().cloned());
    let mut wire_in_outs = HashMap::<String, (Vec<InputNode>, Option<OutputNode>)>::new();
    for (i, input) in model.inputs.iter().enumerate() {
        if let Some((wi, _)) = wire_in_outs.get_mut(input) {
            wi.push(InputNode::ModelInput(i));
        } else {
            wire_in_outs.insert(input.clone(), (vec![InputNode::ModelInput(i)], None));
        }
    }
    for (i, clock) in model.clocks.iter().enumerate() {
        if let Some((wi, _)) = wire_in_outs.get_mut(clock) {
            wi.push(InputNode::ModelClock(i));
        } else {
            wire_in_outs.insert(clock.clone(), (vec![InputNode::ModelClock(i)], None));
        }
    }
    // resolve gate inputs and outputs
    for (i, g) in model.gates.iter().enumerate() {
        for gin in &g.params {
            if let Some((wi, _)) = wire_in_outs.get_mut(gin) {
                wi.push(InputNode::Gate);
            } else {
                wire_in_outs.insert(gin.clone(), (vec![InputNode::Gate], None));
            }
        }
        if let Some((_, wo)) = wire_in_outs.get_mut(&g.output) {
            if wo.is_some() {
                return Err(BLIFError::AlreadyDefinedAsOutput2(
                    model_name.to_string(),
                    g.output.clone(),
                ));
            } else {
                *wo = Some(OutputNode::Gate(i));
            }
        } else {
            wire_in_outs.insert(g.output.clone(), (vec![], Some(OutputNode::Gate(i))));
        }
    }

    let mut sc_mappings = Vec::<SubcircuitMapping>::new();
    for (i, sc) in model.subcircuits.iter().enumerate() {
        if let Some(subc_model) = model_map.get(&sc.model) {
            if subc_model.circuit.as_ref().unwrap().1.iter().any(|c| {
                matches!(
                    c,
                    CircuitMapping::Input(true) | CircuitMapping::Output(true)
                )
            }) {
                return Err(BLIFError::ModelHaveLatches(
                    sc.filename.to_string(),
                    sc.line_no,
                ));
            }
            if subc_model
                .circuit
                .as_ref()
                .unwrap()
                .1
                .iter()
                .any(|c| matches!(c, CircuitMapping::Clock))
            {
                return Err(BLIFError::ModelHaveClocks(
                    sc.filename.to_string(),
                    sc.line_no,
                ));
            }
            let sc_input_map = HashMap::<String, usize>::from_iter(
                subc_model
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (x.clone(), i)),
            );
            let sc_output_map = HashMap::<String, usize>::from_iter(
                subc_model
                    .outputs
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (x.clone(), i)),
            );
            let mut sc_mapping = SubcircuitMapping {
                inputs: vec![None; subc_model.inputs.len()],
                outputs: vec![None; subc_model.outputs.len()],
            };
            // model_wire - subcircuit model wire, wire - current model wire
            for (model_wire, wire) in &sc.mappings {
                if let Some(input_index) = sc_input_map.get(model_wire) {
                    sc_mapping.inputs[*input_index] = Some(wire.clone());
                }
                if let Some(output_index) = sc_output_map.get(model_wire) {
                    sc_mapping.outputs[*output_index] = Some(wire.clone());
                }
            }
            for sci in &sc_mapping.inputs {
                if let Some(sci) = sci {
                    if model_output_set.contains(sci) {
                        return Err(BLIFError::DefinedAsModelOutput(
                            sc.filename.to_string(),
                            sc.line_no,
                            sci.clone(),
                        ));
                    }
                }
            }
            // checking sc mapping: for circuit input and output
            for sci in &sc_mapping.outputs {
                if let Some(sci) = sci {
                    if let Some((wi, _)) = wire_in_outs.get(sci) {
                        // check whether output is not in inputs of model
                        if wi
                            .iter()
                            .any(|input| matches!(input, InputNode::ModelInput(_)))
                        {
                            return Err(BLIFError::DefinedAsModelInput(
                                sc.filename.to_string(),
                                sc.line_no,
                                sci.clone(),
                            ));
                        }
                        // check whether output is not in clocks of model
                        if wi
                            .iter()
                            .any(|input| matches!(input, InputNode::ModelClock(_)))
                        {
                            return Err(BLIFError::DefinedAsModelClock(
                                sc.filename.to_string(),
                                sc.line_no,
                                sci.clone(),
                            ));
                        }
                    }
                }
            }
            // register connections
            for scin in &sc_mapping.inputs {
                if let Some(scin) = scin.as_ref() {
                    if let Some((wi, _)) = wire_in_outs.get_mut(scin) {
                        wi.push(InputNode::Subcircuit);
                    } else {
                        wire_in_outs.insert(scin.clone(), (vec![InputNode::Subcircuit], None));
                    }
                }
            }
            for (scouti, scout) in sc_mapping.outputs.iter().enumerate() {
                if let Some(scout) = scout.as_ref() {
                    if let Some((_, wo)) = wire_in_outs.get_mut(scout) {
                        if wo.is_some() {
                            return Err(BLIFError::AlreadyDefinedAsOutput2(
                                model_name.to_string(),
                                scout.clone(),
                            ));
                        } else {
                            *wo = Some(OutputNode::Subcircuit(i, scouti));
                        }
                    } else {
                        wire_in_outs.insert(
                            scout.clone(),
                            (vec![], Some(OutputNode::Subcircuit(i, scouti))),
                        );
                    }
                }
            }
            sc_mappings.push(sc_mapping);
        } else {
            return Err(BLIFError::UnknownModel(
                sc.filename.clone(),
                sc.line_no,
                sc.model.clone(),
            ));
        }
    }
    // check whether name tied to some output
    for (name, (wi, wo)) in &wire_in_outs {
        if wo.is_some() && wi.is_empty() && !model_output_set.contains(name) {
            return Err(BLIFError::UndefinedWire(
                model_name.to_string(),
                name.clone(),
            ));
        }
    }

    // creating circuit
    let (circuit, circuit_mapping) = callsys(|| {
        // boolvar_map - map of expressions to names used in model
        let mut boolvar_map = HashMap::<String, BoolVarSys>::new();
        // visited nodes in graphs
        let mut visited = HashSet::<String>::new();
        // path_visited - to detect cycles
        let mut path_visited = HashSet::<String>::new();
        let mut stack = vec![];
        for outname in &model.outputs {
            if let Some((wi, wo)) = wire_in_outs.get(outname) {
                let node = if let Some(wo) = wo {
                    match wo {
                        OutputNode::Gate(g) => Node::Gate(*g),
                        OutputNode::Subcircuit(sc, sco) => Node::Subcircuit(*sc, *sco),
                    }
                } else if !wi.is_empty() {
                    wi.iter()
                        .filter_map(|x| match x {
                            InputNode::ModelInput(mi) => Some(Node::ModelInput(*mi)),
                            InputNode::ModelClock(mi) => Some(Node::ModelClock(*mi)),
                            _ => None,
                        })
                        .next()
                        .unwrap()
                } else {
                    panic!("Unexpected!");
                };
                stack.push(StackEntry {
                    name: outname.clone(),
                    node,
                    way: 0,
                });
            } else {
                return Err(BLIFError::UndefinedWire(
                    model_name.to_string(),
                    outname.clone(),
                ));
            };

            while !stack.is_empty() {
                let top = stack.last_mut().unwrap();
                let way = top.way;
                let way_num = match top.node {
                    Node::ModelInput(_) | Node::ModelClock(_) => 0,
                    Node::Gate(j) => model.gates[j].params.len(),
                    Node::Subcircuit(j, _) => sc_mappings[j].inputs.len(),
                };
                let name = match top.node {
                    Node::ModelInput(j) => model.inputs[j].clone(),
                    Node::ModelClock(j) => model.clocks[j].clone(),
                    Node::Gate(j) => model.gates[j].output.clone(),
                    Node::Subcircuit(j, k) => {
                        sc_mappings[j].outputs[k].clone().unwrap_or(String::new())
                    }
                };
                if way == 0 {
                    if !path_visited.contains(&top.name) {
                        path_visited.insert(top.name.clone());
                    } else {
                        return Err(BLIFError::CycleInModel(
                            model_name.to_string(),
                            top.name.clone(),
                        ));
                    }
                    if !visited.contains(&top.name) {
                        visited.insert(top.name.clone());
                    } else {
                        path_visited.remove(&top.name);
                        stack.pop();
                        continue;
                    }
                }
                if way < way_num {
                    let child_name = match top.node {
                        Node::Gate(j) => Some(model.gates[j].params[way].clone()),
                        Node::Subcircuit(j, _) => sc_mappings[j].inputs[way].clone(),
                        _ => None,
                    };
                    top.way += 1;
                    if let Some(child_name) = child_name {
                        let node = if let Some((wi, wo)) = wire_in_outs.get(&child_name) {
                            if let Some(wo) = wo {
                                match wo {
                                    OutputNode::Gate(g) => Node::Gate(*g),
                                    OutputNode::Subcircuit(sc, sco) => Node::Subcircuit(*sc, *sco),
                                }
                            } else if !wi.is_empty() {
                                wi.iter()
                                    .filter_map(|x| match x {
                                        InputNode::ModelInput(mi) => Some(Node::ModelInput(*mi)),
                                        InputNode::ModelClock(mi) => Some(Node::ModelClock(*mi)),
                                        _ => None,
                                    })
                                    .next()
                                    .unwrap()
                            } else {
                                panic!("Unexpected!");
                            }
                        } else {
                            return Err(BLIFError::UndefinedWire(
                                model_name.to_string(),
                                top.name.clone(),
                            ));
                        };
                        stack.push(StackEntry {
                            name: child_name.clone(),
                            node,
                            way: 0,
                        });
                    }
                } else {
                    // resolve outputs
                    match top.node {
                        Node::ModelInput(_) | Node::ModelClock(_) => {
                            if !boolvar_map.contains_key(&name) {
                                boolvar_map.insert(name.clone(), BoolVarSys::var());
                            }
                        }
                        Node::Gate(j) => {
                            // add gate's circuit to expressions and resolve gate output.
                            if !boolvar_map.contains_key(&name) {
                                let gate = &model.gates[j];
                                // resolve gate params (inputs)
                                let expr = match &gate.circuit {
                                    TableCircuit::Value(v) => BoolVarSys::from(*v),
                                    TableCircuit::Circuit((circuit, input_map)) => {
                                        BoolVarSys::from_circuit(
                                            circuit.clone(),
                                            input_map.iter().enumerate().filter_map(|(idx, p)| {
                                                if p.is_some() {
                                                    Some(boolvar_map[&gate.params[idx]].clone())
                                                } else {
                                                    None
                                                }
                                            }),
                                        )[0]
                                        .clone()
                                    }
                                };
                                // add output to boolvar_map expression
                                boolvar_map.insert(name.clone(), expr);
                            }
                        }
                        Node::Subcircuit(j, _) => {
                            // generate subcircuit's circuit to expressions
                            // and resolve subcircuit outputs.
                            if !boolvar_map.contains_key(&name) {
                                let sc_mapping = &sc_mappings[j];
                                let subc_model = &model_map[&model.subcircuits[j].model];
                                let circuit_mapping = &subc_model.circuit.as_ref().unwrap().1;
                                let total_input_len = subc_model.inputs.len();
                                // add outputs to expressions
                                let circ_outputs = BoolVarSys::from_circuit(
                                    subc_model.circuit.as_ref().unwrap().0.clone(),
                                    circuit_mapping[0..total_input_len]
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(idx, p)| {
                                            if matches!(
                                                p,
                                                CircuitMapping::Input(_) | CircuitMapping::Clock
                                            ) {
                                                if let Some(scmap_name) = &sc_mapping.inputs[idx] {
                                                    Some(boolvar_map[scmap_name].clone())
                                                } else {
                                                    Some(BoolVarSys::from(false))
                                                }
                                            } else {
                                                Some(BoolVarSys::from(false))
                                            }
                                        }),
                                );
                                let mut out_count = 0;
                                // add subcircuit outputs to boolvar_map expression
                                for (i, c) in circuit_mapping[total_input_len..].iter().enumerate()
                                {
                                    match c {
                                        CircuitMapping::Value(v) => {
                                            if let Some(scname) = sc_mapping.outputs[i].as_ref() {
                                                boolvar_map
                                                    .insert(scname.clone(), BoolVarSys::from(*v));
                                            }
                                        }
                                        CircuitMapping::Output(_) => {
                                            let old_out_count = out_count;
                                            out_count += 1;
                                            if let Some(scname) = sc_mapping.outputs[i].as_ref() {
                                                boolvar_map.insert(
                                                    scname.clone(),
                                                    circ_outputs[old_out_count].clone(),
                                                );
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        }
                    };
                    path_visited.remove(&top.name);
                    stack.pop();
                }
            }
        }
        // generate circuit
        let latch_inputs =
            HashSet::<String>::from_iter(model.latches.iter().map(|(s, _)| s.clone()));
        let latch_outputs =
            HashSet::<String>::from_iter(model.latches.iter().map(|(_, s)| s.clone()));
        let outputs = UDynVarSys::from_iter(model.outputs.iter().map(|s| boolvar_map[s].clone()));
        let (circuit, input_map) = outputs.to_translated_circuit_with_map(
            model
                .inputs
                .iter()
                .filter_map(|s| boolvar_map.get(s).cloned())
                .chain(
                    model
                        .clocks
                        .iter()
                        .filter_map(|s| boolvar_map.get(s).cloned()),
                ),
        );
        // fix input map - because some model inputs can be removed while filtering
        let input_map = {
            let mut input_map_new = vec![None; model.inputs.len() + model.clocks.len()];
            for (newi, (i, _)) in model
                .inputs
                .iter()
                .chain(model.clocks.iter())
                .enumerate()
                .filter(|(_, s)| boolvar_map.contains_key(*s))
                .enumerate()
            {
                input_map_new[i] = input_map[newi];
            }
            input_map_new
        };
        // output mapping
        let circuit_out_mapping = model
            .outputs
            .iter()
            .map(|s| {
                let b = boolvar_map[s].clone();
                if let Some(v) = b.value() {
                    CircuitMapping::Value(v)
                } else {
                    CircuitMapping::Output(latch_inputs.contains(s))
                }
            })
            .collect::<Vec<_>>();
        // input mapping
        let circuit_mapping = model
            .inputs
            .iter()
            .zip(input_map.iter())
            .map(|(x, opti)| {
                if opti.is_some() {
                    CircuitMapping::Input(latch_outputs.contains(x))
                } else {
                    CircuitMapping::NoMapping
                }
            })
            .chain(
                model
                    .clocks
                    .iter()
                    .zip(input_map[model.inputs.len()..].iter())
                    .map(|(_, opti)| {
                        if opti.is_some() {
                            CircuitMapping::Clock
                        } else {
                            CircuitMapping::NoMapping
                        }
                    }),
            )
            .chain(circuit_out_mapping.into_iter())
            .collect::<Vec<_>>();
        Ok((circuit, circuit_mapping))
    })?;
    let model = model_map.get_mut(model_name).unwrap();
    model.circuit = Some((circuit, circuit_mapping));
    Ok(())
}

fn parse_file<P: AsRef<Path> + Debug>(path: P) -> Result<(ModelMap, String), BLIFError> {
    let mut circuit_cache = CircuitCache::new();
    let mut gate_cache = GateCache::new();
    let mut model_map = ModelMap::new();
    struct Stack {
        path: String,
        reader: BLIFTokensReader<File>,
    }
    let mut stack = vec![];
    let mut first_model = None;
    stack.push(Stack {
        path: path.as_ref().to_str().unwrap().to_string(),
        reader: BLIFTokensReader::<File>::new(File::open(path)?),
    });

    'a: while !stack.is_empty() {
        if stack.len() >= 100 {
            return Err(BLIFError::TooBigSearchDepth);
        }
        let top = stack.last_mut().unwrap();
        while let Some((_, line)) = top.reader.read_tokens()? {
            if line[0] == ".search" {
                stack.push(Stack {
                    path: line[1].clone(),
                    reader: BLIFTokensReader::<File>::new(File::open(&line[1])?),
                });
                continue 'a; // to main loop at stack
            } else {
                // undo reading last tokens
                top.reader.unread_tokens();
                let (name, model) = parse_model(
                    &top.path,
                    &mut top.reader,
                    &mut circuit_cache,
                    &mut gate_cache,
                )?;
                if first_model.is_none() {
                    first_model = Some(name.clone());
                }
                if !model_map.contains_key(&name) {
                    model_map.insert(name.clone(), model);
                } else {
                    return Err(BLIFError::ModelNameUsed(name.clone()));
                }
            }
        }
        // if end of current file then pop stack
        stack.pop();
    }

    if let Some(first_model) = first_model {
        Ok((model_map, first_model))
    } else {
        Err(BLIFError::NoModels)
    }
}

fn resolve_model(top_name: &str, model_map: &mut ModelMap) -> Result<(), BLIFError> {
    struct StackEntry {
        name: String,
        way: usize,
    }
    let mut visited = HashSet::<String>::new();
    // path_visited - to detect cycles
    let mut path_visited = HashSet::<String>::new();
    let mut stack = vec![];
    stack.push(StackEntry {
        name: top_name.to_string(),
        way: 0,
    });
    while !stack.is_empty() {
        let top = stack.last_mut().unwrap();
        let way = top.way;
        let model = model_map.get(&top.name).unwrap();
        if way == 0 {
            if !path_visited.contains(&top.name) {
                path_visited.insert(top.name.clone());
            } else {
                return Err(BLIFError::CycleInModelHierarchy(top.name.clone()));
            }
            if !visited.contains(&top.name) {
                visited.insert(top.name.clone());
            } else {
                path_visited.remove(&top.name);
                stack.pop();
                continue;
            }
        }
        if way < model.subcircuits.len() {
            top.way += 1;
            if !model_map.contains_key(&model.subcircuits[way].model) {
                return Err(BLIFError::UnknownModel(
                    model.subcircuits[way].filename.clone(),
                    model.subcircuits[way].line_no,
                    model.subcircuits[way].model.clone(),
                ));
            }
            stack.push(StackEntry {
                name: model.subcircuits[way].model.clone(),
                way: 0,
            });
        } else {
            gen_model_circuit(&top.name, model_map)?;
            path_visited.remove(&top.name);
            stack.pop();
        }
    }
    Ok(())
}

pub fn from_blif<P: AsRef<Path> + Debug>(
    path: P,
) -> Result<(Circuit<usize>, Vec<(String, AssignEntry)>), BLIFError> {
    let (mut model_map, model_name) = parse_file(path)?;
    resolve_model(&model_name, &mut model_map)?;
    Ok(model_map.remove(&model_name).unwrap().top_mapping())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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

    fn parse_model_helper(text: &str) -> Result<(String, Model), String> {
        let mut circuit_cache = CircuitCache::new();
        let mut gate_cache = GateCache::new();
        let mut bytes = BLIFTokensReader::new(text.as_bytes());
        parse_model("top.blif", &mut bytes, &mut circuit_cache, &mut gate_cache)
            .map_err(|e| e.to_string())
    }

    fn strs_to_vec_string<'a>(iter: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        iter.into_iter().map(|s| s.to_string()).collect()
    }
    fn strs2_to_vec_string<'a>(
        iter: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Vec<(String, String)> {
        iter.into_iter()
            .map(|(s1, s2)| (s1.to_string(), s2.to_string()))
            .collect()
    }

    #[test]
    fn test_parse_model() {
        assert_eq!(
            Ok((
                "simple".to_string(),
                Model {
                    inputs: vec![],
                    outputs: strs_to_vec_string(["x", "y", "z"]),
                    latches: vec![],
                    clocks: vec![],
                    gates: vec![
                        Gate {
                            params: vec![],
                            output: "x".to_string(),
                            circuit: TableCircuit::Value(false),
                        },
                        Gate {
                            params: vec![],
                            output: "y".to_string(),
                            circuit: TableCircuit::Value(true),
                        },
                        Gate {
                            params: vec![],
                            output: "z".to_string(),
                            circuit: TableCircuit::Value(false),
                        }
                    ],
                    subcircuits: vec![],
                    circuit: None,
                }
            )),
            parse_model_helper(
                r##".model simple
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##
            )
        );
        assert_eq!(
            Ok((
                "simple2".to_string(),
                Model {
                    inputs: strs_to_vec_string(["a", "b", "c", "d", "e", "f", "i"]),
                    outputs: strs_to_vec_string(["x", "y", "z", "w", "t", "t1", "x1", "z1"]),
                    latches: strs2_to_vec_string([("x", "a"), ("y", "c")]),
                    clocks: strs_to_vec_string(["g", "h"]),
                    gates: vec![
                        Gate {
                            params: strs_to_vec_string(["a", "b"]),
                            output: "x".to_string(),
                            circuit: TableCircuit::Value(true),
                        },
                        Gate {
                            params: strs_to_vec_string(["c", "d"]),
                            output: "y".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 nor(0,1):0}(2)").unwrap(),
                                vec![Some(0), Some(1)],
                            )),
                        },
                        Gate {
                            params: strs_to_vec_string(["e", "f"]),
                            output: "z".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 nimpl(0,1):0n}(2)").unwrap(),
                                vec![Some(0), Some(1)],
                            )),
                        },
                        Gate {
                            params: strs_to_vec_string(["g", "h"]),
                            output: "w".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 nor(0,1):0n}(2)").unwrap(),
                                vec![Some(0), Some(1)],
                            )),
                        },
                        Gate {
                            params: strs_to_vec_string(["a", "y", "i"]),
                            output: "t".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 nimpl(0,1):0n}(2)").unwrap(),
                                vec![None, Some(0), Some(1)],
                            )),
                        },
                        Gate {
                            params: strs_to_vec_string(["h", "y", "z", "c"]),
                            output: "t1".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str(
                                    r##"{0 1 2 3 and(1,2) nor(1,2)
nor(0,4) xor(3,4) nimpl(7,5) nor(6,8):0}(4)"##
                                )
                                .unwrap(),
                                vec![Some(0), Some(1), Some(2), Some(3)],
                            )),
                        },
                    ],
                    subcircuits: vec![Subcircuit {
                        model: "calc0".to_string(),
                        mappings: strs2_to_vec_string([
                            ("a", "b"),
                            ("c", "d"),
                            ("s", "x1"),
                            ("s1", "z1")
                        ]),
                        filename: "top.blif".to_string(),
                        line_no: 33,
                    }],
                    circuit: None,
                }
            )),
            parse_model_helper(
                r##".model simple2
.inputs a b
.inputs c d
.inputs e f
.clock g h
.inputs i
.outputs x y z w
.outputs t t1 x1 z1
.latch x a
.latch y c
.names a b x
00 1
10 1
01 1
11 1
.names c d y
10 0
10 1
01 0
11 0
.names e f z
0- 1
-1 1
.names g h w
10 1
01 1
11 1
.names a y i t
100 1
--1 0
--1 1
00- 1
.subckt calc0 a=b c=d s=x1 s1=z1
.names h y z c t1
1-00 1
-111 1
10-0 1
100- 1
.end
# model to ignore
.model simple
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##
            )
        );
        // error handling
        assert_eq!(
            Err("top.blif:5: Model declarations in model commands".to_string()),
            parse_model_helper(
                r##".model simple
.outputs x y
.outputs z
.names x
.outputs t
.names y
1
.names z
0
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:2: Expected .model".to_string()),
            parse_model_helper(
                r##"
.names y
1
.names z
0
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:1: Expected model name".to_string()),
            parse_model_helper(
                r##".model
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:1: Unsupported External Don't Care".to_string()),
            parse_model_helper(
                r##".exdc
.names y
1
.names z
0
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:3: Model input defined as input and clock a".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.clock a
.outputs x
.names a b x
11 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:3: Model input defined as input and clock a".to_string()),
            parse_model_helper(
                r##".model test1
.clock a
.inputs a b
.outputs x
.names a b x
11 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Unsupported latch input and output".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.latch a b
.names a b x
11 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Unsupported latch input and output".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.latch a x
.names a b x
11 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Unsupported latch input and output".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.latch x x
.names a b x
11 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Too few parameters".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.latch x
.names a b x
11 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:5: Bad gate PLA table".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.names a b x
110 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:5: Bad gate PLA table".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.names a b x
0 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:5: Bad gate PLA table".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.names a b x
x1 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:5: Bad gate PLA table".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.names a b x
0x 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:5: Bad gate PLA table".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.names a b x
10 -
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Defined as model input b".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x y
.names a x b
10 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:5: Defined as model clock b".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a
.clock b
.outputs x y
.names a x b
10 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:6: Already defined as output x".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.names a b x
10 1
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Too few parameters".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.names
10 1
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Too few parameters".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.subckt
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Bad subcircuit complex1 mapping".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.subckt complex1 a=c b = a
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Bad subcircuit complex1 mapping".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.subckt complex1 a=c =b
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Duplicate a in subcircuit complex1 mapping".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.subckt complex1 a=c a=b
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:3: Model input duplicate d".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.inputs d
.outputs x
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif:4: Model output duplicate x".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b c d
.outputs x
.outputs x
.names c d x
01 1
.end
"##
            )
        );
        assert_eq!(
            Err("top.blif: Expected .end".to_string()),
            parse_model_helper(
                r##".model test1
.inputs a b
.outputs x
.names a b x
01 1
"##
            )
        );
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CircuitData {
        inputs: Vec<String>,
        clocks: Vec<String>,
        outputs: Vec<String>,
        latches: Vec<(String, String)>,
        circuit: (Circuit<usize>, Vec<CircuitMapping>),
    }

    impl From<Model> for CircuitData {
        fn from(m: Model) -> Self {
            Self {
                inputs: m.inputs,
                clocks: m.clocks,
                outputs: m.outputs,
                latches: m.latches,
                circuit: m.circuit.unwrap(),
            }
        }
    }

    fn gen_model_circuit_helper(text: &str, model_num: usize) -> Result<CircuitData, String> {
        println!("ModelStart:");
        let mut circuit_cache = CircuitCache::new();
        let mut gate_cache = GateCache::new();
        let mut model_map = ModelMap::new();
        let mut bytes = BLIFTokensReader::new(text.as_bytes());
        let (main_model_name, main_model) =
            parse_model("top.blif", &mut bytes, &mut circuit_cache, &mut gate_cache)
                .map_err(|e| e.to_string())
                .unwrap();
        model_map.insert(main_model_name.clone(), main_model);
        for _ in 0..model_num {
            let (model_name, model) =
                parse_model("top.blif", &mut bytes, &mut circuit_cache, &mut gate_cache)
                    .map_err(|e| e.to_string())
                    .unwrap();
            model_map.insert(model_name.clone(), model);
            gen_model_circuit(&model_name, &mut model_map).map_err(|e| e.to_string())?;
        }
        gen_model_circuit(&main_model_name, &mut model_map).map_err(|e| e.to_string())?;
        for g in &model_map[&main_model_name].gates {
            println!("ModelGate: {:?}", g);
        }
        Ok(CircuitData::from(model_map[&main_model_name].clone()))
    }

    #[test]
    fn test_gen_model_circuit() {
        use gatesim::Gate;
        use CircuitMapping::*;
        assert_eq!(
            Ok(CircuitData {
                inputs: vec![],
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y", "z"]),
                latches: vec![],
                circuit: (
                    Circuit::new(0, [], []).unwrap(),
                    vec![Value(false), Value(true), Value(false)]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##,
                0
            )
        );
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c"]),
                clocks: strs_to_vec_string(["d"]),
                outputs: strs_to_vec_string(["a", "b", "c", "d"]),
                latches: vec![],
                circuit: (
                    Circuit::new(4, [], [(0, false), (1, false), (2, false), (3, false)]).unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Clock,
                        Output(false),
                        Output(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.clock d
.outputs a b c d
.end
"##,
                0
            )
        );
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y", "z"]),
                latches: vec![],
                circuit: (
                    Circuit::new(
                        3,
                        [
                            Gate::new_and(0, 1),
                            Gate::new_nor(1, 2),
                            Gate::new_nimpl(0, 2),
                        ],
                        [(3, false), (4, true), (5, true)]
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs x y z
.names a b x
11 1
.names b c y
1- 1
-1 1
.names a c z
0- 1
-1 1
.end
"##,
                0
            )
        );
        assert_eq!(
            Err("Wire z in model simple is undefined".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs x y
.names a b x
11 1
.names b c y
1- 1
-1 1
.names a c z
0- 1
-1 1
.end
"##,
                0
            )
        );
        // graph of connection
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3
and(0,1) nor(2,3) nor(4,5):0n nimpl(0,2) nimpl(6,7):1n}(4)"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d
.outputs x y
.names a b t0
11 1
.names c d t1
00 1
.names a c t2
10 1
.names t0 t1 x
1- 1
-1 1
.names x t2 y
1- 1
-1 1
.end
"##,
                0
            )
        );
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4
and(0,1) and(5,2) xor(0,2) nor(6,7) nor(2,3) nimpl(9,4) nor(7,10) and(8,11):0n nor(8,11):1}(5)
"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y
.names a b c t0
111 1
.names a c e t1
100 1
010 1
101 1
011 1
.names c d e t2
000 1
.names t0 t1 u0
1- 1
-1 1
.names t1 t2 u1
1- 1
-1 1
.names u0 u1 x
1- 1
-1 1
.names u0 u1 y
11 1
.end
"##,
                0
            )
        );
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4
and(0,1) and(5,2) xor(0,2) xor(7, 4) nor(6,8) nor(2,3) nimpl(10,4) nor(8,11)
and(9,12):0n nor(9,12):1}(5)
"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y
.names a b c t0
111 1
.names a c e t1
100 1
010 1
001 1
111 1
.names c d e t2
000 1
.names t0 t1 u0
1- 1
-1 1
.names t1 t2 u1
1- 1
-1 1
.names u0 u1 x
1- 1
-1 1
.names u0 u1 y
11 1
.end
"##,
                0
            )
        );
        // test various passing inputs into gate
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x"]),
                latches: vec![],
                circuit: (
                    Circuit::new(2, [Gate::new_and(0, 1)], [(2, false)]).unwrap(),
                    vec![
                        NoMapping,
                        Input(false),
                        NoMapping,
                        Input(false),
                        NoMapping,
                        Output(false)
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x
.names a b c d e x
-1-1- 1
.end
"##,
                0
            )
        );
        // subcircuits
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4
and(0,1) and(5,2) xor(0,2) xor(7, 4) nor(6,8) nor(2,3) nimpl(10,4) nor(8,11)
and(9,12):0n nor(9,12):1}(5)
"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y
.subckt model0 a0=a a1=b a2=c x0=t0
.subckt model1 a0=a a1=c a2=e x0=t1
.subckt model2 a0=c a1=d a2=e x0=t2
.names t0 t1 u0
1- 1
-1 1
.names t1 t2 u1
1- 1
-1 1
.names u0 u1 x
1- 1
-1 1
.names u0 u1 y
11 1
.end
.model model0
.inputs a0 a1 a2
.outputs x0
.names a0 a1 a2 x0
111 1
.end
.model model1
.inputs a0 a1 a2
.outputs x0
.names a0 a1 a2 x0
100 1
010 1
001 1
111 1
.end
.model model2
.inputs a0 a1 a2
.outputs x0
.names a0 a1 a2 x0
000 1
.end
"##,
                3
            )
        );
        // subcircuits
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4
and(0,1) and(5,2) xor(0,2) xor(7, 4) nor(6,8) nor(2,3) nimpl(10,4) nor(8,11)
and(9,12):0n nor(9,12):1}(5)
"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y
.subckt model0 a0=a a1=b a2=c a3=d a4=e x0=t0 x1=t1 x2=t2
.names t0 t1 u0
1- 1
-1 1
.names t1 t2 u1
1- 1
-1 1
.names u0 u1 x
1- 1
-1 1
.names u0 u1 y
11 1
.end
.model model0
.inputs a0 a1 a2 a3 a4
.outputs x0 x1 x2
.names a0 a1 a2 x0
111 1
.names a0 a2 a4 x1
100 1
010 1
001 1
111 1
.names a2 a3 a4 x2
000 1
.end
"##,
                1
            )
        );
        // subcircuits 2
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4
and(0,1) and(5,2) xor(0,2) xor(7, 4) nor(6,8) nor(2,3) nimpl(10,4) nor(8,11)
and(9,12):0n nor(9,12):1}(5)
"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y
.subckt model0 a0=a a1=b a2=c a3=d a4=e x0=t0 x1=t1 x2=t2
.names t0 t1 u0
1- 1
-1 1
.names t1 t2 u1
1- 1
-1 1
.names u0 u1 x
1- 1
-1 1
.names u0 u1 y
11 1
.end
.model model0
.inputs a0 a1 a2 a3 a4
.outputs x0 x1 x2
.names a0 a1 a2 x0
111 1
.names a0 a2 a4 x1
100 1
010 1
001 1
111 1
.names a2 a3 a4 x2
000 1
.end
"##,
                1
            )
        );
        // subcircuits 3
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y", "z", "w", "or"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4 xor(0,1) xor(5,2) and(0,1) xor(0,1) and(8,2) nor(7,9)
xor(10,3) xor(11,4) and(6,12):0 nimpl(3,10) xor(10,3) nimpl(4,15) nor(14,16) nimpl(10,17):1
xor(6,12):2n xor(10,17):3n nor(13,18) and(19,20) and(21,22):4n}(5)
"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                        Output(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y z w or
.subckt fulladder a=a b=b cin=c s=s0 cout=c0
.subckt fulladder a=c0 b=d cin=e s=s1 cout=c1
.names s0 s1 x
10 1
.names c0 c1 y
01 1
.names s0 s1 z
10 1
01 1
.names c0 c1 w
00 1
11 1
.names x y z w or
0000 0
.end
.model fulladder
.inputs a b cin
.outputs s cout
.names a b k
10 1
01 1
.names k cin s
10 1
01 1
.names a b cin cout
11- 1
1-1 1
-11 1
.end
"##,
                1
            )
        );
        // subcircuits 4: unused inputs and outputs
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str("{0 1 2 3 4 xor(1,2) nimpl(5,0):0 nimpl(4,3):1}(5)").unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e
.outputs x y
.subckt mpx4 m=a n=b p=c y0=x
.subckt mpx4 m=d o=e y1=y
.end
.model mpx4
.inputs m n o p
.outputs y0 y1
.names m n o p y0
0100 1
0001 1
.names m n o p y1
0010 1
0001 1
.end
"##,
                1
            )
        );
        // subcircuits 5
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e", "f", "g", "h"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y", "z", "w"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4 5 6 7 nor(0,2) xor(1,3) and(8,9) nor(2,4)
xor(3,5) and(11,12) nor(4,6) xor(5,7) and(14,15) nor(13,16) and(10,17):0 nor(0,1) xor(2,3)
and(19,20) nor(4,5) xor(6,7) and(22,23) and(21,24) nor(2,3) xor(4,5) and(26,27) nimpl(25,28):1
and(10,13) and(16,30):2 nor(21,28) and(24,32):3}(8)"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d e f g h
.outputs x y z w
.subckt mpx4 m=a n=b o=c p=d y0=t0 y1=t3
.subckt mpx4 m=c n=d o=e p=f y0=t1 y1=t4
.subckt mpx4 m=e n=f o=g p=h y0=t2 y1=t5
.names t0 t1 t2 x
100 1
.names t3 t4 t5 y
101 1
.names t0 t1 t2 z
111 1
.names t3 t4 t5 w
001 1
.end
.model mpx4
.inputs m n o p
.outputs y0 y1
.names m n o p y0
0100 1
0001 1
.names m n o p y1
0010 1
0001 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "e"]),
                clocks: strs_to_vec_string(["c", "d"]),
                outputs: strs_to_vec_string(["x", "y", "z"]),
                latches: strs2_to_vec_string([("y", "a"), ("x", "e")]),
                circuit: (
                    Circuit::from_str(
                        "{0 1 2 3 4 and(0,1):0 nor(3,4) and(6,2):1 nor(0,3) nimpl(8,2):2n}(5)"
                    )
                    .unwrap(),
                    vec![
                        Input(true),
                        Input(false),
                        Input(true),
                        Clock,
                        Clock,
                        Output(true),
                        Output(true),
                        Output(false)
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a
.clock c d
.inputs b e
.outputs x y z
.latch y a
.latch x e
.names a b x
11 1
.names c d e y
001 1
.names a c e z
1-- 1
-1- 1
--1 1
.end
"##,
                0
            )
        );
        // clock mapping
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "t0"]),
                clocks: strs_to_vec_string(["d", "e", "f", "t1"]),
                outputs: strs_to_vec_string(["x", "y"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str("{0 1 2 3 and(0,1):0 and(2,3):1}(4)").unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        NoMapping,
                        NoMapping,
                        NoMapping,
                        Clock,
                        NoMapping,
                        Clock,
                        Output(false),
                        Output(false)
                    ]
                )
            }),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c t0
.outputs x y
.clock d e f t1
.names a b x
11 1
.names e t1 y
11 1
.end
"##,
                0
            )
        );
        // error handling
        assert_eq!(
            Err("Already defined as output in simple:t0".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs x
.names a b t0
11 1
.subckt trivial x0=a x1=c y=t0
.names t0 c x
01 1
.end
.model trivial
.inputs x0 x1
.outputs y
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("Already defined as output in simple:t0".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs x
.subckt trivial x0=a x1=c y=t0
.names a b t0
11 1
.names t0 c x
01 1
.end
.model trivial
.inputs x0 x1
.outputs y
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("top.blif:4: Model have latches".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b
.outputs x
.subckt trivial x0=a x1=b y=x
.end
.model trivial
.inputs x0 x1
.outputs y
.latch y x0
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("top.blif:4: Model have clocks".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b
.outputs x
.subckt trivial x0=a x1=b y=x
.end
.model trivial
.inputs x1
.clock x0
.outputs y
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("top.blif:6: Defined as model output x".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b
.outputs x y z
.names a z
1 1
.subckt trivial x0=x x1=b y=y
.end
.model trivial
.inputs x0 x1
.outputs y
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("top.blif:6: Defined as model input d".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c d
.outputs z
.names c z
1 1
.subckt trivial x0=a x1=b y=d
.end
.model trivial
.inputs x0 x1
.outputs y
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("top.blif:7: Defined as model clock d".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.clock d
.outputs z
.names c z
1 1
.subckt trivial x0=a x1=b y=d
.end
.model trivial
.inputs x0 x1
.outputs y
.names x0 x1 y
01 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("Wire y in model simple is undefined".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs c
.outputs z y
.names c z
1 1
.end
"##,
                0
            )
        );
        assert_eq!(
            Err("Cycle in model simple caused by t0".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs z y
.names a z
1 1
.names t0 b t0
00 1
.names t0 c y
10 1
.end
"##,
                0
            )
        );
        assert_eq!(
            Err("Cycle in model simple caused by t0".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs z y
.names a z
1 1
.subckt and a=t0 b=b x=t0
.names t0 c y
10 1
.end
.model and
.inputs a b
.outputs x
.names a b x
11 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("Cycle in model simple caused by t1".to_string()),
            gen_model_circuit_helper(
                r##".model simple
.inputs a b c
.outputs z y
.names a z
1 1
.names t1 t2 t3
10 1
.names c b tx
00 1
.names tx t3 t0
01 1
.names t0 b t1
00 1
.names t3 c t2
11 1
.names t1 t2 y
10 1
.end
"##,
                0
            )
        );
    }

    fn strt_to_vec_string<'a, T: Clone>(
        iter: impl IntoIterator<Item = (&'a str, T)>,
    ) -> Vec<(String, T)> {
        iter.into_iter()
            .map(|(s1, t)| (s1.to_string(), t.clone()))
            .collect()
    }

    fn model_top_mapping_helper(text: &str) -> (Circuit<usize>, Vec<(String, AssignEntry)>) {
        let mut circuit_cache = CircuitCache::new();
        let mut gate_cache = GateCache::new();
        let mut model_map = ModelMap::new();
        let mut bytes = BLIFTokensReader::new(text.as_bytes());
        let (main_model_name, main_model) =
            parse_model("top.blif", &mut bytes, &mut circuit_cache, &mut gate_cache)
                .map_err(|e| e.to_string())
                .unwrap();
        model_map.insert(main_model_name.clone(), main_model);
        gen_model_circuit(&main_model_name, &mut model_map)
            .map_err(|e| e.to_string())
            .unwrap();
        // println!("Model: {:?}", model_map[&main_model_name]);
        let (c, m) = model_map[&main_model_name].clone().top_mapping();
        println!("Circuit: {}", c);
        (c, m)
    }

    #[test]
    fn test_model_top_mapping() {
        assert_eq!(
            (
                Circuit::new(0, [], []).unwrap(),
                strt_to_vec_string([
                    ("x", AssignEntry::Value(false)),
                    ("y", AssignEntry::Value(true)),
                    ("z", AssignEntry::Value(false)),
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str(
                    r##"{0 1 2 3 4 and(0,1) and(5,2):0 nor(0,2) nimpl(7,3):1n
nor(1,2) nimpl(9,3):2n and(2,3) and(11,4):3}(5)"##
                )
                .unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(0, false)),
                    ("a1", AssignEntry::Var(1, false)),
                    ("a2", AssignEntry::Var(2, false)),
                    ("a3", AssignEntry::Var(3, false)),
                    ("a4", AssignEntry::Var(4, false)),
                    ("x0", AssignEntry::Var(6, false)),
                    ("x1", AssignEntry::Var(8, true)),
                    ("x2", AssignEntry::Var(10, true)),
                    ("x3", AssignEntry::Var(12, false)),
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a2 a3 a4
.outputs x0 x1 x2 x3
.names a0 a1 a2 x0
111 1
.names a0 a2 a3 x1
000 0
.names a1 a2 a3 x2
000 0
.names a2 a3 a4 x3
111 1
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str(
                    r##"{0 1 2 3 4 and(2,3) and(5,0):0 nor(2,0) nimpl(7,4):1n
nor(3,0) nimpl(9,4):2n and(0,4) and(11,1):3}(5)"##
                )
                .unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(2, false)),
                    ("a1", AssignEntry::Var(3, false)),
                    ("a3", AssignEntry::Var(4, false)),
                    ("a2", AssignEntry::Var(0, false)),
                    ("a4", AssignEntry::Var(1, false)),
                    ("x0", AssignEntry::Var(6, false)),
                    ("x1", AssignEntry::Var(8, true)),
                    ("x2", AssignEntry::Var(10, true)),
                    ("x3", AssignEntry::Var(12, false)),
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3
.clock a2 a4
.outputs x0 x1 x2 x3
.names a0 a1 a2 x0
111 1
.names a0 a2 a3 x1
000 0
.names a1 a2 a3 x2
000 0
.names a2 a3 a4 x3
111 1
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str("{0 1 2 3 4 and(2,3) and(5,0):0 and(0,4) and(7,1):1}(5)")
                    .unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(2, false)),
                    ("a1", AssignEntry::Var(3, false)),
                    ("a3", AssignEntry::Var(4, false)),
                    ("a2", AssignEntry::Var(0, false)),
                    ("a4", AssignEntry::Var(1, false)),
                    ("x0", AssignEntry::Var(6, false)),
                    ("x1", AssignEntry::Value(false)),
                    ("x2", AssignEntry::Value(true)),
                    ("x3", AssignEntry::Var(8, false)),
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3
.clock a2 a4
.outputs x0 x1 x2 x3
.names a0 a1 a2 x0
111 1
.names x1
.names x2
1
.names a2 a3 a4 x3
111 1
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str("{0 1 2 and(1,2) and(3,0):0 nor(1,2) nimpl(5,0):1n}(3)").unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(1, false)),
                    ("a1", AssignEntry::Var(2, false)),
                    ("a3", AssignEntry::NoMap),
                    ("a2", AssignEntry::NoMap),
                    ("a4", AssignEntry::Var(0, false)),
                    ("x0", AssignEntry::Var(4, false)),
                    ("x1", AssignEntry::Value(false)),
                    ("x2", AssignEntry::Value(true)),
                    ("x3", AssignEntry::Var(6, true)),
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3
.clock a2 a4
.outputs x0 x1 x2 x3
.names a0 a1 a4 x0
111 1
.names x1
.names x2
1
.names a0 a1 a4 x3
000 0
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str(
                    r##"{0 1 2 3 4 5 6 7 8 and(6,0):2 nor(0,4):4n nor(4,2):3n and(2,5):1
nor(5,1):5n and(1,7):0 and(3,8):6}(9)"##
                )
                .unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(6, false)),
                    ("a1", AssignEntry::Var(0, false)),
                    ("a3", AssignEntry::Var(2, false)),
                    ("a5", AssignEntry::Var(1, false)),
                    ("a6", AssignEntry::Var(7, false)),
                    ("a7", AssignEntry::Var(3, false)),
                    ("a8", AssignEntry::Var(8, false)),
                    ("a2", AssignEntry::Var(4, false)),
                    ("a4", AssignEntry::Var(5, false)),
                    ("x0", AssignEntry::Var(9, false)),
                    ("x1", AssignEntry::Var(10, true)),
                    ("x2", AssignEntry::Var(11, true)),
                    ("x3", AssignEntry::Var(12, false)),
                    ("x4", AssignEntry::Var(13, true)),
                    ("x5", AssignEntry::Var(14, false)),
                    ("x6", AssignEntry::Var(15, false))
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3 a5 a6 a7 a8
.clock a2 a4
.outputs x0 x1 x2 x3 x4 x5 x6
.latch x5 a1
.latch x3 a5
.latch x0 a3
.latch x2 a7
.names a0 a1 x0
11 1
.names a1 a2 x1
00 0
.names a2 a3 x2
00 0
.names a3 a4 x3
11 1
.names a4 a5 x4
00 0
.names a5 a6 x5
11 1
.names a7 a8 x6
11 1
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str(
                    r##"{0 1 2 3 4 5 and(4,3):2 nor(4,3):4n nor(3,0):3n and(1,0):1 nor(0,2):5n 
and(1,2):0 and(2,5):6}(6)"##
                )
                .unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(4, false)),
                    ("a1", AssignEntry::NoMap),
                    ("a3", AssignEntry::Var(1, false)),
                    ("a5", AssignEntry::Var(0, false)),
                    ("a6", AssignEntry::NoMap),
                    ("a7", AssignEntry::Var(2, false)),
                    ("a8", AssignEntry::Var(5, false)),
                    ("a2", AssignEntry::Var(3, false)),
                    ("a4", AssignEntry::NoMap),
                    ("x0", AssignEntry::Var(6, false)),
                    ("x1", AssignEntry::Var(7, true)),
                    ("x2", AssignEntry::Var(8, true)),
                    ("x3", AssignEntry::Var(9, false)),
                    ("x4", AssignEntry::Var(10, true)),
                    ("x5", AssignEntry::Var(11, false)),
                    ("x6", AssignEntry::Var(12, false))
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3 a5 a6 a7 a8
.clock a2 a4
.outputs x0 x1 x2 x3 x4 x5 x6
.latch x5 a1
.latch x3 a5
.latch x0 a3
.latch x2 a7
.names a0 a2 x0
11 1
.names a0 a2 x1
00 0
.names a2 a5 x2
00 0
.names a3 a5 x3
11 1
.names a5 a7 x4
00 0
.names a3 a7 x5
11 1
.names a7 a8 x6
11 1
.end
"##
            )
        );
        assert_eq!(
            (
                Circuit::from_str("{0 1 2 3 4 5 and(4,3):1 nor(3,0):2n and(1,0):0 and(2,5):3}(6)")
                    .unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(4, false)),
                    ("a1", AssignEntry::NoMap),
                    ("a3", AssignEntry::Var(1, false)),
                    ("a5", AssignEntry::Var(0, false)),
                    ("a6", AssignEntry::NoMap),
                    ("a7", AssignEntry::Var(2, false)),
                    ("a8", AssignEntry::Var(5, false)),
                    ("a2", AssignEntry::Var(3, false)),
                    ("a4", AssignEntry::NoMap),
                    ("x0", AssignEntry::Var(6, false)),
                    ("x1", AssignEntry::Value(true)),
                    ("x2", AssignEntry::Var(7, true)),
                    ("x3", AssignEntry::Var(8, false)),
                    ("x4", AssignEntry::Value(false)),
                    ("x5", AssignEntry::Value(true)),
                    ("x6", AssignEntry::Var(9, false))
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3 a5 a6 a7 a8
.clock a2 a4
.outputs x0 x1 x2 x3 x4 x5 x6
.latch x5 a1
.latch x3 a5
.latch x0 a3
.latch x2 a7
.names a0 a2 x0
11 1
.names x1
1
.names a2 a5 x2
00 0
.names a3 a5 x3
11 1
.names x4
.names x5
1
.names a7 a8 x6
11 1
.end
"##
            )
        );
        // with direct input to output in model
        assert_eq!(
            (
                Circuit::from_str("{0:2 1:0n 2 3 4 5 and(4,3):1 and(2,5):3}(6)").unwrap(),
                strt_to_vec_string([
                    ("a0", AssignEntry::Var(4, false)),
                    ("a1", AssignEntry::NoMap),
                    ("a3", AssignEntry::Var(1, false)),
                    ("a5", AssignEntry::Var(0, false)),
                    ("a6", AssignEntry::NoMap),
                    ("a7", AssignEntry::Var(2, false)),
                    ("a8", AssignEntry::Var(5, false)),
                    ("a2", AssignEntry::Var(3, false)),
                    ("a4", AssignEntry::NoMap),
                    ("x0", AssignEntry::Var(6, false)),
                    ("x1", AssignEntry::Value(true)),
                    ("x2", AssignEntry::Var(0, false)),
                    ("x3", AssignEntry::Var(1, true)),
                    ("x4", AssignEntry::Value(false)),
                    ("x5", AssignEntry::Value(true)),
                    ("x6", AssignEntry::Var(7, false))
                ])
            ),
            model_top_mapping_helper(
                r##".model simple
.input a0 a1 a3 a5 a6 a7 a8
.clock a2 a4
.outputs x0 x1 x2 x3 x4 x5 x6
.latch x5 a1
.latch x3 a5
.latch x0 a3
.latch x2 a7
.names a0 a2 x0
11 1
.names x1
1
.names a5 x2
1 1
.names a3 x3
0 1
.names x4
.names x5
1
.names a7 a8 x6
11 1
.end
"##
            )
        );
    }

    use std::fs;

    struct FilesToRemove(Vec<String>);

    impl Drop for FilesToRemove {
        fn drop(&mut self) {
            for s in &self.0 {
                let _ = fs::remove_file(s);
            }
        }
    }

    fn write_files(files: impl IntoIterator<Item = (String, String)>) -> FilesToRemove {
        let mut files_to_remove = FilesToRemove(vec![]);
        for (path, content) in files {
            fs::write(&path, content.as_bytes()).unwrap();
            files_to_remove.0.push(path.clone());
        }
        files_to_remove
    }

    fn parse_file_helper(files: impl IntoIterator<Item = (String, String)>) -> (ModelMap, String) {
        let to_remove = write_files(files);
        parse_file(&to_remove.0[0]).unwrap()
    }

    #[test]
    fn test_parse_file() {
        assert_eq!(
            (
                ModelMap::from_iter([(
                    "simple".to_string(),
                    Model {
                        inputs: vec![],
                        outputs: strs_to_vec_string(["x", "y", "z"]),
                        latches: vec![],
                        clocks: vec![],
                        gates: vec![
                            Gate {
                                params: vec![],
                                output: "x".to_string(),
                                circuit: TableCircuit::Value(false),
                            },
                            Gate {
                                params: vec![],
                                output: "y".to_string(),
                                circuit: TableCircuit::Value(true),
                            },
                            Gate {
                                params: vec![],
                                output: "z".to_string(),
                                circuit: TableCircuit::Value(false),
                            }
                        ],
                        subcircuits: vec![],
                        circuit: None,
                    }
                )]),
                "simple".to_string()
            ),
            parse_file_helper(strs2_to_vec_string([(
                "xxxtop.blif",
                r##".model simple
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##
            )]))
        );
        let exp_result = (
            ModelMap::from_iter([
                (
                    "simple".to_string(),
                    Model {
                        inputs: strs_to_vec_string(["a", "b"]),
                        outputs: strs_to_vec_string(["x", "y", "z"]),
                        latches: vec![],
                        clocks: vec![],
                        gates: vec![],
                        subcircuits: vec![
                            Subcircuit {
                                model: "and".to_string(),
                                mappings: strs2_to_vec_string([
                                    ("a0", "a"),
                                    ("a1", "b"),
                                    ("x", "x"),
                                ]),
                                filename: "xxxtop.blif".to_string(),
                                line_no: 4,
                            },
                            Subcircuit {
                                model: "or".to_string(),
                                mappings: strs2_to_vec_string([
                                    ("a0", "a"),
                                    ("a1", "b"),
                                    ("x", "y"),
                                ]),
                                filename: "xxxtop.blif".to_string(),
                                line_no: 5,
                            },
                            Subcircuit {
                                model: "xor".to_string(),
                                mappings: strs2_to_vec_string([
                                    ("a0", "a"),
                                    ("a1", "b"),
                                    ("x", "z"),
                                ]),
                                filename: "xxxtop.blif".to_string(),
                                line_no: 6,
                            },
                        ],
                        circuit: None,
                    },
                ),
                (
                    "and".to_string(),
                    Model {
                        inputs: strs_to_vec_string(["a0", "a1"]),
                        outputs: strs_to_vec_string(["x"]),
                        latches: vec![],
                        clocks: vec![],
                        gates: vec![Gate {
                            params: strs_to_vec_string(["a0", "a1"]),
                            output: "x".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 and(0,1):0}(2)").unwrap(),
                                vec![Some(0), Some(1)],
                            )),
                        }],
                        subcircuits: vec![],
                        circuit: None,
                    },
                ),
                (
                    "or".to_string(),
                    Model {
                        inputs: strs_to_vec_string(["a0", "a1"]),
                        outputs: strs_to_vec_string(["x"]),
                        latches: vec![],
                        clocks: vec![],
                        gates: vec![Gate {
                            params: strs_to_vec_string(["a0", "a1"]),
                            output: "x".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 nor(0,1):0n}(2)").unwrap(),
                                vec![Some(0), Some(1)],
                            )),
                        }],
                        subcircuits: vec![],
                        circuit: None,
                    },
                ),
                (
                    "xor".to_string(),
                    Model {
                        inputs: strs_to_vec_string(["a0", "a1"]),
                        outputs: strs_to_vec_string(["x"]),
                        latches: vec![],
                        clocks: vec![],
                        gates: vec![Gate {
                            params: strs_to_vec_string(["a0", "a1"]),
                            output: "x".to_string(),
                            circuit: TableCircuit::Circuit((
                                Circuit::from_str("{0 1 xor(0,1):0}(2)").unwrap(),
                                vec![Some(0), Some(1)],
                            )),
                        }],
                        subcircuits: vec![],
                        circuit: None,
                    },
                ),
            ]),
            "simple".to_string(),
        );
        assert_eq!(
            exp_result.clone(),
            parse_file_helper(strs2_to_vec_string([
                (
                    "xxxtop.blif",
                    r##".model simple
.inputs a b
.outputs x y z
.subckt and a0=a a1=b x=x
.subckt or a0=a a1=b x=y
.subckt xor a0=a a1=b x=z
.end
.search xxxand.blif
.search xxxor.blif
.search xxxxor.blif
"##
                ),
                (
                    "xxxand.blif",
                    r##".model and
.input a0 a1
.outputs x
.names a0 a1 x
11 1
.end
"##
                ),
                (
                    "xxxor.blif",
                    r##".model or
.input a0 a1
.outputs x
.names a0 a1 x
1- 1
-1 1
.end
"##
                ),
                (
                    "xxxxor.blif",
                    r##".model xor
.input a0 a1
.outputs x
.names a0 a1 x
10 1
01 1
.end
"##
                ),
            ]))
        );
        assert_eq!(
            exp_result.clone(),
            parse_file_helper(strs2_to_vec_string([
                (
                    "xxxtop.blif",
                    r##".model simple
.inputs a b
.outputs x y z
.subckt and a0=a a1=b x=x
.subckt or a0=a a1=b x=y
.subckt xor a0=a a1=b x=z
.end
.search xxxgates.blif
"##
                ),
                (
                    "xxxgates.blif",
                    r##".search xxxand.blif
.search xxxor.blif
.search xxxxor.blif
"##
                ),
                (
                    "xxxand.blif",
                    r##".model and
.input a0 a1
.outputs x
.names a0 a1 x
11 1
.end
"##
                ),
                (
                    "xxxor.blif",
                    r##".model or
.input a0 a1
.outputs x
.names a0 a1 x
1- 1
-1 1
.end
"##
                ),
                (
                    "xxxxor.blif",
                    r##".model xor
.input a0 a1
.outputs x
.names a0 a1 x
10 1
01 1
.end
"##
                ),
            ]))
        );
        assert_eq!(
            exp_result.clone(),
            parse_file_helper(strs2_to_vec_string([
                (
                    "xxxtop.blif",
                    r##".model simple
.inputs a b
.outputs x y z
.subckt and a0=a a1=b x=x
.subckt or a0=a a1=b x=y
.subckt xor a0=a a1=b x=z
.end
.search xxxgates.blif
"##
                ),
                (
                    "xxxgates.blif",
                    r##".search xxxand.blif
.search xxxor.blif
"##
                ),
                (
                    "xxxand.blif",
                    r##".model and
.input a0 a1
.outputs x
.names a0 a1 x
11 1
.end
"##
                ),
                (
                    "xxxor.blif",
                    r##".model or
.input a0 a1
.outputs x
.names a0 a1 x
1- 1
-1 1
.end
.search xxxxor.blif
"##
                ),
                (
                    "xxxxor.blif",
                    r##".model xor
.input a0 a1
.outputs x
.names a0 a1 x
10 1
01 1
.end
"##
                ),
            ]))
        );
        assert_eq!(
            exp_result.clone(),
            parse_file_helper(strs2_to_vec_string([
                (
                    "xxxmain.blif",
                    r##".search xxxtop.blif
.search xxxgates.blif
"##
                ),
                (
                    "xxxtop.blif",
                    r##".model simple
.inputs a b
.outputs x y z
.subckt and a0=a a1=b x=x
.subckt or a0=a a1=b x=y
.subckt xor a0=a a1=b x=z
.end
"##
                ),
                (
                    "xxxgates.blif",
                    r##".search xxxand.blif
.search xxxor.blif
"##
                ),
                (
                    "xxxand.blif",
                    r##".model and
.input a0 a1
.outputs x
.names a0 a1 x
11 1
.end
"##
                ),
                (
                    "xxxor.blif",
                    r##".model or
.input a0 a1
.outputs x
.names a0 a1 x
1- 1
-1 1
.end
.search xxxxor.blif
"##
                ),
                (
                    "xxxxor.blif",
                    r##".model xor
.input a0 a1
.outputs x
.names a0 a1 x
10 1
01 1
.end
"##
                ),
            ]))
        );
    }

    fn resolve_model_helper(text: &str, model_num: usize) -> Result<CircuitData, String> {
        println!("ModelStart:");
        let mut circuit_cache = CircuitCache::new();
        let mut gate_cache = GateCache::new();
        let mut model_map = ModelMap::new();
        let mut bytes = BLIFTokensReader::new(text.as_bytes());
        let (main_model_name, main_model) =
            parse_model("top.blif", &mut bytes, &mut circuit_cache, &mut gate_cache)
                .map_err(|e| e.to_string())
                .unwrap();
        model_map.insert(main_model_name.clone(), main_model);
        for _ in 0..model_num {
            let (model_name, model) =
                parse_model("top.blif", &mut bytes, &mut circuit_cache, &mut gate_cache)
                    .map_err(|e| e.to_string())
                    .unwrap();
            model_map.insert(model_name.clone(), model);
        }
        resolve_model(&main_model_name, &mut model_map).map_err(|e| e.to_string())?;
        for g in &model_map[&main_model_name].gates {
            println!("ModelGate: {:?}", g);
        }
        Ok(CircuitData::from(model_map[&main_model_name].clone()))
    }

    #[test]
    fn test_resolve_model() {
        use CircuitMapping::*;
        assert_eq!(
            Ok(CircuitData {
                inputs: vec![],
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y", "z"]),
                latches: vec![],
                circuit: (
                    Circuit::new(0, [], []).unwrap(),
                    vec![Value(false), Value(true), Value(false)]
                )
            }),
            resolve_model_helper(
                r##".model simple
.outputs x y
.outputs z
.names x
.names y
1
.names z
0
.end
"##,
                0
            )
        );
        // subcircuits 5
        assert_eq!(
            Ok(CircuitData {
                inputs: strs_to_vec_string(["a", "b", "c", "d", "e", "f", "g", "h"]),
                clocks: vec![],
                outputs: strs_to_vec_string(["x", "y", "z", "w"]),
                latches: vec![],
                circuit: (
                    Circuit::from_str(
                        r##"{0 1 2 3 4 5 6 7 nor(0,2) xor(1,3) and(8,9) nor(2,4)
xor(3,5) and(11,12) nor(4,6) xor(5,7) and(14,15) nor(13,16) and(10,17):0 nor(0,1) xor(2,3)
and(19,20) nor(4,5) xor(6,7) and(22,23) and(21,24) nor(2,3) xor(4,5) and(26,27) nimpl(25,28):1
and(10,13) and(16,30):2 nor(21,28) and(24,32):3}(8)"##
                    )
                    .unwrap(),
                    vec![
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Input(false),
                        Output(false),
                        Output(false),
                        Output(false),
                        Output(false),
                    ]
                )
            }),
            resolve_model_helper(
                r##".model simple
.inputs a b c d e f g h
.outputs x y z w
.subckt mpx4 m=a n=b o=c p=d y0=t0 y1=t3
.subckt mpx4 m=c n=d o=e p=f y0=t1 y1=t4
.subckt mpx4 m=e n=f o=g p=h y0=t2 y1=t5
.names t0 t1 t2 x
100 1
.names t3 t4 t5 y
101 1
.names t0 t1 t2 z
111 1
.names t3 t4 t5 w
001 1
.end
.model mpx4
.inputs m n o p
.outputs y0 y1
.names m n o p y0
0100 1
0001 1
.names m n o p y1
0010 1
0001 1
.end
"##,
                1
            )
        );
        assert_eq!(
            Err("Cycle in model hierarchy caused by mpx4".to_string()),
            resolve_model_helper(
                r##".model simple
.inputs a b c d e f g h
.outputs x y z w
.subckt mpx4 m=a n=b o=c p=d y0=t0 y1=t3
.subckt mpx4 m=c n=d o=e p=f y0=t1 y1=t4
.subckt mpx4 m=e n=f o=g p=h y0=t2 y1=t5
.names t0 t1 t2 x
100 1
.names t3 t4 t5 y
101 1
.names t0 t1 t2 z
111 1
.names t3 t4 t5 w
001 1
.end
.model mpx4
.inputs m n o p
.outputs y0 y1
.names m n o p y0
0100 1
0001 1
.names m n o p t0
0010 1
0001 1
.subckt mpx5 m=m n=n o=o p=p y0=t1
.names t0 t1 y1
11 1
.end
.model mpx5
.inputs m n o p
.outputs y0 y1
.subckt mpx4 m=m n=n o=o p=p y0=y0 y1=y1
.end
"##,
                2
            )
        );
        assert_eq!(
            Err("top.blif:25: Model with name mpx5 is undefined".to_string()),
            resolve_model_helper(
                r##".model simple
.inputs a b c d e f g h
.outputs x y z w
.subckt mpx4 m=a n=b o=c p=d y0=t0 y1=t3
.subckt mpx4 m=c n=d o=e p=f y0=t1 y1=t4
.subckt mpx4 m=e n=f o=g p=h y0=t2 y1=t5
.names t0 t1 t2 x
100 1
.names t3 t4 t5 y
101 1
.names t0 t1 t2 z
111 1
.names t3 t4 t5 w
001 1
.end
.model mpx4
.inputs m n o p
.outputs y0 y1
.names m n o p y0
0100 1
0001 1
.names m n o p t0
0010 1
0001 1
.subckt mpx5 m=m n=n o=o p=p y0=t1
.names t0 t1 y1
11 1
.end
"##,
                1
            )
        );
    }
}
