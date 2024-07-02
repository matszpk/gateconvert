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
}
