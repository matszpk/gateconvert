use gateconvert::cnf;
use gatesim::*;

fn to_cnf_helper(circuit: Circuit<usize>) -> Result<String, String> {
    let mut out = vec![];
    cnf::to_cnf(&circuit, &mut out)
        .map(|_| String::from_utf8(out).unwrap())
        .map_err(|x| x.to_string())
}

#[test]
fn test_to_cnf() {
    assert_eq!(
        Ok("p cnf 4 6\n1 -4 0\n2 -4 0\n-3 -4 0\n-1 -2 3 4 0\n2 4 0\n-2 -4 0\n".to_string()),
        to_cnf_helper(
            Circuit::new(
                3,
                [
                    Gate::new_and(0, 1),
                    Gate::new_nimpl(3, 2),
                    Gate::new_xor(1, 4),
                ],
                [(5, false)]
            )
            .unwrap()
        )
    );
}

fn from_cnf_helper(code: &str) -> Result<(Circuit<usize>, Vec<Option<usize>>), String> {
    let mut b = code.as_bytes();
    cnf::from_cnf(&mut b).map_err(|x| x.to_string())
}

#[test]
fn test_from_cnf() {
    assert_eq!(
        Ok((
            Circuit::new(
                4,
                [
                    Gate::new_nimpl(3, 0),  // !or(1, !4)
                    Gate::new_nimpl(3, 1),  // !or(2, !4)
                    Gate::new_nor(4, 5),    // and(or(1, !4), or(2, !4))
                    Gate::new_and(2, 3),    // !or(!3, !4)
                    Gate::new_nimpl(6, 7),  // and(or(1, !4), or(2, !4), or(!3, !4))
                    Gate::new_and(0, 1),    // !or(!1, !2)
                    Gate::new_nimpl(9, 2),  // !or(or(!1, !2), 3)
                    Gate::new_nimpl(10, 3), // !or(or(or(!1, !2), 3), 4)
                    // and(and(or(1, !4), or(2, !4), or(!3, !4)), or(!1, !2, 3, 4))
                    Gate::new_nimpl(8, 11),
                    Gate::new_nor(1, 3), // !or(2, 4),
                    // and(and(and(or(1, !4), or(2, !4), or(!3, !4)), or(!1, !2, 3, 4)), or(2, 4))
                    Gate::new_nimpl(12, 13),
                    Gate::new_and(1, 3), // or(!2, !4),
                    // and(and(and(and(or(1, !4), or(2, !4), or(!3, !4)), or(!1, !2, 3, 4)),
                    //      or(2, 4)), or(!2, !4))
                    Gate::new_nimpl(14, 15),
                ],
                [(16, false)]
            )
            .unwrap(),
            vec![Some(0), Some(1), Some(2), Some(3)]
        )),
        from_cnf_helper("p cnf 4 6\n1 -4 0\n2 -4 0\n-3 -4 0\n-1 -2 3 4 0\n2 4 0\n-2 -4 0\n"),
    );
    assert_eq!(
        Err("4:4: expected literal or terminating zero, found \"-4x\"".to_string()),
        from_cnf_helper("p cnf 4 6\n1 -4 0\n2 -4 0\n-3 -4x 0\n-1 -2 3 4 0\n2 4 0\n-2 -4 0\n"),
    );
    assert_eq!(
        Ok((
            Circuit::new(
                2,
                [
                    Gate::new_nor(0, 1),  // !or(1, 4)
                ],
                [(2, true)]
            )
            .unwrap(),
            vec![Some(0), None, None, Some(1)]
        )),
        from_cnf_helper("p cnf 4 3\n1 4 0\n3 -3 0\n-2 2 0\n"),
    );
}
