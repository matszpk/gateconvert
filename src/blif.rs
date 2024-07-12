use crate::AssignEntry;
use gategen::boolvar::*;
use gategen::dynintvar::*;
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
    #[error("{0}:{1}: Model with name {2} is undefined")]
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

#[derive(Clone, Debug)]
struct MappingKey {
    model: String,
    subcircuit: Option<usize>,
    wire: String,
}

pub fn blif_assign_map_to_string(map: &[(MappingKey, AssignEntry)]) -> String {
    let mut out = String::new();
    for (k, t) in map {
        out += &k.model;
        out.push(':');
        if let Some(subc) = k.subcircuit {
            out += &subc.to_string();
            out.push(':');
        }
        out += &k.wire;
        out.push(' ');
        out += &t.to_string();
        out.push('\n');
    }
    out
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
    let mut last_line_no = 1;
    while let Some((line_no, line)) = reader.read_tokens()? {
        last_line_no = line_no;
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
                after_model_decls = true;
                return Err(BLIFError::UnsupportedFSM(filename.to_string(), line_no));
            }
            ".gate" | ".mlatch" => {
                after_model_decls = true;
                return Err(BLIFError::UnsupportedGate(filename.to_string(), line_no));
            }
            ".end" => {
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
    // next phase - checking graph of gates and subcircuits - check whether graph have cycles.
    // next phase will be done while resolving graph of models.
    Ok((model_name, model))
}

fn gen_model_circuit(model_name: String, model_map: &mut ModelMap) -> Result<(), BLIFError> {
    let model = model_map.get(&model_name).unwrap();
    // all subcircuit must be resolved and they must have generated circuits.
    assert!(model
        .subcircuits
        .iter()
        .all(|sc| model_map.get(&sc.model).unwrap().circuit.is_some()));
    #[derive(Clone)]
    enum InputNode {
        ModelInput(usize),
        ModelClock(usize),
        Gate(usize, usize),       // gate index, parameter index
        Subcircuit(usize, usize), // subcircuit index, input index
    }
    #[derive(Clone)]
    enum OutputNode {
        Gate(usize),              // gate index
        Subcircuit(usize, usize), // subcircuit index, output index
    }
    #[derive(Clone)]
    enum Node {
        ModelInput(usize),
        ModelClock(usize),
        ModelOutput(usize),
        Gate(usize),
        Subcircuit(usize, usize),
        Zero,
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
        for (gini, gin) in g.params.iter().enumerate() {
            if let Some((wi, _)) = wire_in_outs.get_mut(gin) {
                wi.push(InputNode::Gate(i, gini));
            } else {
                wire_in_outs.insert(gin.clone(), (vec![InputNode::Gate(i, gini)], None));
            }
        }
        if let Some((_, wo)) = wire_in_outs.get_mut(&g.output) {
            if wo.is_some() {
                return Err(BLIFError::AlreadyDefinedAsOutput2(
                    model_name.clone(),
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
            for (model_wire, wire) in &sc.mappings {
                if let Some(input_index) = sc_input_map.get(model_wire) {
                    sc_mapping.inputs[*input_index] = Some(wire.clone());
                }
                if let Some(output_index) = sc_output_map.get(model_wire) {
                    sc_mapping.outputs[*output_index] = Some(wire.clone());
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
            for (scini, scin) in sc_mapping.inputs.iter().enumerate() {
                if let Some(scin) = scin.as_ref() {
                    if let Some((wi, _)) = wire_in_outs.get_mut(scin) {
                        wi.push(InputNode::Subcircuit(i, scini));
                    } else {
                        wire_in_outs
                            .insert(scin.clone(), (vec![InputNode::Subcircuit(i, scini)], None));
                    }
                }
            }
            for (scouti, scout) in sc_mapping.outputs.iter().enumerate() {
                if let Some(scout) = scout.as_ref() {
                    if let Some((_, wo)) = wire_in_outs.get_mut(scout) {
                        if wo.is_some() {
                            return Err(BLIFError::AlreadyDefinedAsOutput2(
                                model_name.clone(),
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

    // creating circuit
    let (circuit, circuit_mapping) = callsys(|| {
        let mut boolvar_map = HashMap::<String, BoolVarSys>::new();
        let mut visited = HashSet::<String>::new();
        let mut path_visited = HashSet::<String>::new();
        let mut stack = vec![];
        for (i, outname) in model.outputs.iter().enumerate() {
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
                    model_name.clone(),
                    outname.clone(),
                ));
            };

            while !stack.is_empty() {
                let mut top = stack.last_mut().unwrap();
                let way = top.way;
                let way_num = match top.node {
                    Node::Zero | Node::ModelInput(_) | Node::ModelClock(_) => 0,
                    Node::ModelOutput(_) => 1,
                    Node::Gate(j) => model.gates[j].params.len(),
                    Node::Subcircuit(j, _) => sc_mappings[j].outputs.len(),
                };
                let name = match top.node {
                    Node::Zero => String::new(),
                    Node::ModelInput(j) => model.inputs[j].clone(),
                    Node::ModelOutput(j) => model.outputs[j].clone(),
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
                            model_name.clone(),
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
                                model_name.clone(),
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
                    match top.node {
                        Node::Zero => {
                            boolvar_map.insert(name.clone(), BoolVarSys::from(false));
                        }
                        Node::ModelInput(_) | Node::ModelClock(_) => {
                            if !boolvar_map.contains_key(&name) {
                                boolvar_map.insert(name.clone(), BoolVarSys::var());
                            }
                        }
                        Node::ModelOutput(_) => (),
                        Node::Gate(j) => {
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
                                boolvar_map.insert(name.clone(), expr);
                            }
                        }
                        Node::Subcircuit(j, k) => {
                            if !boolvar_map.contains_key(&name) {
                                let sc_mapping = &sc_mappings[j];
                                let subc_model = &model_map[&model.subcircuits[j].model];
                                let circuit_mapping = &subc_model.circuit.as_ref().unwrap().1;
                                let output_count = circuit_mapping
                                    .iter()
                                    .filter(|c| matches!(c, CircuitMapping::Output(_)))
                                    .count();
                                let total_input_len = model.inputs.len();
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
                                for (i, c) in circuit_mapping[total_input_len..].iter().enumerate()
                                {
                                    match c {
                                        CircuitMapping::Value(v) => {
                                            boolvar_map.insert(
                                                model.outputs[i].clone(),
                                                BoolVarSys::from(*v),
                                            );
                                        }
                                        CircuitMapping::Output(ci) => {
                                            let old_out_count = out_count;
                                            out_count += 1;
                                            boolvar_map.insert(
                                                model.outputs[i].clone(),
                                                circ_outputs[old_out_count].clone(),
                                            );
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
        let circuit_mapping = model
            .inputs
            .iter()
            .zip(input_map.iter())
            .map(|(x, opti)| {
                if let Some(new_index) = opti {
                    CircuitMapping::Input(latch_outputs.contains(x))
                } else {
                    CircuitMapping::NoMapping
                }
            })
            .chain(model.clocks.iter().zip(input_map.iter()).map(|(x, opti)| {
                if let Some(new_index) = opti {
                    CircuitMapping::Clock
                } else {
                    CircuitMapping::NoMapping
                }
            }))
            .chain(circuit_out_mapping.into_iter())
            .collect::<Vec<_>>();
        Ok((circuit, circuit_mapping))
    })?;
    let model = model_map.get_mut(&model_name).unwrap();
    model.circuit = Some((circuit, circuit_mapping));
    Ok(())
}

fn resolve_model(top: String, model_map: &mut ModelMap) {}

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
                                    r##"{0 1 2 3 nor(1,2) and(1,2)
nimpl(3,4) nor(0,6) nor(5,7) xor(6,8):0}(4)"##
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
    }
}
