//! Pure verified functions for deployment orchestration.
//!
//! These functions are deterministic, have no I/O, and no async.
//! Time and configuration are passed as explicit parameters.
//!
//! Formally verified — see `verus/quorum_spec.rs` for proofs.
//!
//! # Functions
//!
//! - [`quorum::quorum_size`]: Minimum voters needed for majority
//! - [`quorum::can_upgrade_node`]: Check if upgrading one more node is safe
//! - [`quorum::max_concurrent_upgrades`]: Upper bound on parallel upgrades

pub mod quorum;
pub use quorum::*;
