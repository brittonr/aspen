//! Canonical durable-state artifacts and adapter shells.
//!
//! Pure transition laws are owned by `molten-core`; this module adds canonical
//! Preserves/BLAKE3 evidence plus capability-rooted Redb and deterministic
//! adapter shells.

#![allow(
    tigerstyle::module_file_count,
    reason = "durability separates core projection, ports, shells, mechanisms, and tests into owned modules"
)]

mod adapters;
mod canonical;
mod ports;
mod shell;

#[cfg(test)]
pub(crate) mod tests;

pub use adapters::*;
pub use canonical::*;
pub use molten_core::fabric_durability::*;
pub use ports::*;
pub use shell::*;
