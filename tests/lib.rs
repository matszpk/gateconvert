use gateconvert::*;

#[test]
fn test_assign_map_to_string() {
    assert_eq!(
        concat!(
            "8 0\n10 -\n12 -\n14 -\n16 1\n2 -\n4 -\n6 2\n34 false\n26 5\n",
            "40 true\n36 false\n42 !1\n24 false\n27 !5\n32 false\n41 false\n"
        )
        .to_string(),
        assign_map_to_string(&[
            (8, AssignEntry::Var(0, false)), // 0
            (10, AssignEntry::NoMap),
            (12, AssignEntry::NoMap),
            (14, AssignEntry::NoMap),
            (16, AssignEntry::Var(1, false)),
            (2, AssignEntry::NoMap),
            (4, AssignEntry::NoMap),
            (6, AssignEntry::Var(2, false)),  // 2
            (34, AssignEntry::Value(false)),  // 34 24 26, 24 0 18 -> 34 0 26 -> false
            (26, AssignEntry::Var(5, false)), // ok
            (40, AssignEntry::Value(true)),   // 40 37 39, 36 0 10, 38 0 15 -> true
            (36, AssignEntry::Value(false)),  // false
            (42, AssignEntry::Var(1, true)),  // 42 40 17 -> 42 1 17 -> !16
            (24, AssignEntry::Value(false)),  // false
            (27, AssignEntry::Var(5, true)),  // ok
            (32, AssignEntry::Value(false)),  // 32 28 30, 30 0 15 -> 32 28 0 -> false
            (41, AssignEntry::Value(false))   // false
        ])
    );
}
