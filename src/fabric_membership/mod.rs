//! Canonical evidence and thin adapters for membership and placement.

mod adapters;
mod canonical;

pub use adapters::*;
pub use canonical::*;
pub use molten_core::fabric_membership::*;

#[cfg(test)]
mod tests;
