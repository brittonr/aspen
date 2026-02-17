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
//! - `sql` (default): Include SQL query types and traits

pub mod app_registry;
pub mod cluster;
pub mod constants;
pub mod context;
pub mod crypto;
pub mod error;
pub mod hlc;
pub mod kv;
pub mod layer;
pub mod prelude;
pub mod simulation;
pub mod spec;
#[cfg(feature = "sql")]
pub mod sql;
pub mod storage;
pub mod traits;
pub mod transport;
pub mod types;
pub mod utils;
pub mod vault;
/// Verified pure functions for core distributed system operations.
pub mod verified;

/// Test support utilities (test-only).
///
/// Provides minimal deterministic implementations of core traits for
/// internal unit tests. For external testing, use `aspen-testing`.
#[cfg(test)]
pub(crate) mod test_support;
// Re-export all public types at crate root for convenience

// App registry types
pub use app_registry::AppManifest;
pub use app_registry::AppRegistry;
pub use app_registry::SharedAppRegistry;
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
pub use constants::api::*;
pub use constants::ci::*;
pub use constants::coordination::*;
pub use constants::directory::*;
pub use constants::network::*;
pub use constants::raft::*;
// Duration constant re-exports for backward compatibility
pub use constants::raft_compat::MEMBERSHIP_COOLDOWN;
// Context traits
pub use context::AspenDocsTicket;
#[cfg(feature = "global-discovery")]
pub use context::ContentDiscovery;
#[cfg(feature = "global-discovery")]
pub use context::ContentNodeAddr;
#[cfg(feature = "global-discovery")]
pub use context::ContentProviderInfo;
pub use context::DocsEntry;
pub use context::DocsStatus;
pub use context::DocsSyncProvider;
pub use context::EndpointProvider;
// Watch registry types
pub use context::InMemoryWatchRegistry;
pub use context::KeyOrigin;
pub use context::NetworkFactory;
pub use context::PeerConnectionState;
pub use context::PeerImporter;
pub use context::PeerInfo;
pub use context::PeerManager;
pub use context::ShardTopology;
pub use context::StateMachineProvider;
pub use context::SubscriptionFilter;
pub use context::SyncStatus;
pub use context::WatchInfo;
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
// Layer types (Tuple, Subspace, Directory)
pub use layer::AllocationError;
pub use layer::DirectoryError;
pub use layer::DirectoryLayer;
pub use layer::DirectorySubspace;
pub use layer::Element;
pub use layer::HighContentionAllocator;
pub use layer::Subspace;
pub use layer::SubspaceError;
pub use layer::Tuple;
pub use layer::TupleError;
// Simulation types
pub use simulation::SimulationArtifact;
pub use simulation::SimulationArtifactBuilder;
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
pub use transport::BlobAnnouncedCallback;
pub use transport::BlobAnnouncedInfo;
pub use transport::DiscoveredPeer;
pub use transport::DiscoveryHandle;
pub use transport::IrohTransportExt;
pub use transport::NetworkTransport;
pub use transport::PeerDiscoveredCallback;
pub use transport::PeerDiscovery;
pub use transport::StaleTopologyInfo;
pub use transport::TopologyStaleCallback;
// Types
pub use types::ClusterMetrics;
pub use types::NodeAddress;
pub use types::NodeId;
pub use types::NodeState;
pub use types::SnapshotLogId;
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
