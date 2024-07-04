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
}
