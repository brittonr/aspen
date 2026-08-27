#![allow(
    tigerstyle::non_trait_imports,
    reason = "world promotion shell adapters keep transaction and dispatch effects explicit"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world promotion protocol names remain explicit at the product boundary"
)]

mod ports;
mod records;
mod service;
mod store;

pub use ports::*;
pub use records::*;
pub use service::*;
pub use store::*;

#[cfg(test)]
mod tests;
