# Aspen Core Export Map

- Source: `crates/aspen-core/src/lib.rs`
- Status: validated against the OpenSpec no-std export contract.

## Alloc-only root exports
- `cluster::AddLearnerRequest`
- `cluster::ChangeMembershipRequest`
- `cluster::ClusterNode`
- `cluster::ClusterState`
- `cluster::InitRequest`
- `cluster::TrustConfig`
- `constants::api::*`
- `constants::ci::*`
- `constants::coordination::*`
- `constants::directory::*`
- `constants::network::*`
- `constants::raft::*`
- `crypto::Signature`
- `error::ControlPlaneError`
- `error::KeyValueStoreError`
- `hlc::HLC`
- `hlc::HlcTimestamp`
- `hlc::ID as HlcId`
- `hlc::NTP64`
- `hlc::SerializableTimestamp`
- `hlc::create_hlc`
- `hlc::new_timestamp`
- `hlc::to_unix_ms`
- `hlc::update_from_timestamp`
- `kv::BatchCondition`
- `kv::BatchOperation`
- `kv::CompareOp`
- `kv::CompareTarget`
- `kv::DeleteRequest`
- `kv::DeleteResult`
- `kv::KeyValueWithRevision`
- `kv::ReadConsistency`
- `kv::ReadRequest`
- `kv::ReadResult`
- `kv::ScanRequest`
- `kv::ScanResult`
- `kv::TxnCompare`
- `kv::TxnOp`
- `kv::TxnOpResult`
- `kv::WriteCommand`
- `kv::WriteOp`
- `kv::WriteRequest`
- `kv::WriteResult`
- `kv::validate_write_command`
- `protocol::Alarm`
- `protocol::Envelope`
- `protocol::ProtocolCtx`
- `protocol::TestCtx`
- `storage::KvEntry`
- `storage::SM_KV_TABLE`
- `traits::ClusterController`
- `traits::CoordinationBackend`
- `traits::KeyValueStore`
- `types::ClusterMetrics`
- `types::NodeAddress`
- `types::NodeId`
- `types::NodeState`
- `types::SnapshotLogId`
- `vault::SYSTEM_PREFIX`
- `vault::VaultError`
- `vault::is_system_key`
- `vault::validate_client_key`
- `verified::build_scan_metadata`
- `verified::decode_continuation_token`
- `verified::encode_continuation_token`
- `verified::execute_scan`
- `verified::filter_scan_entries`
- `verified::normalize_scan_limit`
- `verified::paginate_entries`

## Std-gated root exports
- `app_registry::AppManifest`
- `app_registry::AppRegistry`
- `app_registry::SharedAppRegistry`
- `app_registry::shared_registry`
- `constants::raft_compat::MEMBERSHIP_COOLDOWN`
- `context::AspenDocsTicket`
- `context::DocsEntry`
- `context::DocsStatus`
- `context::DocsSyncProvider`
- `context::EndpointProvider`
- `context::InMemoryWatchRegistry`
- `context::KeyOrigin`
- `context::NetworkFactory`
- `context::PeerConnectionState`
- `context::PeerImporter`
- `context::PeerInfo`
- `context::PeerManager`
- `context::ServiceExecutor`
- `context::ShardTopology`
- `context::StateMachineProvider`
- `context::SubscriptionFilter`
- `context::SyncStatus`
- `context::WatchInfo`
- `context::WatchRegistry`
- `simulation::SimulationArtifact`
- `simulation::SimulationArtifactBuilder`
- `simulation::SimulationStatus`
- `transport::BlobAnnouncedCallback`
- `transport::BlobAnnouncedInfo`
- `transport::DiscoveredPeer`
- `transport::DiscoveryHandle`
- `transport::IrohTransportExt`
- `transport::MembershipAddressUpdater`
- `transport::NetworkTransport`
- `transport::PeerDiscoveredCallback`
- `transport::PeerDiscovery`
- `transport::StaleTopologyInfo`
- `transport::TopologyStaleCallback`
- `utils::ensure_disk_space_available`

## Global-discovery root exports
- `context::ContentDiscovery`
- `context::ContentNodeAddr`
- `context::ContentProviderInfo`

## Layer root exports
- `layer::AllocationError`
- `layer::DirectoryError`
- `layer::DirectoryLayer`
- `layer::DirectorySubspace`
- `layer::Element`
- `layer::HighContentionAllocator`
- `layer::Subspace`
- `layer::SubspaceError`
- `layer::Tuple`
- `layer::TupleError`

## SQL root exports
- `sql::SqlColumnInfo`
- `sql::SqlConsistency`
- `sql::SqlQueryError`
- `sql::SqlQueryExecutor`
- `sql::SqlQueryRequest`
- `sql::SqlQueryResult`
- `sql::SqlValue`
- `sql::effective_sql_limit`
- `sql::effective_sql_timeout_ms`
- `sql::validate_sql_query`
- `sql::validate_sql_request`

## Alloc-only module-path families
- `aspen_core::circuit_breaker::*`
- `aspen_core::prelude::*`
- `aspen_core::spec::*`
- `aspen_core::verified::scan::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}`

## Test-only shell infrastructure
- `test_support` stays crate-private behind `#[cfg(all(test, feature = "std"))]` and has no public root path.
