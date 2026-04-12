//! Unit tests for RaftNode.

#[cfg(feature = "trust")]
use std::collections::BTreeMap;
#[cfg(feature = "trust")]
use std::collections::BTreeSet;
use std::sync::Arc;
#[cfg(feature = "trust")]
use std::sync::atomic::AtomicBool;
#[cfg(feature = "trust")]
use std::sync::atomic::Ordering;
#[cfg(feature = "trust")]
use std::time::Duration;

use aspen_cluster_types::AddLearnerRequest;
use aspen_cluster_types::ChangeMembershipRequest;
use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::ControlPlaneError;
use aspen_cluster_types::InitRequest;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadConsistency;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::ClusterController;
use aspen_traits::CoordinationBackend;
use aspen_traits::KeyValueStore;
#[cfg(feature = "trust")]
use aspen_trust::chain;
#[cfg(feature = "trust")]
use aspen_trust::shamir;
#[cfg(feature = "trust")]
use async_trait::async_trait;
#[cfg(feature = "trust")]
use iroh::EndpointAddr;
#[cfg(feature = "trust")]
use iroh::SecretKey;
use openraft::Config;
use tempfile::TempDir;
#[cfg(feature = "trust")]
use tokio::sync::Notify;
#[cfg(feature = "trust")]
use tokio_util::sync::CancellationToken;

use super::RaftNode;
use super::arc_wrapper::ArcRaftNode;
use super::conversions::node_state_from_openraft;
use super::health::RaftNodeHealth;
use crate::madsim_network::FailureInjector;
use crate::madsim_network::MadsimNetworkFactory;
use crate::madsim_network::MadsimRaftRouter;

/// Maximum concurrent operations (mirrors the constant in mod.rs for testing)
const MAX_CONCURRENT_OPS: usize = 1000;
use crate::StateMachineVariant;
use crate::storage::InMemoryLogStore;
use crate::storage::InMemoryStateMachine;
use crate::storage_shared::SharedRedbStorage;
use crate::types::AppTypeConfig;
use crate::types::NodeId;

/// Create a test Raft node with in-memory storage using the madsim network factory.
async fn create_test_node(node_id: u64) -> RaftNode {
    let config = Config {
        cluster_name: "test-cluster".to_string(),
        ..Default::default()
    };
    let config = Arc::new(config);

    let log_storage = InMemoryLogStore::default();
    let state_machine = Arc::new(InMemoryStateMachine::default());

    // Create madsim network infrastructure
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let network_factory = MadsimNetworkFactory::new(NodeId(node_id), router.clone(), failure_injector);

    let raft: openraft::Raft<AppTypeConfig> =
        openraft::Raft::new(NodeId(node_id), config, network_factory, log_storage, state_machine.clone())
            .await
            .expect("Failed to create Raft instance");

    // Register node with router for RPC dispatch
    router
        .register_node(NodeId(node_id), format!("127.0.0.1:{}", 26000 + node_id), raft.clone())
        .expect("Failed to register node with router");

    RaftNode::new(NodeId(node_id), Arc::new(raft), StateMachineVariant::InMemory(state_machine))
}

struct RedbTestNode {
    node: Arc<RaftNode>,
    storage: Arc<SharedRedbStorage>,
    _temp_dir: Option<TempDir>,
}

async fn create_test_redb_node_at_path(
    node_id: u64,
    router: Arc<MadsimRaftRouter>,
    failure_injector: Arc<FailureInjector>,
    cluster_name: &str,
    db_path: &std::path::Path,
) -> RedbTestNode {
    let storage = SharedRedbStorage::new(db_path, &node_id.to_string()).expect("create redb storage");
    let network_factory = MadsimNetworkFactory::new(NodeId(node_id), router.clone(), failure_injector);
    let config = Arc::new(Config {
        cluster_name: cluster_name.to_string(),
        ..Default::default()
    });

    let raft: openraft::Raft<AppTypeConfig> =
        openraft::Raft::new(NodeId(node_id), config, network_factory, storage.clone(), storage.clone())
            .await
            .expect("Failed to create redb raft instance");

    router
        .register_node(NodeId(node_id), format!("127.0.0.1:{}", 27000 + node_id), raft.clone())
        .expect("Failed to register redb node with router");

    let storage = Arc::new(storage);
    let node = Arc::new(RaftNode::new(NodeId(node_id), Arc::new(raft), StateMachineVariant::Redb(storage.clone())));

    RedbTestNode {
        node,
        storage,
        _temp_dir: None,
    }
}

async fn create_test_redb_node(
    node_id: u64,
    router: Arc<MadsimRaftRouter>,
    failure_injector: Arc<FailureInjector>,
    cluster_name: &str,
) -> RedbTestNode {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join(format!("node-{node_id}.redb"));
    let mut node = create_test_redb_node_at_path(node_id, router, failure_injector, cluster_name, &db_path).await;
    node._temp_dir = Some(temp_dir);
    node
}

#[cfg(feature = "trust")]
fn endpoint_addr_for_node(_node_id: u64) -> EndpointAddr {
    let key = SecretKey::generate(&mut rand::rng());
    EndpointAddr::from_parts(key.public(), std::iter::empty())
}

#[cfg(feature = "trust")]
fn cluster_node_with_endpoint(node_id: u64, endpoint: EndpointAddr) -> ClusterNode {
    let mut node = ClusterNode::new(node_id, format!("node-{node_id}"), None);
    node.node_addr = Some(aspen_cluster_types::NodeAddress::new(endpoint));
    node
}

#[cfg(feature = "trust")]
#[derive(Clone)]
struct TestTrustShareClient {
    storages_by_endpoint: BTreeMap<String, Arc<SharedRedbStorage>>,
    block_started: Option<Arc<Notify>>,
    block_release: Option<Arc<Notify>>,
}

#[cfg(feature = "trust")]
#[derive(Clone)]
struct ExpungedTrustShareClient {
    epoch: u64,
}

#[cfg(feature = "trust")]
#[derive(Clone)]
struct TestSecretsShareCollector {
    storages_by_node: BTreeMap<u64, std::sync::Weak<SharedRedbStorage>>,
    threshold: u32,
}

#[cfg(feature = "trust")]
struct StorageReadAdapter {
    storage: Arc<SharedRedbStorage>,
}

#[cfg(feature = "trust")]
struct KeyManagerEncryptionProvider {
    manager: Arc<aspen_trust::key_manager::KeyManager>,
    collector: Arc<dyn aspen_trust::key_manager::ShareCollector>,
}

#[cfg(feature = "trust")]
struct KeyValueStoreReencryptionAdapter {
    kv: Arc<dyn aspen_core::KeyValueStore>,
    pause_armed: AtomicBool,
    delay_ms_after_arm: u64,
}

#[cfg(feature = "trust")]
impl TestTrustShareClient {
    fn immediate(storages_by_endpoint: BTreeMap<String, Arc<SharedRedbStorage>>) -> Self {
        Self {
            storages_by_endpoint,
            block_started: None,
            block_release: None,
        }
    }

    fn blocking(
        storages_by_endpoint: BTreeMap<String, Arc<SharedRedbStorage>>,
        block_started: Arc<Notify>,
        block_release: Arc<Notify>,
    ) -> Self {
        Self {
            storages_by_endpoint,
            block_started: Some(block_started),
            block_release: Some(block_release),
        }
    }
}

#[cfg(feature = "trust")]
impl TestSecretsShareCollector {
    fn new(storages_by_node: BTreeMap<u64, Arc<SharedRedbStorage>>, threshold: u32) -> Self {
        Self {
            storages_by_node: storages_by_node
                .into_iter()
                .map(|(node_id, storage)| (node_id, Arc::downgrade(&storage)))
                .collect(),
            threshold,
        }
    }
}

#[cfg(feature = "trust")]
impl KeyValueStoreReencryptionAdapter {
    fn new(kv: Arc<dyn aspen_core::KeyValueStore>, delay_ms_after_arm: u64) -> Self {
        Self {
            kv,
            pause_armed: AtomicBool::new(false),
            delay_ms_after_arm,
        }
    }

    fn arm_pause(&self) {
        self.pause_armed.store(true, Ordering::SeqCst);
    }
}

#[cfg(feature = "trust")]
#[async_trait]
impl crate::trust_share_client::TrustShareClient for TestTrustShareClient {
    async fn get_share(
        &self,
        target: EndpointAddr,
        epoch: u64,
    ) -> anyhow::Result<aspen_trust::protocol::ShareResponse> {
        if let Some(started) = &self.block_started {
            started.notify_waiters();
        }
        if let Some(release) = &self.block_release {
            release.notified().await;
        }

        let key = target.id.to_string();
        let storage = self
            .storages_by_endpoint
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("missing storage for endpoint {}", target.id))?;
        let share = storage.load_share(epoch)?.ok_or_else(|| anyhow::anyhow!("missing share for epoch {epoch}"))?;
        Ok(aspen_trust::protocol::ShareResponse { epoch, share })
    }

    async fn send_expunged(&self, _target: EndpointAddr, _epoch: u64) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "trust")]
#[async_trait]
impl crate::trust_share_client::TrustShareClient for ExpungedTrustShareClient {
    async fn get_share(
        &self,
        _target: EndpointAddr,
        _epoch: u64,
    ) -> anyhow::Result<aspen_trust::protocol::ShareResponse> {
        Err(crate::trust_share_client::ExpungedByPeer { epoch: self.epoch }.into())
    }

    async fn send_expunged(&self, _target: EndpointAddr, _epoch: u64) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "trust")]
#[async_trait]
impl aspen_trust::key_manager::ShareCollector for TestSecretsShareCollector {
    async fn collect_shares(
        &self,
        epoch: u64,
    ) -> Result<Vec<shamir::Share>, aspen_trust::key_manager::ShareCollectionError> {
        let mut shares = Vec::new();

        for (node_id, storage_weak) in &self.storages_by_node {
            let Some(storage) = storage_weak.upgrade() else {
                continue;
            };
            let share = storage.load_share(epoch).map_err(|e| {
                aspen_trust::key_manager::ShareCollectionError::CollectionFailed {
                    reason: format!("failed to load share for node {node_id}: {e}"),
                }
            })?;
            if let Some(share) = share {
                shares.push(share);
            }
        }

        if shares.len() < self.threshold as usize {
            return Err(aspen_trust::key_manager::ShareCollectionError::BelowQuorum {
                collected: shares.len() as u32,
                threshold: self.threshold,
            });
        }

        Ok(shares)
    }
}

#[cfg(feature = "trust")]
#[async_trait]
impl aspen_secrets::SecretsEncryptionProvider for KeyManagerEncryptionProvider {
    async fn get_encryption(
        &self,
    ) -> std::result::Result<
        Arc<aspen_trust::encryption::SecretsEncryption>,
        aspen_trust::encryption::SecretsUnavailableError,
    > {
        self.manager.ensure_initialized(self.collector.as_ref()).await
    }
}

#[cfg(feature = "trust")]
#[async_trait]
impl aspen_traits::KeyValueStore for KeyValueStoreReencryptionAdapter {
    async fn write(&self, request: WriteRequest) -> Result<aspen_kv_types::WriteResult, KeyValueStoreError> {
        let result = self.kv.write(request).await?;

        if self.pause_armed.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(self.delay_ms_after_arm)).await;
        }

        Ok(result)
    }

    async fn read(&self, request: ReadRequest) -> Result<aspen_kv_types::ReadResult, KeyValueStoreError> {
        self.kv.read(request).await
    }

    async fn delete(&self, request: DeleteRequest) -> Result<aspen_kv_types::DeleteResult, KeyValueStoreError> {
        self.kv.delete(request).await
    }

    async fn scan(&self, request: ScanRequest) -> Result<aspen_kv_types::ScanResult, KeyValueStoreError> {
        self.kv.scan(request).await
    }
}

#[cfg(feature = "trust")]
#[async_trait]
impl aspen_traits::KeyValueStore for StorageReadAdapter {
    async fn write(&self, _request: WriteRequest) -> Result<aspen_kv_types::WriteResult, KeyValueStoreError> {
        Err(KeyValueStoreError::Failed {
            reason: "StorageReadAdapter only supports reads in this test".to_string(),
        })
    }

    async fn read(&self, request: ReadRequest) -> Result<aspen_kv_types::ReadResult, KeyValueStoreError> {
        let kv = self.storage.get_with_revision(&request.key).map_err(|error| KeyValueStoreError::Failed {
            reason: format!("storage read failed: {error}"),
        })?;
        Ok(aspen_kv_types::ReadResult { kv })
    }

    async fn delete(&self, _request: DeleteRequest) -> Result<aspen_kv_types::DeleteResult, KeyValueStoreError> {
        Err(KeyValueStoreError::Failed {
            reason: "StorageReadAdapter only supports reads in this test".to_string(),
        })
    }

    async fn scan(&self, request: ScanRequest) -> Result<aspen_kv_types::ScanResult, KeyValueStoreError> {
        let entries = self
            .storage
            .scan(&request.prefix, request.continuation_token.as_deref(), request.limit_results)
            .map_err(|error| KeyValueStoreError::Failed {
                reason: format!("storage scan failed: {error}"),
            })?;
        let result_count = entries.len() as u32;
        Ok(aspen_kv_types::ScanResult {
            entries,
            result_count,
            is_truncated: false,
            continuation_token: None,
        })
    }
}

#[cfg(feature = "trust")]
fn rebuild_secret_from_storage(storage: &[Arc<SharedRedbStorage>], epoch: u64, threshold: usize) -> [u8; 32] {
    let shares: Vec<_> =
        storage.iter().take(threshold).map(|store| store.load_share(epoch).unwrap().unwrap()).collect();
    shamir::reconstruct_secret(&shares).unwrap()
}

#[cfg(feature = "trust")]
fn secret_storage_key(mount: &str, path: &str) -> String {
    format!("{}{mount}/{path}", aspen_secrets::SECRETS_SYSTEM_PREFIX)
}

#[cfg(feature = "trust")]
fn load_secret_storage_epoch(storage: &SharedRedbStorage, mount: &str, path: &str) -> u64 {
    use base64::Engine;

    let full_key = secret_storage_key(mount, path);
    let entry = storage.get_with_revision(&full_key).unwrap().unwrap();
    let decoded = base64::engine::general_purpose::STANDARD.decode(entry.value).unwrap();
    aspen_trust::encryption::SecretsEncryption::peek_epoch(&decoded).unwrap()
}

#[cfg(feature = "trust")]
async fn wait_for_trust_epoch(storages: &[Arc<SharedRedbStorage>], epoch: u64, timeout: Duration) {
    let result = tokio::time::timeout(timeout, async {
        loop {
            let all_ready = storages.iter().all(|storage| storage.load_current_trust_epoch().unwrap() == Some(epoch));
            if all_ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    if result.is_err() {
        let epochs: Vec<_> = storages.iter().map(|storage| storage.load_current_trust_epoch().unwrap()).collect();
        panic!("trust epoch should converge to {epoch}, got {:?}", epochs);
    }
}

#[cfg(feature = "trust")]
async fn wait_for_leader(node: &RaftNode, expected_leader: u64, timeout: Duration) {
    tokio::time::timeout(timeout, async {
        loop {
            let metrics = node.raft().metrics().borrow().clone();
            if metrics.current_leader == Some(NodeId(expected_leader)) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("leader should be elected");
}

/// Helper to set the initialized flag for testing.
fn set_initialized(node: &RaftNode, value: bool) {
    node.set_initialized_for_test(value);
}

/// Test RaftNode creation.
#[tokio::test]
async fn test_raft_node_creation() {
    let node = create_test_node(1).await;
    assert_eq!(node.node_id().0, 1);
    assert!(!node.is_initialized());
}

/// Test node ID accessor.
#[tokio::test]
async fn test_node_id() {
    let node = create_test_node(42).await;
    assert_eq!(node.node_id().0, 42);
}

/// Test is_initialized starts as false.
#[tokio::test]
async fn test_is_initialized_default() {
    let node = create_test_node(1).await;
    assert!(!node.is_initialized());
}

/// Test batching disabled by default.
#[tokio::test]
async fn test_batching_disabled_by_default() {
    let node = create_test_node(1).await;
    assert!(!node.is_batching_enabled());
}

/// Test state machine accessor.
#[tokio::test]
async fn test_state_machine_accessor() {
    let node = create_test_node(1).await;
    match node.state_machine() {
        StateMachineVariant::InMemory(_) => {} // Expected
        StateMachineVariant::Redb(_) => panic!("Expected InMemory state machine"),
    }
}

/// Test raft accessor.
#[tokio::test]
async fn test_raft_accessor() {
    let node = create_test_node(1).await;
    let _raft = node.raft();
    // Just verify it doesn't panic
}

/// Test ClusterController::is_initialized trait method.
#[tokio::test]
async fn test_cluster_controller_is_initialized() {
    let node = create_test_node(1).await;
    // Before init, should return false
    assert!(!ClusterController::is_initialized(&node));
}

/// Test ClusterController::init with empty members fails.
#[tokio::test]
async fn test_init_empty_members_fails() {
    let node = create_test_node(1).await;
    let request = InitRequest {
        initial_members: vec![],
        trust: Default::default(),
    };

    let result = ClusterController::init(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("must not be empty"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test ClusterController::init with members missing iroh_addr fails.
#[tokio::test]
async fn test_init_missing_iroh_addr_fails() {
    let node = create_test_node(1).await;
    let request = InitRequest {
        initial_members: vec![ClusterNode::new(1, "test-node-1", None)], // No node_addr
        trust: Default::default(),
    };

    let result = ClusterController::init(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("node_addr must be set"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

#[cfg(feature = "trust")]
#[tokio::test]
async fn test_multi_node_trust_init_persists_follower_shares() {
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "trust-init-redb-test";

    let node1 = create_test_redb_node(1, router.clone(), failure_injector.clone(), cluster_name).await;
    let node2 = create_test_redb_node(2, router.clone(), failure_injector.clone(), cluster_name).await;
    let node3 = create_test_redb_node(3, router.clone(), failure_injector, cluster_name).await;

    let members: Vec<ClusterNode> = (1u64..=3)
        .map(|id| {
            let secret = iroh::SecretKey::generate(&mut rand::rng());
            let mut node = ClusterNode::new(id, format!("node-{id}"), None);
            node.node_addr = Some(aspen_cluster_types::NodeAddress::new(iroh::EndpointAddr::from_parts(
                secret.public(),
                std::iter::empty(),
            )));
            node
        })
        .collect();

    ClusterController::init(&node1.node, InitRequest {
        initial_members: members,
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init 3-node trust cluster");

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let ready = [node1.storage.as_ref(), node2.storage.as_ref(), node3.storage.as_ref()]
                .iter()
                .all(|storage| storage.load_share(1).unwrap().is_some() && storage.load_digests(1).unwrap().len() == 3);
            if ready {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("followers should persist trust shares after committed init");

    let share1 = node1.storage.load_share(1).unwrap().unwrap();
    let share2 = node2.storage.load_share(1).unwrap().unwrap();
    let share3 = node3.storage.load_share(1).unwrap().unwrap();

    assert_ne!(share1, share2);
    assert_ne!(share1, share3);
    assert_ne!(share2, share3);

    let secret_12 = aspen_trust::shamir::reconstruct_secret(&[share1.clone(), share2.clone()]).unwrap();
    let secret_13 = aspen_trust::shamir::reconstruct_secret(&[share1.clone(), share3.clone()]).unwrap();
    let secret_23 = aspen_trust::shamir::reconstruct_secret(&[share2, share3]).unwrap();
    assert_eq!(secret_12, secret_13);
    assert_eq!(secret_12, secret_23);

    let digests1 = node1.storage.load_digests(1).unwrap();
    let digests2 = node2.storage.load_digests(1).unwrap();
    let digests3 = node3.storage.load_digests(1).unwrap();
    assert_eq!(digests1.len(), 3);
    assert_eq!(digests1, digests2);
    assert_eq!(digests1, digests3);
}

#[cfg(feature = "trust")]
#[tokio::test]
async fn test_secrets_become_unavailable_below_quorum_and_recover_after_nodes_return() {
    use aspen_secrets::AspenSecretsBackend;
    use aspen_secrets::SecretsBackend;

    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "trust-secrets-quorum-redb-test";
    let cluster_id = b"trust-secrets-quorum-cluster".to_vec();
    let temp_dir = TempDir::new().expect("temp dir");
    let node1_path = temp_dir.path().join("node-1.redb");
    let node2_path = temp_dir.path().join("node-2.redb");
    let node3_path = temp_dir.path().join("node-3.redb");

    let node1 =
        create_test_redb_node_at_path(1, router.clone(), failure_injector.clone(), cluster_name, &node1_path).await;
    let node2 =
        create_test_redb_node_at_path(2, router.clone(), failure_injector.clone(), cluster_name, &node2_path).await;
    let node3 =
        create_test_redb_node_at_path(3, router.clone(), failure_injector.clone(), cluster_name, &node3_path).await;

    let endpoint1 = endpoint_addr_for_node(1);
    let endpoint2 = endpoint_addr_for_node(2);
    let endpoint3 = endpoint_addr_for_node(3);

    ClusterController::init(&node1.node, InitRequest {
        initial_members: vec![
            cluster_node_with_endpoint(1, endpoint1),
            cluster_node_with_endpoint(2, endpoint2),
            cluster_node_with_endpoint(3, endpoint3),
        ],
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init 3-node trust cluster");

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let shares_ready = [node1.storage.as_ref(), node2.storage.as_ref(), node3.storage.as_ref()]
                .iter()
                .all(|storage| storage.load_share(1).unwrap().is_some());
            if shares_ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("all nodes should persist epoch 1 trust shares");

    let initial_collector = TestSecretsShareCollector::new(
        BTreeMap::from([
            (1_u64, node1.storage.clone()),
            (2_u64, node2.storage.clone()),
            (3_u64, node3.storage.clone()),
        ]),
        2,
    );
    let config = aspen_trust::key_manager::KeyManagerConfig {
        cluster_id: cluster_id.clone(),
        node_id: 1,
        initial_nonce_counter: 0,
        epoch: 1,
    };
    let writer_manager = aspen_trust::key_manager::KeyManager::new(aspen_trust::key_manager::KeyManagerConfig {
        cluster_id: cluster_id.clone(),
        node_id: config.node_id,
        initial_nonce_counter: config.initial_nonce_counter,
        epoch: config.epoch,
    });
    let writer_encryption = writer_manager
        .ensure_initialized(&initial_collector)
        .await
        .expect("quorum should reconstruct the at-rest key");
    let writer_kv_store: Arc<dyn aspen_core::KeyValueStore> = node1.node.clone();
    let writer_backend = AspenSecretsBackend::with_encryption(writer_kv_store, "quorum", writer_encryption);
    let plaintext = b"quorum-gated-secret";
    writer_backend.put("service/api-key", plaintext).await.expect("store secret through real backend");
    drop(writer_backend);
    drop(writer_manager);

    node2.node.raft().shutdown().await.expect("shutdown node 2");
    node3.node.raft().shutdown().await.expect("shutdown node 3");
    drop(node2);
    drop(node3);

    node1.node.raft().shutdown().await.expect("shutdown node 1 before reopen");
    drop(node1);

    let node1_reopened =
        create_test_redb_node_at_path(1, router.clone(), failure_injector.clone(), cluster_name, &node1_path).await;
    let below_quorum_collector: Arc<dyn aspen_trust::key_manager::ShareCollector> =
        Arc::new(TestSecretsShareCollector::new(BTreeMap::from([(1_u64, node1_reopened.storage.clone())]), 2));
    let unavailable_manager =
        Arc::new(aspen_trust::key_manager::KeyManager::new(aspen_trust::key_manager::KeyManagerConfig {
            cluster_id: cluster_id.clone(),
            node_id: config.node_id,
            initial_nonce_counter: config.initial_nonce_counter,
            epoch: config.epoch,
        }));
    let reopened_kv_store: Arc<dyn aspen_core::KeyValueStore> = Arc::new(StorageReadAdapter {
        storage: node1_reopened.storage.clone(),
    });
    let unavailable_backend = AspenSecretsBackend::with_encryption_provider(
        reopened_kv_store.clone(),
        "quorum",
        Arc::new(KeyManagerEncryptionProvider {
            manager: unavailable_manager,
            collector: below_quorum_collector,
        }),
    );
    let unavailable_read = unavailable_backend.get("service/api-key").await;
    assert!(
        matches!(
            unavailable_read,
            Err(aspen_secrets::SecretsError::SecretsUnavailable {
                source: aspen_trust::encryption::SecretsUnavailableError::BelowQuorum
            })
        ),
        "expected secrets unavailable from backend read, got {:?}",
        unavailable_read
    );

    let node2_reopened =
        create_test_redb_node_at_path(2, router.clone(), failure_injector.clone(), cluster_name, &node2_path).await;
    let node3_reopened =
        create_test_redb_node_at_path(3, router.clone(), failure_injector.clone(), cluster_name, &node3_path).await;

    let recovered_collector: Arc<dyn aspen_trust::key_manager::ShareCollector> =
        Arc::new(TestSecretsShareCollector::new(
            BTreeMap::from([
                (1_u64, node1_reopened.storage.clone()),
                (2_u64, node2_reopened.storage.clone()),
                (3_u64, node3_reopened.storage.clone()),
            ]),
            2,
        ));
    let recovered_manager = Arc::new(aspen_trust::key_manager::KeyManager::new(config));
    let recovered_backend = AspenSecretsBackend::with_encryption_provider(
        reopened_kv_store.clone(),
        "quorum",
        Arc::new(KeyManagerEncryptionProvider {
            manager: recovered_manager,
            collector: recovered_collector,
        }),
    );
    let recovered = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match recovered_backend.get("service/api-key").await {
                Ok(Some(value)) if value == plaintext => break value,
                _ => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("secret should become readable again after nodes restart");
    assert_eq!(recovered, plaintext);
}

#[cfg(all(feature = "trust", feature = "secrets"))]
#[tokio::test]
async fn test_membership_change_reencrypts_secrets_and_preserves_mixed_epoch_reads() {
    use aspen_secrets::AspenSecretsBackend;
    use aspen_secrets::SecretsBackend;

    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "trust-secrets-rotation-redb-test";
    let cluster_id = b"trust-secrets-rotation-cluster".to_vec();

    let mut node1 = create_test_redb_node(1, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node2 = create_test_redb_node(2, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node3 = create_test_redb_node(3, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node4 = create_test_redb_node(4, router.clone(), failure_injector, cluster_name).await;

    let endpoint1 = endpoint_addr_for_node(1);
    let endpoint2 = endpoint_addr_for_node(2);
    let endpoint3 = endpoint_addr_for_node(3);
    let endpoint4 = endpoint_addr_for_node(4);
    let storages_by_endpoint = BTreeMap::from([
        (endpoint1.id.to_string(), node1.storage.clone()),
        (endpoint2.id.to_string(), node2.storage.clone()),
        (endpoint3.id.to_string(), node3.storage.clone()),
        (endpoint4.id.to_string(), node4.storage.clone()),
    ]);
    let client: Arc<dyn crate::trust_share_client::TrustShareClient> =
        Arc::new(TestTrustShareClient::immediate(storages_by_endpoint));

    Arc::get_mut(&mut node1.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node2.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node3.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node4.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());

    ClusterController::init(&node1.node, InitRequest {
        initial_members: vec![
            cluster_node_with_endpoint(1, endpoint1.clone()),
            cluster_node_with_endpoint(2, endpoint2.clone()),
            cluster_node_with_endpoint(3, endpoint3.clone()),
        ],
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init 3-node trust cluster");

    wait_for_leader(&node1.node, 1, Duration::from_secs(10)).await;

    let wrapped_kv_store = Arc::new(KeyValueStoreReencryptionAdapter::new(node1.node.clone(), 100));
    let provider =
        crate::secrets_at_rest::build_trust_aware_secrets_provider(node1.node.clone(), wrapped_kv_store.clone())
            .expect("build trust-aware secrets provider");
    let backend = AspenSecretsBackend::with_encryption_provider(wrapped_kv_store.clone(), "rotation", provider);

    let expected_values = [
        ("app/a", b"alpha-secret".to_vec()),
        ("app/b", b"bravo-secret".to_vec()),
        ("app/c", b"charlie-secret".to_vec()),
    ];
    for (path, value) in &expected_values {
        backend.put(path, value).await.expect("write epoch 1 secret");
        assert_eq!(load_secret_storage_epoch(&node1.storage, "rotation", path), 1);
    }

    wrapped_kv_store.arm_pause();
    node4.node.raft().trigger().elect().await.expect("wake node 4 raft core before promotion");
    ClusterController::add_learner(&node1.node, AddLearnerRequest {
        learner: cluster_node_with_endpoint(4, endpoint4.clone()),
    })
    .await
    .expect("add learner 4");

    ClusterController::change_membership(&node1.node, ChangeMembershipRequest {
        members: vec![1, 2, 3, 4],
    })
    .await
    .expect("promote node 4 and rotate trust");

    let new_epoch = node1.storage.load_current_trust_epoch().unwrap().unwrap();
    assert!(new_epoch > 1);
    wait_for_trust_epoch(
        &[node1.storage.clone(), node2.storage.clone(), node3.storage.clone()],
        new_epoch,
        Duration::from_secs(15),
    )
    .await;

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let has_new_epoch = expected_values
                .iter()
                .any(|(path, _)| load_secret_storage_epoch(&node1.storage, "rotation", path) == new_epoch);
            let has_old_epoch = expected_values
                .iter()
                .any(|(path, _)| load_secret_storage_epoch(&node1.storage, "rotation", path) == 1);
            if has_new_epoch && has_old_epoch {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("membership change should trigger automatic re-encryption with a mixed-epoch window");

    assert_eq!(load_secret_storage_epoch(&node1.storage, "rotation", "app/a"), new_epoch);
    assert_eq!(load_secret_storage_epoch(&node1.storage, "rotation", "app/c"), 1);
    assert_eq!(backend.get("app/a").await.expect("read migrated value").as_deref(), Some(&b"alpha-secret"[..]));
    assert_eq!(
        backend.get("app/c").await.expect("read old-epoch value during transition").as_deref(),
        Some(&b"charlie-secret"[..])
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let all_rotated = expected_values
                .iter()
                .all(|(path, _)| load_secret_storage_epoch(&node1.storage, "rotation", path) == new_epoch);
            if all_rotated {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("all secrets should be re-encrypted at the new epoch");

    for (path, value) in &expected_values {
        let stored = backend.get(path).await.expect("read value after automatic re-encryption");
        assert_eq!(stored.as_deref(), Some(value.as_slice()));
    }
}

#[cfg(all(feature = "trust", feature = "secrets"))]
#[tokio::test]
async fn test_reencryption_checkpoint_resumes_after_restart() {
    use aspen_secrets::AspenSecretsBackend;
    use aspen_secrets::SecretsBackend;
    use base64::Engine;

    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "trust-secrets-restart-redb-test";
    let cluster_id = b"trust-secrets-restart-cluster".to_vec();

    let node1_dir = TempDir::new().expect("node1 temp dir");
    let node1_path = node1_dir.path().join("node-1.redb");
    let mut node1 =
        create_test_redb_node_at_path(1, router.clone(), failure_injector.clone(), cluster_name, &node1_path).await;
    let mut node2 = create_test_redb_node(2, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node3 = create_test_redb_node(3, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node4 = create_test_redb_node(4, router.clone(), failure_injector, cluster_name).await;

    let endpoint1 = endpoint_addr_for_node(1);
    let endpoint2 = endpoint_addr_for_node(2);
    let endpoint3 = endpoint_addr_for_node(3);
    let endpoint4 = endpoint_addr_for_node(4);
    let storages_by_endpoint = BTreeMap::from([
        (endpoint1.id.to_string(), node1.storage.clone()),
        (endpoint2.id.to_string(), node2.storage.clone()),
        (endpoint3.id.to_string(), node3.storage.clone()),
        (endpoint4.id.to_string(), node4.storage.clone()),
    ]);
    let client: Arc<dyn crate::trust_share_client::TrustShareClient> =
        Arc::new(TestTrustShareClient::immediate(storages_by_endpoint));

    Arc::get_mut(&mut node1.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node2.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node3.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node4.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());

    ClusterController::init(&node1.node, InitRequest {
        initial_members: vec![
            cluster_node_with_endpoint(1, endpoint1.clone()),
            cluster_node_with_endpoint(2, endpoint2.clone()),
            cluster_node_with_endpoint(3, endpoint3.clone()),
        ],
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init 3-node trust cluster");

    wait_for_leader(&node1.node, 1, Duration::from_secs(10)).await;

    let epoch1_collector: Arc<dyn aspen_trust::key_manager::ShareCollector> = Arc::new(TestSecretsShareCollector::new(
        BTreeMap::from([
            (1_u64, node1.storage.clone()),
            (2_u64, node2.storage.clone()),
            (3_u64, node3.storage.clone()),
        ]),
        2,
    ));
    let epoch1_backend = AspenSecretsBackend::with_encryption_provider(
        node1.node.clone() as Arc<dyn aspen_core::KeyValueStore>,
        "restart",
        Arc::new(KeyManagerEncryptionProvider {
            manager: Arc::new(aspen_trust::key_manager::KeyManager::new(aspen_trust::key_manager::KeyManagerConfig {
                cluster_id: cluster_id.clone(),
                node_id: 1,
                initial_nonce_counter: node1.storage.load_nonce_counter(1).unwrap().unwrap_or(0),
                epoch: 1,
            })),
            collector: epoch1_collector,
        }),
    );

    let expected_values: Vec<_> = (0_u32..105)
        .map(|index| (format!("app/{index:03}"), format!("secret-{index:03}").into_bytes()))
        .collect();
    for (path, value) in &expected_values {
        epoch1_backend.put(path, value).await.expect("write epoch 1 secret");
        assert_eq!(load_secret_storage_epoch(&node1.storage, "restart", path), 1);
    }

    node4.node.raft().trigger().elect().await.expect("wake node 4 raft core before promotion");
    ClusterController::add_learner(&node1.node, AddLearnerRequest {
        learner: cluster_node_with_endpoint(4, endpoint4.clone()),
    })
    .await
    .expect("add learner 4");
    ClusterController::change_membership(&node1.node, ChangeMembershipRequest {
        members: vec![1, 2, 3, 4],
    })
    .await
    .expect("promote node 4 and rotate trust");

    let current_epoch = node1.storage.load_current_trust_epoch().unwrap().unwrap();
    assert!(current_epoch > 1);
    wait_for_trust_epoch(
        &[node1.storage.clone(), node2.storage.clone(), node3.storage.clone()],
        current_epoch,
        Duration::from_secs(15),
    )
    .await;

    let current_encryption = crate::secrets_at_rest::load_current_secrets_encryption_for_tests(node1.node.clone())
        .await
        .expect("load current secrets encryption");

    for (path, value) in expected_values.iter().take(100) {
        let full_key = secret_storage_key("restart", path);
        let stored = node1.storage.get_with_revision(&full_key).unwrap().unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(stored.value).unwrap();
        let plaintext = current_encryption.unwrap_read(&decoded).unwrap();
        assert_eq!(plaintext, *value);
        let (reencrypted, _) = current_encryption.wrap_write(&plaintext).unwrap();
        node1
            .node
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: full_key,
                    value: base64::engine::general_purpose::STANDARD.encode(reencrypted),
                },
            })
            .await
            .expect("write partially re-encrypted value");
    }
    let checkpoint_key = secret_storage_key("restart", &expected_values[99].0);
    node1
        .storage
        .save_reencryption_checkpoint(aspen_secrets::SECRETS_SYSTEM_PREFIX, &checkpoint_key)
        .unwrap();

    assert_eq!(load_secret_storage_epoch(&node1.storage, "restart", &expected_values[0].0), current_epoch);
    assert_eq!(load_secret_storage_epoch(&node1.storage, "restart", &expected_values[104].0), 1);
    assert_eq!(
        node1.storage.load_reencryption_checkpoint(aspen_secrets::SECRETS_SYSTEM_PREFIX).unwrap().as_deref(),
        Some(checkpoint_key.as_str())
    );

    drop(epoch1_backend);

    let resumed_provider = crate::secrets_at_rest::build_trust_aware_secrets_provider(
        node1.node.clone(),
        node1.node.clone() as Arc<dyn aspen_core::KeyValueStore>,
    )
    .expect("build trust-aware provider after partial rotation");
    let resumed_backend = AspenSecretsBackend::with_encryption_provider(
        node1.node.clone() as Arc<dyn aspen_core::KeyValueStore>,
        "restart",
        resumed_provider,
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let all_rotated = expected_values
                .iter()
                .all(|(path, _)| load_secret_storage_epoch(&node1.storage, "restart", path) == current_epoch);
            let checkpoint_cleared =
                node1.storage.load_reencryption_checkpoint(aspen_secrets::SECRETS_SYSTEM_PREFIX).unwrap().is_none();
            if all_rotated && checkpoint_cleared {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("startup watcher should resume re-encryption from checkpoint without another epoch bump");

    for (path, value) in &expected_values {
        let stored = resumed_backend.get(path).await.expect("read resumed value");
        assert_eq!(stored.as_deref(), Some(value.as_slice()));
    }
}

#[cfg(feature = "trust")]
#[tokio::test]
async fn test_multi_node_trust_membership_change_rotates_secret_add_and_remove() {
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "trust-rotation-redb-test";
    let cluster_id = b"trust-rotation-cluster".to_vec();

    let mut node1 = create_test_redb_node(1, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node2 = create_test_redb_node(2, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node3 = create_test_redb_node(3, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node4 = create_test_redb_node(4, router.clone(), failure_injector, cluster_name).await;

    let endpoint1 = endpoint_addr_for_node(1);
    let endpoint2 = endpoint_addr_for_node(2);
    let endpoint3 = endpoint_addr_for_node(3);
    let endpoint4 = endpoint_addr_for_node(4);
    let storages_by_endpoint = BTreeMap::from([
        (endpoint1.id.to_string(), node1.storage.clone()),
        (endpoint2.id.to_string(), node2.storage.clone()),
        (endpoint3.id.to_string(), node3.storage.clone()),
        (endpoint4.id.to_string(), node4.storage.clone()),
    ]);
    let client: Arc<dyn crate::trust_share_client::TrustShareClient> =
        Arc::new(TestTrustShareClient::immediate(storages_by_endpoint));

    Arc::get_mut(&mut node1.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node2.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node3.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node4.node).unwrap().set_trust_share_client(client.clone(), cluster_id.clone());

    ClusterController::init(&node1.node, InitRequest {
        initial_members: vec![
            cluster_node_with_endpoint(1, endpoint1.clone()),
            cluster_node_with_endpoint(2, endpoint2.clone()),
            cluster_node_with_endpoint(3, endpoint3.clone()),
        ],
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init 3-node trust cluster");

    wait_for_leader(&node1.node, 1, Duration::from_secs(10)).await;
    node4.node.raft().trigger().elect().await.expect("wake node 4 raft core before promotion");

    ClusterController::add_learner(&node1.node, AddLearnerRequest {
        learner: cluster_node_with_endpoint(4, endpoint4.clone()),
    })
    .await
    .expect("add learner 4");

    let add_result = ClusterController::change_membership(&node1.node, ChangeMembershipRequest {
        members: vec![1, 2, 3, 4],
    })
    .await
    .expect("promote node 4 and rotate trust");
    assert_eq!(add_result.members.len(), 4);

    let add_epoch = node1.storage.load_current_trust_epoch().unwrap().unwrap();
    let _ = ClusterController::add_learner(&node1.node, AddLearnerRequest {
        learner: cluster_node_with_endpoint(4, endpoint4.clone()),
    })
    .await;
    for _ in 0..5 {
        node1.node.raft().trigger().heartbeat().await.expect("heartbeat after voter promotion");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(add_epoch > 1);
    wait_for_trust_epoch(
        &[node1.storage.clone(), node2.storage.clone(), node3.storage.clone()],
        add_epoch,
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(node1.storage.load_digests(add_epoch).unwrap().len(), 4);
    assert_eq!(node1.storage.load_members(add_epoch).unwrap().len(), 4);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let has_epoch = node4.storage.load_current_trust_epoch().unwrap() == Some(add_epoch);
            let has_share = node4.storage.load_share(add_epoch).unwrap().is_some();
            if has_epoch && has_share {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("new voter should reach the rotated epoch and persist its share");

    let add_secret = rebuild_secret_from_storage(
        &[node1.storage.clone(), node2.storage.clone(), node3.storage.clone()],
        add_epoch,
        3,
    );
    let add_chain = node1.storage.load_encrypted_chain(add_epoch).unwrap().unwrap();
    let add_prior = chain::decrypt_chain(&add_chain, &add_secret, &cluster_id).unwrap();
    assert_eq!(add_chain.prior_count, 1);
    assert_eq!(add_prior.len(), 1);
    assert!(add_prior.contains_key(&1));

    let remove_result =
        ClusterController::change_membership(&node1.node, ChangeMembershipRequest { members: vec![1, 2, 3] })
            .await
            .expect("remove voter 4 and rotate trust again");
    assert_eq!(remove_result.members.len(), 3);

    let remove_epoch = node1.storage.load_current_trust_epoch().unwrap().unwrap();
    assert!(remove_epoch > add_epoch);
    wait_for_trust_epoch(
        &[node1.storage.clone(), node2.storage.clone(), node3.storage.clone()],
        remove_epoch,
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(node1.storage.load_digests(remove_epoch).unwrap().len(), 3);
    assert_eq!(node1.storage.load_members(remove_epoch).unwrap().len(), 3);
    assert_eq!(node4.storage.load_share(remove_epoch).unwrap(), None);

    let remove_secret = rebuild_secret_from_storage(&[node1.storage.clone(), node2.storage.clone()], remove_epoch, 2);
    let remove_chain = node1.storage.load_encrypted_chain(remove_epoch).unwrap().unwrap();
    let remove_prior = chain::decrypt_chain(&remove_chain, &remove_secret, &cluster_id).unwrap();
    assert_eq!(remove_chain.prior_count, 2);
    assert_eq!(remove_prior.len(), 2);
    assert!(remove_prior.contains_key(&1));
    assert!(remove_prior.contains_key(&add_epoch));
}

#[cfg(feature = "trust")]
#[tokio::test]
async fn test_trust_reconfiguration_restarts_after_leader_crash_during_share_collection() {
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "trust-crash-redb-test";
    let cluster_id = b"trust-crash-cluster".to_vec();

    let mut node1 = create_test_redb_node(1, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node2 = create_test_redb_node(2, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node3 = create_test_redb_node(3, router.clone(), failure_injector, cluster_name).await;

    let endpoint1 = endpoint_addr_for_node(1);
    let endpoint2 = endpoint_addr_for_node(2);
    let endpoint3 = endpoint_addr_for_node(3);
    let storages_by_endpoint = BTreeMap::from([
        (endpoint1.id.to_string(), node1.storage.clone()),
        (endpoint2.id.to_string(), node2.storage.clone()),
        (endpoint3.id.to_string(), node3.storage.clone()),
    ]);

    let block_started = Arc::new(Notify::new());
    let block_release = Arc::new(Notify::new());
    let blocked_client: Arc<dyn crate::trust_share_client::TrustShareClient> = Arc::new(
        TestTrustShareClient::blocking(storages_by_endpoint.clone(), block_started.clone(), block_release.clone()),
    );
    let immediate_client: Arc<dyn crate::trust_share_client::TrustShareClient> =
        Arc::new(TestTrustShareClient::immediate(storages_by_endpoint));

    Arc::get_mut(&mut node1.node).unwrap().set_trust_share_client(blocked_client, cluster_id.clone());
    Arc::get_mut(&mut node2.node)
        .unwrap()
        .set_trust_share_client(immediate_client.clone(), cluster_id.clone());
    Arc::get_mut(&mut node3.node)
        .unwrap()
        .set_trust_share_client(immediate_client.clone(), cluster_id.clone());

    ClusterController::init(&node1.node, InitRequest {
        initial_members: vec![
            cluster_node_with_endpoint(1, endpoint1.clone()),
            cluster_node_with_endpoint(2, endpoint2.clone()),
            cluster_node_with_endpoint(3, endpoint3.clone()),
        ],
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init 3-node trust cluster");
    wait_for_leader(&node1.node, 1, Duration::from_secs(10)).await;

    let membership_response = node1
        .node
        .raft()
        .change_membership([2_u64, 3].into_iter().map(NodeId), false)
        .await
        .expect("commit membership change without automatic trust rotation");
    let target_epoch = membership_response.log_id.index();

    let watcher_node2 = crate::trust_reconfig_watcher::spawn_trust_reconfig_watcher(node2.node.clone());
    let watcher_node3 = crate::trust_reconfig_watcher::spawn_trust_reconfig_watcher(node3.node.clone());
    let watcher_tokens: [CancellationToken; 2] = [watcher_node2, watcher_node3];

    let rotate_task = tokio::spawn({
        let node = node1.node.clone();
        async move {
            node.rotate_trust_after_membership_change(
                BTreeSet::from([1_u64, 2, 3]),
                BTreeSet::from([2_u64, 3]),
                target_epoch,
            )
            .await
        }
    });

    tokio::time::timeout(Duration::from_secs(2), block_started.notified())
        .await
        .expect("leader should begin remote share collection");

    node1.node.raft().shutdown().await.expect("shutdown leader raft instance");
    node2.node.raft().trigger().elect().await.expect("force node 2 to start election");

    wait_for_trust_epoch(&[node2.storage.clone(), node3.storage.clone()], target_epoch, Duration::from_secs(15)).await;

    let restarted_secret =
        rebuild_secret_from_storage(&[node2.storage.clone(), node3.storage.clone()], target_epoch, 2);
    let restarted_chain = node2.storage.load_encrypted_chain(target_epoch).unwrap().unwrap();
    let restarted_prior = chain::decrypt_chain(&restarted_chain, &restarted_secret, &cluster_id).unwrap();
    assert_eq!(restarted_chain.prior_count, 1);
    assert!(restarted_prior.contains_key(&1));
    assert_eq!(node2.storage.load_digests(target_epoch).unwrap().len(), 2);
    assert_eq!(node2.storage.load_members(target_epoch).unwrap().len(), 2);

    block_release.notify_waiters();
    let old_leader_result = tokio::time::timeout(Duration::from_secs(5), rotate_task)
        .await
        .expect("old leader task should finish once released")
        .expect("old leader join should succeed");
    assert!(old_leader_result.is_err());

    for token in watcher_tokens {
        token.cancel();
    }
}

#[cfg(feature = "trust")]
#[tokio::test]
async fn test_spawn_trust_peer_probe_marks_removed_node() {
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let cluster_name = "peer-expungement-probe";
    let cluster_id = cluster_name.as_bytes().to_vec();

    let mut node1 = create_test_redb_node(1, router.clone(), failure_injector.clone(), cluster_name).await;
    let mut node2 = create_test_redb_node(2, router.clone(), failure_injector, cluster_name).await;

    let endpoint1 = endpoint_addr_for_node(1);
    let endpoint2 = endpoint_addr_for_node(2);
    let immediate_client: Arc<dyn crate::trust_share_client::TrustShareClient> =
        Arc::new(TestTrustShareClient::immediate(BTreeMap::new()));
    let expunged_client: Arc<dyn crate::trust_share_client::TrustShareClient> =
        Arc::new(ExpungedTrustShareClient { epoch: 7 });

    Arc::get_mut(&mut node1.node).unwrap().set_trust_share_client(immediate_client, cluster_id.clone());
    Arc::get_mut(&mut node2.node).unwrap().set_trust_share_client(expunged_client, cluster_id);

    ClusterController::init(&node1.node, InitRequest {
        initial_members: vec![
            cluster_node_with_endpoint(1, endpoint1),
            cluster_node_with_endpoint(2, endpoint2),
        ],
        trust: aspen_cluster_types::TrustConfig::enabled(),
    })
    .await
    .expect("init trust cluster");
    wait_for_leader(&node1.node, 1, Duration::from_secs(10)).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let ready = [node1.storage.as_ref(), node2.storage.as_ref()]
                .iter()
                .all(|storage| storage.load_share(1).unwrap().is_some() && storage.load_digests(1).unwrap().len() == 2);
            if ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("both nodes should persist epoch 1 trust state");

    let cancel = crate::trust_peer_probe::spawn_trust_peer_probe(node2.node.clone());
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if node2.storage.is_expunged().unwrap() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("startup probe should mark removed node as expunged");
    cancel.cancel();

    let metadata = node2.storage.load_expunged().unwrap().expect("expunged metadata should be stored");
    assert_eq!(metadata.epoch, 7);
    assert_eq!(metadata.removed_by, 1);
    assert!(node2.storage.is_expunged().unwrap());
    assert!(node2.storage.load_share(1).unwrap().is_none());
}

/// Test ensure_initialized returns error before init.
#[tokio::test]
async fn test_ensure_initialized_before_init() {
    let node = create_test_node(1).await;
    let result = node.ensure_initialized();
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        _ => panic!("Expected NotInitialized error"),
    }
}

/// Test KeyValueStore operations require initialization.
#[tokio::test]
async fn test_kv_operations_require_init() {
    let node = create_test_node(1).await;

    // Read should fail when not initialized
    let read_request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Linearizable,
    };
    let result = KeyValueStore::read(&node, read_request).await;
    assert!(result.is_err());

    // Write should fail when not initialized
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        },
    };
    let result = KeyValueStore::write(&node, write_request).await;
    assert!(result.is_err());
}

/// Test change_membership with empty members fails.
#[tokio::test]
async fn test_change_membership_empty_fails() {
    let node = create_test_node(1).await;
    // Manually set initialized flag for this test
    set_initialized(&node, true);

    let request = ChangeMembershipRequest { members: vec![] };
    let result = ClusterController::change_membership(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("at least one voter"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test add_learner with missing iroh_addr fails.
#[tokio::test]
async fn test_add_learner_missing_addr_fails() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = AddLearnerRequest {
        learner: ClusterNode::new(2, "test-node-2", None), // No node_addr
    };
    let result = ClusterController::add_learner(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("node_addr must be set"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test node_state_from_openraft conversion.
#[test]
fn test_node_state_conversion() {
    use aspen_cluster_types::NodeState;

    assert_eq!(node_state_from_openraft(openraft::ServerState::Learner), NodeState::Learner);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Follower), NodeState::Follower);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Candidate), NodeState::Candidate);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Leader), NodeState::Leader);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Shutdown), NodeState::Shutdown);
}

/// Test RaftNodeHealth creation.
#[tokio::test]
async fn test_raft_node_health_creation() {
    let node = Arc::new(create_test_node(1).await);
    let health = RaftNodeHealth::new(node.clone());
    // Health check on uninitialized node
    let status = health.status().await;
    assert!(!status.is_healthy); // Not healthy because no membership
    assert!(!status.has_membership);
}

/// Test RaftNodeHealth with custom threshold.
#[tokio::test]
async fn test_raft_node_health_threshold() {
    let node = Arc::new(create_test_node(1).await);
    let health = RaftNodeHealth::with_threshold(node.clone(), 5);
    // Just verify creation works
    let status = health.status().await;
    assert_eq!(status.consecutive_failures, 0);
}

/// Test RaftNodeHealth failure reset.
#[tokio::test]
async fn test_raft_node_health_reset() {
    let node = Arc::new(create_test_node(1).await);
    let health = RaftNodeHealth::new(node.clone());
    health.reset_failures();
    let status = health.status().await;
    assert_eq!(status.consecutive_failures, 0);
}

/// Test ArcRaftNode wrapper.
#[tokio::test]
async fn test_arc_raft_node_wrapper() {
    let node = Arc::new(create_test_node(1).await);
    let arc_node = ArcRaftNode::from(node.clone());

    // Test Deref - use explicit deref to access RaftNode::node_id() vs CoordinationBackend::node_id()
    assert_eq!((*arc_node).node_id().0, 1);

    // Test inner() and into_inner()
    assert_eq!(Arc::as_ptr(arc_node.inner()), Arc::as_ptr(&node));

    let recovered = arc_node.into_inner();
    assert_eq!(Arc::as_ptr(&recovered), Arc::as_ptr(&node));
}

/// Test ArcRaftNode CoordinationBackend trait.
#[tokio::test]
async fn test_arc_raft_node_coordination_backend() {
    let node = Arc::new(create_test_node(1).await);
    let arc_node = ArcRaftNode::from(node);

    // Test trait methods
    let node_id = CoordinationBackend::node_id(&arc_node).await;
    assert_eq!(node_id, 1);

    let is_leader = CoordinationBackend::is_leader(&arc_node).await;
    assert!(!is_leader); // Uninitialized node is not leader

    let _now = CoordinationBackend::now_unix_ms(&arc_node).await;
    // Just verify it doesn't panic

    let _kv_store = CoordinationBackend::kv_store(&arc_node);
    let _cluster_controller = CoordinationBackend::cluster_controller(&arc_node);
}

/// Test MAX_CONCURRENT_OPS constant.
#[test]
fn test_max_concurrent_ops() {
    assert_eq!(MAX_CONCURRENT_OPS, 1000);
}

// ========================================================================
// ClusterController Validation Tests - Pre-initialization Checks
// ========================================================================

/// Test add_learner requires initialization.
///
/// Note: ensure_initialized() is called before iroh_addr validation,
/// so we expect NotInitialized even without providing an iroh address.
#[tokio::test]
async fn test_add_learner_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = AddLearnerRequest {
        // No iroh_addr needed - ensure_initialized() is checked first
        learner: ClusterNode::new(2, "test-node-2", None),
    };
    let result = ClusterController::add_learner(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test change_membership requires initialization.
#[tokio::test]
async fn test_change_membership_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = ChangeMembershipRequest { members: vec![1] };
    let result = ClusterController::change_membership(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test current_state requires initialization.
#[tokio::test]
async fn test_current_state_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::current_state(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test get_leader requires initialization.
#[tokio::test]
async fn test_get_leader_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::get_leader(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test get_metrics requires initialization.
#[tokio::test]
async fn test_get_metrics_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::get_metrics(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test trigger_snapshot requires initialization.
#[tokio::test]
async fn test_trigger_snapshot_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::trigger_snapshot(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

// ========================================================================
// KeyValueStore Validation Tests - Pre-initialization Checks
// ========================================================================

/// Test scan requires initialization.
#[tokio::test]
async fn test_scan_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = ScanRequest {
        prefix: "test".to_string(),
        limit_results: None,
        continuation_token: None,
    };
    let result = KeyValueStore::scan(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::Failed { reason }) => {
            assert!(reason.contains("not initialized"));
        }
        other => panic!("Expected Failed error with 'not initialized', got {:?}", other),
    }
}

/// Test delete requires initialization.
#[tokio::test]
async fn test_delete_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = DeleteRequest {
        key: "test".to_string(),
    };
    let result = KeyValueStore::delete(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::Failed { reason }) => {
            assert!(reason.contains("not initialized"));
        }
        other => panic!("Expected Failed error with 'not initialized', got {:?}", other),
    }
}

// ========================================================================
// Validation Tests - Input Bounds and Edge Cases
// ========================================================================

/// Test change_membership with single voter is valid.
///
/// While single-node clusters may not be fault-tolerant, they are a valid
/// configuration for development and testing scenarios.
#[tokio::test]
async fn test_change_membership_single_voter_valid() {
    let node = create_test_node(1).await;
    // Manually set initialized flag for this test
    set_initialized(&node, true);

    // Single-node membership change should pass validation
    // (it will fail at Raft level due to no leader, but that's not a validation error)
    let request = ChangeMembershipRequest { members: vec![1] };
    let result = ClusterController::change_membership(&node, request).await;

    // Should NOT be an InvalidRequest error - single voter is valid
    // Timeout or Failed is acceptable (no leader elected)
    if let Err(ControlPlaneError::InvalidRequest { .. }) = result {
        panic!("Single voter should be valid, got InvalidRequest error")
    }
}

/// Test that validate_write_command is called for writes.
///
/// This test verifies that the node properly validates write commands
/// before attempting to apply them through Raft.
#[tokio::test]
async fn test_write_validates_key_size() {
    let node = create_test_node(1).await;
    // Set initialized so we get past that check
    set_initialized(&node, true);

    // Create a write command with invalid key (> MAX_KEY_SIZE = 1KB)
    let oversized_key = "x".repeat(2000);
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: oversized_key,
            value: "value".to_string(),
        },
    };
    let result = KeyValueStore::write(&node, write_request).await;

    // Should fail validation before reaching Raft
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::KeyTooLarge { size, max }) => {
            assert_eq!(size, 2000);
            assert!(max <= 1024); // MAX_KEY_SIZE
        }
        other => panic!("Expected KeyTooLarge error for oversized key, got {:?}", other),
    }
}

/// Test that validate_write_command catches oversized values.
#[tokio::test]
async fn test_write_validates_value_size() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    // Create a write command with invalid value (> MAX_VALUE_SIZE = 1MB)
    let oversized_value = "x".repeat(2_000_000);
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: oversized_value,
        },
    };
    let result = KeyValueStore::write(&node, write_request).await;

    // Should fail validation before reaching Raft
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::ValueTooLarge { size, max }) => {
            assert_eq!(size, 2_000_000);
            assert!(max <= 1_048_576); // MAX_VALUE_SIZE
        }
        other => panic!("Expected ValueTooLarge error for oversized value, got {:?}", other),
    }
}

// ========================================================================
// ReadConsistency Routing Tests
// ========================================================================

/// Test that ReadConsistency::Linearizable routes through ReadIndex.
///
/// The actual ReadIndex call requires a working Raft leader, but we can
/// verify the routing logic and that the operation fails appropriately
/// when there's no leader (which is expected in a single uninitialized node).
#[tokio::test]
async fn test_read_linearizable_uses_read_index() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Linearizable,
    };
    let result = KeyValueStore::read(&node, request).await;

    // Should fail due to not being leader (ReadIndex requires leader)
    // The error indicates the read went through the correct path
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotLeader { .. }) => {
            // Expected: ReadIndex requires leader, which we don't have
        }
        Err(KeyValueStoreError::Timeout { .. }) => {
            // Also acceptable: timeout waiting for ReadIndex
        }
        other => panic!("Expected NotLeader or Timeout for linearizable read without leader, got {:?}", other),
    }
}

/// Test that ReadConsistency::Lease routes through LeaseRead.
#[tokio::test]
async fn test_read_lease_uses_lease_read() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Lease,
    };
    let result = KeyValueStore::read(&node, request).await;

    // Should fail due to not being leader (LeaseRead requires leader)
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotLeader { .. }) => {
            // Expected: LeaseRead requires leader
        }
        Err(KeyValueStoreError::Timeout { .. }) => {
            // Also acceptable: timeout waiting for lease
        }
        other => panic!("Expected NotLeader or Timeout for lease read without leader, got {:?}", other),
    }
}

/// Test that ReadConsistency::Stale skips linearizer and reads directly.
///
/// Stale reads don't require a leader and return immediately from local
/// state machine, which means they should fail with NotFound (no data)
/// rather than NotLeader.
#[tokio::test]
async fn test_read_stale_skips_linearizer() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Stale,
    };
    let result = KeyValueStore::read(&node, request).await;

    // Stale reads go directly to state machine, no leader needed
    // Should fail with NotFound (key doesn't exist) rather than NotLeader
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotFound { key }) => {
            assert_eq!(key, "test");
        }
        other => panic!("Expected NotFound for stale read of missing key, got {:?}", other),
    }
}

/// Test ReadConsistency routing exhaustiveness.
#[test]
fn test_read_consistency_variants_exhaustive() {
    // Verify all ReadConsistency variants are accounted for
    let variants = [
        ReadConsistency::Linearizable,
        ReadConsistency::Lease,
        ReadConsistency::Stale,
    ];

    for consistency in variants {
        // Pattern match to ensure exhaustive handling
        match consistency {
            ReadConsistency::Linearizable => {} // Uses ReadIndex
            ReadConsistency::Lease => {}        // Uses LeaseRead
            ReadConsistency::Stale => {}        // Direct state machine read
        }
    }
}

/// Test transfer_leader requires initialization.
#[tokio::test]
async fn test_transfer_leader_requires_initialization() {
    let node = create_test_node(1).await;
    // Not initialized — should fail
    let result = ClusterController::transfer_leader(&node, 2).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {}
        other => panic!("Expected NotInitialized, got {:?}", other),
    }
}

/// Test transfer_leader call succeeds on an initialized leader node.
///
/// In the madsim test environment, the network layer doesn't support the
/// full transfer_leader protocol (requires v2 network). This test verifies
/// the RaftNode plumbing: the call is accepted and initiated (returns Ok).
/// Full end-to-end leadership transfer is validated by the NixOS VM test.
#[tokio::test]
async fn test_transfer_leader_accepted_on_initialized_leader() {
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());

    let config = Arc::new(Config {
        cluster_name: "transfer-test".to_string(),
        ..Default::default()
    });
    let log_storage = InMemoryLogStore::default();
    let state_machine = Arc::new(InMemoryStateMachine::default());
    let net = MadsimNetworkFactory::new(NodeId(1), router.clone(), failure_injector);

    let raft = openraft::Raft::new(NodeId(1), config, net, log_storage, state_machine.clone())
        .await
        .expect("create raft");

    router.register_node(NodeId(1), "127.0.0.1:27001".to_string(), raft.clone()).expect("register node");

    let node = RaftNode::new(NodeId(1), Arc::new(raft), StateMachineVariant::InMemory(state_machine));

    let secret = iroh::SecretKey::generate(&mut rand::rng());
    let cn = {
        let mut cn = ClusterNode::new(1, "node-1", None);
        cn.node_addr = Some(aspen_cluster_types::NodeAddress::new(iroh::EndpointAddr::from_parts(
            secret.public(),
            std::iter::empty(),
        )));
        cn
    };

    ClusterController::init(&node, InitRequest {
        initial_members: vec![cn],
        trust: Default::default(),
    })
    .await
    .expect("init cluster");

    // Wait for leader election (single node = immediate)
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // transfer_leader to a non-existent target: openraft accepts the trigger
    // (it's fire-and-forget), so this returns Ok even if the target doesn't exist.
    let result = ClusterController::transfer_leader(&node, 99).await;
    assert!(result.is_ok(), "transfer_leader should be accepted: {:?}", result.err());
}

/// Test that scan uses ReadIndex for linearizability.
#[tokio::test]
async fn test_scan_uses_read_index() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ScanRequest {
        prefix: "test".to_string(),
        limit_results: None,
        continuation_token: None,
    };
    let result = KeyValueStore::scan(&node, request).await;

    // Scan always uses ReadIndex for linearizability
    // Should fail due to not being leader
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotLeader { .. }) => {
            // Expected: ReadIndex requires leader
        }
        Err(KeyValueStoreError::Timeout { .. }) => {
            // Also acceptable: timeout waiting for ReadIndex
        }
        other => panic!("Expected NotLeader or Timeout for scan without leader, got {:?}", other),
    }
}
