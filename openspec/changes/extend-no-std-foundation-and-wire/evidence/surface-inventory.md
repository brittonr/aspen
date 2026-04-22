# Aspen core surface inventory

- Core crate dir: `/home/brittonr/git/aspen/crates/aspen-core/src`

## Core public modules
| Source | Guard |
| --- | --- |
| `circuit_breaker` | `always` |
| `cluster` | `always` |
| `constants` | `always` |
| `crypto` | `always` |
| `error` | `always` |
| `hlc` | `always` |
| `kv` | `always` |
| `prelude` | `always` |
| `protocol` | `always` |
| `spec` | `always` |
| `sql` | `#[cfg(feature = "sql")]` |
| `storage` | `always` |
| `traits` | `always` |
| `types` | `always` |
| `vault` | `always` |
| `verified` | `always` |

## Core public re-exports
| Source | Guard |
| --- | --- |
| `cluster::AddLearnerRequest` | `always` |
| `cluster::ChangeMembershipRequest` | `always` |
| `cluster::ClusterNode` | `always` |
| `cluster::ClusterState` | `always` |
| `cluster::InitRequest` | `always` |
| `cluster::TrustConfig` | `always` |
| `constants::api::*` | `always` |
| `constants::ci::*` | `always` |
| `constants::coordination::*` | `always` |
| `constants::directory::*` | `always` |
| `constants::network::*` | `always` |
| `constants::raft::*` | `always` |
| `crypto::Signature` | `always` |
| `error::ControlPlaneError` | `always` |
| `error::KeyValueStoreError` | `always` |
| `hlc::HLC` | `always` |
| `hlc::HlcTimestamp` | `always` |
| `hlc::ID as HlcId` | `always` |
| `hlc::NTP64` | `always` |
| `hlc::SerializableTimestamp` | `always` |
| `hlc::create_hlc` | `always` |
| `hlc::new_timestamp` | `always` |
| `hlc::to_unix_ms` | `always` |
| `hlc::update_from_timestamp` | `always` |
| `kv::BatchCondition` | `always` |
| `kv::BatchOperation` | `always` |
| `kv::CompareOp` | `always` |
| `kv::CompareTarget` | `always` |
| `kv::DeleteRequest` | `always` |
| `kv::DeleteResult` | `always` |
| `kv::KeyValueWithRevision` | `always` |
| `kv::ReadConsistency` | `always` |
| `kv::ReadRequest` | `always` |
| `kv::ReadResult` | `always` |
| `kv::ScanRequest` | `always` |
| `kv::ScanResult` | `always` |
| `kv::TxnCompare` | `always` |
| `kv::TxnOp` | `always` |
| `kv::TxnOpResult` | `always` |
| `kv::WriteCommand` | `always` |
| `kv::WriteOp` | `always` |
| `kv::WriteRequest` | `always` |
| `kv::WriteResult` | `always` |
| `kv::validate_write_command` | `always` |
| `protocol::Alarm` | `always` |
| `protocol::Envelope` | `always` |
| `protocol::ProtocolCtx` | `always` |
| `protocol::TestCtx` | `always` |
| `sql::SqlColumnInfo` | `#[cfg(feature = "sql")]` |
| `sql::SqlConsistency` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryError` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryExecutor` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryRequest` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryResult` | `#[cfg(feature = "sql")]` |
| `sql::SqlValue` | `#[cfg(feature = "sql")]` |
| `sql::effective_sql_limit` | `#[cfg(feature = "sql")]` |
| `sql::effective_sql_timeout_ms` | `#[cfg(feature = "sql")]` |
| `sql::validate_sql_query` | `#[cfg(feature = "sql")]` |
| `sql::validate_sql_request` | `#[cfg(feature = "sql")]` |
| `storage::KvEntry` | `always` |
| `traits::ClusterController` | `always` |
| `traits::CoordinationBackend` | `always` |
| `traits::KeyValueStore` | `always` |
| `types::ClusterMetrics` | `always` |
| `types::NodeAddress` | `always` |
| `types::NodeId` | `always` |
| `types::NodeState` | `always` |
| `types::SnapshotLogId` | `always` |
| `vault::SYSTEM_PREFIX` | `always` |
| `vault::VaultError` | `always` |
| `vault::is_system_key` | `always` |
| `vault::validate_client_key` | `always` |
| `verified::build_scan_metadata` | `always` |
| `verified::decode_continuation_token` | `always` |
| `verified::encode_continuation_token` | `always` |
| `verified::execute_scan` | `always` |
| `verified::filter_scan_entries` | `always` |
| `verified::normalize_scan_limit` | `always` |
| `verified::paginate_entries` | `always` |

- Shell crate dir: `/home/brittonr/git/aspen/crates/aspen-core-shell/src`

## Shell public modules
| Source | Guard |
| --- | --- |
| `app_registry` | `always` |
| `constants` | `always` |
| `context` | `always` |
| `layer` | `#[cfg(feature = "layer")]` |
| `simulation` | `always` |
| `storage` | `always` |
| `transport` | `always` |
| `utils` | `always` |

## Shell public re-exports
| Source | Guard |
| --- | --- |
| `aspen_core::circuit_breaker` | `always` |
| `aspen_core::cluster` | `always` |
| `aspen_core::crypto` | `always` |
| `aspen_core::error` | `always` |
| `aspen_core::hlc` | `always` |
| `aspen_core::kv` | `always` |
| `aspen_core::prelude` | `always` |
| `aspen_core::protocol` | `always` |
| `aspen_core::spec` | `always` |
| `aspen_core::sql` | `#[cfg(feature = "sql")]` |
| `aspen_core::traits` | `always` |
| `aspen_core::types` | `always` |
| `aspen_core::vault` | `always` |
| `aspen_core::verified` | `always` |
| `app_registry::AppManifest` | `always` |
| `app_registry::AppRegistry` | `always` |
| `app_registry::SharedAppRegistry` | `always` |
| `app_registry::shared_registry` | `always` |
| `constants::api::*` | `always` |
| `constants::ci::*` | `always` |
| `constants::coordination::*` | `always` |
| `constants::directory::*` | `always` |
| `constants::network::*` | `always` |
| `constants::raft::*` | `always` |
| `constants::raft_compat::MEMBERSHIP_COOLDOWN` | `always` |
| `context::AspenDocsTicket` | `always` |
| `context::ContentDiscovery` | `#[cfg(feature = "global-discovery")]` |
| `context::ContentNodeAddr` | `#[cfg(feature = "global-discovery")]` |
| `context::ContentProviderInfo` | `#[cfg(feature = "global-discovery")]` |
| `context::DocsEntry` | `always` |
| `context::DocsStatus` | `always` |
| `context::DocsSyncProvider` | `always` |
| `context::EndpointProvider` | `always` |
| `context::InMemoryWatchRegistry` | `always` |
| `context::KeyOrigin` | `always` |
| `context::NetworkFactory` | `always` |
| `context::PeerConnectionState` | `always` |
| `context::PeerImporter` | `always` |
| `context::PeerInfo` | `always` |
| `context::PeerManager` | `always` |
| `context::ServiceExecutor` | `always` |
| `context::ShardTopology` | `always` |
| `context::StateMachineProvider` | `always` |
| `context::SubscriptionFilter` | `always` |
| `context::SyncStatus` | `always` |
| `context::WatchInfo` | `always` |
| `context::WatchRegistry` | `always` |
| `crypto::Signature` | `always` |
| `error::ControlPlaneError` | `always` |
| `error::KeyValueStoreError` | `always` |
| `hlc::HLC` | `always` |
| `hlc::HlcTimestamp` | `always` |
| `hlc::ID as HlcId` | `always` |
| `hlc::NTP64` | `always` |
| `hlc::SerializableTimestamp` | `always` |
| `hlc::create_hlc` | `always` |
| `hlc::new_timestamp` | `always` |
| `hlc::to_unix_ms` | `always` |
| `hlc::update_from_timestamp` | `always` |
| `kv::BatchCondition` | `always` |
| `kv::BatchOperation` | `always` |
| `kv::CompareOp` | `always` |
| `kv::CompareTarget` | `always` |
| `kv::DeleteRequest` | `always` |
| `kv::DeleteResult` | `always` |
| `kv::KeyValueWithRevision` | `always` |
| `kv::ReadConsistency` | `always` |
| `kv::ReadRequest` | `always` |
| `kv::ReadResult` | `always` |
| `kv::ScanRequest` | `always` |
| `kv::ScanResult` | `always` |
| `kv::TxnCompare` | `always` |
| `kv::TxnOp` | `always` |
| `kv::TxnOpResult` | `always` |
| `kv::WriteCommand` | `always` |
| `kv::WriteOp` | `always` |
| `kv::WriteRequest` | `always` |
| `kv::WriteResult` | `always` |
| `kv::validate_write_command` | `always` |
| `layer::AllocationError` | `#[cfg(feature = "layer")]` |
| `layer::DirectoryError` | `#[cfg(feature = "layer")]` |
| `layer::DirectoryLayer` | `#[cfg(feature = "layer")]` |
| `layer::DirectorySubspace` | `#[cfg(feature = "layer")]` |
| `layer::Element` | `#[cfg(feature = "layer")]` |
| `layer::HighContentionAllocator` | `#[cfg(feature = "layer")]` |
| `layer::Subspace` | `#[cfg(feature = "layer")]` |
| `layer::SubspaceError` | `#[cfg(feature = "layer")]` |
| `layer::Tuple` | `#[cfg(feature = "layer")]` |
| `layer::TupleError` | `#[cfg(feature = "layer")]` |
| `protocol::Alarm` | `always` |
| `protocol::Envelope` | `always` |
| `protocol::ProtocolCtx` | `always` |
| `protocol::TestCtx` | `always` |
| `simulation::SimulationArtifact` | `always` |
| `simulation::SimulationArtifactBuilder` | `always` |
| `simulation::SimulationStatus` | `always` |
| `sql::SqlColumnInfo` | `#[cfg(feature = "sql")]` |
| `sql::SqlConsistency` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryError` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryExecutor` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryRequest` | `#[cfg(feature = "sql")]` |
| `sql::SqlQueryResult` | `#[cfg(feature = "sql")]` |
| `sql::SqlValue` | `#[cfg(feature = "sql")]` |
| `sql::effective_sql_limit` | `#[cfg(feature = "sql")]` |
| `sql::effective_sql_timeout_ms` | `#[cfg(feature = "sql")]` |
| `sql::validate_sql_query` | `#[cfg(feature = "sql")]` |
| `sql::validate_sql_request` | `#[cfg(feature = "sql")]` |
| `storage::KvEntry` | `always` |
| `storage::SM_KV_TABLE` | `always` |
| `traits::ClusterController` | `always` |
| `traits::CoordinationBackend` | `always` |
| `traits::KeyValueStore` | `always` |
| `transport::BlobAnnouncedCallback` | `always` |
| `transport::BlobAnnouncedInfo` | `always` |
| `transport::DiscoveredPeer` | `always` |
| `transport::DiscoveryHandle` | `always` |
| `transport::IrohTransportExt` | `always` |
| `transport::MembershipAddressUpdater` | `always` |
| `transport::NetworkTransport` | `always` |
| `transport::PeerDiscoveredCallback` | `always` |
| `transport::PeerDiscovery` | `always` |
| `transport::StaleTopologyInfo` | `always` |
| `transport::TopologyStaleCallback` | `always` |
| `types::ClusterMetrics` | `always` |
| `types::NodeAddress` | `always` |
| `types::NodeId` | `always` |
| `types::NodeState` | `always` |
| `types::SnapshotLogId` | `always` |
| `utils::ensure_disk_space_available` | `always` |
| `vault::SYSTEM_PREFIX` | `always` |
| `vault::VaultError` | `always` |
| `vault::is_system_key` | `always` |
| `vault::validate_client_key` | `always` |
| `verified::build_scan_metadata` | `always` |
| `verified::decode_continuation_token` | `always` |
| `verified::encode_continuation_token` | `always` |
| `verified::execute_scan` | `always` |
| `verified::filter_scan_entries` | `always` |
| `verified::normalize_scan_limit` | `always` |
| `verified::paginate_entries` | `always` |

