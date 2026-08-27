#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-merge shell composes canonical records, explicit ports, and publish-last orchestration"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world-merge protocol names remain explicit at the product boundary"
)]

mod ports;
mod records;
mod service;

pub use ports::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
