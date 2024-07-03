use gateconvert::btor2;
use gatesim::*;

fn to_btor2_helper(circuit: Circuit<usize>, state_len: usize) -> String {
    let mut out = vec![];
    btor2::to_btor2(&circuit, state_len, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_to_btor2() {
    // ascii
    assert_eq!(
        "1 sort bitvec 1\n",
        to_btor2_helper(Circuit::new(0, [], []).unwrap(), 0).as_str()
    );
    assert_eq!(
        r##"1 sort bitvec 1
2 input 1
3 input 1
4 and 1 2 3
5 or 1 2 3
6 implies 1 2 3
7 xor 1 2 3
8 output 4
9 not 1 5
10 output 9
11 not 1 6
12 output 11
13 output 7
14 not 1 4
15 output 14
16 output 5
17 output 6
18 not 1 7
19 output 18
"##,
        to_btor2_helper(
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
        r##"1 sort bitvec 1
2 state 1
3 state 1
4 state 1
5 input 1
6 input 1
7 and 1 2 3
8 or 1 4 5
9 implies 1 2 4
10 xor 1 3 6
11 not 1 7
12 next 1 2 11
13 not 1 8
14 next 1 3 13
15 not 1 9
16 next 1 4 15
17 not 1 10
18 output 17
"##,
        to_btor2_helper(
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
        r##"1 sort bitvec 1
2 input 1
3 input 1
4 input 1
5 input 1
6 and 1 2 4
7 and 1 3 4
8 and 1 2 5
9 and 1 3 5
10 xor 1 7 8
11 and 1 7 8
12 xor 1 9 11
13 and 1 9 11
14 xor 1 10 12
15 output 6
16 output 10
17 output 12
18 output 13
19 not 1 14
20 output 19
"##,
        to_btor2_helper(
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
