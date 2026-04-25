//! Pure, deterministic functions suitable for Verus formal verification.
//!
//! All functions in this module have no I/O, no async, and no time dependency.
//! Time, randomness, and configuration are passed as explicit parameters.
//!
//! Formally verified — see `verus/` for proofs.

pub mod commit_hash;
pub mod diff;
pub mod hash;

pub use hash::CHAIN_HASH_BYTES;
pub use hash::ChainHash;
pub use hash::GENESIS_HASH;
pub use hash::constant_time_compare;
pub use hash::hash_from_hex;
pub use hash::hash_to_hex;
