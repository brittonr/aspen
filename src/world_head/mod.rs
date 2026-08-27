#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-head shell visibly composes canonical records, crypto adapters, ports, and durable storage"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world-head protocol names remain explicit at the product boundary"
)]

mod ports;
mod records;
mod service;
mod signing;
mod store;

pub use ports::*;
pub use records::*;
pub use service::*;
pub use signing::*;
pub use store::*;

#[cfg(test)]
mod tests;
