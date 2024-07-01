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
}
