#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-commit boundary composes explicit domain DTOs, canonical records, and narrow port contracts"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public world-commit names remain explicit because the crate contains other commit and evidence domains"
)]

mod codec;
mod records;
mod shell;
mod store;
mod wire;

pub use codec::*;
pub use records::*;
pub use shell::*;
pub use store::*;
pub use wire::*;

#[cfg(test)]
mod tests;
