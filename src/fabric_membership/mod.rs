//! Canonical evidence and thin adapters for membership and placement.

#![allow(
    tigerstyle::module_file_count,
    reason = "membership separates core projection, ports, shells, mechanisms, and tests into owned modules"
)]

mod adapters;
mod canonical;
mod ports;
mod shell;

pub use adapters::*;
pub use canonical::*;
pub use molten_core::fabric_membership::*;
pub use ports::*;
pub use shell::*;

#[cfg(test)]
mod tests;
