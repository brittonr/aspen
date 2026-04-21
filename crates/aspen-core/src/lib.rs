//! Core API types and traits for Aspen distributed systems.
//!
//! This crate provides the foundational types, traits, and constants used throughout
//! the Aspen ecosystem. It is designed to be a lightweight dependency that can be
//! used by both the core Aspen crate and external consumers.
//!
//! # Key Components
//!
//! - **Traits**: `ClusterController`, `KeyValueStore`, `SqlQueryExecutor`
//! - **Types**: `NodeId`, `NodeAddress`, `WriteCommand`, `ReadRequest`, etc.
//! - **Storage**: `SM_KV_TABLE`, `KvEntry` (centralized storage constants)
//! - **Constants**: Tiger Style resource limits
//!
//! # Feature Flags
//!
//! - `sql`: Include alloc-safe SQL query types and traits
//! - `std`: Include shell-oriented runtime helpers and re-exports

#![cfg_attr(not(any(test, feature = "std")), no_std)]

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

#[cfg(feature = "std")]
pub mod app_registry;
pub mod circuit_breaker;
pub mod cluster;
pub mod constants;
#[cfg(feature = "std")]
pub mod context;
pub mod crypto;
pub mod error;
pub mod hlc;
pub mod kv;
#[cfg(all(feature = "std", feature = "layer"))]
pub mod layer;
pub mod prelude;
pub mod protocol;
#[cfg(feature = "std")]
pub mod simulation;
pub mod spec;
#[cfg(feature = "sql")]
pub mod sql;
pub mod storage;
pub mod traits;
#[cfg(feature = "std")]
pub mod transport;
pub mod types;
#[cfg(feature = "std")]
pub mod utils;
pub mod vault;
/// Verified pure functions for core distributed system operations.
pub mod verified;

/// Test support utilities (test-only).
///
/// Provides minimal deterministic implementations of core traits for
/// internal unit tests. For external testing, use `aspen-testing`.
#[cfg(all(test, feature = "std"))]
pub(crate) mod test_support;
// Re-export all public types at crate root for convenience

// App registry types
#[cfg(feature = "std")]
pub use app_registry::AppManifest;
#[cfg(feature = "std")]
pub use app_registry::AppRegistry;
#[cfg(feature = "std")]
pub use app_registry::SharedAppRegistry;
#[cfg(feature = "std")]
pub use app_registry::shared_registry;
// Re-export all constants at crate root for backward compatibility
// This allows `use aspen_core::CONSTANT_NAME` instead of `use
// aspen_core::constants::module::CONSTANT_NAME`
// Cluster types
pub use cluster::AddLearnerRequest;
pub use cluster::ChangeMembershipRequest;
pub use cluster::ClusterNode;
pub use cluster::ClusterState;
pub use cluster::InitRequest;
pub use cluster::TrustConfig;
pub use constants::api::*;
pub use constants::ci::*;
pub use constants::coordination::*;
pub use constants::directory::*;
pub use constants::network::*;
pub use constants::raft::*;
// Duration constant re-exports for backward compatibility
#[cfg(feature = "std")]
pub use constants::raft_compat::MEMBERSHIP_COOLDOWN;
// Context traits
#[cfg(feature = "std")]
pub use context::AspenDocsTicket;
#[cfg(all(feature = "std", feature = "global-discovery"))]
pub use context::ContentDiscovery;
#[cfg(all(feature = "std", feature = "global-discovery"))]
pub use context::ContentNodeAddr;
#[cfg(all(feature = "std", feature = "global-discovery"))]
pub use context::ContentProviderInfo;
#[cfg(feature = "std")]
pub use context::DocsEntry;
#[cfg(feature = "std")]
pub use context::DocsStatus;
#[cfg(feature = "std")]
pub use context::DocsSyncProvider;
#[cfg(feature = "std")]
pub use context::EndpointProvider;
// Watch registry types
#[cfg(feature = "std")]
pub use context::InMemoryWatchRegistry;
#[cfg(feature = "std")]
pub use context::KeyOrigin;
#[cfg(feature = "std")]
pub use context::NetworkFactory;
#[cfg(feature = "std")]
pub use context::PeerConnectionState;
#[cfg(feature = "std")]
pub use context::PeerImporter;
#[cfg(feature = "std")]
pub use context::PeerInfo;
#[cfg(feature = "std")]
pub use context::PeerManager;
// Service executor for WASM plugin host dispatch
#[cfg(feature = "std")]
pub use context::ServiceExecutor;
#[cfg(feature = "std")]
pub use context::ShardTopology;
#[cfg(feature = "std")]
pub use context::StateMachineProvider;
#[cfg(feature = "std")]
pub use context::SubscriptionFilter;
#[cfg(feature = "std")]
pub use context::SyncStatus;
#[cfg(feature = "std")]
pub use context::WatchInfo;
#[cfg(feature = "std")]
pub use context::WatchRegistry;
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
// Layer types (Tuple, Subspace, Directory) — requires std + aspen-layer
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::AllocationError;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::DirectoryError;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::DirectoryLayer;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::DirectorySubspace;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::Element;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::HighContentionAllocator;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::Subspace;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::SubspaceError;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::Tuple;
#[cfg(all(feature = "std", feature = "layer"))]
pub use layer::TupleError;
// Protocol types (sans-IO infrastructure)
pub use protocol::Alarm;
pub use protocol::Envelope;
pub use protocol::ProtocolCtx;
pub use protocol::TestCtx;
// Simulation types
#[cfg(feature = "std")]
pub use simulation::SimulationArtifact;
#[cfg(feature = "std")]
pub use simulation::SimulationArtifactBuilder;
#[cfg(feature = "std")]
pub use simulation::SimulationStatus;
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
pub use storage::SM_KV_TABLE;
// Traits
pub use traits::ClusterController;
pub use traits::CoordinationBackend;
pub use traits::KeyValueStore;
// Transport types
#[cfg(feature = "std")]
pub use transport::BlobAnnouncedCallback;
#[cfg(feature = "std")]
pub use transport::BlobAnnouncedInfo;
#[cfg(feature = "std")]
pub use transport::DiscoveredPeer;
#[cfg(feature = "std")]
pub use transport::DiscoveryHandle;
#[cfg(feature = "std")]
pub use transport::IrohTransportExt;
#[cfg(feature = "std")]
pub use transport::MembershipAddressUpdater;
#[cfg(feature = "std")]
pub use transport::NetworkTransport;
#[cfg(feature = "std")]
pub use transport::PeerDiscoveredCallback;
#[cfg(feature = "std")]
pub use transport::PeerDiscovery;
#[cfg(feature = "std")]
pub use transport::StaleTopologyInfo;
#[cfg(feature = "std")]
pub use transport::TopologyStaleCallback;
// Types
pub use types::ClusterMetrics;
pub use types::NodeAddress;
pub use types::NodeId;
pub use types::NodeState;
pub use types::SnapshotLogId;
#[cfg(feature = "std")]
pub use utils::ensure_disk_space_available;
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
