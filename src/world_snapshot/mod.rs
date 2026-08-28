#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-snapshot shell owns canonical records and later narrow effect adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world-snapshot protocol names stay explicit at the product boundary"
)]

mod chaoscontrol;
mod ports;
mod records;
mod service;
#[cfg(feature = "world-snapshot-vm-cohort")]
mod vm;

pub use chaoscontrol::*;
pub use ports::*;
pub use records::*;
pub use service::*;
#[cfg(feature = "world-snapshot-vm-cohort")]
pub use vm::*;

#[cfg(test)]
mod tests;
