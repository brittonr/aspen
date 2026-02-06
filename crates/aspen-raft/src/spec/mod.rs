//! Formal verification specifications for storage_shared.rs
//!
//! This module contains Verus specifications that prove correctness
//! of the single-fsync storage implementation.
//!
//! # Overview
//!
//! The specs verify seven key invariants:
//!
//! 1. **Log-State Atomicity**: Either both log and state are durable, or neither
//! 2. **Chain Continuity**: Each entry's hash depends on its predecessor
//! 3. **Chain Tip Synchronization**: chain_tip always reflects the latest entry
//! 4. **Response Cache Consistency**: Responses are cached for applied entries only
//! 5. **Monotonic last_applied**: last_applied never decreases
//! 6. **Monotonic Purge**: last_purged never decreases
//! 7. **Snapshot Integrity**: Snapshots include valid chain hashes
//!
//! # Module Structure
//!
//! - `chain_hash`: Specifications for chain hash computation
//! - `storage_state`: State machine model for verification
//! - `append`: Verification of append() operation
//! - `truncate`: Verification of truncate() operation
//! - `purge`: Verification of purge() operation
//! - `snapshot`: Verification of snapshot operations
//!
//! # Usage
//!
//! These specifications are only compiled when the `verus` feature is enabled:
//!
//! ```bash
//! # Verify specifications
//! verus --crate-type=lib crates/aspen-raft/src/spec/chain_hash.rs
//!
//! # Or run the verification script
//! ./scripts/verify-storage.sh
//! ```
//!
//! # Design Rationale
//!
//! The specifications model the storage layer as a state machine with:
//! - Abstract log entries (index, term, data)
//! - Chain hashes linking each entry to its predecessor
//! - State machine state (KV store, leases, last_applied)
//!
//! Each operation (append, truncate, purge, snapshot) is specified with:
//! - Preconditions (what must hold before the operation)
//! - Postconditions (what holds after the operation)
//! - Preservation proofs (which invariants are maintained)
//!
//! # Trusted Axioms
//!
//! Blake3 is modeled as an uninterpreted function with two trusted axioms:
//! - Output is always 32 bytes
//! - The function is deterministic (same input -> same output)
//!
//! These are standard cryptographic assumptions that don't require proof.

pub mod append;
pub mod chain_hash;
pub mod purge;
pub mod snapshot;
pub mod storage_state;
pub mod truncate;

// Re-export key specifications for use in storage_shared.rs
pub use chain_hash::ChainHashSpec;
pub use chain_hash::GENESIS_HASH_SPEC;
pub use storage_state::StorageStateSpec;
