use gateconvert::aiger;
use gatesim::*;

fn to_aiger_ascii_helper(circuit: Circuit<usize>, state_len: usize) -> String {
    let mut out = vec![];
    aiger::to_aiger(&circuit, state_len, &mut out, false).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_to_aiger() {
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
}
