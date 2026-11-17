//! Bounded Molten chain projection and operation-identity accounting.
//!
//! The optional `chaoscontrol-conformance` consumer encodes committed-apply
//! facts for the pinned producer contract and imports bounded instrumentation
//! data. Neither projection nor import attests to guest or KVM execution.
//! Guest control and execution qualification remain separate shell concerns.

mod identity;
mod ledger;
mod projection;

#[cfg(feature = "chaoscontrol-conformance")]
pub mod conformance;

pub use identity::*;
pub use ledger::*;
pub use projection::*;

#[cfg(test)]
mod tests;
