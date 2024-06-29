use gatesim::*;

use crate::vcircuit::*;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;

use crate::VNegs::{self, *};

/// VBinOpCircuit - circuit that have only basic logic operations (AND, OR, XOR)
/// with boolean negations. It can be used if generator can use only basic logical operations.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct VBinOpCircuit<T: Clone + Copy> {
    pub(crate) input_len: T,
    pub(crate) gates: Vec<(VGate<T>, VNegs)>,
    pub(crate) outputs: Vec<(T, bool)>,
}

impl<T: Clone + Copy> From<Circuit<T>> for VBinOpCircuit<T> {
    fn from(circuit: Circuit<T>) -> VBinOpCircuit<T> {
        VBinOpCircuit {
            input_len: circuit.input_len(),
            gates: circuit
                .gates()
                .into_iter()
                .map(|g| VGate::from(*g).to_binop())
                .collect::<Vec<_>>(),
            outputs: circuit.outputs().to_vec(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct SubTree<T> {
    root: T,
    // gates: gate entry: first - index in circuit gates
    // second - index in circuit gates of successor
    gates: Vec<(T, T)>,
}

impl<T: Clone + Copy + Ord> SubTree<T> {
    fn sort_and_dedup(&mut self) {
        self.gates.sort();
        self.gates.dedup();
    }
}
impl<T> SubTree<T>
where
    T: Clone + Copy + Ord + PartialEq + Eq + Hash,
    T: Default + TryFrom<usize>,
    <T as TryFrom<usize>>::Error: Debug,
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: Debug,
{
    // gates must be ordered by index. last gate MUST BE in root.
    fn fill_successors(&mut self, circuit: &VBinOpCircuit<T>) {
        let input_len = usize::try_from(circuit.input_len).unwrap();
        for i in 0..self.gates.len() {
            let oi = usize::try_from(self.gates[i].0).unwrap() - input_len;
            let g = circuit.gates[oi].0;
            if g.i0 >= circuit.input_len {
                if let Ok(p) = self.gates[0..i].binary_search(&(g.i0, T::default())) {
                    self.gates[p].1 = self.gates[i].0;
                }
            }
            if g.i1 >= circuit.input_len {
                if let Ok(p) = self.gates[0..i].binary_search(&(g.i1, T::default())) {
                    self.gates[p].1 = self.gates[i].0;
                }
            }
        }
        self.gates.pop();
    }

    #[inline]
    pub(crate) fn find_index(&self, t: T) -> Option<usize> {
        if t == self.root {
            Some(self.gates.len())
        } else {
            self.gates.binary_search_by_key(&t, |(x, _)| *x).ok()
        }
    }

    #[inline]
    pub(crate) fn gates(&self) -> &[(T, T)] {
        &self.gates
    }
    #[inline]
    pub(crate) fn root(&self) -> T {
        self.root
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct SubTreeCopy<'a, T: Clone + Copy> {
    input_len: T,
    subtree: &'a SubTree<T>,
    gates: Vec<(VGate<T>, VNegs)>,
}

impl<'a, T> SubTreeCopy<'a, T>
where
    T: Clone + Copy + Ord + PartialEq + Eq + Hash,
    T: Default + TryFrom<usize>,
    <T as TryFrom<usize>>::Error: Debug,
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: Debug,
{
    fn new(circuit: &VBinOpCircuit<T>, subtree: &'a SubTree<T>) -> Self {
        let input_len = usize::try_from(circuit.input_len).unwrap();
        Self {
            input_len: circuit.input_len,
            subtree,
            gates: subtree
                .gates
                .iter()
                .copied()
                .chain(std::iter::once((subtree.root, T::default())))
                .map(|(x, _)| circuit.gates[usize::try_from(x).unwrap() - input_len])
                .collect::<Vec<_>>(),
        }
    }

    #[inline]
    fn gate_index(&self, t: T) -> Option<usize> {
        self.subtree.find_index(t)
    }

    #[inline]
    fn circuit_index(&self, index: usize) -> T {
        if index < self.subtree.gates.len() {
            self.subtree.gates[index].0
        } else {
            self.subtree.root
        }
    }

    fn optimize_negs(&mut self) {
        for (oi, (i, next_i)) in self.subtree.gates.iter().enumerate() {
            let next_oi = self.gate_index(*next_i).unwrap();
            let (g, neg) = self.gates[oi];
            let (next_g, next_neg) = self.gates[next_oi];
            if neg != NoNegs {
                let (new_g, new_neg) = g.binop_neg(neg);
                let (new_next_g, new_next_neg) = if next_g.i0 == *i {
                    next_g.binop_neg_args(next_neg, true, false)
                } else {
                    next_g.binop_neg_args(next_neg, false, true)
                };
                let old_neg_count = usize::from(neg != NoNegs) + usize::from(next_neg != NoNegs);
                let new_neg_count =
                    usize::from(new_neg != NoNegs) + usize::from(new_next_neg != NoNegs);
                if old_neg_count > new_neg_count
                    || (old_neg_count == new_neg_count
                        && (new_next_neg != NegInput1 || next_neg != NegOutput))
                {
                    // apply if negation reduction or negation move forward.
                    self.gates[oi] = (new_g, new_neg);
                    self.gates[next_oi] = (new_next_g, new_next_neg);
                }
            }
            // check single reduction subtree.
            let (next_g, next_neg) = self.gates[next_oi];
            let (roi, in_next) = if (next_neg == NegInput1) && *i == next_g.i1 {
                // if this fork (from negated input)
                (Some(oi), true)
            } else if next_neg == NegOutput {
                (Some(next_oi), false)
            } else {
                (None, false)
            };

            if let Some(roi) = roi {
                let (rg, rneg) = self.gates[roi];
                if rg.i0 < self.input_len || rg.i1 < self.input_len {
                    continue;
                }
                let rg_oi0 = self.gate_index(rg.i0);
                let rg_oi1 = self.gate_index(rg.i1);
                if rg_oi0.is_none() || rg_oi1.is_none() {
                    continue;
                }
                let rg_oi0 = rg_oi0.unwrap();
                let rg_oi1 = rg_oi1.unwrap();
                let (rg0g, rg0neg) = self.gates[rg_oi0];
                let (rg1g, rg1neg) = self.gates[rg_oi1];
                if rg.func == VGateFunc::Xor
                    || rg0g.func == VGateFunc::Xor
                    || rg1g.func == VGateFunc::Xor
                {
                    continue;
                }

                if rg0neg == NegInput1 && rg1neg == NegInput1 {
                    // found - just change subtree.
                    self.gates[rg_oi0] = rg0g.binop_neg(rg0neg);
                    self.gates[rg_oi1] = rg1g.binop_neg(rg1neg);
                    self.gates[roi] = rg.binop_neg_args(rneg, true, true);
                    if in_next {
                        // propagate to next after ROI
                        self.gates[roi].1 = NoNegs;
                        self.gates[next_oi].1 = NoNegs;
                    }
                    continue;
                }
                if rg0g.i0 < self.input_len
                    || rg0g.i1 < self.input_len
                    || rg1g.i0 < self.input_len
                    || rg1g.i1 < self.input_len
                {
                    continue;
                }
                let rg_oi00 = self.gate_index(rg0g.i0);
                let rg_oi01 = self.gate_index(rg0g.i1);
                let rg_oi10 = self.gate_index(rg1g.i0);
                let rg_oi11 = self.gate_index(rg1g.i1);
                if rg_oi00.is_none() || rg_oi01.is_none() || rg_oi10.is_none() || rg_oi11.is_none()
                {
                    continue;
                }
                let rg_oi00 = rg_oi00.unwrap();
                let rg_oi01 = rg_oi01.unwrap();
                let rg_oi10 = rg_oi10.unwrap();
                let rg_oi11 = rg_oi11.unwrap();
                let (rg00g, rg00neg) = self.gates[rg_oi00];
                let (rg01g, rg01neg) = self.gates[rg_oi01];
                let (rg10g, rg10neg) = self.gates[rg_oi10];
                let (rg11g, rg11neg) = self.gates[rg_oi11];
                if rg00g.func == VGateFunc::Xor
                    || rg01g.func == VGateFunc::Xor
                    || rg10g.func == VGateFunc::Xor
                    || rg11g.func == VGateFunc::Xor
                {
                    continue;
                }
                if rg00neg == NegInput1
                    && rg01neg == NegInput1
                    && rg10neg == NegInput1
                    && rg11neg == NegInput1
                    && rg0neg == NoNegs
                    && rg1neg == NoNegs
                {
                    // found - just change subtree.
                    self.gates[rg_oi00] = rg00g.binop_neg(rg00neg);
                    self.gates[rg_oi01] = rg01g.binop_neg(rg01neg);
                    self.gates[rg_oi10] = rg10g.binop_neg(rg10neg);
                    self.gates[rg_oi11] = rg11g.binop_neg(rg11neg);
                    self.gates[rg_oi0] = rg0g.binop_neg_args(rg0neg, true, true);
                    self.gates[rg_oi1] = rg1g.binop_neg_args(rg1neg, true, true);
                    let (rg0g, rg0neg) = self.gates[rg_oi0];
                    let (rg1g, rg1neg) = self.gates[rg_oi1];
                    self.gates[rg_oi0] = rg0g.binop_neg(rg0neg);
                    self.gates[rg_oi1] = rg1g.binop_neg(rg1neg);
                    self.gates[roi] = rg.binop_neg_args(rneg, true, true);
                    if in_next {
                        // propagate to next after ROI
                        self.gates[roi].1 = NoNegs;
                        self.gates[next_oi].1 = NoNegs;
                    }
                }
            }
        }
    }

    #[inline]
    fn count_negs(&self) -> usize {
        self.gates.iter().filter(|(_, n)| *n != NoNegs).count()
    }
}

impl<T> VBinOpCircuit<T>
where
    T: Clone + Copy + Ord + PartialEq + Eq + Hash,
    T: Default + TryFrom<usize>,
    <T as TryFrom<usize>>::Error: Debug,
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: Debug,
{
    // return subtrees
    pub(crate) fn subtrees(&self) -> Vec<SubTree<T>> {
        // println!("XorSubtreeStart");
        let input_len = usize::try_from(self.input_len).unwrap();
        let mut usage = vec![0u8; self.gates.len()];
        for (g, _) in &self.gates {
            if g.i0 >= self.input_len {
                let i0 = usize::try_from(g.i0).unwrap() - input_len;
                if usage[i0] < 2 {
                    usage[i0] += 1;
                }
            }
            if g.i1 >= self.input_len {
                let i1 = usize::try_from(g.i1).unwrap() - input_len;
                if usage[i1] < 2 {
                    usage[i1] += 1;
                }
            }
        }

        for (o, _) in self.outputs.iter() {
            if *o >= self.input_len {
                let o = usize::try_from(*o).unwrap() - input_len;
                if usage[o] < 2 {
                    usage[o] += 1;
                }
            }
        }

        #[derive(Clone, Copy)]
        struct StackEntry<T> {
            node: usize,
            way: usize,
            subtree_index: Option<T>,
        }
        let gate_num = self.gates.len();
        let mut visited = vec![false; gate_num];
        let mut subtree_map = BTreeMap::new();

        // traverse through circuit
        for (o, _) in self.outputs.iter() {
            if *o < self.input_len {
                continue;
            }
            let oidx = usize::try_from(*o).unwrap() - input_len;
            let mut stack = Vec::<StackEntry<T>>::new();
            stack.push(StackEntry {
                node: oidx,
                way: 0,
                subtree_index: None,
            });

            while !stack.is_empty() {
                let top = stack.last_mut().unwrap();
                let node_index = top.node;
                let way = top.way;

                let node_out_index = T::try_from(node_index + input_len).unwrap();
                if way == 0 {
                    if !visited[node_index] {
                        visited[node_index] = true;
                    } else {
                        stack.pop();
                        continue;
                    }

                    top.way += 1;
                    let gi0 = self.gates[node_index].0.i0;
                    if gi0 >= self.input_len {
                        let new_node_index = usize::try_from(gi0).unwrap() - input_len;
                        // println!("To next 0: {:?} {:?} {:?}", node_index + input_len, gi0,
                        //          self.gates[node_index].0);
                        // determine subtree_index
                        let subtree_index = if let Some(subtree_index) = top.subtree_index {
                            // propagate only to gate with usage<2
                            if usage[new_node_index] < 2 {
                                Some(subtree_index)
                            } else {
                                None
                            }
                        } else if usage[new_node_index] < 2 {
                            // if without subtree_index then its node index is subtree_index
                            Some(node_out_index)
                        } else {
                            None
                        };
                        stack.push(StackEntry {
                            node: new_node_index,
                            way: 0,
                            subtree_index,
                        });
                    }
                } else if way == 1 {
                    top.way += 1;
                    let gi1 = self.gates[node_index].0.i1;
                    if gi1 >= self.input_len {
                        // println!("To next 1: {:?} {:?}: {:?}", node_index + input_len, gi1,
                        //          self.gates[node_index].0);
                        let new_node_index = usize::try_from(gi1).unwrap() - input_len;
                        // determine subtree_index
                        let subtree_index = if let Some(subtree_index) = top.subtree_index {
                            // propagate xor only to gate with usage<2
                            if usage[new_node_index] < 2 {
                                Some(subtree_index)
                            } else {
                                None
                            }
                        } else if usage[new_node_index] < 2 {
                            // if without subtree_index then its node index is subtree_index
                            Some(node_out_index)
                        } else {
                            None
                        };
                        stack.push(StackEntry {
                            node: new_node_index,
                            way: 0,
                            subtree_index,
                        });
                    }
                } else {
                    // println!("Top node: {:?} {:?} {:?}", top.node + input_len,
                    //          top.way, top.subtree_index);
                    if let Some(subtree_index) = top.subtree_index {
                        // add to subtree_map
                        subtree_map.insert(node_out_index, subtree_index);
                    } else {
                        // add same root
                        subtree_map.insert(node_out_index, node_out_index);
                    }
                    stack.pop();
                }
            }
        }

        let mut subtree_object_map = BTreeMap::<T, SubTree<T>>::new();
        for (x, root) in subtree_map {
            if let Some(st) = subtree_object_map.get_mut(&root) {
                st.gates.push((x, T::default()));
            } else {
                subtree_object_map.insert(
                    root,
                    SubTree {
                        root,
                        gates: vec![(x, T::default())],
                    },
                );
            }
        }

        subtree_object_map
            .into_iter()
            .map(|(_, mut v)| {
                v.sort_and_dedup();
                v.fill_successors(self);
                v
            })
            .collect::<Vec<_>>()
    }

    fn apply_subtree(&mut self, subtree: SubTreeCopy<T>) {
        let input_len = usize::try_from(self.input_len).unwrap();
        for i in 0..subtree.gates.len() {
            let oi = subtree.circuit_index(i);
            let oi = usize::try_from(oi).unwrap() - input_len;
            self.gates[oi] = subtree.gates[i];
        }
    }

    // get list of subtree dependencies:
    // entry: entry for current subtree
    //    list of entries
    //    (n, g, a):
    //      n - index of next subtree that connected to current subtree
    //      g - index of gate in next subtree (from 0 (first gate in next subtree))
    //          that have connection to current subtree (its root).
    //      a - boolean. false if root of current subtree in first input of gate
    //          where is connection to current subtree. true - if second input of gate.
    pub(crate) fn subtree_dependencies(&self, subtrees: &[SubTree<T>]) -> Vec<Vec<(T, T, bool)>> {
        let input_len = usize::try_from(self.input_len).unwrap();
        let mut deps: Vec<Vec<(T, T, bool)>> = vec![vec![]; subtrees.len()];
        for (i, st) in subtrees.iter().enumerate() {
            for (stgi, (gi, _)) in st
                .gates
                .iter()
                .copied()
                .chain(std::iter::once((st.root, T::default())))
                .enumerate()
            {
                let (g, _) = self.gates[usize::try_from(gi).unwrap() - input_len];
                let stgi = T::try_from(stgi).unwrap();
                if g.i0 >= self.input_len {
                    if let Ok(p) = subtrees.binary_search_by_key(&g.i0, |st| st.root) {
                        deps[p].push((T::try_from(i).unwrap(), stgi, false));
                    }
                }
                if g.i1 >= self.input_len {
                    if let Ok(p) = subtrees.binary_search_by_key(&g.i1, |st| st.root) {
                        deps[p].push((T::try_from(i).unwrap(), stgi, true));
                    }
                }
            }
        }
        for d in &mut deps {
            d.sort();
        }
        deps
    }

    pub(crate) fn optimize_negs(&mut self) {
        let subtrees = self.subtrees();
        let mut subtree_copies = subtrees
            .iter()
            .map(|st| SubTreeCopy::new(self, &st))
            .collect::<Vec<_>>();
        // preliminary optimizations
        for st in &mut subtree_copies {
            st.optimize_negs();
        }
        // apply
        for st in subtree_copies {
            self.apply_subtree(st);
        }
        //println!("Circuit after prelim: {:?}", self);
        // after preliminary optimizations
        let subtrees = self.subtrees();
        let mut subtree_copies = subtrees
            .iter()
            .map(|st| SubTreeCopy::new(self, &st))
            .collect::<Vec<_>>();
        let subtree_deps = self.subtree_dependencies(&subtrees);

        let mut circ_out_map = HashMap::<T, Vec<usize>>::new();
        for (i, (x, _)) in self.outputs.iter().enumerate() {
            if let Some(list) = circ_out_map.get_mut(x) {
                list.push(i);
            } else {
                circ_out_map.insert(*x, vec![i]);
            }
        }

        // single choice: subtree root that can be negated or not.
        // multi choice: choice-bucket -list of single choices.
        const MAX_MULTI_CHOICE: usize = 4;
        // multichoice map - dep to multichoice indexes
        let mut multi_choice_map = HashMap::<T, Vec<usize>>::new();
        let mut multi_choices: Vec<Vec<T>> = vec![];
        for (i, _) in subtree_copies.iter_mut().enumerate() {
            let deps = &subtree_deps[i];
            let mut found = false;
            let mut added = false;
            for (dep, _, _) in deps {
                //println!("mccreate: {}: {:?}", i, dep);
                if let Some(mcs) = multi_choice_map.get_mut(&dep) {
                    for mc in mcs.iter() {
                        if multi_choices[*mc].len() < MAX_MULTI_CHOICE {
                            // add subtree to multichoice
                            //println!("mccreate: add {:?}: {:?} {:?}", mc, i, dep);
                            multi_choices[*mc].push(T::try_from(i).unwrap());
                            added = true;
                            break;
                        }
                    }
                    found = true;
                    if added {
                        break;
                    }
                }
            }
            if !found || !added {
                // if not found or not added then create new multichoice and register in
                // multi_choice_map
                let cur_choice = multi_choices.len();
                for (dep, _, _) in deps {
                    if let Some(mcs) = multi_choice_map.get_mut(dep) {
                        mcs.push(cur_choice);
                    } else {
                        multi_choice_map.insert(*dep, vec![cur_choice]);
                    }
                }
                multi_choices.push(vec![T::try_from(i).unwrap()]);
            }
        }

        // find combinations
        for mc in multi_choices {
            //println!("MC: {:?}", mc);
            let mut orig_subtrees = HashMap::new();
            for st_i in &mc {
                let st_i = usize::try_from(*st_i).unwrap();
                if !orig_subtrees.contains_key(&st_i) {
                    orig_subtrees.insert(st_i, subtree_copies[st_i].clone());
                }
                for (dep_st_i, _, _) in &subtree_deps[st_i] {
                    let dep_st_i = usize::try_from(*dep_st_i).unwrap();
                    if !orig_subtrees.contains_key(&dep_st_i) {
                        orig_subtrees.insert(dep_st_i, subtree_copies[dep_st_i].clone());
                    }
                }
            }
            let mut best_subtrees = orig_subtrees.clone();
            let mut best_comb = 0;

            let mut best_neg_count = best_subtrees
                .values()
                .map(|st| {
                    // calculate circuit outputs negs - including combination value = 0
                    // that is: no negations included into subtree roots.
                    let circ_output_negs = if let Some(list) = circ_out_map.get(&st.subtree.root) {
                        list.iter().filter(|x| self.outputs[**x].1).count()
                    } else {
                        0
                    };
                    st.count_negs() + circ_output_negs
                })
                .sum::<usize>();

            for c in 0..1 << mc.len() {
                let mut cur_subtrees = HashMap::<usize, (SubTreeCopy<T>, bool)>::from_iter(
                    orig_subtrees.iter().map(|(k, v)| (*k, (v.clone(), false))),
                );
                // change subtrees
                let mut circ_output_neg_count = 0;
                for (b, st_b) in mc.iter().enumerate() {
                    let st_b = usize::try_from(*st_b).unwrap();
                    let (st, _) = cur_subtrees.get_mut(&st_b).unwrap();
                    if ((1 << b) & c) != 0 {
                        // if root of subtree will be negated then:
                        // summarize circuit output negs
                        if let Some(list) = circ_out_map.get(&st.subtree.root) {
                            for x in list {
                                circ_output_neg_count += usize::from(!self.outputs[*x].1);
                            }
                        }
                        // change negation at root of subtree
                        let (r, rneg) = st.gates.last().unwrap();
                        *st.gates.last_mut().unwrap() = r.binop_neg(*rneg);
                        let st_root = st.subtree.root;
                        // change negation at further dependencies
                        for (dep_dst_i, p, in_garg) in &subtree_deps[st_b] {
                            let dep_dst_i = usize::try_from(*dep_dst_i).unwrap();
                            let (dst, dst_mod) = cur_subtrees.get_mut(&dep_dst_i).unwrap();
                            let dst_gi = usize::try_from(*p).unwrap();
                            let (arg, arg_neg) = dst.gates[dst_gi];
                            if arg.i0 != arg.i1 {
                                // determine from real gate: because it can be changed earlier!
                                let garg = st_root == arg.i1;
                                dst.gates[dst_gi] = arg.binop_neg_args(arg_neg, !garg, garg);
                            } else if !*in_garg {
                                // if double occurrence then get from original deps.
                                // apply change only for first entry in subtree_deps list.
                                // for first argument occurrence in gate.
                                dst.gates[dst_gi] = arg.binop_neg_args(arg_neg, true, true);
                            }
                            *dst_mod = true;
                        }
                    } else {
                        // if root of subtree will NOT be negated then:
                        // summarize circuit output negs
                        if let Some(list) = circ_out_map.get(&st.subtree.root) {
                            for x in list {
                                circ_output_neg_count += usize::from(self.outputs[*x].1);
                            }
                        }
                    }
                }
                for (st, _) in cur_subtrees.values_mut().filter(|(_, m)| *m) {
                    //println!("  cursubtree: {:?}", st.gates);
                    st.optimize_negs();
                    //println!("  cursubtree 2: {:?}", st.gates);
                }

                let cur_neg_count = cur_subtrees
                    .values()
                    .map(|(st, _)| st.count_negs())
                    .sum::<usize>()
                    + circ_output_neg_count;
                if cur_neg_count < best_neg_count {
                    best_subtrees =
                        HashMap::from_iter(cur_subtrees.into_iter().map(|(k, (st, _))| (k, st)));
                    best_neg_count = cur_neg_count;
                    best_comb = c;
                }
            }
            // apply to circuit outputs
            for (b, st_b) in mc.iter().enumerate() {
                let st_b = usize::try_from(*st_b).unwrap();
                let st = best_subtrees.get(&st_b).unwrap();
                if ((1 << b) & best_comb) != 0 {
                    if let Some(list) = circ_out_map.get(&st.subtree.root) {
                        for x in list {
                            self.outputs[*x].1 = !self.outputs[*x].1;
                        }
                    }
                }
            }
            // apply changes into subtrees
            for (i, st) in best_subtrees {
                subtree_copies[i] = st;
            }
        }
        // apply
        for st in subtree_copies {
            self.apply_subtree(st);
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
    fn vgate_or<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Or, i0, i1)
    }
    fn vgate_xor<T: Clone + Copy>(i0: T, i1: T) -> VGate<T> {
        vgate(VGateFunc::Xor, i0, i1)
    }

    #[test]
    fn test_vbinopcircuit_from() {
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_xor(2, 3), NoNegs),
                    (vgate_and(2, 3), NegInput1),
                    (vgate_and(0, 1), NoNegs),
                    (vgate_or(5, 6), NegOutput),
                ],
                outputs: vec![(4, true), (7, false)],
            },
            VBinOpCircuit::from(
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
                .unwrap()
            )
        );
    }

    #[test]
    fn test_vbinopcircuit_subtrees() {
        assert_eq!(
            vec![SubTree {
                root: 4,
                gates: vec![(3, 4)]
            }],
            VBinOpCircuit::from(
                Circuit::new(3, [Gate::new_xor(0, 1), Gate::new_xor(2, 3)], [(4, true)]).unwrap()
            )
            .subtrees()
        );

        assert_eq!(
            vec![
                SubTree {
                    root: 3,
                    gates: vec![],
                },
                SubTree {
                    root: 4,
                    gates: vec![],
                },
                SubTree {
                    root: 7,
                    gates: vec![(5, 7), (6, 7)],
                },
            ],
            VBinOpCircuit::from(
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
                .unwrap()
            )
            .subtrees()
        );

        assert_eq!(
            vec![
                SubTree {
                    root: 3,
                    gates: vec![],
                },
                SubTree {
                    root: 4,
                    gates: vec![],
                },
                SubTree {
                    root: 7,
                    gates: vec![(5, 7), (6, 7)],
                },
            ],
            VBinOpCircuit::from(
                Circuit::new(
                    3,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_xor(3, 2),
                        Gate::new_nimpl(3, 2),
                        Gate::new_and(0, 1),
                        Gate::new_nor(5, 6),
                    ],
                    [(4, true), (7, false)],
                )
                .unwrap()
            )
            .subtrees()
        );

        assert_eq!(
            vec![SubTree {
                root: 7,
                gates: vec![(3, 4), (4, 5), (5, 7), (6, 7)]
            }],
            VBinOpCircuit::from(
                Circuit::new(
                    3,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_xor(2, 3),
                        Gate::new_nimpl(2, 4),
                        Gate::new_and(0, 1),
                        Gate::new_nor(5, 6),
                    ],
                    [(7, false)],
                )
                .unwrap()
            )
            .subtrees()
        );

        assert_eq!(
            vec![
                SubTree {
                    root: 4,
                    gates: vec![(3, 4)]
                },
                SubTree {
                    root: 7,
                    gates: vec![(5, 7), (6, 7)]
                },
                SubTree {
                    root: 10,
                    gates: vec![(8, 10), (9, 10)]
                },
            ],
            VBinOpCircuit::from(
                Circuit::new(
                    3,
                    [
                        Gate::new_nimpl(0, 1),
                        Gate::new_xor(3, 2),
                        Gate::new_nimpl(4, 2),
                        Gate::new_and(0, 1),
                        Gate::new_nor(5, 6),
                        Gate::new_nor(2, 7),
                        Gate::new_xor(1, 7),
                        Gate::new_and(8, 9),
                    ],
                    [(4, true), (10, false)],
                )
                .unwrap()
            )
            .subtrees()
        );

        assert_eq!(
            vec![
                SubTree {
                    root: 8,
                    gates: vec![(4, 6), (6, 8)]
                },
                SubTree {
                    root: 9,
                    gates: vec![(5, 7), (7, 9)]
                },
                SubTree {
                    root: 10,
                    gates: vec![]
                },
                SubTree {
                    root: 11,
                    gates: vec![]
                },
                SubTree {
                    root: 12,
                    gates: vec![]
                },
                SubTree {
                    root: 18,
                    gates: vec![(13, 16), (14, 16), (15, 18), (16, 18)]
                },
                SubTree {
                    root: 22,
                    gates: vec![(17, 20), (19, 21), (20, 22), (21, 22)]
                },
            ],
            VBinOpCircuit::from(
                Circuit::new(
                    4,
                    [
                        Gate::new_nimpl(0, 1),   // 4
                        Gate::new_and(0, 3),     // 5
                        Gate::new_xor(1, 4),     // 6
                        Gate::new_and(3, 5),     // 7
                        Gate::new_xor(2, 6),     // 8
                        Gate::new_xor(3, 7),     // 9
                        Gate::new_nor(8, 9),     // 10
                        Gate::new_and(8, 9),     // 11
                        Gate::new_nimpl(8, 9),   // 12
                        Gate::new_nor(0, 10),    // 13
                        Gate::new_nor(1, 11),    // 14
                        Gate::new_xor(2, 12),    // 15
                        Gate::new_xor(13, 14),   // 16
                        Gate::new_and(0, 10),    // 17 tree4
                        Gate::new_nor(15, 16),   // 18 tree3
                        Gate::new_nimpl(1, 12),  // 19 tree4
                        Gate::new_nimpl(11, 17), // 20
                        Gate::new_nimpl(3, 19),  // 21
                        Gate::new_xor(20, 21),   // 22
                    ],
                    [(18, true), (22, false)],
                )
                .unwrap()
            )
            .subtrees()
        );
    }

    #[test]
    fn test_vbinopcircuit_apply_subtree() {
        let mut circuit = VBinOpCircuit::from(
            Circuit::new(
                3,
                [
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(3, 2),
                    Gate::new_nimpl(4, 2),
                    Gate::new_and(0, 1),
                    Gate::new_nor(5, 6),
                    Gate::new_nor(2, 7),
                    Gate::new_xor(1, 7),
                    Gate::new_and(8, 9),
                ],
                [(4, true), (10, false)],
            )
            .unwrap(),
        );
        let subtrees = circuit.subtrees();
        let mut subtree_copies = subtrees
            .iter()
            .map(|st| SubTreeCopy::new(&circuit, st))
            .collect::<Vec<_>>();
        subtree_copies[0].gates[0] = (vgate_or(0, 1), NegInput1);
        subtree_copies[1].gates[2] = (vgate_and(5, 6), NoNegs);
        subtree_copies[2].gates[1] = (vgate_xor(1, 7), NegOutput);
        for st in subtree_copies {
            circuit.apply_subtree(st);
        }
        assert_eq!(
            circuit,
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_xor(3, 2), NoNegs),
                    (vgate_and(4, 2), NegInput1),
                    (vgate_and(0, 1), NoNegs),
                    (vgate_and(5, 6), NoNegs),
                    (vgate_or(2, 7), NegOutput),
                    (vgate_xor(1, 7), NegOutput),
                    (vgate_and(8, 9), NoNegs),
                ],
                outputs: vec![(4, true), (10, false)],
            }
        );
    }

    fn vbinop_optimize_negs_in_subtree<T>(circuit: VBinOpCircuit<T>) -> VBinOpCircuit<T>
    where
        T: Clone + Copy + Ord + PartialEq + Eq + Hash + Debug,
        T: Default + TryFrom<usize>,
        <T as TryFrom<usize>>::Error: Debug,
        usize: TryFrom<T>,
        <usize as TryFrom<T>>::Error: Debug,
    {
        let mut circuit = circuit.clone();
        let subtree = circuit.subtrees().pop().unwrap();
        let mut subtree_copy = SubTreeCopy::new(&circuit, &subtree);
        subtree_copy.optimize_negs();
        circuit.apply_subtree(subtree_copy);
        circuit
    }

    #[test]
    fn test_vbinopcircuit_optimize_negs_in_subtree() {
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NoNegs), (vgate_and(2, 3), NoNegs)],
                outputs: vec![(4, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NoNegs), (vgate_and(2, 3), NoNegs)],
                outputs: vec![(4, false)]
            })
        );

        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NoNegs), (vgate_and(2, 3), NegInput1)],
                outputs: vec![(4, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NegOutput), (vgate_and(2, 3), NoNegs)],
                outputs: vec![(4, false)]
            })
        );
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NoNegs), (vgate_and(2, 3), NegInput1)],
                outputs: vec![(4, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NegOutput), (vgate_and(3, 2), NoNegs)],
                outputs: vec![(4, false)]
            })
        );
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NoNegs), (vgate_or(3, 2), NegInput1),],
                outputs: vec![(4, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NegOutput), (vgate_and(2, 3), NegOutput)],
                outputs: vec![(4, false)]
            })
        );
        // ignore this move
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NegInput1), (vgate_and(2, 3), NegOutput)],
                outputs: vec![(4, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NegInput1), (vgate_and(2, 3), NegOutput)],
                outputs: vec![(4, false)]
            })
        );
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_or(1, 0), NegInput1), (vgate_or(3, 2), NegOutput)],
                outputs: vec![(4, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![(vgate_and(0, 1), NegInput1), (vgate_and(3, 2), NegInput1)],
                outputs: vec![(4, false)]
            })
        );

        // more complex examples
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NoNegs),
                    (vgate_or(2, 4), NoNegs)
                ],
                outputs: vec![(5, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NegOutput),
                    (vgate_or(2, 4), NegInput1)
                ],
                outputs: vec![(5, false)]
            })
        );
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(1, 0), NegInput1),
                    (vgate_and(3, 2), NoNegs),
                    (vgate_and(2, 4), NoNegs)
                ],
                outputs: vec![(5, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(3, 2), NegInput1),
                    (vgate_and(2, 4), NegInput1)
                ],
                outputs: vec![(5, false)]
            })
        );

        assert_eq!(
            VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NoNegs),
                    (vgate_or(9, 10), NoNegs),
                    (vgate_and(4, 11), NegInput1),
                    (vgate_and(12, 5), NoNegs),
                    (vgate_and(13, 6), NoNegs),
                    (vgate_and(14, 7), NoNegs),
                    (vgate_and(15, 8), NegOutput),
                ],
                outputs: vec![(16, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NegOutput),
                    (vgate_or(2, 3), NegOutput),
                    (vgate_and(9, 10), NegOutput),
                    (vgate_or(11, 4), NegInput1),
                    (vgate_or(12, 5), NegInput1),
                    (vgate_or(13, 6), NegInput1),
                    (vgate_or(14, 7), NegInput1),
                    (vgate_or(15, 8), NegInput1),
                ],
                outputs: vec![(16, false)]
            })
        );

        assert_eq!(
            VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NoNegs),
                    (vgate_or(10, 9), NegInput1),
                    (vgate_or(4, 11), NoNegs),
                    (vgate_and(12, 5), NegInput1),
                    (vgate_or(6, 13), NoNegs),
                    (vgate_and(14, 7), NegInput1),
                    (vgate_or(8, 15), NoNegs),
                ],
                outputs: vec![(16, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NoNegs),
                    (vgate_and(9, 10), NegInput1),
                    (vgate_or(4, 11), NegInput1),
                    (vgate_or(5, 12), NegInput1),
                    (vgate_or(6, 13), NegInput1),
                    (vgate_or(7, 14), NegInput1),
                    (vgate_or(8, 15), NegInput1),
                ],
                outputs: vec![(16, false)]
            })
        );

        // move negation through XOR subtree
        assert_eq!(
            VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NoNegs),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_or(6, 7), NegInput1),
                    (vgate_xor(8, 9), NoNegs),
                    (vgate_xor(10, 11), NoNegs),
                    (vgate_xor(12, 13), NoNegs),
                    (vgate_xor(14, 15), NegOutput),
                ],
                outputs: vec![(16, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NegOutput),
                    (vgate_or(2, 3), NegOutput),
                    (vgate_and(4, 5), NegOutput),
                    (vgate_or(6, 7), NegInput1),
                    (vgate_xor(8, 9), NoNegs),
                    (vgate_xor(10, 11), NoNegs),
                    (vgate_xor(12, 13), NoNegs),
                    (vgate_xor(14, 15), NoNegs),
                ],
                outputs: vec![(16, false)]
            })
        );
        // move negation through XOR subtree 2
        assert_eq!(
            VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_or(6, 7), NegInput1),
                    (vgate_xor(8, 9), NoNegs),
                    (vgate_xor(10, 11), NoNegs),
                    (vgate_xor(12, 13), NoNegs),
                    (vgate_xor(14, 15), NoNegs),
                ],
                outputs: vec![(16, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 9,
                gates: vec![
                    (vgate_or(0, 1), NegOutput),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NegOutput),
                    (vgate_or(6, 7), NegInput1),
                    (vgate_xor(8, 9), NoNegs),
                    (vgate_xor(10, 11), NoNegs),
                    (vgate_xor(12, 13), NoNegs),
                    (vgate_xor(14, 15), NoNegs),
                ],
                outputs: vec![(16, false)]
            })
        );
        // single reduce subtree
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(1, 0), NegInput1),
                    (vgate_and(3, 2), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                ],
                outputs: vec![(6, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NegOutput),
                ],
                outputs: vec![(6, false)]
            })
        );
        // no single reduce subtree
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_xor(0, 1), NoNegs),
                    (vgate_and(3, 2), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                ],
                outputs: vec![(6, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_xor(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NegOutput),
                ],
                outputs: vec![(6, false)]
            })
        );
        // no single reduce subtree
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NegOutput),
                ],
                outputs: vec![(6, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NoNegs),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NegOutput),
                ],
                outputs: vec![(6, false)]
            })
        );
        // single reduce subtree
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(1, 0), NegInput1),
                    (vgate_and(3, 2), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(7, 6), NoNegs),
                ],
                outputs: vec![(8, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(7, 6), NegInput1),
                ],
                outputs: vec![(8, false)]
            })
        );
        // no single reduce subtree - neg input at bad root gate input
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(6, 7), NegInput1),
                ],
                outputs: vec![(8, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(6, 7), NegInput1),
                ],
                outputs: vec![(8, false)]
            })
        );
        // single reduce subtree
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(1, 0), NegInput1),
                    (vgate_and(3, 2), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(7, 6), NoNegs),
                ],
                outputs: vec![(8, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(4, 5), NegOutput),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(7, 6), NoNegs),
                ],
                outputs: vec![(8, false)]
            })
        );
        // single reduce subtree level 2
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(1, 0), NegInput1),
                    (vgate_and(3, 2), NegInput1),
                    (vgate_or(2, 0), NegInput1),
                    (vgate_and(3, 1), NegInput1),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_and(6, 7), NoNegs),
                    (vgate_and(8, 9), NoNegs),
                ],
                outputs: vec![(10, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(0, 2), NegInput1),
                    (vgate_or(1, 3), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                    (vgate_or(6, 7), NoNegs),
                    (vgate_or(8, 9), NegOutput),
                ],
                outputs: vec![(10, false)]
            })
        );
        // no single reduce subtree level 2
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(0, 2), NoNegs),
                    (vgate_or(1, 3), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                    (vgate_or(6, 7), NoNegs),
                    (vgate_or(8, 9), NegOutput),
                ],
                outputs: vec![(10, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(0, 2), NoNegs),
                    (vgate_or(1, 3), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                    (vgate_or(6, 7), NoNegs),
                    (vgate_or(8, 9), NegOutput),
                ],
                outputs: vec![(10, false)]
            })
        );
        // single reduce subtree level 2 - in_next
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_or(1, 0), NegInput1),
                    (vgate_and(3, 2), NegInput1),
                    (vgate_or(2, 0), NegInput1),
                    (vgate_and(3, 1), NegInput1),
                    (vgate_and(4, 5), NoNegs),
                    (vgate_and(6, 7), NoNegs),
                    (vgate_and(8, 9), NoNegs),
                    (vgate_or(0, 3), NoNegs),
                    (vgate_and(11, 10), NoNegs),
                ],
                outputs: vec![(12, false)]
            },
            vbinop_optimize_negs_in_subtree(VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_or(2, 3), NegInput1),
                    (vgate_and(0, 2), NegInput1),
                    (vgate_or(1, 3), NegInput1),
                    (vgate_or(4, 5), NoNegs),
                    (vgate_or(6, 7), NoNegs),
                    (vgate_or(8, 9), NoNegs),
                    (vgate_or(0, 3), NoNegs),
                    (vgate_and(11, 10), NegInput1),
                ],
                outputs: vec![(12, false)]
            })
        );
    }

    #[test]
    fn test_vbinopcircuit_subtree_dependencies() {
        let circuit = VBinOpCircuit::from(
            Circuit::new(
                3,
                [
                    Gate::new_nimpl(0, 1),
                    Gate::new_xor(3, 2),
                    Gate::new_nimpl(4, 2),
                    Gate::new_and(0, 1),
                    Gate::new_nor(5, 6),
                    Gate::new_nor(2, 7),
                    Gate::new_xor(1, 7),
                    Gate::new_and(8, 9),
                ],
                [(4, true), (10, false)],
            )
            .unwrap(),
        );
        let subtrees = circuit.subtrees();
        assert_eq!(
            vec![
                vec![(1, 0, false)],
                vec![(2, 0, true), (2, 1, true)],
                vec![]
            ],
            circuit.subtree_dependencies(&subtrees),
        );

        let circuit = VBinOpCircuit::from(
            Circuit::new(
                4,
                [
                    Gate::new_nimpl(0, 1),   // 4
                    Gate::new_and(0, 3),     // 5
                    Gate::new_xor(1, 4),     // 6
                    Gate::new_and(3, 5),     // 7
                    Gate::new_xor(2, 6),     // 8
                    Gate::new_xor(3, 7),     // 9
                    Gate::new_nor(8, 9),     // 10
                    Gate::new_and(8, 9),     // 11
                    Gate::new_nimpl(8, 9),   // 12
                    Gate::new_nor(0, 10),    // 13
                    Gate::new_nor(1, 11),    // 14
                    Gate::new_xor(2, 12),    // 15
                    Gate::new_xor(13, 14),   // 16
                    Gate::new_and(0, 10),    // 17 tree4
                    Gate::new_nor(15, 16),   // 18 tree3
                    Gate::new_nimpl(1, 12),  // 19 tree4
                    Gate::new_nimpl(11, 17), // 20
                    Gate::new_nimpl(3, 19),  // 21
                    Gate::new_xor(20, 21),   // 22
                ],
                [(18, true), (22, false)],
            )
            .unwrap(),
        );
        let subtrees = circuit.subtrees();
        assert_eq!(
            vec![
                vec![(2, 0, false), (3, 0, false), (4, 0, false)],
                vec![(2, 0, true), (3, 0, true), (4, 0, true)],
                vec![(5, 0, true), (6, 0, true)],
                vec![(5, 1, true), (6, 2, false)],
                vec![(5, 2, true), (6, 1, true)],
                vec![],
                vec![]
            ],
            circuit.subtree_dependencies(&subtrees),
        );
    }

    #[test]
    fn test_vbinopcircuit_optimize_negs() {
        let mut circuit = VBinOpCircuit::from(
            Circuit::new(
                4,
                [
                    Gate::new_nimpl(0, 1),   // 4
                    Gate::new_and(0, 3),     // 5
                    Gate::new_xor(1, 4),     // 6
                    Gate::new_and(3, 5),     // 7
                    Gate::new_xor(2, 6),     // 8
                    Gate::new_xor(3, 7),     // 9
                    Gate::new_nor(8, 9),     // 10
                    Gate::new_and(8, 9),     // 11
                    Gate::new_nimpl(8, 9),   // 12
                    Gate::new_nor(0, 10),    // 13
                    Gate::new_nor(1, 11),    // 14
                    Gate::new_xor(2, 12),    // 15
                    Gate::new_xor(13, 14),   // 16
                    Gate::new_and(0, 10),    // 17 tree4
                    Gate::new_nor(15, 16),   // 18 tree3
                    Gate::new_nimpl(1, 12),  // 19 tree4
                    Gate::new_nimpl(11, 17), // 20
                    Gate::new_nimpl(3, 19),  // 21
                    Gate::new_xor(20, 21),   // 22
                ],
                [(18, true), (22, false)],
            )
            .unwrap(),
        );
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_and(0, 3), NoNegs),
                    (vgate_xor(1, 4), NoNegs),
                    (vgate_and(3, 5), NoNegs),
                    (vgate_xor(2, 6), NoNegs),
                    (vgate_xor(3, 7), NoNegs),
                    (vgate_or(8, 9), NegOutput),
                    (vgate_and(8, 9), NoNegs),
                    (vgate_and(8, 9), NegInput1),
                    (vgate_or(0, 10), NoNegs),
                    (vgate_or(1, 11), NoNegs),
                    (vgate_xor(2, 12), NoNegs),
                    (vgate_xor(13, 14), NoNegs),
                    (vgate_and(0, 10), NoNegs),
                    (vgate_or(15, 16), NoNegs),
                    (vgate_or(12, 1), NegInput1),
                    (vgate_and(11, 17), NegInput1),
                    (vgate_and(3, 19), NoNegs),
                    (vgate_xor(20, 21), NoNegs),
                ],
                outputs: vec![(18, false), (22, false)]
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 4,
            gates: vec![
                (vgate_and(0, 1), NoNegs),      // 4
                (vgate_and(0, 2), NoNegs),      // 5
                (vgate_or(4, 5), NoNegs),       // 6
                (vgate_and(0, 3), NoNegs),      // 7
                (vgate_and(1, 2), NoNegs),      // 8
                (vgate_or(7, 8), NoNegs),       // 9
                (vgate_and(1, 3), NoNegs),      // 10
                (vgate_and(2, 3), NoNegs),      // 11
                (vgate_or(10, 11), NoNegs),     // 12
                (vgate_or(0, 6), NegInput1),    // 13
                (vgate_or(1, 9), NegInput1),    // 14
                (vgate_or(13, 12), NegInput1),  // 15
                (vgate_and(14, 15), NoNegs),    // 16
                (vgate_and(0, 6), NegInput1),   // 17
                (vgate_and(1, 9), NegInput1),   // 18
                (vgate_and(17, 12), NegInput1), // 19
                (vgate_or(18, 19), NoNegs),     // 20
            ],
            outputs: vec![(16, false), (20, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),     // 4
                    (vgate_and(0, 2), NoNegs),     // 5
                    (vgate_or(4, 5), NegOutput),   // 6
                    (vgate_and(0, 3), NoNegs),     // 7
                    (vgate_and(1, 2), NoNegs),     // 8
                    (vgate_or(7, 8), NegOutput),   // 9
                    (vgate_and(1, 3), NoNegs),     // 10
                    (vgate_and(2, 3), NoNegs),     // 11
                    (vgate_or(10, 11), NegOutput), // 12
                    (vgate_or(6, 0), NoNegs),      // 13
                    (vgate_or(1, 9), NoNegs),      // 14
                    (vgate_or(13, 12), NoNegs),    // 15
                    (vgate_and(14, 15), NoNegs),   // 16
                    (vgate_and(6, 0), NoNegs),     // 17
                    (vgate_and(1, 9), NoNegs),     // 18
                    (vgate_and(17, 12), NoNegs),   // 19
                    (vgate_or(18, 19), NoNegs),    // 20
                ],
                outputs: vec![(16, false), (20, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 4,
            gates: vec![
                (vgate_and(0, 1), NoNegs),     // 4
                (vgate_and(0, 2), NoNegs),     // 5
                (vgate_or(4, 5), NoNegs),      // 6
                (vgate_and(0, 3), NoNegs),     // 7
                (vgate_and(1, 2), NoNegs),     // 8
                (vgate_or(7, 8), NoNegs),      // 9
                (vgate_and(1, 3), NoNegs),     // 10
                (vgate_and(2, 3), NoNegs),     // 11
                (vgate_or(10, 11), NoNegs),    // 12
                (vgate_or(0, 6), NegInput1),   // 13
                (vgate_or(1, 9), NoNegs),      // 14
                (vgate_or(13, 12), NegInput1), // 15
                (vgate_and(14, 15), NoNegs),   // 16
                (vgate_and(0, 6), NegInput1),  // 17
                (vgate_and(1, 9), NegInput1),  // 18
                (vgate_and(17, 12), NoNegs),   // 19
                (vgate_or(18, 19), NoNegs),    // 20
            ],
            outputs: vec![(16, false), (20, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),     // 4
                    (vgate_and(0, 2), NoNegs),     // 5
                    (vgate_or(4, 5), NegOutput),   // 6
                    (vgate_and(0, 3), NoNegs),     // 7
                    (vgate_and(1, 2), NoNegs),     // 8
                    (vgate_or(7, 8), NoNegs),      // 9
                    (vgate_and(1, 3), NoNegs),     // 10
                    (vgate_and(2, 3), NoNegs),     // 11
                    (vgate_or(10, 11), NoNegs),    // 12
                    (vgate_or(6, 0), NoNegs),      // 13
                    (vgate_or(1, 9), NoNegs),      // 14
                    (vgate_or(13, 12), NegInput1), // 15
                    (vgate_and(14, 15), NoNegs),   // 16
                    (vgate_and(0, 6), NoNegs),     // 17
                    (vgate_and(1, 9), NegInput1),  // 18
                    (vgate_and(17, 12), NoNegs),   // 19
                    (vgate_or(18, 19), NoNegs),    // 20
                ],
                outputs: vec![(16, false), (20, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 4,
            gates: vec![
                (vgate_and(0, 1), NoNegs),      // 4
                (vgate_and(0, 2), NoNegs),      // 5
                (vgate_or(4, 5), NegOutput),    // 6
                (vgate_and(0, 3), NoNegs),      // 7
                (vgate_and(1, 2), NoNegs),      // 8
                (vgate_or(7, 8), NegOutput),    // 9
                (vgate_and(1, 3), NoNegs),      // 10
                (vgate_and(2, 3), NoNegs),      // 11
                (vgate_or(10, 11), NegOutput),  // 12
                (vgate_or(0, 6), NegInput1),    // 13
                (vgate_or(1, 9), NegInput1),    // 14
                (vgate_or(13, 12), NegInput1),  // 15
                (vgate_and(14, 15), NoNegs),    // 16
                (vgate_and(0, 6), NegInput1),   // 17
                (vgate_and(1, 9), NegInput1),   // 18
                (vgate_and(17, 12), NegInput1), // 19
                (vgate_or(18, 19), NoNegs),     // 20
            ],
            outputs: vec![(16, false), (20, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),   // 4
                    (vgate_and(0, 2), NoNegs),   // 5
                    (vgate_or(4, 5), NoNegs),    // 6
                    (vgate_and(0, 3), NoNegs),   // 7
                    (vgate_and(1, 2), NoNegs),   // 8
                    (vgate_or(7, 8), NoNegs),    // 9
                    (vgate_and(1, 3), NoNegs),   // 10
                    (vgate_and(2, 3), NoNegs),   // 11
                    (vgate_or(10, 11), NoNegs),  // 12
                    (vgate_or(6, 0), NoNegs),    // 13
                    (vgate_or(1, 9), NoNegs),    // 14
                    (vgate_or(13, 12), NoNegs),  // 15
                    (vgate_and(14, 15), NoNegs), // 16
                    (vgate_and(6, 0), NoNegs),   // 17
                    (vgate_and(1, 9), NoNegs),   // 18
                    (vgate_and(17, 12), NoNegs), // 19
                    (vgate_or(18, 19), NoNegs),  // 20
                ],
                outputs: vec![(16, false), (20, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 4,
            gates: vec![
                (vgate_and(0, 1), NoNegs),      // 4
                (vgate_and(0, 2), NoNegs),      // 5
                (vgate_and(0, 3), NoNegs),      // 6
                (vgate_and(1, 2), NoNegs),      // 7
                (vgate_and(1, 3), NoNegs),      // 8
                (vgate_and(2, 3), NoNegs),      // 9
                (vgate_or(4, 5), NegOutput),    // 10
                (vgate_or(6, 7), NegOutput),    // 11
                (vgate_or(8, 9), NegOutput),    // 12
                (vgate_or(0, 10), NegInput1),   // 13
                (vgate_and(0, 10), NegInput1),  // 14
                (vgate_or(1, 11), NegInput1),   // 15
                (vgate_and(1, 11), NegInput1),  // 16
                (vgate_or(13, 12), NegInput1),  // 17
                (vgate_and(14, 12), NegInput1), // 18
                (vgate_and(15, 17), NoNegs),    // 19
                (vgate_or(16, 18), NoNegs),     // 20
            ],
            outputs: vec![(19, false), (20, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),   // 4
                    (vgate_and(0, 2), NoNegs),   // 5
                    (vgate_and(0, 3), NoNegs),   // 6
                    (vgate_and(1, 2), NoNegs),   // 7
                    (vgate_and(1, 3), NoNegs),   // 8
                    (vgate_and(2, 3), NoNegs),   // 9
                    (vgate_or(4, 5), NoNegs),    // 10
                    (vgate_or(6, 7), NoNegs),    // 11
                    (vgate_or(8, 9), NoNegs),    // 12
                    (vgate_or(10, 0), NoNegs),   // 13
                    (vgate_and(10, 0), NoNegs),  // 14
                    (vgate_or(1, 11), NoNegs),   // 15
                    (vgate_and(1, 11), NoNegs),  // 16
                    (vgate_or(13, 12), NoNegs),  // 17
                    (vgate_and(14, 12), NoNegs), // 18
                    (vgate_and(15, 17), NoNegs), // 19
                    (vgate_or(16, 18), NoNegs),  // 20
                ],
                outputs: vec![(19, false), (20, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 4,
            gates: vec![
                (vgate_and(0, 1), NoNegs),     // 4
                (vgate_and(0, 2), NoNegs),     // 5
                (vgate_or(4, 5), NoNegs),      // 6
                (vgate_and(0, 3), NoNegs),     // 7
                (vgate_and(1, 2), NoNegs),     // 8
                (vgate_or(7, 8), NoNegs),      // 9
                (vgate_and(1, 3), NoNegs),     // 10
                (vgate_and(2, 3), NoNegs),     // 11
                (vgate_or(10, 11), NoNegs),    // 12
                (vgate_xor(1, 3), NoNegs),     // 13
                (vgate_xor(2, 3), NoNegs),     // 14
                (vgate_or(13, 14), NoNegs),    // 15
                (vgate_xor(0, 3), NoNegs),     // 16
                (vgate_xor(1, 2), NoNegs),     // 17
                (vgate_or(16, 17), NoNegs),    // 18
                (vgate_or(0, 2), NoNegs),      // 19
                (vgate_or(1, 3), NoNegs),      // 20
                (vgate_and(19, 20), NoNegs),   // 21
                (vgate_or(0, 6), NegInput1),   // 22
                (vgate_or(0, 9), NegInput1),   // 23
                (vgate_or(1, 12), NegInput1),  // 24
                (vgate_or(2, 15), NegInput1),  // 25
                (vgate_or(3, 18), NegInput1),  // 26
                (vgate_or(3, 21), NegInput1),  // 27
                (vgate_xor(22, 23), NoNegs),   // 28
                (vgate_xor(24, 25), NoNegs),   // 29
                (vgate_xor(26, 27), NoNegs),   // 30
                (vgate_and(28, 29), NoNegs),   // 31
                (vgate_and(30, 31), NoNegs),   // 32
                (vgate_and(0, 6), NegInput1),  // 33
                (vgate_and(0, 9), NegInput1),  // 34
                (vgate_and(1, 12), NegInput1), // 35
                (vgate_and(2, 15), NegInput1), // 36
                (vgate_and(3, 18), NegInput1), // 37
                (vgate_and(3, 21), NegInput1), // 38
                (vgate_xor(33, 34), NoNegs),   // 39
                (vgate_xor(35, 36), NoNegs),   // 40
                (vgate_xor(37, 38), NoNegs),   // 41
                (vgate_or(39, 40), NoNegs),    // 42
                (vgate_or(41, 42), NoNegs),    // 43
            ],
            outputs: vec![(32, false), (43, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 4,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),      // 4
                    (vgate_and(0, 2), NoNegs),      // 5
                    (vgate_or(4, 5), NegOutput),    // 6
                    (vgate_and(0, 3), NoNegs),      // 7
                    (vgate_and(1, 2), NoNegs),      // 8
                    (vgate_or(7, 8), NegOutput),    // 9
                    (vgate_and(1, 3), NoNegs),      // 10
                    (vgate_and(2, 3), NoNegs),      // 11
                    (vgate_or(10, 11), NegOutput),  // 12
                    (vgate_xor(1, 3), NoNegs),      // 13
                    (vgate_xor(2, 3), NoNegs),      // 14
                    (vgate_or(13, 14), NegOutput),  // 15
                    (vgate_xor(0, 3), NoNegs),      // 16
                    (vgate_xor(1, 2), NoNegs),      // 17
                    (vgate_or(16, 17), NegOutput),  // 18
                    (vgate_or(0, 2), NoNegs),       // 19
                    (vgate_or(1, 3), NoNegs),       // 20
                    (vgate_and(19, 20), NegOutput), // 21
                    (vgate_or(0, 6), NoNegs),       // 22
                    (vgate_or(0, 9), NoNegs),       // 23
                    (vgate_or(1, 12), NoNegs),      // 24
                    (vgate_or(2, 15), NoNegs),      // 25
                    (vgate_or(3, 18), NoNegs),      // 26
                    (vgate_or(3, 21), NoNegs),      // 27
                    (vgate_xor(22, 23), NoNegs),    // 28
                    (vgate_xor(24, 25), NoNegs),    // 29
                    (vgate_xor(26, 27), NoNegs),    // 30
                    (vgate_and(28, 29), NoNegs),    // 31
                    (vgate_and(30, 31), NoNegs),    // 32
                    (vgate_and(0, 6), NoNegs),      // 33
                    (vgate_and(0, 9), NoNegs),      // 34
                    (vgate_and(1, 12), NoNegs),     // 35
                    (vgate_and(2, 15), NoNegs),     // 36
                    (vgate_and(3, 18), NoNegs),     // 37
                    (vgate_and(3, 21), NoNegs),     // 38
                    (vgate_xor(33, 34), NoNegs),    // 39
                    (vgate_xor(35, 36), NoNegs),    // 40
                    (vgate_xor(37, 38), NoNegs),    // 41
                    (vgate_or(39, 40), NoNegs),     // 42
                    (vgate_or(41, 42), NoNegs),     // 43
                ],
                outputs: vec![(32, false), (43, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit::from(
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
        );
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(0, 1), NegInput1),
                    (vgate_xor(2, 3), NoNegs),
                    (vgate_and(2, 3), NegInput1),
                    (vgate_and(0, 1), NoNegs),
                    (vgate_or(5, 6), NegOutput),
                ],
                outputs: vec![(4, true), (7, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 3,
            gates: vec![
                (vgate_and(0, 1), NegInput1),
                (vgate_xor(2, 3), NegOutput),
                (vgate_and(2, 3), NegInput1),
                (vgate_and(0, 1), NoNegs),
                (vgate_or(5, 6), NegOutput),
            ],
            outputs: vec![(4, true), (7, true)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_or(1, 0), NegInput1),
                    (vgate_xor(2, 3), NoNegs),
                    (vgate_and(2, 3), NoNegs),
                    (vgate_and(0, 1), NoNegs),
                    (vgate_or(5, 6), NoNegs),
                ],
                outputs: vec![(4, true), (7, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 3,
            gates: vec![
                (vgate_and(0, 1), NoNegs),
                (vgate_or(1, 2), NoNegs),
                (vgate_and(3, 4), NegOutput),
                (vgate_or(3, 4), NegOutput),
                (vgate_xor(0, 3), NegOutput),
                (vgate_xor(1, 3), NegOutput),
                (vgate_or(5, 6), NoNegs),
                (vgate_or(7, 9), NoNegs),
                (vgate_or(8, 10), NoNegs),
            ],
            outputs: vec![(11, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),
                    (vgate_or(1, 2), NoNegs),
                    (vgate_and(3, 4), NoNegs),
                    (vgate_or(3, 4), NoNegs),
                    (vgate_xor(0, 3), NoNegs),
                    (vgate_xor(1, 3), NoNegs),
                    (vgate_and(6, 5), NoNegs),
                    (vgate_and(9, 7), NoNegs),
                    (vgate_and(10, 8), NegOutput),
                ],
                outputs: vec![(11, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 3,
            gates: vec![
                (vgate_and(0, 1), NegOutput),
                (vgate_or(0, 2), NegOutput),
                (vgate_and(0, 2), NoNegs),
                (vgate_xor(3, 4), NoNegs),
                (vgate_xor(5, 3), NoNegs),
                (vgate_xor(6, 7), NoNegs),
            ],
            outputs: vec![(8, true)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(0, 1), NoNegs),
                    (vgate_or(0, 2), NoNegs),
                    (vgate_and(0, 2), NoNegs),
                    (vgate_xor(3, 4), NoNegs),
                    (vgate_xor(5, 3), NoNegs),
                    (vgate_xor(6, 7), NoNegs),
                ],
                outputs: vec![(8, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 3,
            gates: vec![
                (vgate_and(0, 1), NoNegs),    // 3
                (vgate_or(3, 3), NegInput1),  // 4
                (vgate_or(2, 4), NoNegs),     // 5
                (vgate_and(0, 3), NegInput1), // 6
                (vgate_and(1, 3), NegInput1), // 7
                (vgate_and(2, 3), NegInput1), // 8
                (vgate_or(0, 3), NegInput1),  // 9
            ],
            outputs: vec![(5, false), (6, false), (7, false), (8, false), (9, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(0, 1), NegOutput), // 3
                    (vgate_or(3, 3), NegInput1),  // 4
                    (vgate_or(2, 4), NoNegs),     // 5
                    (vgate_and(0, 3), NoNegs),    // 6
                    (vgate_and(1, 3), NoNegs),    // 7
                    (vgate_and(2, 3), NoNegs),    // 8
                    (vgate_or(0, 3), NoNegs),     // 9
                ],
                outputs: vec![(5, false), (6, false), (7, false), (8, false), (9, false)],
            },
            circuit
        );

        let mut circuit = VBinOpCircuit {
            input_len: 3,
            gates: vec![
                (vgate_and(0, 1), NoNegs),    // 3
                (vgate_or(3, 3), NoNegs),     // 4
                (vgate_or(2, 4), NoNegs),     // 5
                (vgate_and(0, 3), NegInput1), // 6
                (vgate_and(1, 3), NegInput1), // 7
                (vgate_and(2, 3), NegInput1), // 8
                (vgate_or(0, 3), NegInput1),  // 9
            ],
            outputs: vec![(5, false), (6, false), (7, false), (8, false), (9, false)],
        };
        circuit.optimize_negs();
        assert_eq!(
            VBinOpCircuit {
                input_len: 3,
                gates: vec![
                    (vgate_and(0, 1), NegOutput), // 3
                    (vgate_and(3, 3), NoNegs),    // 4
                    (vgate_or(2, 4), NegInput1),  // 5
                    (vgate_and(0, 3), NoNegs),    // 6
                    (vgate_and(1, 3), NoNegs),    // 7
                    (vgate_and(2, 3), NoNegs),    // 8
                    (vgate_or(0, 3), NoNegs),     // 9
                ],
                outputs: vec![(5, false), (6, false), (7, false), (8, false), (9, false)],
            },
            circuit
        );
    }
}
