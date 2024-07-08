use gatesim::*;

use crate::blif_pla::*;

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

pub fn to_blif(
    circuit: &Circuit<usize>,
    state_len: usize,
    clock_num: usize,
    model_name: &str,
    out: &mut impl Write,
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
}

impl<R: Read> BLIFTokensReader<R> {
    fn new(r: R) -> Self {
        Self {
            br: BufReader::new(r),
            line_no: 1,
        }
    }

    // returns line number and tokens
    fn read_tokens(&mut self) -> io::Result<Option<(usize, Vec<String>)>> {
        let mut line = String::new();
        let current_line_no = self.line_no;
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
        Ok(if !empty {
            Some((
                current_line_no,
                line.trim()
                    .split_whitespace()
                    .map(|x| x.to_string())
                    .collect(),
            ))
        } else {
            None
        })
    }
}

#[derive(Clone, Debug)]
struct Gate {
    params: Vec<String>,
    pla_table: (Vec<PLACell>, bool, usize),
}

#[derive(Clone, Debug)]
struct Subcircuit {
    model: String,
    mappings: Vec<String>,
}

#[derive(Clone, Debug)]
struct Model {
    inputs: Vec<String>,
    outputs: Vec<String>,
    latches: Vec<(String, String)>,
    clocks: Vec<String>,
    gates: Vec<Gate>,
    subcircuits: Vec<Subcircuit>
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
                (7, vec![]),
                (8, vec![]),
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
