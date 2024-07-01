#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VNegs {
    NoNegs,
    NegInput1, // second input in gate
    NegOutput,
}

pub mod cnf;
pub mod vbinopcircuit;
pub mod vcircuit;

pub fn map_to_string<T: ToString>(map: &[Option<T>]) -> String {
    map.into_iter()
        .map(|x| {
            x.as_ref()
                .map(|x| (*x).to_string())
                .unwrap_or("-".to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}
