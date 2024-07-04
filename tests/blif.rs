use gateconvert::blif;
use gatesim::*;

fn to_blif_helper(circuit: Circuit<usize>, state_len: usize) -> String {
    let mut out = vec![];
    blif::to_blif(&circuit, state_len, "top", &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_to_blif() {
    assert_eq!(
        ".model top\n.end\n",
        to_blif_helper(Circuit::new(0, [], []).unwrap(), 0).as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.outputs i2
.outputs i3
.outputs i4
.outputs i5
.outputs n2
.outputs n3
.outputs n4
.outputs n5
.names i0 i1 i2
11 1
.names i0 i1 i3
00 1
.names i0 i1 i4
10 1
.names i0 i1 i5
10 1
01 1
.names i2 n2
0 1
.names i3 n3
0 1
.names i4 n4
0 1
.names i5 n5
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
            0
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.outputs i2
.outputs i3
.outputs i4
.outputs i5
.outputs n2
.outputs n3
.outputs n4
.outputs n5
.outputs i2
.outputs i3
.outputs i4
.outputs i5
.outputs n2
.outputs n3
.outputs n4
.outputs n5
.names i0 i1 i2
11 1
.names i0 i1 i3
00 1
.names i0 i1 i4
10 1
.names i0 i1 i5
10 1
01 1
.names i2 n2
0 1
.names i3 n3
0 1
.names i4 n4
0 1
.names i5 n5
0 1
.names i2 n2
0 1
.names i3 n3
0 1
.names i4 n4
0 1
.names i5 n5
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
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.inputs i3
.inputs i4
.outputs n5
.outputs i6
.outputs i7
.outputs n8
.latch n5 i0
.latch i6 i1
.latch i7 i2
.names i0 i1 i5
11 1
.names i2 i3 i6
00 1
.names i0 i2 i7
10 1
.names i1 i4 i8
10 1
01 1
.names i5 n5
0 1
.names i8 n8
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
            3
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
.outputs n5
.outputs i5
.outputs n5
.outputs i6
.outputs i7
.outputs n8
.outputs i6
.latch n5 i0
.latch i5 i1
.latch n5 i2
.names i0 i1 i5
11 1
.names i2 i3 i6
00 1
.names i0 i2 i7
10 1
.names i1 i4 i8
10 1
01 1
.names i5 n5
0 1
.names i5 n5
0 1
.names i8 n8
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
            3
        )
        .as_str()
    );
    assert_eq!(
        r##".model top
.inputs i0
.inputs i1
.inputs i2
.inputs i3
.outputs i4
.outputs i8
.outputs i10
.outputs i11
.outputs n12
.names i0 i2 i4
11 1
.names i1 i2 i5
11 1
.names i0 i3 i6
11 1
.names i1 i3 i7
11 1
.names i5 i6 i8
10 1
01 1
.names i5 i6 i9
11 1
.names i7 i9 i10
10 1
01 1
.names i7 i9 i11
11 1
.names i8 i10 i12
10 1
01 1
.names i12 n12
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
            0
        )
        .as_str()
    );
}
