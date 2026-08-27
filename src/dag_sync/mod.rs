#![allow(
    tigerstyle::non_trait_imports,
    reason = "the DAG-sync shell owns canonical protocol records and later narrow effect adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "DAG-sync protocol names remain explicit at the product boundary"
)]

mod conformance;
mod domain;
mod ports;
mod records;
mod service;
mod status;

pub use conformance::*;
pub use domain::*;
pub use ports::*;
pub use records::*;
pub use service::*;
pub use status::*;

#[cfg(test)]
mod tests;
