#![allow(
    tigerstyle::non_trait_imports,
    reason = "world promotion shell adapters keep transaction and dispatch effects explicit"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world promotion protocol names remain explicit at the product boundary"
)]

mod current;
mod ports;
mod records;
mod service;
mod store;

pub use current::*;
pub use ports::*;
pub use records::*;
pub use service::*;
pub use store::*;

#[cfg(test)]
#[path = "currenttests.rs"]
mod currenttests;

#[cfg(test)]
mod tests;
