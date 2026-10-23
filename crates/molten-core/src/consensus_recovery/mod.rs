//! Protocol-aware recovery of Raft state after a storage fault.
//!
//! Local database repair restores structure; it does not prove that a replica
//! retained committed history. This core decides repair from Raft facts keyed
//! by term and index: what the participant's own storage holds and what
//! bounded admitted peer reports establish. It never plans the truncation of a
//! committed entry, discards an entry only when a quorum establishes that it
//! was not committed, and waits when commitment is unknown. Term and vote state
//! is node-specific and is never copied from a peer. Networking, persistence,
//! and time stay in the shell, which executes the typed plans.

#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded recovery decisions and local collections, never infrastructure adapters"
)]

mod admission;
mod progress;
mod receipt;
mod repair;

pub use admission::*;
pub use progress::*;
pub use receipt::*;
pub use repair::*;

#[cfg(test)]
mod tests;

/// The most peer reports one decision weighs.
pub const MAX_RECOVERY_PEER_REPORTS: usize = 64;
