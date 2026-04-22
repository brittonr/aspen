# Aspen Core Surface Inventory

- Crate dir: `crates/aspen-core/src`
- Source: `crates/aspen-core/src/lib.rs`
- Public modules: `22`
- Root re-exports: `132`

## Public modules
| Module | Guard |
| --- | --- |
| `app_registry` | `always` |
| `circuit_breaker` | `always` |
| `cluster` | `always` |
| `constants` | `always` |
| `context` | `always` |
| `crypto` | `always` |
| `error` | `always` |
| `hlc` | `always` |
| `kv` | `always` |
| `layer` | `#[cfg(feature = "layer")]` |
| `prelude` | `always` |
| `protocol` | `always` |
| `simulation` | `always` |
| `spec` | `always` |
| `sql` | `#[cfg(feature = "sql")]` |
| `storage` | `always` |
| `traits` | `always` |
| `transport` | `always` |
| `types` | `always` |
| `utils` | `always` |
| `vault` | `always` |
| `verified` | `always` |

## Root re-exports
| Export | Source | Guard |
| --- | --- | --- |
| `AppManifest` | `app_registry::AppManifest` | `always` |
| `AppRegistry` | `app_registry::AppRegistry` | `always` |
| `SharedAppRegistry` | `app_registry::SharedAppRegistry` | `always` |
| `shared_registry` | `app_registry::shared_registry` | `always` |
| `AddLearnerRequest` | `cluster::AddLearnerRequest` | `always` |
| `ChangeMembershipRequest` | `cluster::ChangeMembershipRequest` | `always` |
| `ClusterNode` | `cluster::ClusterNode` | `always` |
| `ClusterState` | `cluster::ClusterState` | `always` |
| `InitRequest` | `cluster::InitRequest` | `always` |
| `TrustConfig` | `cluster::TrustConfig` | `always` |
| `*` | `constants::api::*` | `always` |
| `*` | `constants::ci::*` | `always` |
| `*` | `constants::coordination::*` | `always` |
| `*` | `constants::directory::*` | `always` |
| `*` | `constants::network::*` | `always` |
| `*` | `constants::raft::*` | `always` |
| `MEMBERSHIP_COOLDOWN` | `constants::raft_compat::MEMBERSHIP_COOLDOWN` | `always` |
| `AspenDocsTicket` | `context::AspenDocsTicket` | `always` |
| `ContentDiscovery` | `context::ContentDiscovery` | `#[cfg(feature = "global-discovery")]` |
| `ContentNodeAddr` | `context::ContentNodeAddr` | `#[cfg(feature = "global-discovery")]` |
| `ContentProviderInfo` | `context::ContentProviderInfo` | `#[cfg(feature = "global-discovery")]` |
| `DocsEntry` | `context::DocsEntry` | `always` |
| `DocsStatus` | `context::DocsStatus` | `always` |
| `DocsSyncProvider` | `context::DocsSyncProvider` | `always` |
| `EndpointProvider` | `context::EndpointProvider` | `always` |
| `InMemoryWatchRegistry` | `context::InMemoryWatchRegistry` | `always` |
| `KeyOrigin` | `context::KeyOrigin` | `always` |
| `NetworkFactory` | `context::NetworkFactory` | `always` |
| `PeerConnectionState` | `context::PeerConnectionState` | `always` |
| `PeerImporter` | `context::PeerImporter` | `always` |
| `PeerInfo` | `context::PeerInfo` | `always` |
| `PeerManager` | `context::PeerManager` | `always` |
| `ServiceExecutor` | `context::ServiceExecutor` | `always` |
| `ShardTopology` | `context::ShardTopology` | `always` |
| `StateMachineProvider` | `context::StateMachineProvider` | `always` |
| `SubscriptionFilter` | `context::SubscriptionFilter` | `always` |
| `SyncStatus` | `context::SyncStatus` | `always` |
| `WatchInfo` | `context::WatchInfo` | `always` |
| `WatchRegistry` | `context::WatchRegistry` | `always` |
| `Signature` | `crypto::Signature` | `always` |
| `ControlPlaneError` | `error::ControlPlaneError` | `always` |
| `KeyValueStoreError` | `error::KeyValueStoreError` | `always` |
| `HLC` | `hlc::HLC` | `always` |
| `HlcTimestamp` | `hlc::HlcTimestamp` | `always` |
| `HlcId` | `hlc::ID as HlcId` | `always` |
| `NTP64` | `hlc::NTP64` | `always` |
| `SerializableTimestamp` | `hlc::SerializableTimestamp` | `always` |
| `create_hlc` | `hlc::create_hlc` | `always` |
| `new_timestamp` | `hlc::new_timestamp` | `always` |
| `to_unix_ms` | `hlc::to_unix_ms` | `always` |
| `update_from_timestamp` | `hlc::update_from_timestamp` | `always` |
| `BatchCondition` | `kv::BatchCondition` | `always` |
| `BatchOperation` | `kv::BatchOperation` | `always` |
| `CompareOp` | `kv::CompareOp` | `always` |
| `CompareTarget` | `kv::CompareTarget` | `always` |
| `DeleteRequest` | `kv::DeleteRequest` | `always` |
| `DeleteResult` | `kv::DeleteResult` | `always` |
| `KeyValueWithRevision` | `kv::KeyValueWithRevision` | `always` |
| `ReadConsistency` | `kv::ReadConsistency` | `always` |
| `ReadRequest` | `kv::ReadRequest` | `always` |
| `ReadResult` | `kv::ReadResult` | `always` |
| `ScanRequest` | `kv::ScanRequest` | `always` |
| `ScanResult` | `kv::ScanResult` | `always` |
| `TxnCompare` | `kv::TxnCompare` | `always` |
| `TxnOp` | `kv::TxnOp` | `always` |
| `TxnOpResult` | `kv::TxnOpResult` | `always` |
| `WriteCommand` | `kv::WriteCommand` | `always` |
| `WriteOp` | `kv::WriteOp` | `always` |
| `WriteRequest` | `kv::WriteRequest` | `always` |
| `WriteResult` | `kv::WriteResult` | `always` |
| `validate_write_command` | `kv::validate_write_command` | `always` |
| `AllocationError` | `layer::AllocationError` | `#[cfg(feature = "layer")]` |
| `DirectoryError` | `layer::DirectoryError` | `#[cfg(feature = "layer")]` |
| `DirectoryLayer` | `layer::DirectoryLayer` | `#[cfg(feature = "layer")]` |
| `DirectorySubspace` | `layer::DirectorySubspace` | `#[cfg(feature = "layer")]` |
| `Element` | `layer::Element` | `#[cfg(feature = "layer")]` |
| `HighContentionAllocator` | `layer::HighContentionAllocator` | `#[cfg(feature = "layer")]` |
| `Subspace` | `layer::Subspace` | `#[cfg(feature = "layer")]` |
| `SubspaceError` | `layer::SubspaceError` | `#[cfg(feature = "layer")]` |
| `Tuple` | `layer::Tuple` | `#[cfg(feature = "layer")]` |
| `TupleError` | `layer::TupleError` | `#[cfg(feature = "layer")]` |
| `Alarm` | `protocol::Alarm` | `always` |
| `Envelope` | `protocol::Envelope` | `always` |
| `ProtocolCtx` | `protocol::ProtocolCtx` | `always` |
| `TestCtx` | `protocol::TestCtx` | `always` |
| `SimulationArtifact` | `simulation::SimulationArtifact` | `always` |
| `SimulationArtifactBuilder` | `simulation::SimulationArtifactBuilder` | `always` |
| `SimulationStatus` | `simulation::SimulationStatus` | `always` |
| `SqlColumnInfo` | `sql::SqlColumnInfo` | `#[cfg(feature = "sql")]` |
| `SqlConsistency` | `sql::SqlConsistency` | `#[cfg(feature = "sql")]` |
| `SqlQueryError` | `sql::SqlQueryError` | `#[cfg(feature = "sql")]` |
| `SqlQueryExecutor` | `sql::SqlQueryExecutor` | `#[cfg(feature = "sql")]` |
| `SqlQueryRequest` | `sql::SqlQueryRequest` | `#[cfg(feature = "sql")]` |
| `SqlQueryResult` | `sql::SqlQueryResult` | `#[cfg(feature = "sql")]` |
| `SqlValue` | `sql::SqlValue` | `#[cfg(feature = "sql")]` |
| `effective_sql_limit` | `sql::effective_sql_limit` | `#[cfg(feature = "sql")]` |
| `effective_sql_timeout_ms` | `sql::effective_sql_timeout_ms` | `#[cfg(feature = "sql")]` |
| `validate_sql_query` | `sql::validate_sql_query` | `#[cfg(feature = "sql")]` |
| `validate_sql_request` | `sql::validate_sql_request` | `#[cfg(feature = "sql")]` |
| `KvEntry` | `storage::KvEntry` | `always` |
| `SM_KV_TABLE` | `storage::SM_KV_TABLE` | `always` |
| `ClusterController` | `traits::ClusterController` | `always` |
| `CoordinationBackend` | `traits::CoordinationBackend` | `always` |
| `KeyValueStore` | `traits::KeyValueStore` | `always` |
| `BlobAnnouncedCallback` | `transport::BlobAnnouncedCallback` | `always` |
| `BlobAnnouncedInfo` | `transport::BlobAnnouncedInfo` | `always` |
| `DiscoveredPeer` | `transport::DiscoveredPeer` | `always` |
| `DiscoveryHandle` | `transport::DiscoveryHandle` | `always` |
| `IrohTransportExt` | `transport::IrohTransportExt` | `always` |
| `MembershipAddressUpdater` | `transport::MembershipAddressUpdater` | `always` |
| `NetworkTransport` | `transport::NetworkTransport` | `always` |
| `PeerDiscoveredCallback` | `transport::PeerDiscoveredCallback` | `always` |
| `PeerDiscovery` | `transport::PeerDiscovery` | `always` |
| `StaleTopologyInfo` | `transport::StaleTopologyInfo` | `always` |
| `TopologyStaleCallback` | `transport::TopologyStaleCallback` | `always` |
| `ClusterMetrics` | `types::ClusterMetrics` | `always` |
| `NodeAddress` | `types::NodeAddress` | `always` |
| `NodeId` | `types::NodeId` | `always` |
| `NodeState` | `types::NodeState` | `always` |
| `SnapshotLogId` | `types::SnapshotLogId` | `always` |
| `ensure_disk_space_available` | `utils::ensure_disk_space_available` | `always` |
| `SYSTEM_PREFIX` | `vault::SYSTEM_PREFIX` | `always` |
| `VaultError` | `vault::VaultError` | `always` |
| `is_system_key` | `vault::is_system_key` | `always` |
| `validate_client_key` | `vault::validate_client_key` | `always` |
| `build_scan_metadata` | `verified::build_scan_metadata` | `always` |
| `decode_continuation_token` | `verified::decode_continuation_token` | `always` |
| `encode_continuation_token` | `verified::encode_continuation_token` | `always` |
| `execute_scan` | `verified::execute_scan` | `always` |
| `filter_scan_entries` | `verified::filter_scan_entries` | `always` |
| `normalize_scan_limit` | `verified::normalize_scan_limit` | `always` |
| `paginate_entries` | `verified::paginate_entries` | `always` |

## Notes
- Inventory source is declarative surface in `src/lib.rs` only.
- Guards reflect contiguous `#[cfg(...)]` attributes immediately above each `pub mod` or `pub use` line.
- This initial checker intentionally does not enforce policy yet; later tasks extend it with export-map and source-audit rules.
