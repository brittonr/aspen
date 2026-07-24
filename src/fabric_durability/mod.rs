//! Canonical durable-state artifacts and adapter shells.
//!
//! Pure transition laws are owned by `molten-core`; this module adds canonical
//! Preserves/BLAKE3 evidence plus capability-rooted Redb and deterministic
//! adapter shells.

mod adapters;
mod canonical;

#[cfg(test)]
pub(crate) mod tests;

pub use adapters::*;
pub use canonical::*;
pub use molten_core::fabric_durability::*;
