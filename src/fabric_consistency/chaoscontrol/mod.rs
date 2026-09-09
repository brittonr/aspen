//! Bounded Molten-side foundation for the ChaosControl SMR-chain conformance
//! cohort. The projection, observation accounting, and operation-identity
//! surfaces here are pure deterministic logic over in-memory inputs; guest
//! control, KVM campaigns, bundle reads, and receipt persistence stay in
//! shells owned by later phases of the conformance change.
//!
//! The chain digest framing mirrors the archived ChaosControl
//! `smr-chain` contract byte for byte so both sides derive identical genesis
//! and transition digests. Full cross-replica continuous safety evaluation and
//! external evidence import stay with their own later tasks.

mod identity;
mod ledger;
mod projection;

pub use identity::*;
pub use ledger::*;
pub use projection::*;

#[cfg(test)]
mod tests;
