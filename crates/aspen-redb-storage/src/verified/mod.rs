//! Pure verified functions for storage operations.
//!
//! This module contains pure functions for storage operations including:
//! - KV entry version computation, TTL expiration, CAS validation
//! - Chain hashing for integrity verification
//! - TTL calculation helpers
//!
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - All calculations use saturating arithmetic to prevent overflow
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes
//! - Fixed size types prevent unbounded allocation

pub mod heuristics;
pub mod integrity;
pub mod kv;

// ============================================================================
// Re-exports: Heuristics
// ============================================================================

pub use heuristics::calculate_expires_at_ms;
// ============================================================================
// Re-exports: Integrity (Chain Hashing)
// ============================================================================
pub use integrity::ChainCorruption;
pub use integrity::ChainHash;
pub use integrity::ChainTipState;
pub use integrity::GENESIS_HASH;
pub use integrity::SnapshotIntegrity;
pub use integrity::compute_entry_hash;
pub use integrity::constant_time_compare;
pub use integrity::hash_from_hex;
pub use integrity::hash_to_hex;
pub use integrity::verify_entry_hash;
// CAS Validation
pub use kv::CasValidationError;
// ============================================================================
// Re-exports: KV Storage
// ============================================================================

// Version Computation
pub use kv::KvVersions;
// Lease Entry
pub use kv::LeaseEntryData;
pub use kv::check_cas_condition;
// TTL Expiration
pub use kv::compute_key_expiration;
pub use kv::compute_kv_versions;
pub use kv::compute_lease_refresh;
pub use kv::create_lease_entry;
pub use kv::is_lease_expired;
pub use kv::validate_cas_precondition;
pub use kv::validate_cas_precondition_str;
