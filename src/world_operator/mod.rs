//! Preview-first shell composition for Molten world operations.
//!
//! Each registered handler delegates one operation kind to its existing owner.
//! The workflow shell orders those handlers and links their records. It does
//! not reimplement component domain decisions.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world operator shell composes explicit core plans and component adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world operator protocol names remain explicit at the product boundary"
)]

mod ports;
mod records;
mod service;

pub use ports::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
