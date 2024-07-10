use gateconvert::blif;
use gatesim::*;

fn to_blif_helper(circuit: Circuit<usize>, state_len: usize, clock_num: usize) -> String {
    let mut out = vec![];
    blif::to_blif(&circuit, state_len, clock_num, "top", &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_to_blif() {
    assert_eq!(
        ".model top\n.end\n",
        to_blif_helper(Circuit::new(0, [], []).unwrap(), 0, 0).as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.outputs o4
.outputs o5
.outputs o6
.outputs o7
.names i0 i1 o0
11 1
.names i0 i1 o1
00 1
.names i0 i1 o2
10 1
.names i0 i1 o3
10 1
01 1
.names o0 o4
0 1
.names o1 o5
0 1
.names o2 o6
0 1
.names o3 o7
0 1
.end
"##,
        to_blif_helper(
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
            0,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.outputs o4
.outputs o5
.outputs o6
.outputs o7
.names i0 i1 o4
11 1
.names i0 i1 o5
00 1
.names i0 i1 o6
10 1
.names i0 i1 o7
10 1
01 1
.names o4 o0
0 1
.names o5 o1
0 1
.names o6 o2
0 1
.names o7 o3
0 1
.end
"##,
        to_blif_helper(
            Circuit::new(
                2,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(0, 1),
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(0, 1),
                ],
                [
                    (2, true),
                    (3, true),
                    (4, true),
                    (5, true),
                    (2, false),
                    (3, false),
                    (4, false),
                    (5, false),
                ]
            )
            .unwrap(),
            0,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.outputs o4
.outputs o5
.outputs o6
.outputs o7
.outputs o8
.outputs o9
.outputs o10
.outputs o11
.outputs o12
.outputs o13
.outputs o14
.outputs o15
.names i0 i1 o0
11 1
.names i0 i1 o1
00 1
.names i0 i1 o2
10 1
.names i0 i1 o3
10 1
01 1
.names o0 o4
0 1
.names o1 o5
0 1
.names o2 o6
0 1
.names o3 o7
0 1
.names o0 o8
1 1
.names o1 o9
1 1
.names o2 o10
1 1
.names o3 o11
1 1
.names o4 o12
1 1
.names o5 o13
1 1
.names o6 o14
1 1
.names o7 o15
1 1
.end
"##,
        to_blif_helper(
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
            0,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.inputs i3
.inputs i4
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.latch o0 i0
.latch o1 i1
.latch o2 i2
.names i0 i1 i5
11 1
.names i2 i3 o1
00 1
.names i0 i2 o2
10 1
.names i1 i4 i8
10 1
01 1
.names i5 o0
0 1
.names i8 o3
0 1
.end
"##,
        to_blif_helper(
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
            3,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.inputs i3
.inputs i4
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.outputs o4
.outputs o5
.outputs o6
.latch o0 i0
.latch o1 i1
.latch o2 i2
.names i0 i1 o1
11 1
.names i2 i3 o3
00 1
.names i0 i2 o4
10 1
.names i1 i4 i8
10 1
01 1
.names o1 o0
0 1
.names i8 o5
0 1
.names o0 o2
1 1
.names o3 o6
1 1
.end
"##,
        to_blif_helper(
            Circuit::new(
                5,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(2, 3),
                    Gate::new_nimpl(0, 2),
                    Gate::new_xor(1, 4),
                ],
                [
                    (5, true),
                    (5, false),
                    (5, true),
                    (6, false),
                    (7, false),
                    (8, true),
                    (6, false)
                ]
            )
            .unwrap(),
            3,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.inputs i3
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.outputs o4
.names i0 i2 o0
11 1
.names i1 i2 i5
11 1
.names i0 i3 i6
11 1
.names i1 i3 i7
11 1
.names i5 i6 o1
10 1
01 1
.names i5 i6 i9
11 1
.names i7 i9 o2
10 1
01 1
.names i7 i9 o3
11 1
.names o1 o2 i12
10 1
01 1
.names i12 o4
0 1
.end
"##,
        to_blif_helper(
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
            0,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.inputs i3
.inputs i4
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.outputs o4
.outputs o5
.outputs o6
.outputs o7
.outputs o8
.outputs o9
.outputs o10
.outputs o11
.outputs o12
.outputs o13
.outputs o14
.outputs o15
.outputs o16
.outputs o17
.outputs o18
.outputs o19
.names o0 i1 o4
11 1
.names o2 o4 o10
00 1
.names i3 o10 o6
10 1
.names i4 o6 o8
10 1
01 1
.names o0 o3
0 1
.names o2 o1
0 1
.names o4 o11
0 1
.names o10 o5
0 1
.names o6 o9
0 1
.names o8 o7
0 1
.names o7 o12
1 1
.names o6 o13
1 1
.names o5 o14
1 1
.names o4 o15
1 1
.names o11 o16
1 1
.names o10 o17
1 1
.names o9 o18
1 1
.names o8 o19
1 1
.end
"##,
        to_blif_helper(
            Circuit::new(
                5,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(2, 5),
                    Gate::new_nimpl(3, 6),
                    Gate::new_xor(4, 7),
                ],
                [
                    (0, false), // 0
                    (2, true),  // 1
                    (2, false), // 2
                    (0, true),  // 3
                    (5, false), // 4
                    (6, true),  // 5
                    (7, false), // 6
                    (8, true),  // 7
                    (8, false), // 8
                    (7, true),  // 9
                    (6, false), // 10
                    (5, true),  // 11
                    (8, true),  // 12
                    (7, false), // 13
                    (6, true),  // 14
                    (5, false), // 15
                    (5, true),  // 16
                    (6, false), // 17
                    (7, true),  // 18
                    (8, false), // 19
                ]
            )
            .unwrap(),
            0,
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.clock i3
.clock i4
.inputs i5
.inputs i6
.outputs o0
.outputs o1
.outputs o2
.outputs o3
.latch o0 i0
.latch o1 i1
.latch o2 i2
.names i0 i1 i7
11 1
.names i2 i3 o1
00 1
.names i3 i4 o2
10 1
.names i5 i6 i10
10 1
01 1
.names i7 o0
0 1
.names i10 o3
0 1
.end
"##,
        to_blif_helper(
            Circuit::new(
                7,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nor(2, 3),
                    Gate::new_nimpl(3, 4),
                    Gate::new_xor(5, 6),
                ],
                [(7, true), (8, false), (9, false), (10, true)]
            )
            .unwrap(),
            3,
            2
        )
        .as_str()
    );
}
