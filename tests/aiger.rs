use gateconvert::aiger::{self, AIGEREntry};
use gatesim::*;

fn to_aiger_ascii_helper(circuit: Circuit<usize>, state_len: usize) -> String {
    let mut out = vec![];
    aiger::to_aiger(&circuit, state_len, &mut out, false).unwrap();
    String::from_utf8(out).unwrap()
}

fn to_aiger_bin_helper(circuit: Circuit<usize>, state_len: usize) -> Vec<u8> {
    let mut out = vec![];
    aiger::to_aiger(&circuit, state_len, &mut out, true).unwrap();
    out
}

#[test]
fn test_to_aiger() {
    // ascii
    assert_eq!(
        "aag 0 0 0 0 0\n",
        to_aiger_ascii_helper(Circuit::new(0, [], []).unwrap(), 0).as_str()
    );
    assert_eq!(
        concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 4\n14 3 5\n16 13 15\n"
        ),
        to_aiger_ascii_helper(
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            0
        )
        .as_str()
    );
    assert_eq!(
        concat!(
            "aag 11 2 3 1 6\n2\n4\n6 13\n8 14\n10 16\n23\n",
            "12 6 8\n14 11 3\n16 6 11\n18 8 4\n20 9 5\n22 19 21\n"
        ),
        to_aiger_ascii_helper(
            Circuit::new(
                5,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(2, 3),
                    Gate::new_nimpl(0, 2),
                    Gate::new_xor(1, 4),
                ],
                [(5, true), (6, false), (7, false), (8, true)]
            )
            .unwrap(),
            3
        )
        .as_str()
    );
    assert_eq!(
        r##"aag 19 4 0 5 15
2
4
6
8
10
22
30
32
39
10 2 6
12 4 6
14 2 8
16 4 8
18 12 14
20 13 15
22 19 21
24 12 14
26 16 24
28 17 25
30 27 29
32 16 24
34 22 30
36 23 31
38 35 37
"##,
        to_aiger_ascii_helper(
            Circuit::new(
                4,
                [
                    Gate::new_and(0, 2),
                    Gate::new_and(1, 2),
                    Gate::new_and(0, 3),
                    Gate::new_and(1, 3),
                    // add a1*b0 + a0*b1
                    Gate::new_xor(5, 6),
                    Gate::new_and(5, 6),
                    // add c(a1*b0 + a0*b1) + a1*b1
                    Gate::new_xor(7, 9),
                    Gate::new_and(7, 9),
                    Gate::new_xor(8, 10),
                ],
                [(4, false), (8, false), (10, false), (11, false), (12, true)],
            )
            .unwrap(),
            0
        )
        .as_str()
    );

    // binary mode
    assert_eq!(
        b"aig 0 0 0 0 0\n",
        to_aiger_bin_helper(Circuit::new(0, [], []).unwrap(), 0).as_slice()
    );
    assert_eq!(
        &[
            97, 105, 103, 32, 56, 32, 50, 32, 48, 32, 56, 32, 54, 10, 54, 10, 56, 10, 49, 48, 10,
            49, 54, 10, 55, 10, 57, 10, 49, 49, 10, 49, 55, 10, 2, 2, 3, 2, 5, 3, 8, 2, 9, 2, 1, 2
        ],
        to_aiger_bin_helper(
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            0
        )
        .as_slice()
    );
    assert_eq!(
        &[
            97, 105, 103, 32, 49, 49, 32, 50, 32, 51, 32, 49, 32, 54, 10, 49, 51, 10, 49, 52, 10,
            49, 54, 10, 50, 51, 10, 4, 2, 3, 8, 5, 5, 10, 4, 11, 4, 1, 2
        ],
        to_aiger_bin_helper(
            Circuit::new(
                5,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(2, 3),
                    Gate::new_nimpl(0, 2),
                    Gate::new_xor(1, 4),
                ],
                [(5, true), (6, false), (7, false), (8, true)]
            )
            .unwrap(),
            3
        )
        .as_slice()
    );
    assert_eq!(
        &[
            97, 105, 103, 32, 49, 57, 32, 52, 32, 48, 32, 53, 32, 49, 53, 10, 49, 48, 10, 50, 50,
            10, 51, 48, 10, 51, 50, 10, 51, 57, 10, 4, 4, 6, 2, 6, 6, 8, 4, 4, 2, 5, 2, 1, 2, 10,
            2, 2, 8, 3, 8, 1, 2, 8, 8, 4, 8, 5, 8, 1, 2
        ],
        to_aiger_bin_helper(
            Circuit::new(
                4,
                [
                    Gate::new_and(0, 2),
                    Gate::new_and(1, 2),
                    Gate::new_and(0, 3),
                    Gate::new_and(1, 3),
                    // add a1*b0 + a0*b1
                    Gate::new_xor(5, 6),
                    Gate::new_and(5, 6),
                    // add c(a1*b0 + a0*b1) + a1*b1
                    Gate::new_xor(7, 9),
                    Gate::new_and(7, 9),
                    Gate::new_xor(8, 10),
                ],
                [(4, false), (8, false), (10, false), (11, false), (12, true)],
            )
            .unwrap(),
            0
        )
        .as_slice()
    );
}

pub fn from_aiger_ascii_helper(
    input: &str,
) -> Result<(Circuit<usize>, Vec<(usize, AIGEREntry)>), String> {
    let mut bytes = input.as_bytes();
    aiger::from_aiger(&mut bytes, true).map_err(|e| e.to_string())
}

#[test]
fn test_from_aiger() {
    assert_eq!(
        Ok((Circuit::new(0, [], []).unwrap(), vec![],)),
        from_aiger_ascii_helper("aag 0 0 0 0 0\n")
    );
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(5, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(5, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 4\n14 3 5\n16 13 15\n"
        )),
    );
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(1, 0),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(5, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(5, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 4 2\n14 3 5\n16 13 15\n"
        )),
    );
    // first testcase with bad input
    assert_eq!(
        Err("Bad input".to_string()),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n2\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 4\n14 3 5\n16 13 15\n"
        )),
    );
    // first testcase with bad and gate
    assert_eq!(
        Err("AndGate bad output".to_string()),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n14 2 4\n14 3 5\n16 13 15\n"
        )),
    );
    // simplified version of first testcase (no duplicates)
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (12, AIGEREntry::Var(5, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (13, AIGEREntry::Var(5, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 6 2 0 8 4\n2\n4\n6\n8\n10\n12\n7\n9\n11\n13\n",
            "6 2 4\n8 3 5\n10 2 5\n12 7 9\n"
        )),
    );
    // modified version of first testcase. changed XOR to NXOR.
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, true),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, false),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(5, true)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(5, false)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 5\n14 3 4\n16 13 15\n"
        )),
    );
    // modified version of first testcase. changed XOR to other
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_nimpl(1, 0),
                    Gate::new_and(5, 6),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (7, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (7, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(7, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(7, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 5\n14 3 4\n16 12 14\n"
        )),
    );
    // modified version of first testcase. changed XOR to other
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_nimpl(1, 0),
                    Gate::new_nimpl(6, 5),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (7, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (7, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(7, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(7, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 5\n14 3 4\n16 13 14\n"
        )),
    );
    // modified version of first testcase. changed XOR to other
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_and(0, 1),
                    Gate::new_nor(5, 6),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (7, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (7, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(7, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(7, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 5\n14 2 4\n16 13 15\n"
        )),
    );
    // reordered variables
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            vec![
                (12, AIGEREntry::Var(0, false)),
                (14, AIGEREntry::Var(1, false)),
                (16, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (6, AIGEREntry::Var(5, false)),
                (17, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (11, AIGEREntry::Var(4, true)),
                (7, AIGEREntry::Var(5, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n12\n14\n16\n8\n10\n6\n17\n9\n11\n7\n",
            "16 12 14\n8 13 15\n10 12 15\n4 12 14\n2 13 15\n6 5 3\n"
        )),
    );
    // inputs as outputs
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, false),
                    (3, false),
                    (0, true),
                    (4, false),
                    (5, false),
                    (2, true),
                    (3, true),
                    (1, false),
                    (4, true),
                    (5, true),
                ]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (3, AIGEREntry::Var(0, true)),
                (10, AIGEREntry::Var(4, false)),
                (16, AIGEREntry::Var(5, false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Var(3, true)),
                (4, AIGEREntry::Var(1, false)),
                (11, AIGEREntry::Var(4, true)),
                (17, AIGEREntry::Var(5, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 10 6\n2\n4\n6\n8\n3\n10\n16\n7\n9\n4\n11\n17\n",
            "6 2 4\n8 3 5\n10 2 5\n12 2 4\n14 3 5\n16 13 15\n"
        )),
    );
    // latches
    assert_eq!(
        Ok((
            Circuit::new(
                5,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(2, 3),
                    Gate::new_nimpl(0, 2),
                    Gate::new_xor(1, 4),
                ],
                [(5, true), (6, false), (7, false), (8, true)]
            )
            .unwrap(),
            vec![
                (6, AIGEREntry::Var(0, false)),
                (8, AIGEREntry::Var(1, false)),
                (10, AIGEREntry::Var(2, false)),
                (2, AIGEREntry::Var(3, false)),
                (4, AIGEREntry::Var(4, false)),
                (13, AIGEREntry::Var(5, true)),
                (14, AIGEREntry::Var(6, false)),
                (16, AIGEREntry::Var(7, false)),
                (23, AIGEREntry::Var(8, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 11 2 3 1 6\n2\n4\n6 13\n8 14\n10 16\n23\n",
            "12 6 8\n14 11 3\n16 6 11\n18 8 4\n20 9 5\n22 19 21\n"
        )),
    );
    assert_eq!(
        Err("Latch bad state".to_string()),
        from_aiger_ascii_helper(concat!(
            "aag 11 2 3 1 6\n2\n4\n6 13\n8 14\n6 16\n23\n",
            "12 6 8\n14 11 3\n16 6 11\n18 8 4\n20 9 5\n22 19 21\n"
        )),
    );
    // more complex testcase
    assert_eq!(
        Ok((
            Circuit::new(
                4,
                [
                    Gate::new_and(0, 2),
                    Gate::new_and(1, 2),
                    Gate::new_and(0, 3),
                    // add a1*b0 + a0*b1
                    Gate::new_xor(5, 6),
                    Gate::new_and(1, 3),
                    Gate::new_and(5, 6),
                    // add c(a1*b0 + a0*b1) + a1*b1
                    Gate::new_xor(8, 9),
                    Gate::new_and(8, 9),
                    Gate::new_xor(7, 10),
                ],
                [(4, false), (7, false), (10, false), (11, false), (12, true)],
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (22, AIGEREntry::Var(7, false)),
                (30, AIGEREntry::Var(10, false)),
                (32, AIGEREntry::Var(11, false)),
                (39, AIGEREntry::Var(12, true)),
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 19 4 0 5 15
2
4
6
8
10
22
30
32
39
10 2 6
12 4 6
14 2 8
16 4 8
18 12 14
20 13 15
22 19 21
24 12 14
26 16 24
28 17 25
30 27 29
32 16 24
34 22 30
36 23 31
38 35 37
"##
        ),
    );
    // this same as previous but without duplicates
    assert_eq!(
        Ok((
            Circuit::new(
                4,
                [
                    Gate::new_and(0, 2),
                    Gate::new_and(1, 2),
                    Gate::new_and(0, 3),
                    // add a1*b0 + a0*b1
                    Gate::new_xor(5, 6),
                    Gate::new_and(1, 3),
                    Gate::new_and(5, 6),
                    // add c(a1*b0 + a0*b1) + a1*b1
                    Gate::new_xor(8, 9),
                    Gate::new_and(8, 9),
                    Gate::new_xor(7, 10),
                ],
                [(4, false), (7, false), (10, false), (11, false), (12, true)],
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (10, AIGEREntry::Var(4, false)),
                (22, AIGEREntry::Var(7, false)),
                (28, AIGEREntry::Var(10, false)),
                (24, AIGEREntry::Var(11, false)),
                (35, AIGEREntry::Var(12, true)),
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 17 4 0 5 13
2
4
6
8
10
22
28
24
35
10 2 6
12 4 6
14 2 8
16 4 8
18 12 14
20 13 15
22 19 21
24 16 18
26 17 19
28 25 27
30 22 28
32 23 29
34 31 33
"##
        ),
    );
    // more complex testcase - with cycles!
    assert_eq!(
        Err("Cycles in AIGER".to_string()),
        from_aiger_ascii_helper(
            r##"aag 19 4 0 5 15
2
4
6
8
10
22
30
32
39
10 2 6
12 4 6
14 2 8
16 4 31
18 12 14
20 13 15
22 19 21
24 12 14
26 16 24
28 17 25
30 27 29
32 16 24
34 22 30
36 23 31
38 35 37
"##
        ),
    );
    assert_eq!(
        Err("Cycles in AIGER".to_string()),
        from_aiger_ascii_helper(
            r##"aag 19 4 0 5 15
2
4
6
8
10
22
30
32
39
10 2 6
12 4 6
14 2 8
16 4 30
18 12 14
20 13 15
22 19 21
24 12 14
26 16 24
28 17 25
30 27 29
32 16 24
34 22 30
36 23 31
38 35 37
"##
        ),
    );
    // more complex case with latches
    assert_eq!(
        Ok((
            Circuit::new(
                8,
                [
                    Gate::new_nimpl(6, 5),
                    Gate::new_nimpl(8, 4),
                    Gate::new_and(7, 0),
                    Gate::new_nimpl(0, 7),
                    Gate::new_nimpl(10, 11),
                    Gate::new_and(9, 12),
                    Gate::new_and(0, 1),
                    Gate::new_nimpl(2, 3),
                    Gate::new_nor(14, 15),
                    Gate::new_nimpl(16, 4),
                    Gate::new_and(1, 2),
                    Gate::new_nor(2, 3),
                    Gate::new_and(18, 19),
                ],
                [
                    (13, false),
                    (12, false),
                    (16, false),
                    (14, false),
                    (17, false),
                    (9, false),
                    (12, true),
                    (20, false),
                    (16, true),
                ],
            )
            .unwrap(),
            vec![
                (8, AIGEREntry::Var(0, false)),
                (10, AIGEREntry::Var(1, false)),
                (12, AIGEREntry::Var(2, false)),
                (14, AIGEREntry::Var(3, false)),
                (16, AIGEREntry::Var(4, false)),
                (2, AIGEREntry::Var(5, false)),
                (4, AIGEREntry::Var(6, false)),
                (6, AIGEREntry::Var(7, false)),
                (34, AIGEREntry::Var(13, false)),
                (26, AIGEREntry::Var(12, false)),
                (40, AIGEREntry::Var(16, false)),
                (36, AIGEREntry::Var(14, false)),
                (42, AIGEREntry::Var(17, false)),
                (24, AIGEREntry::Var(9, false)),
                (27, AIGEREntry::Var(12, true)),
                (32, AIGEREntry::Var(20, false)),
                (41, AIGEREntry::Var(16, true))
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 21 3 5 4 13
2
4
6
8 34
10 26
12 40
14 36
16 42
24
27
32
41
18 3 4
20 6 8
22 7 8
24 17 18
26 20 23
28 10 12
30 13 15
32 28 30
34 24 26
36 8 10
38 12 15
40 37 39
42 40 17
"##
        ),
    );
    // more complex case with latches - with constant
    assert_eq!(
        Ok((
            Circuit::new(
                3,
                [
                    Gate::new_and(2, 0),
                    Gate::new_nimpl(0, 2),
                    Gate::new_nimpl(3, 4),
                ],
                [(5, false), (1, true), (5, true)],
            )
            .unwrap(),
            vec![
                (8, AIGEREntry::Var(0, false)), // 0
                (10, AIGEREntry::NoMap),
                (12, AIGEREntry::NoMap),
                (14, AIGEREntry::NoMap),
                (16, AIGEREntry::Var(1, false)),
                (2, AIGEREntry::NoMap),
                (4, AIGEREntry::NoMap),
                (6, AIGEREntry::Var(2, false)),  // 2
                (34, AIGEREntry::Value(false)),  // 34 24 26, 24 0 18 -> 34 0 26 -> false
                (26, AIGEREntry::Var(5, false)), // ok
                (40, AIGEREntry::Value(true)),   // 40 37 39, 36 0 10, 38 0 15 -> true
                (36, AIGEREntry::Value(false)),  // false
                (42, AIGEREntry::Var(1, true)),  // 42 40 17 -> 42 1 17 -> !16
                (24, AIGEREntry::Value(false)),  // false
                (27, AIGEREntry::Var(5, true)),  // ok
                (32, AIGEREntry::Value(false)),  // 32 28 30, 30 0 15 -> 32 28 0 -> false
                (41, AIGEREntry::Value(false))   // false
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 21 3 5 4 13
2
4
6
8 34
10 26
12 40
14 36
16 42
24
27
32
41
18 3 4
20 6 8
22 7 8
24 0 18
26 20 23
28 10 12
30 0 15
32 28 30
34 24 26
36 0 10
38 0 15
40 37 39
42 40 17
"##
        ),
    );
    // with acyclic graph - many usages
    assert_eq!(
        Ok((
            Circuit::new(
                4,
                [
                    Gate::new_xor(0, 1),
                    Gate::new_and(2, 3),
                    Gate::new_nimpl(4, 5),
                    Gate::new_nimpl(0, 4),
                    Gate::new_nimpl(5, 2),
                    Gate::new_nor(7, 8),
                    Gate::new_xor(6, 9),
                ],
                [(6, true), (9, false), (10, true)],
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (19, AIGEREntry::Var(6, true)),
                (24, AIGEREntry::Var(9, false)),
                (31, AIGEREntry::Var(10, true))
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 15 4 0 3 11
2
4
6
8
19
24
31
10 2 4
12 3 5
14 11 13
16 6 8
18 14 17
20 15 2
22 16 7
24 21 23
26 19 25
28 24 18
30 27 29
"##
        ),
    );
    // with acyclic graph - many usages - reordered and gates
    assert_eq!(
        Ok((
            Circuit::new(
                4,
                [
                    Gate::new_xor(0, 1),
                    Gate::new_and(2, 3),
                    Gate::new_nimpl(4, 5),
                    Gate::new_nimpl(0, 4),
                    Gate::new_nimpl(5, 2),
                    Gate::new_nor(7, 8),
                    Gate::new_xor(6, 9),
                ],
                [(6, true), (9, false), (10, true)],
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Var(3, false)),
                (19, AIGEREntry::Var(6, true)),
                (24, AIGEREntry::Var(9, false)),
                (31, AIGEREntry::Var(10, true))
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 15 4 0 3 11
2
4
6
8
19
24
31
14 11 13
12 3 5
10 2 4
26 19 25
16 6 8
30 27 29
20 15 2
18 14 17
28 24 18
22 16 7
24 21 23
"##
        ),
    );
    // testcase with constants
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [Gate::new_nor(0, 1), Gate::new_xor(0, 1)],
                [(2, false), (3, false), (2, true), (3, true),]
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Value(false)),
                (8, AIGEREntry::Var(2, false)),
                (10, AIGEREntry::Value(true)),
                (16, AIGEREntry::Var(3, false)),
                (7, AIGEREntry::Value(true)),
                (9, AIGEREntry::Var(2, true)),
                (11, AIGEREntry::Value(false)),
                (17, AIGEREntry::Var(3, true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 0 1\n8 3 5\n10 1 1\n12 2 4\n14 3 5\n16 13 15\n"
        )),
    );
    assert_eq!(
        Ok((
            Circuit::new(2, [Gate::new_and(0, 1)], [(2, false), (2, true)]).unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Value(true)),
                (10, AIGEREntry::Value(false)),
                (16, AIGEREntry::Value(false)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Value(false)),
                (11, AIGEREntry::Value(true)),
                (17, AIGEREntry::Value(true)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 1 1\n10 1 0\n12 0 0\n14 1 1\n16 13 15\n"
        )),
    );
    assert_eq!(
        Ok((
            Circuit::new(2, [Gate::new_and(0, 1)], [(2, false), (2, true)]).unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (8, AIGEREntry::Value(true)),
                (10, AIGEREntry::Value(false)),
                (16, AIGEREntry::Value(true)),
                (7, AIGEREntry::Var(2, true)),
                (9, AIGEREntry::Value(false)),
                (11, AIGEREntry::Value(true)),
                (17, AIGEREntry::Value(false)),
            ],
        )),
        from_aiger_ascii_helper(concat!(
            "aag 8 2 0 8 6\n2\n4\n6\n8\n10\n16\n7\n9\n11\n17\n",
            "6 2 4\n8 1 1\n10 1 0\n12 0 1\n14 0 1\n16 13 15\n"
        )),
    );
    // with acyclic graph - many usages - with constants and no mapping
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [Gate::new_and(0, 1), Gate::new_nimpl(2, 0),],
                [(3, true), (3, false)],
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::NoMap),
                (4, AIGEREntry::NoMap),
                (6, AIGEREntry::Var(0, false)),
                (8, AIGEREntry::Var(1, false)),
                (19, AIGEREntry::Value(true)),
                (24, AIGEREntry::Var(3, true)),
                (31, AIGEREntry::Var(3, false)),
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 15 4 0 3 11
2
4
6
8
19
24
31
10 2 4
12 3 5
14 11 13
16 6 8
18 14 0
20 15 0
22 16 7
24 21 23
26 19 25
28 24 18
30 27 29
"##
        ),
    );
    // from other source
    assert_eq!(
        Ok((
            Circuit::new(
                3,
                [
                    Gate::new_xor(2, 0),
                    Gate::new_nor(3, 1),
                    Gate::new_xor(2, 0),
                    Gate::new_and(5, 1),
                    Gate::new_nor(4, 6),
                ],
                [(7, false)],
            )
            .unwrap(),
            vec![
                (2, AIGEREntry::Var(0, false)),
                (4, AIGEREntry::Var(1, false)),
                (6, AIGEREntry::Var(2, false)),
                (24, AIGEREntry::Var(7, false)),
                (0, AIGEREntry::Value(false))
            ],
        )),
        from_aiger_ascii_helper(
            r##"aag 12 3 0 2 9
2
4
6
24
0
8 7 3
10 6 2
12 11 9
14 12 4
16 7 2
18 6 3
20 19 17
22 20 5
24 23 15
"##,
        ),
    );
}
