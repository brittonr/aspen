//! Pure functions extracted from Raft module for improved testability.
//!
//! This module implements the "Functional Core, Imperative Shell" pattern by
//! extracting pure business logic from impure async functions. All functions
//! here are deterministic and side-effect free, making them ideal for:
//!
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Fuzzing for edge case discovery
//! - Formal verification with Verus
//! - WASM compilation (no async/I/O dependencies)
//!
//! # Module Organization
//!
//! - [`heuristics`]: TTL calculation, clock drift detection, supervisor logic,
//!   connection pooling, and node failure detection
//! - [`integrity`]: Chain hashing for Raft log integrity verification (Blake3)
//! - [`encoding`]: Binary encoding/decoding for wire protocols
//! - [`network`]: RPC error classification, sharded message handling, response health
//! - [`kv`]: KV entry versioning, TTL computation, CAS validation, lease construction
//!
//! # Tiger Style
//!
//! - All calculations bounded by explicit limits from constants.rs
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes
//! - Fixed size types prevent unbounded allocation

// Submodules
mod encoding;
mod heuristics;
mod integrity;
pub mod kv;
mod network;

// ============================================================================
// Re-exports: Heuristics
// ============================================================================

// TTL Calculation
pub use heuristics::calculate_expires_at_ms;

// Clock Drift Detection
pub use heuristics::calculate_ntp_clock_offset;
pub use heuristics::classify_drift_severity;
pub use heuristics::compute_ewma;

// Supervisor
pub use heuristics::calculate_backoff_duration;
pub use heuristics::should_allow_restart;

// Connection Pool
pub use heuristics::calculate_connection_retry_backoff;
pub use heuristics::transition_connection_health;

// Node Failure Detection
pub use heuristics::classify_node_failure;
pub use heuristics::should_evict_oldest_unreachable;

// ============================================================================
// Re-exports: Integrity (Chain Hashing)
// ============================================================================

// Types
pub use integrity::ChainCorruption;
pub use integrity::ChainHash;
pub use integrity::ChainTipState;
pub use integrity::GENESIS_HASH;
pub use integrity::SnapshotIntegrity;

// Functions
pub use integrity::compute_entry_hash;
pub use integrity::constant_time_compare;
pub use integrity::hash_from_hex;
pub use integrity::hash_to_hex;
pub use integrity::verify_entry_hash;

// ============================================================================
// Re-exports: Encoding
// ============================================================================

pub use encoding::SHARD_PREFIX_SIZE;
pub use encoding::decode_shard_prefix;
pub use encoding::encode_shard_prefix;
pub use encoding::try_decode_shard_prefix;

// ============================================================================
// Re-exports: Network
// ============================================================================

// RPC Error Classification
pub use network::classify_rpc_error;

// Sharded Message Handling
pub use network::extract_sharded_response;
pub use network::maybe_prefix_shard_id;

// Response Handling
pub use network::classify_response_health;
pub use network::deserialize_rpc_response;

// ============================================================================
// Re-exports: KV Storage
// ============================================================================

// Version Computation
pub use kv::KvVersions;
pub use kv::compute_kv_versions;

// TTL Expiration
pub use kv::compute_key_expiration;

// CAS Validation
pub use kv::CasValidationError;
pub use kv::check_cas_condition;
pub use kv::validate_cas_precondition;
pub use kv::validate_cas_precondition_str;

// Lease Entry
pub use kv::LeaseEntryData;
pub use kv::compute_lease_refresh;
pub use kv::create_lease_entry;
pub use kv::is_lease_expired;

// ============================================================================
// Re-exports: Spec Predicates (Verification)
// ============================================================================
//
// These predicates are pure functions used for formal verification and testing
// of storage state invariants. They are extracted from the spec module to make
// them accessible alongside other pure functions.
//
// Invariant Predicates:
// - `chain_tip_synchronized`: Verifies chain tip matches the latest log entry
// - `last_applied_monotonic`: Verifies last_applied only increases
// - `purge_monotonic`: Verifies last_purged only increases
// - `response_cache_consistent`: Verifies response cache entries are valid
// - `storage_invariant`: Combined check of all storage invariants

// Types
pub use crate::spec::storage_state::GhostStorageState;
pub use crate::spec::storage_state::KvEntrySpec;
pub use crate::spec::storage_state::StorageStateSpec;

// Predicate Functions
pub use crate::spec::storage_state::chain_tip_synchronized;
pub use crate::spec::storage_state::last_applied_monotonic;
pub use crate::spec::storage_state::purge_monotonic;
pub use crate::spec::storage_state::response_cache_consistent;
pub use crate::spec::storage_state::storage_invariant;
