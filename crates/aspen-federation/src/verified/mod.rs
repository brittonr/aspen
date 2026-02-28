//! Verified pure functions for federation.
//!
//! These functions are deterministic, side-effect-free, and suitable for
//! formal verification with Verus. They form the functional core of
//! federation's quorum and fork detection logic.
//!
//! # Formally Verified — see `verus/` for proofs (when added).

pub mod fork_detection;
pub mod quorum;

pub use fork_detection::ForkBranch;
pub use fork_detection::ForkInfo;
pub use fork_detection::detect_ref_forks;
pub use quorum::QuorumCheckResult;
pub use quorum::QuorumFailure;
pub use quorum::SeederReport;
pub use quorum::calculate_seeder_quorum;
pub use quorum::check_quorum;
