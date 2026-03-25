//! Pure functions for execution cache logic.
//!
//! Deterministic, no I/O, no async. Formally verified — see `verus/` for proofs.

pub mod cache_key;

pub use cache_key::compute_cache_key;
pub use cache_key::compute_env_hash;
