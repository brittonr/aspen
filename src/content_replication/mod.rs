#![allow(
    tigerstyle::path_segment_repetition,
    reason = "replication boundary names stay explicit when imported by product adapters"
)]

mod conformance;
mod model;
mod multiprocess;
mod ports;
mod records;
mod service;

pub use conformance::*;
pub use model::*;
pub use multiprocess::*;
pub use ports::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
