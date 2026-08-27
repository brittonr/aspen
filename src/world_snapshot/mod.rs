#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-snapshot shell owns canonical records and later narrow effect adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world-snapshot protocol names stay explicit at the product boundary"
)]

mod ports;
mod records;
mod service;

pub use ports::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
