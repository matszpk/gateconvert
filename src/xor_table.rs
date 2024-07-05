// xor table

use gategen::boolexpr::*;
use gategen::dynintexpr::*;
use gategen::VarLit;
use gatesim::*;
use gateutil::*;

use std::cell::RefCell;
use std::env;
use std::fmt::Debug;
use std::ops::Neg;
use std::rc::Rc;
use std::str::FromStr;

pub fn dynint_xor_ite<T, const SIGN: bool>(
    c: BoolExprNode<T>,
    t: DynIntExprNode<T, SIGN>,
    e: DynIntExprNode<T, SIGN>,
) -> DynIntExprNode<T, SIGN>
where
    T: VarLit + Neg<Output = T> + Debug,
    isize: TryFrom<T>,
    <T as TryInto<usize>>::Error: Debug,
    <T as TryFrom<usize>>::Error: Debug,
    <isize as TryFrom<T>>::Error: Debug,
{
    t.clone() ^ (DynIntExprNode::<T, SIGN>::filled_expr(t.len(), c) & e)
}

pub fn dynint_xor_table<T, I, const SIGN: bool>(
    creator: Rc<RefCell<ExprCreator<T>>>,
    index: DynIntExprNode<T, SIGN>,
    table_iter: I,
) -> DynIntExprNode<T, SIGN>
where
    T: VarLit + Neg<Output = T> + Debug,
    isize: TryFrom<T>,
    <T as TryInto<usize>>::Error: Debug,
    <T as TryFrom<usize>>::Error: Debug,
    <isize as TryFrom<T>>::Error: Debug,
    I: IntoIterator<Item = DynIntExprNode<T, SIGN>>,
{
    let mut ites = vec![];
    let mut iter = table_iter.into_iter();
    while let Some(v) = iter.next() {
        if let Some(v2) = iter.next() {
            ites.push(dynint_xor_ite(index.bit(0), v, v2));
        } else {
            panic!("Odd number of elements");
        }
    }

    for step in 1..(index.len()) {
        if (ites.len() & 1) != 0 {
            panic!("Odd number of elements");
        }
        for i in 0..(ites.len() >> 1) {
            ites[i] = dynint_xor_ite(
                index.bit(step),
                ites[i << 1].clone(),
                ites[(i << 1) + 1].clone(),
            );
        }
        ites.resize(
            ites.len() >> 1,
            DynIntExprNode::filled(creator.clone(), ites[0].len(), false),
        );
    }

    ites.pop().unwrap()
}

//              RESULT
//        /-----      ---\
//       XC0             XC4
//    /--   --\       /--   --\
//   XB0     XB2     XB4     XB6
//  /   \   /   \   /   \   /   \
// XA0 XA1 XA2 XA3 XA4 XA5 XA6 XA7
//
// RESULT=A0,A0=XA0
// RESULT=A1,A1=XA0^XA1,XA1=XA0^A1
// RESULT=A2,A2=XA0^XA2,XA2=XA0^A2
// RESULT=A3,A3=XB0^XA2^XA3,XB0=XA0^XA1,XA3=XB0^XA2^A3
// RESULT=A4,A2=XA0^XA4,XA4=XA0^A4
// RESULT=A5,A5=XB0^XA4^XA5,XB0=XA0^XA1,XA5=XB0^XA4^A5
// RESULT=A6,A6=XC0^XA4^XA6,XC0=XA0^XA2,XA6=XC0^XA4^A6
// RESULT=A7,A7=XC0^XB4^XA6^XA7,XC0=XB0^XB2,XB2=XA2^XA3,XB4=XA4^XA5,XA7=XC0^XB4^XA6^A7
pub fn dynint_extend_prep_xor_table<T, I, const SIGN: bool>(
    out: &mut Vec<DynIntExprNode<T, SIGN>>,
    temp_out: &mut Vec<Vec<DynIntExprNode<T, SIGN>>>,
    table_iter: I,
) where
    T: VarLit + Neg<Output = T> + Debug,
    isize: TryFrom<T>,
    <T as TryInto<usize>>::Error: Debug,
    <T as TryFrom<usize>>::Error: Debug,
    <isize as TryFrom<T>>::Error: Debug,
    I: IntoIterator<Item = DynIntExprNode<T, SIGN>>,
{
    let initial_len = out.len();

    for (i, v) in table_iter.into_iter().enumerate() {
        let i = i + initial_len;
        if i.count_ones() == 1 {
            //println!("XX: {}", i);
            // add next level to previous temps
            for v in temp_out.iter_mut() {
                //println!("XXZZ: {} {} {}", i, v.len(), j);
                v.push(v.last().unwrap().clone());
            }
        }
        let mut xv = v.clone();
        temp_out.push(vec![]);
        temp_out[i].push(v.clone());
        if i != 0 {
            let bit_num = (usize::BITS - i.leading_zeros() - 1) as usize;
            //println!("Bitnum:: {} {}", bit_num, i);
            for bit in (0..=bit_num).rev() {
                let shift = 1 << bit;
                if (i & shift) != 0 {
                    xv ^= temp_out[i ^ shift][bit].clone();
                }
                // push to temp_out (tree) - after reverse first level is final XAxxx
                // next are XBxxxx.
                temp_out[i].push(xv.clone());
            }
        }
        // reverse to make correct order of levels in temp_out.
        temp_out[i].reverse();
        out.push(xv.clone());
    }
}

pub fn gen_table_circuit_bool(
    ec: Rc<RefCell<ExprCreatorSys>>,
    int_input: Vec<BoolExprNode<isize>>,
    table: Vec<BoolExprNode<isize>>,
) -> (Circuit<usize>, Vec<Option<usize>>) {
    let table_len = table.len();
    assert_eq!(table_len.count_ones(), 1);
    let input_len = (usize::BITS - table_len.leading_zeros() - 1) as usize;
    let input = if !int_input.is_empty() {
        UDynExprNode::from_boolexprs(int_input)
            .concat(UDynExprNode::variable(ec.clone(), input_len))
    } else {
        UDynExprNode::variable(ec.clone(), input_len)
    };
    let table = table
        .into_iter()
        .map(|x| UDynExprNode::filled_expr(1, x.clone()));
    let mut xor_elem_outputs = vec![];
    let mut temp_elem_outputs = vec![];
    dynint_extend_prep_xor_table(&mut xor_elem_outputs, &mut temp_elem_outputs, table);
    let output = dynint_xor_table(ec.clone(), input.clone(), xor_elem_outputs);
    output.to_translated_circuit_with_map(input.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    use gategen::generic_array::typenum::*;
    use gategen::intexpr::*;

    fn gen_table_circuit(output_len: usize, table: Vec<u64>) -> Circuit<usize> {
        let table_len = table.len();
        assert_eq!(table_len.count_ones(), 1);
        let input_len = (usize::BITS - table_len.leading_zeros() - 1) as usize;
        let ec = ExprCreatorSys::new();
        let input = UDynExprNode::variable(ec.clone(), input_len);
        let table = table
            .into_iter()
            .map(|x| UDynExprNode::try_constant_n(ec.clone(), output_len, x).unwrap());
        let mut xor_elem_outputs = vec![];
        let mut temp_elem_outputs = vec![];
        dynint_extend_prep_xor_table(&mut xor_elem_outputs, &mut temp_elem_outputs, table);
        let output = dynint_xor_table(ec.clone(), input.clone(), xor_elem_outputs);
        let (circuit, input_map) = output.to_circuit();
        let input_list = input_map_to_input_list(input_map, input.iter());
        translate_inputs_rev(circuit, input_list)
    }

    const RIJNDAEL_SBOX_TBL: [u8; 256] = [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab,
        0x76, 0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4,
        0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71,
        0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2,
        0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6,
        0xb3, 0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb,
        0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf, 0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45,
        0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8, 0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5,
        0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44,
        0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a,
        0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49,
        0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d,
        0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08, 0xba, 0x78, 0x25,
        0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70, 0x3e,
        0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e, 0xe1,
        0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
        0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb,
        0x16,
    ];

    fn hash_function_64(bits: usize, value: u64) -> usize {
        let mask = u64::try_from((1u128 << bits) - 1).unwrap();
        let half_bits = bits >> 1;
        let temp = value * 9615409803190489167u64;
        let temp = (temp << half_bits) | (temp >> (bits - half_bits));
        let hash = (value * 6171710485021949031u64) ^ temp ^ 0xb89d2ecda078ca1f;
        usize::try_from(hash & mask).unwrap()
    }

    #[test]
    fn test_dynint_xor_table() {
        let table = vec![5, 11, 14, 0, 6, 3, 13, 2];
        let circuit = gen_table_circuit(4, table.clone());
        for v in 0..8 {
            let out = circuit.eval((0..3).map(|b| ((v >> b) & 1) != 0));
            let out = out
                .into_iter()
                .enumerate()
                .fold(0, |a, (b, x)| a | (u64::from(x) << b));
            assert_eq!(table[v], out);
            println!("Value: {}: {}", v, out);
        }
        let table = vec![
            131, 84, 94, 12, 2, 45, 201, 175, 237, 95, 86, 173, 133, 62, 89, 118,
        ];
        let circuit = gen_table_circuit(8, table.clone());
        for v in 0..16 {
            let out = circuit.eval((0..4).map(|b| ((v >> b) & 1) != 0));
            let out = out
                .into_iter()
                .enumerate()
                .fold(0, |a, (b, x)| a | (u64::from(x) << b));
            assert_eq!(table[v], out);
            println!("Value: {}: {}", v, out);
        }
        let table = RIJNDAEL_SBOX_TBL
            .iter()
            .map(|x| u64::from(*x))
            .collect::<Vec<_>>();
        let circuit = gen_table_circuit(8, table.clone());
        for v in 0..256 {
            let out = circuit.eval((0..8).map(|b| ((v >> b) & 1) != 0));
            let out = out
                .into_iter()
                .enumerate()
                .fold(0, |a, (b, x)| a | (u64::from(x) << b));
            assert_eq!(table[v], out);
            println!("Value: {}: {}", v, out);
        }
        // with input len = 16, table len: 1 << 16 = 65536
        // let table = (0..1 << 16)
        //     .map(|x| u64::try_from(hash_function_64(32, x)).unwrap())
        //     .collect::<Vec<_>>();
        // let circuit = gen_table_circuit(32, table.clone());
        // let mut input_64 = vec![
        //     0xaaaaaaaaaaaaaaaau64,
        //     0xccccccccccccccccu64,
        //     0xf0f0f0f0f0f0f0f0u64,
        //     0xff00ff00ff00ff00u64,
        //     0xffff0000ffff0000u64,
        //     0xffffffff00000000u64,
        //     0,
        //     0,
        //     0,
        //     0,
        //     0,
        //     0,
        //     0,
        //     0,
        //     0,
        //     0,
        // ];
        // for v in 0..1 << (16 - 6) {
        //     for i in 0..(16 - 6) {
        //         input_64[i + 6] = if ((v >> i) & 1) != 0 { u64::MAX } else { 0 };
        //     }
        //     let out = circuit.eval(input_64.clone());
        //     for i in 0..64 {
        //         let out = out
        //             .iter()
        //             .enumerate()
        //             .fold(0, |a, (b, x)| a | (u64::from((x >> i) & 1) << b));
        //         assert_eq!(table[(v << 6) + i], out);
        //         println!("Value: {}: {}", (v << 6) + i, out);
        //     }
        // }
    }
}
