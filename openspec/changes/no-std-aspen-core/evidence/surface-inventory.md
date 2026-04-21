# Aspen Core Surface Inventory

- Crate dir: `crates/aspen-core/src`
- Source: `crates/aspen-core/src/lib.rs`
- Public modules: `22`
- Root re-exports: `132`

## Public modules
| Module | Guard |
| --- | --- |
| `app_registry` | `#[cfg(feature = "std")]` |
| `circuit_breaker` | `always` |
| `cluster` | `always` |
| `constants` | `always` |
| `context` | `#[cfg(feature = "std")]` |
| `crypto` | `always` |
| `error` | `always` |
| `hlc` | `always` |
| `kv` | `always` |
| `layer` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `prelude` | `always` |
| `protocol` | `always` |
| `simulation` | `#[cfg(feature = "std")]` |
| `spec` | `always` |
| `sql` | `#[cfg(feature = "sql")]` |
| `storage` | `always` |
| `traits` | `always` |
| `transport` | `#[cfg(feature = "std")]` |
| `types` | `always` |
| `utils` | `#[cfg(feature = "std")]` |
| `vault` | `always` |
| `verified` | `always` |

## Root re-exports
| Export | Source | Guard |
| --- | --- | --- |
| `AppManifest` | `app_registry::AppManifest` | `#[cfg(feature = "std")]` |
| `AppRegistry` | `app_registry::AppRegistry` | `#[cfg(feature = "std")]` |
| `SharedAppRegistry` | `app_registry::SharedAppRegistry` | `#[cfg(feature = "std")]` |
| `shared_registry` | `app_registry::shared_registry` | `#[cfg(feature = "std")]` |
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
| `MEMBERSHIP_COOLDOWN` | `constants::raft_compat::MEMBERSHIP_COOLDOWN` | `#[cfg(feature = "std")]` |
| `AspenDocsTicket` | `context::AspenDocsTicket` | `#[cfg(feature = "std")]` |
| `ContentDiscovery` | `context::ContentDiscovery` | `#[cfg(all(feature = "std", feature = "global-discovery"))]` |
| `ContentNodeAddr` | `context::ContentNodeAddr` | `#[cfg(all(feature = "std", feature = "global-discovery"))]` |
| `ContentProviderInfo` | `context::ContentProviderInfo` | `#[cfg(all(feature = "std", feature = "global-discovery"))]` |
| `DocsEntry` | `context::DocsEntry` | `#[cfg(feature = "std")]` |
| `DocsStatus` | `context::DocsStatus` | `#[cfg(feature = "std")]` |
| `DocsSyncProvider` | `context::DocsSyncProvider` | `#[cfg(feature = "std")]` |
| `EndpointProvider` | `context::EndpointProvider` | `#[cfg(feature = "std")]` |
| `InMemoryWatchRegistry` | `context::InMemoryWatchRegistry` | `#[cfg(feature = "std")]` |
| `KeyOrigin` | `context::KeyOrigin` | `#[cfg(feature = "std")]` |
| `NetworkFactory` | `context::NetworkFactory` | `#[cfg(feature = "std")]` |
| `PeerConnectionState` | `context::PeerConnectionState` | `#[cfg(feature = "std")]` |
| `PeerImporter` | `context::PeerImporter` | `#[cfg(feature = "std")]` |
| `PeerInfo` | `context::PeerInfo` | `#[cfg(feature = "std")]` |
| `PeerManager` | `context::PeerManager` | `#[cfg(feature = "std")]` |
| `ServiceExecutor` | `context::ServiceExecutor` | `#[cfg(feature = "std")]` |
| `ShardTopology` | `context::ShardTopology` | `#[cfg(feature = "std")]` |
| `StateMachineProvider` | `context::StateMachineProvider` | `#[cfg(feature = "std")]` |
| `SubscriptionFilter` | `context::SubscriptionFilter` | `#[cfg(feature = "std")]` |
| `SyncStatus` | `context::SyncStatus` | `#[cfg(feature = "std")]` |
| `WatchInfo` | `context::WatchInfo` | `#[cfg(feature = "std")]` |
| `WatchRegistry` | `context::WatchRegistry` | `#[cfg(feature = "std")]` |
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
| `AllocationError` | `layer::AllocationError` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `DirectoryError` | `layer::DirectoryError` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `DirectoryLayer` | `layer::DirectoryLayer` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `DirectorySubspace` | `layer::DirectorySubspace` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `Element` | `layer::Element` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `HighContentionAllocator` | `layer::HighContentionAllocator` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `Subspace` | `layer::Subspace` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `SubspaceError` | `layer::SubspaceError` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `Tuple` | `layer::Tuple` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `TupleError` | `layer::TupleError` | `#[cfg(all(feature = "std", feature = "layer"))]` |
| `Alarm` | `protocol::Alarm` | `always` |
| `Envelope` | `protocol::Envelope` | `always` |
| `ProtocolCtx` | `protocol::ProtocolCtx` | `always` |
| `TestCtx` | `protocol::TestCtx` | `always` |
| `SimulationArtifact` | `simulation::SimulationArtifact` | `#[cfg(feature = "std")]` |
| `SimulationArtifactBuilder` | `simulation::SimulationArtifactBuilder` | `#[cfg(feature = "std")]` |
| `SimulationStatus` | `simulation::SimulationStatus` | `#[cfg(feature = "std")]` |
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
| `BlobAnnouncedCallback` | `transport::BlobAnnouncedCallback` | `#[cfg(feature = "std")]` |
| `BlobAnnouncedInfo` | `transport::BlobAnnouncedInfo` | `#[cfg(feature = "std")]` |
| `DiscoveredPeer` | `transport::DiscoveredPeer` | `#[cfg(feature = "std")]` |
| `DiscoveryHandle` | `transport::DiscoveryHandle` | `#[cfg(feature = "std")]` |
| `IrohTransportExt` | `transport::IrohTransportExt` | `#[cfg(feature = "std")]` |
| `MembershipAddressUpdater` | `transport::MembershipAddressUpdater` | `#[cfg(feature = "std")]` |
| `NetworkTransport` | `transport::NetworkTransport` | `#[cfg(feature = "std")]` |
| `PeerDiscoveredCallback` | `transport::PeerDiscoveredCallback` | `#[cfg(feature = "std")]` |
| `PeerDiscovery` | `transport::PeerDiscovery` | `#[cfg(feature = "std")]` |
| `StaleTopologyInfo` | `transport::StaleTopologyInfo` | `#[cfg(feature = "std")]` |
| `TopologyStaleCallback` | `transport::TopologyStaleCallback` | `#[cfg(feature = "std")]` |
| `ClusterMetrics` | `types::ClusterMetrics` | `always` |
| `NodeAddress` | `types::NodeAddress` | `always` |
| `NodeId` | `types::NodeId` | `always` |
| `NodeState` | `types::NodeState` | `always` |
| `SnapshotLogId` | `types::SnapshotLogId` | `always` |
| `ensure_disk_space_available` | `utils::ensure_disk_space_available` | `#[cfg(feature = "std")]` |
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

## Validation notes
- The root public module and re-export inventory matches the documented no-std contract.
- `verified::*` is limited to scan helpers plus core pure re-exports from `hlc`, `kv`, and `types`.
- `test_support` remains crate-private and test-only; there is no public `aspen_core::test_support` path.
- Alloc-only module-path families remain present for `circuit_breaker`, `prelude`, `spec`, and `verified` scan helpers.
