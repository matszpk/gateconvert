use gateconvert::vhdl;
use gatesim::*;

fn to_vhdl_helper(circuit: Circuit<usize>, optimize_negs: bool) -> String {
    let mut out = vec![];
    vhdl::to_vhdl(&circuit, "top", "behavior", optimize_negs, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_to_vhdl() {
    assert_eq!(
        r##"library ieee;
use ieee.std_logic_1164.all;
entity top is
    port(
    );
end top;
architecture behavior of top is
begin
end behavior;
"##,
        to_vhdl_helper(Circuit::new(0, [], []).unwrap(), false).as_str()
    );
    assert_eq!(
        r##"library ieee;
use ieee.std_logic_1164.all;
entity top is
    port(
        i0 : in std_logic;
        i1 : in std_logic;
        o0 : out std_logic;
        o1 : out std_logic;
        o2 : out std_logic;
        o3 : out std_logic;
        o4 : out std_logic;
        o5 : out std_logic;
        o6 : out std_logic;
        o7 : out std_logic
    );
end top;
architecture behavior of top is
begin
    o0 <= i0 and i1;
    o1 <= i0 nor i1;
    o2 <= i0 and not i1;
    o3 <= i0 xor i1;
    o4 <= not o0;
    o5 <= not o1;
    o6 <= not o2;
    o7 <= not o3;
end behavior;
"##,
        to_vhdl_helper(
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
        r##"library ieee;
use ieee.std_logic_1164.all;
entity top is
    port(
        i0 : in std_logic;
        i1 : in std_logic;
        o0 : out std_logic;
        o1 : out std_logic;
        o2 : out std_logic;
        o3 : out std_logic;
        o4 : out std_logic;
        o5 : out std_logic;
        o6 : out std_logic;
        o7 : out std_logic
    );
end top;
architecture behavior of top is
begin
    o4 <= i0 and i1;
    o5 <= i0 nor i1;
    o6 <= i0 and not i1;
    o7 <= i0 xor i1;
    o0 <= not o4;
    o1 <= not o5;
    o2 <= not o6;
    o3 <= not o7;
end behavior;
"##,
        to_vhdl_helper(
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
        r##"library ieee;
use ieee.std_logic_1164.all;
entity top is
    port(
        i0 : in std_logic;
        i1 : in std_logic;
        o0 : out std_logic;
        o1 : out std_logic;
        o2 : out std_logic;
        o3 : out std_logic;
        o4 : out std_logic;
        o5 : out std_logic;
        o6 : out std_logic;
        o7 : out std_logic;
        o8 : out std_logic;
        o9 : out std_logic;
        o10 : out std_logic;
        o11 : out std_logic;
        o12 : out std_logic;
        o13 : out std_logic;
        o14 : out std_logic;
        o15 : out std_logic
    );
end top;
architecture behavior of top is
begin
    o0 <= i0 and i1;
    o1 <= i0 nor i1;
    o2 <= i0 and not i1;
    o3 <= i0 xor i1;
    o4 <= not o0;
    o5 <= not o1;
    o6 <= not o2;
    o7 <= not o3;
    o8 <= o0;
    o9 <= o1;
    o10 <= o2;
    o11 <= o3;
    o12 <= o4;
    o13 <= o5;
    o14 <= o6;
    o15 <= o7;
end behavior;
"##,
        to_vhdl_helper(
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
        r##"library ieee;
use ieee.std_logic_1164.all;
entity top is
    port(
        i0 : in std_logic;
        i1 : in std_logic;
        i2 : in std_logic;
        i3 : in std_logic;
        o0 : out std_logic;
        o1 : out std_logic;
        o2 : out std_logic;
        o3 : out std_logic;
        o4 : out std_logic
    );
end top;
architecture behavior of top is
    signal i5 : std_logic;
    signal i6 : std_logic;
    signal i7 : std_logic;
    signal i9 : std_logic;
    signal i12 : std_logic;
begin
    o0 <= i0 and i2;
    i5 <= i1 and i2;
    i6 <= i0 and i3;
    i7 <= i1 and i3;
    o1 <= i5 xor i6;
    i9 <= i5 and i6;
    o2 <= i7 xor i9;
    o3 <= i7 and i9;
    i12 <= o1 xor o2;
    o4 <= not i12;
end behavior;
"##,
        to_vhdl_helper(
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
        r##"library ieee;
use ieee.std_logic_1164.all;
entity top is
    port(
        i0 : in std_logic;
        i1 : in std_logic;
        i2 : in std_logic;
        i3 : in std_logic;
        i4 : in std_logic;
        o0 : out std_logic;
        o1 : out std_logic;
        o2 : out std_logic;
        o3 : out std_logic;
        o4 : out std_logic;
        o5 : out std_logic;
        o6 : out std_logic;
        o7 : out std_logic;
        o8 : out std_logic;
        o9 : out std_logic;
        o10 : out std_logic;
        o11 : out std_logic;
        o12 : out std_logic;
        o13 : out std_logic;
        o14 : out std_logic;
        o15 : out std_logic;
        o16 : out std_logic;
        o17 : out std_logic;
        o18 : out std_logic;
        o19 : out std_logic
    );
end top;
architecture behavior of top is
begin
    o4 <= o0 and i1;
    o10 <= o2 nor o4;
    o6 <= i3 and not o10;
    o8 <= i4 xor o6;
    o3 <= not o0;
    o1 <= not o2;
    o11 <= not o4;
    o5 <= not o10;
    o9 <= not o6;
    o7 <= not o8;
    o12 <= o7;
    o13 <= o6;
    o14 <= o5;
    o15 <= o4;
    o16 <= o11;
    o17 <= o10;
    o18 <= o9;
    o19 <= o8;
end behavior;
"##,
        to_vhdl_helper(
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
