//! Verified pure functions for Raft consensus operations.
//!
//! This module contains the production implementations of pure business logic
//! for distributed consensus. All functions are:
//!
//! - **Deterministic**: No I/O, no system calls, time passed as explicit parameter
//! - **Verified**: Formally proved correct using Verus (see `verus/` directory)
//! - **Production-ready**: Compiled normally by cargo with no ghost code overhead
//!
//! # Architecture
//!
//! This module implements the "Functional Core, Imperative Shell" (FCIS) pattern:
//!
//! - **verified/** (this module): Production exec functions compiled by cargo
//! - **verus/**: Standalone Verus specs with ensures/requires clauses verified by Verus
//!
//! # Module Organization
//!
//! - [`heuristics`]: TTL calculation, clock drift detection, supervisor logic, connection pooling,
//!   and node failure detection
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
mod auth;
pub mod conversion;
mod encoding;
mod heuristics;
mod integrity;
pub mod kv;
pub mod membership;
mod network;
pub mod scan;
mod write_batcher;

// ============================================================================
// Re-exports: Heuristics
// ============================================================================

// TTL Calculation
// ============================================================================
// Re-exports: Auth
// ============================================================================

// Constants (AUTH_HMAC_SIZE same as integrity module's hash size)
pub use auth::AUTH_CHALLENGE_MAX_AGE_SECS;
pub use auth::AUTH_NONCE_SIZE;
// Challenge Validation
pub use auth::calculate_challenge_age_ms;
// Key Derivation
pub use auth::derive_hmac_key;
pub use auth::is_challenge_valid;
// Batch Operations
pub use conversion::BatchSizeError;
// Compare Constants
pub use conversion::COMPARE_OP_EQUAL;
pub use conversion::COMPARE_OP_GREATER;
pub use conversion::COMPARE_OP_LESS;
pub use conversion::COMPARE_OP_NOT_EQUAL;
pub use conversion::COMPARE_TARGET_CREATE_REVISION;
pub use conversion::COMPARE_TARGET_MOD_REVISION;
pub use conversion::COMPARE_TARGET_VALUE;
pub use conversion::COMPARE_TARGET_VERSION;
// ============================================================================
// Re-exports: Conversion (Wire Format Encoding)
// ============================================================================

// Condition Constants
pub use conversion::CONDITION_KEY_EXISTS;
pub use conversion::CONDITION_KEY_NOT_EXISTS;
pub use conversion::CONDITION_VALUE_EQUALS;
pub use conversion::CompactBatchOp;
// Condition Validation
pub use conversion::ConditionResult;
// Transaction Constants
pub use conversion::TXN_OP_DELETE;
pub use conversion::TXN_OP_GET;
pub use conversion::TXN_OP_PUT;
pub use conversion::TXN_OP_RANGE;
pub use conversion::check_condition_met;
// TTL Computation
pub use conversion::compute_ttl_expiration_ms;
pub use conversion::decode_batch_op;
pub use conversion::encode_batch_op;
pub use conversion::evaluate_conditions;
pub use conversion::validate_batch_size;
// ============================================================================
// Re-exports: Encoding
// ============================================================================
pub use encoding::SHARD_PREFIX_SIZE;
pub use encoding::decode_shard_prefix;
pub use encoding::encode_shard_prefix;
pub use encoding::try_decode_shard_prefix;
// Supervisor
pub use heuristics::calculate_backoff_duration;
// Connection Pool
pub use heuristics::calculate_connection_retry_backoff;
pub use heuristics::calculate_expires_at_ms;
// Clock Drift Detection
pub use heuristics::calculate_ntp_clock_offset;
pub use heuristics::classify_drift_severity;
// Node Failure Detection
pub use heuristics::classify_node_failure;
pub use heuristics::compute_ewma;
pub use heuristics::should_allow_restart;
pub use heuristics::should_evict_oldest_unreachable;
pub use heuristics::transition_connection_health;
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
// Cluster State Building
pub use membership::ClassifiedNodes;
// Membership Building
pub use membership::MembershipError;
pub use membership::build_new_membership;
// ============================================================================
// Re-exports: Membership
// ============================================================================

// Learner Lag
pub use membership::calculate_promotion_threshold;
// Quorum Calculation
pub use membership::calculate_quorum_size;
pub use membership::can_remove_voter_safely;
pub use membership::classify_nodes_by_role;
pub use membership::collect_voter_ids;
pub use membership::compute_learner_lag;
pub use membership::fault_tolerance_for_size;
pub use membership::has_quorum;
pub use membership::is_learner_caught_up;
pub use membership::is_promotion_eligible;
pub use membership::min_cluster_size_for_tolerance;
pub use membership::would_exceed_max_voters;
// Response Handling
pub use network::classify_response_health;
// ============================================================================
// Re-exports: Network
// ============================================================================

// RPC Error Classification
pub use network::classify_rpc_error;
pub use network::deserialize_rpc_response;
// Sharded Message Handling
pub use network::extract_sharded_response;
pub use network::maybe_prefix_shard_id;
// Pagination Logic
pub use scan::PaginationResult;
pub use scan::compute_pagination_result;
pub use scan::compute_safe_scan_limit;
// ============================================================================
// Re-exports: Scan and Pagination
// ============================================================================

// Continuation Tokens
pub use scan::decode_continuation_token;
pub use scan::encode_continuation_token;
pub use scan::filter_kv_pairs_after_token;
// ============================================================================
// Re-exports: Write Batcher
// ============================================================================

// Batch Limit Checking
pub use write_batcher::BatchLimitCheck;
// Flush Decision
pub use write_batcher::FlushDecision;
// Size Calculation
pub use write_batcher::calculate_delete_op_size;
pub use write_batcher::calculate_set_op_size;
pub use write_batcher::check_batch_limits;
pub use write_batcher::determine_flush_action;

// Note: constant_time_compare is exported from integrity module

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
