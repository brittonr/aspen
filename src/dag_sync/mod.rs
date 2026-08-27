#![allow(
    tigerstyle::non_trait_imports,
    reason = "the DAG-sync shell owns canonical protocol records and later narrow effect adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "DAG-sync protocol names remain explicit at the product boundary"
)]

mod records;

pub use records::*;

#[cfg(test)]
mod tests;
