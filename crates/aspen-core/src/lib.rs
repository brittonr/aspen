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

pub mod cluster;
pub mod constants;
pub mod context;
pub mod crypto;
pub mod error;
pub mod hlc;
pub mod inmemory;
pub mod kv;
pub mod pure;
pub mod simulation;
#[cfg(feature = "sql")]
pub mod sql;
pub mod storage;
pub mod traits;
pub mod transport;
pub mod types;
pub mod utils;
pub mod vault;

// Re-export all public types at crate root for convenience

// Utils
// Cluster types
pub use cluster::AddLearnerRequest;
pub use cluster::ChangeMembershipRequest;
pub use cluster::ClusterNode;
pub use cluster::ClusterState;
pub use cluster::InitRequest;
// CAS retry constants
pub use constants::CAS_RETRY_INITIAL_BACKOFF_MS;
pub use constants::CAS_RETRY_MAX_BACKOFF_MS;
// Queue constants
pub use constants::DEFAULT_QUEUE_DEDUP_TTL_MS;
pub use constants::DEFAULT_QUEUE_POLL_INTERVAL_MS;
pub use constants::DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS;
// Constants
pub use constants::DEFAULT_SCAN_LIMIT;
// Service registry constants
pub use constants::DEFAULT_SERVICE_TTL_MS;
// SQL types (feature-gated)
#[cfg(feature = "sql")]
pub use constants::DEFAULT_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
pub use constants::DEFAULT_SQL_TIMEOUT_MS;
pub use constants::MAX_CAS_RETRIES;
pub use constants::MAX_KEY_SIZE;
pub use constants::MAX_QUEUE_BATCH_SIZE;
pub use constants::MAX_QUEUE_CLEANUP_BATCH;
pub use constants::MAX_QUEUE_ITEM_SIZE;
pub use constants::MAX_QUEUE_ITEM_TTL_MS;
pub use constants::MAX_QUEUE_POLL_INTERVAL_MS;
pub use constants::MAX_QUEUE_VISIBILITY_TIMEOUT_MS;
pub use constants::MAX_SCAN_RESULTS;
pub use constants::MAX_SERVICE_DISCOVERY_RESULTS;
pub use constants::MAX_SERVICE_TTL_MS;
pub use constants::MAX_SETMULTI_KEYS;
#[cfg(feature = "sql")]
pub use constants::MAX_SQL_PARAMS;
#[cfg(feature = "sql")]
pub use constants::MAX_SQL_QUERY_SIZE;
#[cfg(feature = "sql")]
pub use constants::MAX_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
pub use constants::MAX_SQL_TIMEOUT_MS;
pub use constants::MAX_VALUE_SIZE;
pub use constants::SERVICE_CLEANUP_BATCH;
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
// In-memory deterministic implementations for testing
pub use inmemory::DeterministicClusterController;
pub use inmemory::DeterministicKeyValueStore;
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
// Pure functions
pub use pure::build_scan_metadata;
pub use pure::decode_continuation_token;
pub use pure::encode_continuation_token;
pub use pure::execute_scan;
pub use pure::filter_scan_entries;
pub use pure::normalize_scan_limit;
pub use pure::paginate_entries;
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
pub use transport::DiscoveredPeer;
pub use transport::DiscoveryHandle;
pub use transport::IrohTransportExt;
pub use transport::NetworkTransport;
pub use transport::PeerDiscoveredCallback;
pub use transport::PeerDiscovery;
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
