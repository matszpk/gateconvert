use crate::gatesim::*;

use std::fmt::Debug;

use crate::VNegs::{self, *};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum VGateFunc {
    And,
    Nand,
    Or,
    Nor,
    Impl,
    Nimpl,
    Xor,
}

impl From<GateFunc> for VGateFunc {
    #[inline]
    fn from(gf: GateFunc) -> Self {
        match gf {
            GateFunc::And => VGateFunc::And,
            GateFunc::Nor => VGateFunc::Nor,
            GateFunc::Nimpl => VGateFunc::Nimpl,
            GateFunc::Xor => VGateFunc::Xor,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct VGate<T: Clone + Copy> {
    pub(crate) i0: T,
    pub(crate) i1: T,
    pub(crate) func: VGateFunc,
}

impl<T: Clone + Copy> From<Gate<T>> for VGate<T> {
    fn from(g: Gate<T>) -> Self {
        Self {
            i0: g.i0,
            i1: g.i1,
            func: VGateFunc::from(g.func),
        }
    }
}

impl<T: Clone + Copy> VGate<T> {
    // conversion to operation (and,or,xor,not,impl)
    pub(crate) fn to_binop_and_impl(self) -> (VGate<T>, bool) {
        match self.func {
            VGateFunc::Nand => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::And,
                },
                true,
            ),
            VGateFunc::Nimpl => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::Impl,
                },
                true,
            ),
            VGateFunc::Nor => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::Or,
                },
                true,
            ),
            _ => (self, false),
        }
    }

    // negate argument i0 in gate with conversion to op_and_impl or op_and_nimpl
    pub(crate) fn to_binop_and_ximpl_neg_args(
        self,
        nimpl: bool,
        neg_i0: bool,
        neg_i1: bool,
    ) -> (VGate<T>, bool) {
        let (impl_func, impl_neg) = if nimpl {
            (VGateFunc::Nimpl, true)
        } else {
            (VGateFunc::Impl, false)
        };
        match self.func {
            VGateFunc::And | VGateFunc::Nand => {
                let neg = self.func == VGateFunc::Nand;
                match (neg_i0, neg_i1) {
                    (false, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        neg,
                    ),
                    (true, false) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: impl_func,
                        },
                        !neg ^ impl_neg,
                    ),
                    (false, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: impl_func,
                        },
                        !neg ^ impl_neg,
                    ),
                    (true, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        !neg,
                    ),
                }
            }
            VGateFunc::Or | VGateFunc::Nor => {
                let neg = self.func == VGateFunc::Nor;
                match (neg_i0, neg_i1) {
                    (false, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        neg,
                    ),
                    (true, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: impl_func,
                        },
                        neg ^ impl_neg,
                    ),
                    (false, true) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: impl_func,
                        },
                        neg ^ impl_neg,
                    ),
                    (true, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        !neg,
                    ),
                }
            }
            VGateFunc::Impl | VGateFunc::Nimpl => {
                let neg = self.func == VGateFunc::Nimpl;
                match (neg_i0, neg_i1) {
                    (false, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: impl_func,
                        },
                        neg ^ impl_neg,
                    ),
                    (true, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        neg,
                    ),
                    (false, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        !neg,
                    ),
                    (true, true) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: impl_func,
                        },
                        neg ^ impl_neg,
                    ),
                }
            }
            VGateFunc::Xor => (self, neg_i0 ^ neg_i1),
        }
    }

    // conversion to operation (and,or,xor,not,nimpl)
    pub(crate) fn to_binop_and_nimpl(self) -> (VGate<T>, bool) {
        match self.func {
            VGateFunc::Nand => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::And,
                },
                true,
            ),
            VGateFunc::Impl => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::Nimpl,
                },
                true,
            ),
            VGateFunc::Nor => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::Or,
                },
                true,
            ),
            _ => (self, false),
        }
    }

    // conversion to operation (and,or,xor,not)
    pub(crate) fn to_binop(self) -> (VGate<T>, VNegs) {
        match self.func {
            VGateFunc::Nand => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::And,
                },
                NegOutput,
            ),
            VGateFunc::Nor => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::Or,
                },
                NegOutput,
            ),
            VGateFunc::Impl => (
                VGate {
                    i0: self.i1,
                    i1: self.i0,
                    func: VGateFunc::Or,
                },
                NegInput1,
            ),
            VGateFunc::Nimpl => (
                VGate {
                    i0: self.i0,
                    i1: self.i1,
                    func: VGateFunc::And,
                },
                NegInput1,
            ),
            _ => (self, NoNegs),
        }
    }

    pub(crate) fn binop_neg(self, negs: VNegs) -> (VGate<T>, VNegs) {
        assert!(matches!(
            self.func,
            VGateFunc::And | VGateFunc::Or | VGateFunc::Xor
        ));
        match negs {
            NoNegs => (self, NegOutput),
            NegInput1 => {
                match self.func {
                    VGateFunc::And => (
                        // not and(..,not ..) <-> or(not ..., ...)
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::Or,
                        },
                        NegInput1,
                    ),
                    VGateFunc::Or => (
                        // not or(..,not ..) <-> and(not ..., ...)
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::And,
                        },
                        NegInput1,
                    ),
                    VGateFunc::Xor => (self, NoNegs), // not xor(..., not...) <-> xor(...,...)
                    _ => {
                        panic!("Unsupported");
                    }
                }
            }
            NegOutput => (self, NoNegs),
        }
    }

    pub(crate) fn binop_neg_args(
        self,
        negs: VNegs,
        neg_i0: bool,
        neg_i1: bool,
    ) -> (VGate<T>, VNegs) {
        assert!(matches!(
            self.func,
            VGateFunc::And | VGateFunc::Or | VGateFunc::Xor
        ));
        match negs {
            NoNegs => match self.func {
                VGateFunc::And => match (neg_i0, neg_i1) {
                    (false, false) => (self, NoNegs),
                    (true, false) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::And,
                        },
                        NegInput1,
                    ),
                    (false, true) => (self, NegInput1),
                    (true, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        NegOutput,
                    ),
                },
                VGateFunc::Or => match (neg_i0, neg_i1) {
                    (false, false) => (self, NoNegs),
                    (true, false) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::Or,
                        },
                        NegInput1,
                    ),
                    (false, true) => (self, NegInput1),
                    (true, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        NegOutput,
                    ),
                },
                VGateFunc::Xor => (self, if neg_i0 ^ neg_i1 { NegOutput } else { NoNegs }),
                _ => {
                    panic!("Unsupported");
                }
            },
            NegInput1 => match self.func {
                VGateFunc::And => match (neg_i0, neg_i1) {
                    (false, false) => (self, NegInput1),
                    (true, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        NegOutput,
                    ),
                    (false, true) => (self, NoNegs),
                    (true, true) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::And,
                        },
                        NegInput1,
                    ),
                },
                VGateFunc::Or => match (neg_i0, neg_i1) {
                    (false, false) => (self, NegInput1),
                    (true, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        NegOutput,
                    ),
                    (false, true) => (self, NoNegs),
                    (true, true) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::Or,
                        },
                        NegInput1,
                    ),
                },
                VGateFunc::Xor => (self, if !neg_i0 ^ neg_i1 { NegOutput } else { NoNegs }),
                _ => {
                    panic!("Unsupported");
                }
            },
            NegOutput => match self.func {
                VGateFunc::And => match (neg_i0, neg_i1) {
                    (false, false) => (self, NegOutput),
                    (true, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        NegInput1,
                    ),
                    (false, true) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::Or,
                        },
                        NegInput1,
                    ),
                    (true, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::Or,
                        },
                        NoNegs,
                    ),
                },
                VGateFunc::Or => match (neg_i0, neg_i1) {
                    (false, false) => (self, NegOutput),
                    (true, false) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        NegInput1,
                    ),
                    (false, true) => (
                        VGate {
                            i0: self.i1,
                            i1: self.i0,
                            func: VGateFunc::And,
                        },
                        NegInput1,
                    ),
                    (true, true) => (
                        VGate {
                            i0: self.i0,
                            i1: self.i1,
                            func: VGateFunc::And,
                        },
                        NoNegs,
                    ),
                },
                VGateFunc::Xor => (self, if !neg_i0 ^ neg_i1 { NegOutput } else { NoNegs }),
                _ => {
                    panic!("Unsupported");
                }
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct VCircuit<T: Clone + Copy> {
    pub(crate) input_len: T,
    pub(crate) gates: Vec<VGate<T>>,
    pub(crate) outputs: Vec<(T, bool)>,
}

/// VCircuit - circuit that have basic logic operations (AND, OR, XOR) and implication
/// or negated implication. It can be used if that set of operations is available
/// while generating code.
impl<T> VCircuit<T>
where
    T: Clone + Copy + Ord + PartialEq + Eq,
    T: Default + TryFrom<usize>,
    <T as TryFrom<usize>>::Error: Debug,
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: Debug,
{
    pub(crate) fn to_op_and_ximpl_circuit(circuit: Circuit<T>, nimpl: bool) -> VCircuit<T> {
        #[derive(Clone, Copy)]
        struct StackEntry {
            node: usize,
            way: usize,
        }

        let input_len_t = circuit.input_len();
        let input_len = usize::try_from(input_len_t).unwrap();
        let gate_num = circuit.len();
        let mut gates = circuit
            .gates()
            .into_iter()
            .map(|g| (VGate::from(*g), false))
            .collect::<Vec<_>>();

        let mut visited = vec![false; gate_num];
        for (o, _) in circuit.outputs().into_iter() {
            if *o < input_len_t {
                continue;
            }
            let oidx = usize::try_from(*o).unwrap() - input_len;
            let mut stack = Vec::new();
            stack.push(StackEntry { node: oidx, way: 0 });

            while !stack.is_empty() {
                let top = stack.last_mut().unwrap();
                let node_index = top.node;
                let way = top.way;

                if way == 0 {
                    if !visited[node_index] {
                        visited[node_index] = true;
                    } else {
                        stack.pop();
                        continue;
                    }

                    top.way += 1;
                    let gi0 = gates[node_index].0.i0;
                    if gi0 >= input_len_t {
                        stack.push(StackEntry {
                            node: usize::try_from(gi0).unwrap() - input_len,
                            way: 0,
                        });
                    }
                } else if way == 1 {
                    top.way += 1;
                    let gi1 = gates[node_index].0.i1;
                    if gi1 >= input_len_t {
                        stack.push(StackEntry {
                            node: usize::try_from(gi1).unwrap() - input_len,
                            way: 0,
                        });
                    }
                } else {
                    // resolve
                    assert!(!gates[node_index].1);
                    let (newg, n) = if nimpl {
                        gates[node_index].0.to_binop_and_nimpl()
                    } else {
                        gates[node_index].0.to_binop_and_impl()
                    };
                    let ni0 = if newg.i0 >= input_len_t {
                        gates[usize::try_from(newg.i0).unwrap() - input_len].1
                    } else {
                        false
                    };
                    let ni1 = if newg.i1 >= input_len_t {
                        gates[usize::try_from(newg.i1).unwrap() - input_len].1
                    } else {
                        false
                    };
                    gates[node_index] = {
                        let (newg2, n2) = newg.to_binop_and_ximpl_neg_args(nimpl, ni0, ni1);
                        (newg2, n ^ n2)
                    };
                    stack.pop();
                }
            }
        }

        let outputs = circuit
            .outputs()
            .into_iter()
            .map(|(o, on)| {
                if *o >= input_len_t {
                    (*o, on ^ gates[usize::try_from(*o).unwrap() - input_len].1)
                } else {
                    (*o, *on)
                }
            })
            .collect::<Vec<_>>();
        VCircuit {
            input_len: circuit.input_len(),
            gates: gates.into_iter().map(|(g, _)| g).collect::<Vec<_>>(),
            outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vgate<T: Clone + Copy>(func: VGateFunc, i0: T, i1: T) -> VGate<T> {
        VGate { i0, i1, func: func }
    }

    fn vgate_and<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::And, i0, i1)
    }
    fn vgate_nand<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Nand, i0, i1)
    }
    fn vgate_or<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Or, i0, i1)
    }
    fn vgate_nor<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Nor, i0, i1)
    }
    fn vgate_impl<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Impl, i0, i1)
    }
    fn vgate_nimpl<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Nimpl, i0, i1)
    }
    fn vgate_xor<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Xor, i0, i1)
    }

    #[test]
    fn test_vgate() {
        assert_eq!(vgate_and(3, 4), VGate::from(Gate::new_and(3, 4)));
        assert_eq!(vgate_nor(3, 5), VGate::from(Gate::new_nor(3, 5)));
        assert_eq!(vgate_nimpl(2, 4), VGate::from(Gate::new_nimpl(2, 4)));
        assert_eq!(vgate_xor(2, 6), VGate::from(Gate::new_xor(2, 6)));
    }

    #[test]
    fn test_vgate_to_binop_and_ximpl() {
        // binop_and_impl
        assert_eq!(
            (vgate_and(3, 5), false),
            vgate_and(3, 5).to_binop_and_impl()
        );
        assert_eq!(
            (vgate_and(3, 5), true),
            vgate_nand(3, 5).to_binop_and_impl()
        );
        assert_eq!((vgate_or(3, 5), false), vgate_or(3, 5).to_binop_and_impl());
        assert_eq!((vgate_or(3, 5), true), vgate_nor(3, 5).to_binop_and_impl());
        assert_eq!(
            (vgate_impl(3, 5), false),
            vgate_impl(3, 5).to_binop_and_impl()
        );
        assert_eq!(
            (vgate_impl(3, 5), true),
            vgate_nimpl(3, 5).to_binop_and_impl()
        );
        assert_eq!(
            (vgate_xor(3, 5), false),
            vgate_xor(3, 5).to_binop_and_impl()
        );

        // binop_and_nimpl
        assert_eq!(
            (vgate_and(3, 5), false),
            vgate_and(3, 5).to_binop_and_nimpl()
        );
        assert_eq!(
            (vgate_and(3, 5), true),
            vgate_nand(3, 5).to_binop_and_nimpl()
        );
        assert_eq!((vgate_or(3, 5), false), vgate_or(3, 5).to_binop_and_nimpl());
        assert_eq!((vgate_or(3, 5), true), vgate_nor(3, 5).to_binop_and_nimpl());
        assert_eq!(
            (vgate_nimpl(3, 5), true),
            vgate_impl(3, 5).to_binop_and_nimpl()
        );
        assert_eq!(
            (vgate_nimpl(3, 5), false),
            vgate_nimpl(3, 5).to_binop_and_nimpl()
        );
        assert_eq!(
            (vgate_xor(3, 5), false),
            vgate_xor(3, 5).to_binop_and_nimpl()
        );

        // to_binop
        assert_eq!((vgate_and(3, 5), NoNegs), vgate_and(3, 5).to_binop());
        assert_eq!((vgate_and(3, 5), NegOutput), vgate_nand(3, 5).to_binop());
        assert_eq!((vgate_or(3, 5), NoNegs), vgate_or(3, 5).to_binop());
        assert_eq!((vgate_or(3, 5), NegOutput), vgate_nor(3, 5).to_binop());
        assert_eq!((vgate_or(5, 3), NegInput1), vgate_impl(3, 5).to_binop());
        assert_eq!((vgate_and(3, 5), NegInput1), vgate_nimpl(3, 5).to_binop());
        assert_eq!((vgate_xor(3, 5), NoNegs), vgate_xor(3, 5).to_binop());
    }

    fn vgate_eval<T: Clone + Copy>(g: VGate<T>, neg_i0: bool, neg_i1: bool) -> u8
    where
        usize: TryFrom<T>,
        <usize as TryFrom<T>>::Error: Debug,
    {
        let i0 = usize::try_from(g.i0).unwrap();
        let i1 = usize::try_from(g.i1).unwrap();
        let neg_i0_mask = if neg_i0 { 0b1111 } else { 0 };
        let neg_i1_mask = if neg_i1 { 0b1111 } else { 0 };
        let v0 = (if i0 == 0 { 0b1010 } else { 0b1100 }) ^ neg_i0_mask;
        let v1 = (if i1 == 0 { 0b1010 } else { 0b1100 }) ^ neg_i1_mask;
        let out = match g.func {
            VGateFunc::And => v0 & v1,
            VGateFunc::Nand => !(v0 & v1),
            VGateFunc::Or => v0 | v1,
            VGateFunc::Nor => !(v0 | v1),
            VGateFunc::Impl => !v0 | v1,
            VGateFunc::Nimpl => !(!v0 | v1),
            VGateFunc::Xor => v0 ^ v1,
        };
        out & 0b1111
    }

    #[test]
    fn test_vgate_to_binop_and_ximpl_neg_args() {
        for func in [
            VGateFunc::And,
            VGateFunc::Nand,
            VGateFunc::Or,
            VGateFunc::Nor,
            VGateFunc::Impl,
            VGateFunc::Nimpl,
            VGateFunc::Xor,
        ] {
            for t in 0..8 {
                let neg_i0 = (t & 1) != 0;
                let neg_i1 = (t & 2) != 0;
                let nimpl = (t & 4) != 0;
                let orig = VGate { i0: 0, i1: 1, func };
                let exp_value = vgate_eval(orig, neg_i0, neg_i1);
                let (trans, neg) = orig.to_binop_and_ximpl_neg_args(nimpl, neg_i0, neg_i1);
                let res_value = vgate_eval(trans, false, false) ^ (if neg { 0b1111 } else { 0 });
                assert_eq!(
                    exp_value,
                    res_value,
                    "func:{:?} nimpl:{} n0:{} n1:{}: trans:{:?}",
                    func,
                    nimpl,
                    neg_i0,
                    neg_i1,
                    (trans, neg)
                );
                let ximpl_func = if nimpl {
                    VGateFunc::Nimpl
                } else {
                    VGateFunc::Impl
                };
                assert!(
                    matches!(trans.func, VGateFunc::And | VGateFunc::Or | VGateFunc::Xor)
                        || trans.func == ximpl_func,
                    "func:{:?} nimpl:{} n0:{} n1:{}: trans:{:?}",
                    func,
                    nimpl,
                    neg_i0,
                    neg_i1,
                    (trans, neg)
                );
            }
        }
    }

    #[test]
    fn test_vgate_binop_neg() {
        for func in [VGateFunc::And, VGateFunc::Or, VGateFunc::Xor] {
            for negs in [NoNegs, NegInput1, NegOutput] {
                let orig = VGate { i0: 0, i1: 1, func };
                let exp_value = vgate_eval(orig, false, matches!(negs, NegInput1))
                    ^ (if matches!(negs, NegOutput) { 0b1111 } else { 0 })
                    // vvv - apply negation
                    ^ 0b1111;
                let (trans, tnegs) = orig.binop_neg(negs);
                let res_value = vgate_eval(trans, false, matches!(tnegs, NegInput1))
                    ^ (if matches!(tnegs, NegOutput) {
                        0b1111
                    } else {
                        0
                    });
                assert_eq!(
                    exp_value, res_value,
                    "func:{:?} negs:{:?}: trans:{:?} tnegs:{:?}",
                    func, negs, trans, tnegs,
                );
                assert!(
                    matches!(trans.func, VGateFunc::And | VGateFunc::Or | VGateFunc::Xor),
                    "func:{:?} negs:{:?}: trans:{:?} tnegs:{:?}",
                    func,
                    negs,
                    trans,
                    tnegs,
                );
            }
        }
    }

    #[test]
    fn test_vgate_binop_neg_args() {
        for func in [VGateFunc::And, VGateFunc::Or, VGateFunc::Xor] {
            for negs in [NoNegs, NegInput1, NegOutput] {
                for t in 0..4 {
                    let neg_i0 = (t & 1) != 0;
                    let neg_i1 = (t & 2) != 0;
                    let orig = VGate { i0: 0, i1: 1, func };
                    let exp_value = vgate_eval(orig, neg_i0, neg_i1 ^ matches!(negs, NegInput1))
                        ^ (if matches!(negs, NegOutput) { 0b1111 } else { 0 });
                    let (trans, tnegs) = orig.binop_neg_args(negs, neg_i0, neg_i1);
                    let res_value = vgate_eval(trans, false, matches!(tnegs, NegInput1))
                        ^ (if matches!(tnegs, NegOutput) {
                            0b1111
                        } else {
                            0
                        });
                    assert_eq!(
                        exp_value, res_value,
                        "func:{:?} negs:{:?} ni0:{} ni1:{}: trans:{:?} tnegs:{:?}",
                        func, negs, neg_i0, neg_i1, trans, tnegs,
                    );
                    assert!(
                        matches!(trans.func, VGateFunc::And | VGateFunc::Or | VGateFunc::Xor),
                        "func:{:?} negs:{:?} ni0:{} ni1:{}: trans:{:?} tnegs:{:?}",
                        func,
                        negs,
                        neg_i0,
                        neg_i1,
                        trans,
                        tnegs,
                    );
                }
            }
        }
    }

    #[test]
    fn test_vcircuit_to_op_and_ximpl_circuit() {
        for (nimpl, gate, vgate, vout_neg) in [
            (false, Gate::new_nimpl(0, 1), vgate_impl(0, 1), true),
            (false, Gate::new_nor(0, 1), vgate_or(0, 1), true),
            (true, Gate::new_nimpl(0, 1), vgate_nimpl(0, 1), false),
            (true, Gate::new_nor(0, 1), vgate_or(0, 1), true),
        ] {
            assert_eq!(
                VCircuit {
                    input_len: 2,
                    gates: vec![vgate],
                    outputs: vec![(2, vout_neg)],
                },
                VCircuit::to_op_and_ximpl_circuit(
                    Circuit::new(2, [gate], [(2, false)],).unwrap(),
                    nimpl
                ),
                "{} {:?} {:?} {}",
                nimpl,
                gate,
                vgate,
                vout_neg
            );
        }

        assert_eq!(
            VCircuit {
                input_len: 3,
                gates: vec![
                    vgate_xor(0, 1),
                    vgate_xor(2, 3),
                    vgate_and(2, 3),
                    vgate_and(0, 1),
                    vgate_or(5, 6),
                ],
                outputs: vec![(4, false), (7, false)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    3,
                    [
                        Gate::new_xor(0, 1),
                        Gate::new_xor(2, 3),
                        Gate::new_and(2, 3),
                        Gate::new_and(0, 1),
                        Gate::new_nor(5, 6),
                    ],
                    [(4, false), (7, true)],
                )
                .unwrap(),
                false
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 3,
                gates: vec![
                    vgate_impl(0, 1),
                    vgate_xor(2, 3),
                    vgate_and(2, 3),
                    vgate_and(0, 1),
                    vgate_or(5, 6),
                ],
                outputs: vec![(4, false), (7, true)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    3,
                    [
                        Gate::new_nimpl(0, 1), // not impl(0,1)
                        Gate::new_xor(2, 3),   // xor(2,not impl(0,1)) = not xor(2,3)
                        Gate::new_nimpl(2, 3), // nimpl(2,not impl(0,1)) = and(2,3)
                        Gate::new_and(0, 1),
                        Gate::new_nor(5, 6),
                    ],
                    [(4, true), (7, false)],
                )
                .unwrap(),
                false
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 3,
                gates: vec![
                    vgate_nimpl(0, 1),
                    vgate_xor(2, 3),
                    vgate_nimpl(2, 3),
                    vgate_and(0, 1),
                    vgate_or(5, 6),
                ],
                outputs: vec![(4, true), (7, true)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    3,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_xor(2, 3),
                        Gate::new_nimpl(2, 3),
                        Gate::new_and(0, 1),
                        Gate::new_nor(5, 6),
                    ],
                    [(4, true), (7, false)],
                )
                .unwrap(),
                true
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![
                    vgate_impl(0, 1),
                    vgate_or(2, 3),
                    vgate_or(1, 3),
                    vgate_xor(1, 4),
                    vgate_xor(5, 7),
                    vgate_xor(6, 8),
                    vgate_impl(3, 9),
                ],
                outputs: vec![(7, false), (8, false), (9, false), (10, true)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_nor(2, 3),
                        Gate::new_nor(1, 3),
                        Gate::new_xor(1, 4), // xor(not,..) = not xor(..,..)
                        Gate::new_xor(5, 7), // xor(not...,xor(not,..)) = xor(xor(..),..)
                        Gate::new_xor(6, 8), // xor(not..,xor(not xor(...) = not xor(...)
                        Gate::new_and(3, 9), // and(..,not..) = not impl(...,...)
                    ],
                    [(7, true), (8, false), (9, true), (10, false)],
                )
                .unwrap(),
                false
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![
                    vgate_nimpl(0, 1),
                    vgate_or(2, 3),
                    vgate_or(1, 3),
                    vgate_xor(1, 4),
                    vgate_xor(5, 7),
                    vgate_xor(6, 8),
                    vgate_and(3, 9),
                ],
                outputs: vec![(7, true), (8, true), (9, true), (10, false)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_nor(2, 3),
                        Gate::new_nor(1, 3),
                        Gate::new_xor(1, 4),
                        Gate::new_xor(5, 7),
                        Gate::new_xor(6, 8),
                        Gate::new_and(3, 9),
                    ],
                    [(7, true), (8, false), (9, true), (10, false)],
                )
                .unwrap(),
                true
            )
        );

        // next testcases
        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![
                    vgate_impl(0, 1),
                    vgate_or(2, 3),
                    vgate_or(1, 3),
                    vgate_and(1, 4),
                    vgate_or(5, 7),
                    vgate_and(6, 8),
                    vgate_xor(3, 4),
                    vgate_or(4, 9),
                ],
                outputs: vec![
                    (5, true),
                    (7, true),
                    (8, true),
                    (9, true),
                    (10, true),
                    (11, false)
                ],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1), // -> not :impl(0, 1)
                        Gate::new_nor(2, 3),   // -> not :or(2, 3)
                        Gate::new_nor(1, 3),   // -> not :or(1, 3)
                        Gate::new_nimpl(1, 4), // -> not: or(not 1, not 4) -> and(1, 4)
                        Gate::new_nimpl(5, 7), // -> not :impl(not 5, 7) -> not: or(5, 7)
                        Gate::new_nor(6, 8),   // -> not :or(not 6, not 8) -> and(6, 8)
                        Gate::new_xor(3, 4),   // -> xor(3, not 4) -> not: xor(3, 4)
                        Gate::new_nimpl(4, 9), // -> not: impl(not 4, 9) -> not: or(4, 9)
                    ],
                    [
                        (5, false),
                        (7, true),
                        (8, false),
                        (9, true),
                        (10, false),
                        (11, true)
                    ],
                )
                .unwrap(),
                false
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![
                    vgate_nimpl(0, 1),
                    vgate_or(2, 3),
                    vgate_or(1, 3),
                    vgate_nimpl(1, 4),
                    vgate_or(5, 7),
                    vgate_and(6, 8),
                    vgate_xor(3, 4),
                    vgate_nimpl(4, 9),
                ],
                outputs: vec![
                    (5, true),
                    (7, true),
                    (8, true),
                    (9, true),
                    (10, false),
                    (11, true)
                ],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_nor(2, 3), // -> not :or(2, 3)
                        Gate::new_nor(1, 3), // -> not :or(1, 3)
                        Gate::new_nimpl(1, 4),
                        Gate::new_nimpl(5, 7), // -> nimpl(not 5, 7) -> not: or(5, 7)
                        Gate::new_nor(6, 8),   // -> not :or(not 6, not 8) -> and(6, 8)
                        Gate::new_xor(3, 4),
                        Gate::new_nimpl(4, 9),
                    ],
                    [
                        (5, false),
                        (7, true),
                        (8, false),
                        (9, true),
                        (10, false),
                        (11, true)
                    ],
                )
                .unwrap(),
                true
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![vgate_impl(0, 1), vgate_or(2, 3), vgate_impl(5, 4)],
                outputs: vec![(4, true), (5, false), (6, true)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1), // -> not :impl(0, 1)
                        Gate::new_nor(2, 3),   // -> not :or(2, 3)
                        Gate::new_nimpl(4, 5), // -> not: impl(not 4, not 5) -> impl(4, 5)
                    ],
                    [(4, false), (5, true), (6, false)],
                )
                .unwrap(),
                false
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![vgate_nimpl(0, 1), vgate_or(2, 3), vgate_and(4, 5)],
                outputs: vec![(4, false), (5, false), (6, false)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1), // -> nimpl(0, 1)
                        Gate::new_nor(2, 3),   // -> not :or(2, 3)
                        Gate::new_nimpl(4, 5), // -> nimpl(4, not 5) -> and(4, 5)
                    ],
                    [(4, false), (5, true), (6, false)],
                )
                .unwrap(),
                true
            )
        );

        assert_eq!(
            VCircuit {
                input_len: 4,
                gates: vec![
                    vgate_impl(0, 1),
                    vgate_or(2, 3),
                    vgate_impl(5, 4),
                    vgate_xor(4, 5)
                ],
                outputs: vec![(4, true), (5, false), (6, true), (7, false)],
            },
            VCircuit::to_op_and_ximpl_circuit(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1), // -> not :impl(0, 1)
                        Gate::new_nor(2, 3),   // -> not :or(2, 3)
                        Gate::new_nimpl(4, 5), // -> not: impl(not 4, not 5) -> impl(4, 5)
                        Gate::new_xor(4, 5),   // -> not: impl(not 4, not 5) -> impl(4, 5)
                    ],
                    [(4, false), (5, true), (6, false), (7, false)],
                )
                .unwrap(),
                false
            )
        );
    }
}
