use gateconvert::verilog;
use gatesim::*;

fn to_verilog_helper(circuit: Circuit<usize>, optimize_negs: bool) -> String {
    let mut out = vec![];
    verilog::to_verilog(&circuit, "top", optimize_negs, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_to_verilog() {
    assert_eq!(
        "module top (\n    );\nendmodule\n",
        to_verilog_helper(Circuit::new(0, [], []).unwrap(), false).as_str()
    );
    assert_eq!(
        r##"module top (
    i0,
    i1,
    o0,
    o1,
    o2,
    o3,
    o4,
    o5,
    o6,
    o7);
    input i0;
    input i1;
    output o0;
    output o1;
    output o2;
    output o3;
    output o4;
    output o5;
    output o6;
    output o7;
    assign o0 = (i0 & i1);
    assign o1 = ~(i0 | i1);
    assign o2 = (i0 & ~i1);
    assign o3 = (i0 ^ i1);
    assign o4 = ~o0;
    assign o5 = ~o1;
    assign o6 = ~o2;
    assign o7 = ~o3;
endmodule
"##,
        to_verilog_helper(
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
            false
        )
        .as_str()
    );
    assert_eq!(
        r##"module top (
    i0,
    i1,
    o0,
    o1,
    o2,
    o3,
    o4,
    o5,
    o6,
    o7);
    input i0;
    input i1;
    output o0;
    output o1;
    output o2;
    output o3;
    output o4;
    output o5;
    output o6;
    output o7;
    assign o4 = (i0 & i1);
    assign o5 = ~(i0 | i1);
    assign o6 = (i0 & ~i1);
    assign o7 = (i0 ^ i1);
    assign o0 = ~o4;
    assign o1 = ~o5;
    assign o2 = ~o6;
    assign o3 = ~o7;
endmodule
"##,
        to_verilog_helper(
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
            false
        )
        .as_str()
    );
    assert_eq!(
        r##"module top (
    i0,
    i1,
    o0,
    o1,
    o2,
    o3,
    o4,
    o5,
    o6,
    o7,
    o8,
    o9,
    o10,
    o11,
    o12,
    o13,
    o14,
    o15);
    input i0;
    input i1;
    output o0;
    output o1;
    output o2;
    output o3;
    output o4;
    output o5;
    output o6;
    output o7;
    output o8;
    output o9;
    output o10;
    output o11;
    output o12;
    output o13;
    output o14;
    output o15;
    assign o0 = (i0 & i1);
    assign o1 = ~(i0 | i1);
    assign o2 = (i0 & ~i1);
    assign o3 = (i0 ^ i1);
    assign o4 = ~o0;
    assign o5 = ~o1;
    assign o6 = ~o2;
    assign o7 = ~o3;
    assign o8 = o0;
    assign o9 = o1;
    assign o10 = o2;
    assign o11 = o3;
    assign o12 = o4;
    assign o13 = o5;
    assign o14 = o6;
    assign o15 = o7;
endmodule
"##,
        to_verilog_helper(
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
            false
        )
        .as_str()
    );
    assert_eq!(
        r##"module top (
    i0,
    i1,
    i2,
    i3,
    o0,
    o1,
    o2,
    o3,
    o4);
    input i0;
    input i1;
    input i2;
    input i3;
    output o0;
    output o1;
    output o2;
    output o3;
    output o4;
    wire i5;
    wire i6;
    wire i7;
    wire i9;
    wire i12;
    assign o0 = (i0 & i2);
    assign i5 = (i1 & i2);
    assign i6 = (i0 & i3);
    assign i7 = (i1 & i3);
    assign o1 = (i5 ^ i6);
    assign i9 = (i5 & i6);
    assign o2 = (i7 ^ i9);
    assign o3 = (i7 & i9);
    assign i12 = (o1 ^ o2);
    assign o4 = ~i12;
endmodule
"##,
        to_verilog_helper(
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
            false
        )
        .as_str()
    );
    assert_eq!(
        r##"module top (
    i0,
    i1,
    i2,
    i3,
    i4,
    o0,
    o1,
    o2,
    o3,
    o4,
    o5,
    o6,
    o7,
    o8,
    o9,
    o10,
    o11,
    o12,
    o13,
    o14,
    o15,
    o16,
    o17,
    o18,
    o19);
    input i0;
    input i1;
    input i2;
    input i3;
    input i4;
    output o0;
    output o1;
    output o2;
    output o3;
    output o4;
    output o5;
    output o6;
    output o7;
    output o8;
    output o9;
    output o10;
    output o11;
    output o12;
    output o13;
    output o14;
    output o15;
    output o16;
    output o17;
    output o18;
    output o19;
    assign o4 = (o0 & i1);
    assign o10 = ~(o2 | o4);
    assign o6 = (i3 & ~o10);
    assign o8 = (i4 ^ o6);
    assign o3 = ~o0;
    assign o1 = ~o2;
    assign o11 = ~o4;
    assign o5 = ~o10;
    assign o9 = ~o6;
    assign o7 = ~o8;
    assign o12 = o7;
    assign o13 = o6;
    assign o14 = o5;
    assign o15 = o4;
    assign o16 = o11;
    assign o17 = o10;
    assign o18 = o9;
    assign o19 = o8;
endmodule
"##,
        to_verilog_helper(
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
            false
        )
        .as_str()
    );
}
