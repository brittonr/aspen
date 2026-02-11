//! Redb-based storage backend for Aspen Raft consensus.
//!
//! This crate provides the storage layer for Aspen's Raft implementation,
//! built on top of the redb embedded database. It implements both Raft log
//! storage and state machine storage in a single unified backend, enabling
//! single-fsync writes for optimal performance (~2-3ms latency).
//!
//! # Architecture
//!
//! The storage layer is organized into several components:
//!
//! - **Verified functions** ([`verified`]): Pure, deterministic functions for storage operations
//!   including KV versioning, chain hashing, and CAS validation
//! - **Storage implementations** (feature-gated): Redb log store, state machine, and unified
//!   SharedRedbStorage
//!
//! # Key Features
//!
//! - **Single-fsync writes**: Log entries and state mutations are bundled into a single
//!   transaction, reducing fsync overhead
//! - **Chain integrity**: Blake3 chain hashing verifies log entry integrity
//! - **Pure verified functions**: Core business logic is deterministic and formally verified using
//!   Verus
//!
//! # Tiger Style
//!
//! - Fixed limits on batch sizes (MAX_BATCH_SIZE, MAX_SETMULTI_KEYS)
//! - Explicit error types with actionable context
//! - Bounded operations prevent unbounded memory use
//! - Saturating arithmetic in all pure functions
//!
//! # Example
//!
//! ```ignore
//! use aspen_redb_storage::verified::{compute_kv_versions, check_cas_condition};
//!
//! // Compute version fields for a new key
//! let versions = compute_kv_versions(None, 100);
//! assert_eq!(versions.version, 1);
//!
//! // Check CAS condition
//! assert!(check_cas_condition(Some("expected"), Some("expected")));
//! ```

// Pure verified functions (always available)
pub mod verified;

// Re-export constants from aspen-core
pub use aspen_core::INTEGRITY_VERSION;
pub use aspen_core::MAX_BATCH_SIZE;
pub use aspen_core::MAX_SETMULTI_KEYS;
pub use aspen_core::MAX_SNAPSHOT_ENTRIES;
// Re-export commonly used verified types and functions at crate root
pub use verified::CasValidationError;
pub use verified::ChainCorruption;
pub use verified::ChainHash;
pub use verified::ChainTipState;
pub use verified::GENESIS_HASH;
pub use verified::KvVersions;
pub use verified::LeaseEntryData;
pub use verified::SnapshotIntegrity;
pub use verified::calculate_expires_at_ms;
pub use verified::check_cas_condition;
pub use verified::compute_entry_hash;
pub use verified::compute_key_expiration;
pub use verified::compute_kv_versions;
pub use verified::compute_lease_refresh;
pub use verified::constant_time_compare;
pub use verified::create_lease_entry;
pub use verified::hash_from_hex;
pub use verified::hash_to_hex;
pub use verified::is_lease_expired;
pub use verified::validate_cas_precondition;
pub use verified::validate_cas_precondition_str;
pub use verified::verify_entry_hash;

// Storage modules will be added as the extraction continues
// TODO: Add storage.rs, storage_shared.rs, storage_validation.rs modules
// after updating their imports to use the verified functions from this crate

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verified_functions_accessible() {
        // Verify pure functions are accessible from crate root
        let versions = compute_kv_versions(None, 100);
        assert_eq!(versions.version, 1);
        assert_eq!(versions.create_revision, 100);
        assert_eq!(versions.mod_revision, 100);
    }

    #[test]
    fn test_chain_hash_functions_accessible() {
        let hash = compute_entry_hash(&GENESIS_HASH, 1, 1, b"test");
        assert_ne!(hash, GENESIS_HASH);
        assert!(verify_entry_hash(&GENESIS_HASH, 1, 1, b"test", &hash));
    }

    #[test]
    fn test_cas_functions_accessible() {
        assert!(check_cas_condition(None, None));
        assert!(check_cas_condition(Some("value"), Some("value")));
        assert!(!check_cas_condition(Some("a"), Some("b")));
    }

    #[test]
    fn test_lease_functions_accessible() {
        let lease = create_lease_entry(3600, 1000);
        assert_eq!(lease.ttl_seconds, 3600);
        assert_eq!(lease.expires_at_ms, 1000 + 3600 * 1000);

        assert!(!is_lease_expired(lease.expires_at_ms, 1000));
        assert!(is_lease_expired(lease.expires_at_ms, lease.expires_at_ms + 1));
    }
}
