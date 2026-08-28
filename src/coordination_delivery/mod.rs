//! Coordination-delivery imperative shell.
//!
//! The pure transition core is in `molten_core::coordination_delivery`. This
//! module owns capability-rooted persistence, compare-and-commit orchestration,
//! logical timer requests, status publication, reconciliation, and canonical
//! receipts.

mod host;
mod ports;
mod records;
mod service;
mod simulation;
mod store;

pub use host::*;
pub use ports::*;
pub use records::*;
pub use service::*;
pub use simulation::*;
pub use store::*;

#[cfg(test)]
mod tests;
