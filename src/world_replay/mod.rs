//! World replay shell, canonical records, adapters, and explicit effect ports.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the replay shell composes explicit application-owned ports"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world replay protocol names remain explicit at the product boundary"
)]

mod adapters;
mod ports;
mod records;
mod service;

pub use adapters::*;
pub use ports::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
