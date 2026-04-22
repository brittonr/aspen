//! Core API types and traits for Aspen distributed systems.
//!
//! This crate provides the alloc-backed foundational types, traits, constants,
//! and verified helpers used throughout the Aspen ecosystem.
//!
//! # Key Components
//!
//! - **Traits**: `ClusterController`, `KeyValueStore`, `SqlQueryExecutor`
//! - **Types**: `NodeId`, `NodeAddress`, `WriteCommand`, `ReadRequest`, etc.
//! - **Storage**: alloc-safe storage record types such as `KvEntry`
//! - **Constants**: Tiger Style resource limits
//! - **Verified helpers**: Pure scan helpers and HLC helpers that remain
//!   alloc-safe and portable
//!
//! # Feature Flags
//!
//! - `sql`: Include alloc-safe SQL query types and traits

#![cfg_attr(not(test), no_std)]

// Phase 3 Tiger Style rollout: keep the current pilot families visible in pilot
// crates while suppressing noisier families until Aspen has cleanup bandwidth.
#![allow(unknown_lints)]
#![allow(no_panic)]
#![deny(ambient_clock, compound_assertion, contradictory_time, ignored_result, no_unwrap)]
#![allow(
    acronym_style,
    ambiguous_params,
    assertion_density,
    bool_naming,
    catch_all_on_enum,
    compound_condition,
    float_for_currency,
    function_length,
    multi_lock_ordering,
    nested_conditionals,
    no_recursion,
    numeric_units,
    platform_dependent_cast,
    raw_arithmetic_overflow,
    unbounded_loop,
    unchecked_division,
    unchecked_narrowing,
    unjustified_allow,
    verified_purity
)]

extern crate alloc;

pub mod circuit_breaker;
pub mod cluster;
pub mod constants;
pub mod crypto;
pub mod error;
pub mod hlc;
pub mod kv;
pub mod prelude;
pub mod protocol;
pub mod spec;
#[cfg(feature = "sql")]
pub mod sql;
pub mod storage;
pub mod traits;
pub mod types;
pub mod vault;
/// Verified pure functions for core distributed system operations.
pub mod verified;

/// Test support utilities (test-only).
///
/// Provides minimal deterministic implementations of core traits for internal
/// unit tests. For external testing, use `aspen-testing`.
#[cfg(test)]
pub(crate) mod test_support;

// Re-export all public types at crate root for convenience.

// Cluster types
pub use cluster::AddLearnerRequest;
pub use cluster::ChangeMembershipRequest;
pub use cluster::ClusterNode;
pub use cluster::ClusterState;
pub use cluster::InitRequest;
pub use cluster::TrustConfig;
// Re-export all constants at crate root for backward compatibility.
pub use constants::api::*;
pub use constants::ci::*;
pub use constants::coordination::*;
pub use constants::directory::*;
pub use constants::network::*;
pub use constants::raft::*;
// Crypto types
pub use crypto::Signature;
// Error types
pub use error::ControlPlaneError;
pub use error::KeyValueStoreError;
// HLC types
pub use hlc::HLC;
pub use hlc::HlcTimestamp;
pub use hlc::ID as HlcId;
pub use hlc::NTP64;
pub use hlc::SerializableTimestamp;
pub use hlc::create_hlc;
pub use hlc::new_timestamp;
pub use hlc::to_unix_ms;
pub use hlc::update_from_timestamp;
// KV types
pub use kv::BatchCondition;
pub use kv::BatchOperation;
pub use kv::CompareOp;
pub use kv::CompareTarget;
pub use kv::DeleteRequest;
pub use kv::DeleteResult;
pub use kv::KeyValueWithRevision;
pub use kv::ReadConsistency;
pub use kv::ReadRequest;
pub use kv::ReadResult;
pub use kv::ScanRequest;
pub use kv::ScanResult;
pub use kv::TxnCompare;
pub use kv::TxnOp;
pub use kv::TxnOpResult;
pub use kv::WriteCommand;
pub use kv::WriteOp;
pub use kv::WriteRequest;
pub use kv::WriteResult;
pub use kv::validate_write_command;
// Protocol types (sans-IO infrastructure)
pub use protocol::Alarm;
pub use protocol::Envelope;
pub use protocol::ProtocolCtx;
pub use protocol::TestCtx;
#[cfg(feature = "sql")]
pub use sql::SqlColumnInfo;
#[cfg(feature = "sql")]
pub use sql::SqlConsistency;
#[cfg(feature = "sql")]
pub use sql::SqlQueryError;
#[cfg(feature = "sql")]
pub use sql::SqlQueryExecutor;
#[cfg(feature = "sql")]
pub use sql::SqlQueryRequest;
#[cfg(feature = "sql")]
pub use sql::SqlQueryResult;
#[cfg(feature = "sql")]
pub use sql::SqlValue;
#[cfg(feature = "sql")]
pub use sql::effective_sql_limit;
#[cfg(feature = "sql")]
pub use sql::effective_sql_timeout_ms;
#[cfg(feature = "sql")]
pub use sql::validate_sql_query;
#[cfg(feature = "sql")]
pub use sql::validate_sql_request;
// Storage types
pub use storage::KvEntry;
// Traits
pub use traits::ClusterController;
pub use traits::CoordinationBackend;
pub use traits::KeyValueStore;
// Types
pub use types::ClusterMetrics;
pub use types::NodeAddress;
pub use types::NodeId;
pub use types::NodeState;
pub use types::SnapshotLogId;
// Vault types
pub use vault::SYSTEM_PREFIX;
pub use vault::VaultError;
pub use vault::is_system_key;
pub use vault::validate_client_key;
// Verified functions (pure, formally verified)
pub use verified::build_scan_metadata;
pub use verified::decode_continuation_token;
pub use verified::encode_continuation_token;
pub use verified::execute_scan;
pub use verified::filter_scan_entries;
pub use verified::normalize_scan_limit;
pub use verified::paginate_entries;
