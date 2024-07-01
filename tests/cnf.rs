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
