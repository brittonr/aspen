//! Integration tests for DeploymentCoordinator.
//!
//! Uses DeterministicKeyValueStore and a mock NodeRpcClient to verify
//! state transitions, concurrent deploy rejection, and failover recovery.

use std::collections::HashMap;
use std::sync::Arc;

use aspen_deploy::DEPLOY_CURRENT_KEY;
use aspen_deploy::DEPLOY_HISTORY_PREFIX;
use aspen_deploy::DeployArtifact;
use aspen_deploy::DeployStrategy;
use aspen_deploy::DeploymentCoordinator;
use aspen_deploy::DeploymentRecord;
use aspen_deploy::DeploymentStatus;
use aspen_deploy::NodeDeployStatus;
use aspen_deploy::coordinator::rpc::NodeRpcClient;
use aspen_deploy::coordinator::rpc::RpcError;
use aspen_deploy::error::DeployError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteRequest;
use aspen_testing::DeterministicKeyValueStore;
use aspen_traits::KeyValueStore;
use tokio::sync::Mutex;

// ============================================================================
// Mock NodeRpcClient
// ============================================================================

struct MockRpcClient {
    upgrade_results: Mutex<HashMap<u64, Option<String>>>,
    rollback_results: Mutex<HashMap<u64, Option<String>>>,
    health_results: Mutex<HashMap<u64, bool>>,
    upgrade_calls: Mutex<Vec<(u64, String)>>,
    rollback_calls: Mutex<Vec<u64>>,
}

impl MockRpcClient {
    fn new() -> Self {
        Self {
            upgrade_results: Mutex::new(HashMap::new()),
            rollback_results: Mutex::new(HashMap::new()),
            health_results: Mutex::new(HashMap::new()),
            upgrade_calls: Mutex::new(Vec::new()),
            rollback_calls: Mutex::new(Vec::new()),
        }
    }

    async fn set_all_healthy(&self, node_ids: &[u64]) {
        let mut health = self.health_results.lock().await;
        for &id in node_ids {
            health.insert(id, true);
        }
    }

    async fn set_upgrade_failure(&self, node_id: u64, msg: &str) {
        self.upgrade_results.lock().await.insert(node_id, Some(msg.to_string()));
    }

    async fn set_unhealthy(&self, node_id: u64) {
        self.health_results.lock().await.insert(node_id, false);
    }

    async fn set_rollback_failure(&self, node_id: u64, msg: &str) {
        self.rollback_results.lock().await.insert(node_id, Some(msg.to_string()));
    }

    async fn get_upgrade_calls(&self) -> Vec<(u64, String)> {
        self.upgrade_calls.lock().await.clone()
    }

    async fn get_rollback_calls(&self) -> Vec<u64> {
        self.rollback_calls.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl NodeRpcClient for MockRpcClient {
    async fn send_upgrade(&self, node_id: u64, artifact_ref: &str) -> std::result::Result<(), RpcError> {
        self.upgrade_calls.lock().await.push((node_id, artifact_ref.to_string()));
        let results = self.upgrade_results.lock().await;
        match results.get(&node_id) {
            Some(Some(msg)) => Err(RpcError::new(msg.clone())),
            _ => Ok(()),
        }
    }

    async fn send_rollback(&self, node_id: u64) -> std::result::Result<(), RpcError> {
        self.rollback_calls.lock().await.push(node_id);
        let results = self.rollback_results.lock().await;
        match results.get(&node_id) {
            Some(Some(msg)) => Err(RpcError::new(msg.clone())),
            _ => Ok(()),
        }
    }

    async fn check_health(&self, node_id: u64) -> std::result::Result<bool, RpcError> {
        let results = self.health_results.lock().await;
        match results.get(&node_id) {
            Some(healthy) => Ok(*healthy),
            None => Ok(true),
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Create a coordinator with DeterministicKeyValueStore (uses dyn dispatch
/// to avoid Arc<Arc<>> ambiguity from blanket impl KeyValueStore for Arc<T>).
fn make_coordinator(
    kv: Arc<dyn KeyValueStore>,
    rpc: Arc<MockRpcClient>,
    node_id: u64,
) -> DeploymentCoordinator<dyn KeyValueStore, MockRpcClient> {
    DeploymentCoordinator::with_timeouts(kv, rpc, node_id, 2, 1)
}

fn test_artifact() -> DeployArtifact {
    DeployArtifact::NixStorePath("/nix/store/abc123-aspen-node-0.2.0".to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Helper: create KV store as Arc<dyn KeyValueStore>.
fn new_kv() -> Arc<dyn KeyValueStore> {
    Arc::new(DeterministicKeyValueStore::new()) as Arc<dyn KeyValueStore>
}

/// Helper: read from the KV store directly.
async fn kv_read(kv: &dyn KeyValueStore, key: &str) -> Option<String> {
    match kv.read(ReadRequest::new(key)).await {
        Ok(r) => r.kv.map(|e| e.value),
        Err(_) => None,
    }
}

/// Helper: write to the KV store directly.
async fn kv_write(kv: &dyn KeyValueStore, key: &str, value: &str) {
    kv.write(WriteRequest::set(key, value)).await.unwrap();
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_start_deployment_creates_pending_record() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv.clone(), rpc, 1);

    let record = coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    assert_eq!(record.deploy_id, "deploy-1");
    assert_eq!(record.status, DeploymentStatus::Pending);
    assert_eq!(record.nodes.len(), 3);
    for node in &record.nodes {
        assert_eq!(node.status, NodeDeployStatus::Pending);
    }

    // Verify it's in KV
    assert!(kv_read(&*kv, DEPLOY_CURRENT_KEY).await.is_some());
}

#[tokio::test]
async fn test_start_deployment_rejects_concurrent() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv, rpc, 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    let result = coord
        .start_deployment("deploy-2".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        DeployError::DeploymentInProgress { deploy_id } => {
            assert_eq!(deploy_id, "deploy-1");
        }
        other => panic!("expected DeploymentInProgress, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_start_deployment_allows_after_terminal() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv.clone(), rpc, 1);

    // Write a completed deployment directly
    let mut record =
        DeploymentRecord::new("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms());
    record.status = DeploymentStatus::Completed;
    let json = serde_json::to_string(&record).unwrap();
    kv_write(&*kv, DEPLOY_CURRENT_KEY, &json).await;

    let new_record = coord
        .start_deployment("deploy-2".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    assert_eq!(new_record.deploy_id, "deploy-2");
    assert_eq!(new_record.status, DeploymentStatus::Pending);
}

#[tokio::test]
async fn test_run_deployment_3_node_cluster() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2, 3]).await;
    let coord = make_coordinator(kv.clone(), rpc.clone(), 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    let result = coord.run_deployment("deploy-1").await.unwrap();

    assert_eq!(result.status, DeploymentStatus::Completed);
    for node in &result.nodes {
        assert_eq!(node.status, NodeDeployStatus::Healthy);
    }

    // Followers (2, 3) before leader (1)
    let calls = rpc.get_upgrade_calls().await;
    assert_eq!(calls.len(), 3);
    let leader_pos = calls.iter().position(|(id, _)| *id == 1).unwrap();
    assert_eq!(leader_pos, 2, "leader should be upgraded last");

    // History created
    let scan = kv
        .scan(ScanRequest {
            prefix: DEPLOY_HISTORY_PREFIX.to_string(),
            limit_results: Some(10),
            continuation_token: None,
        })
        .await
        .unwrap();
    assert_eq!(scan.entries.len(), 1);

    // Current deleted (DeterministicKeyValueStore returns Err(NotFound) for missing keys)
    assert!(kv_read(&*kv, DEPLOY_CURRENT_KEY).await.is_none());
}

#[tokio::test]
async fn test_run_deployment_node_upgrade_failure() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2, 3]).await;
    rpc.set_upgrade_failure(2, "connection refused").await;
    let coord = make_coordinator(kv.clone(), rpc, 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    let result = coord.run_deployment("deploy-1").await;
    assert!(result.is_err());

    // KV should show Failed status
    let raw = kv_read(&*kv, DEPLOY_CURRENT_KEY).await.unwrap();
    let record: DeploymentRecord = serde_json::from_str(&raw).unwrap();
    assert_eq!(record.status, DeploymentStatus::Failed);
    assert!(record.error.unwrap().contains("connection refused"));
}

#[tokio::test]
async fn test_run_deployment_health_timeout() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_unhealthy(2).await; // Node 2 never passes health
    let coord = make_coordinator(kv, rpc, 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    let result = coord.run_deployment("deploy-1").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        DeployError::NodeUpgradeFailed { node_id, reason } => {
            assert_eq!(node_id, 2);
            assert!(reason.contains("timed out"));
        }
        other => panic!("expected NodeUpgradeFailed, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_get_status_active_deployment() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv, rpc, 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    let status = coord.get_status().await.unwrap();
    assert_eq!(status.deploy_id, "deploy-1");
    assert_eq!(status.status, DeploymentStatus::Pending);
}

#[tokio::test]
async fn test_get_status_falls_back_to_history() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2, 3]).await;
    let coord = make_coordinator(kv, rpc, 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();
    coord.run_deployment("deploy-1").await.unwrap();

    let status = coord.get_status().await.unwrap();
    assert_eq!(status.deploy_id, "deploy-1");
    assert_eq!(status.status, DeploymentStatus::Completed);
}

#[tokio::test]
async fn test_get_status_no_deployment() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv, rpc, 1);

    let result = coord.get_status().await;
    assert!(result.is_err());
    match result.unwrap_err() {
        DeployError::NoDeploymentFound => {}
        other => panic!("expected NoDeploymentFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_rollback_deployment() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2, 3]).await;
    let coord = make_coordinator(kv.clone(), rpc.clone(), 1);

    // Complete a deployment, then write a Completed record to _sys:deploy:current
    // for rollback to find.
    let mut record =
        DeploymentRecord::new("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms());
    record.status = DeploymentStatus::Completed;
    for node in &mut record.nodes {
        node.status = NodeDeployStatus::Healthy;
    }
    let json = serde_json::to_string(&record).unwrap();
    kv_write(&*kv, DEPLOY_CURRENT_KEY, &json).await;

    let result = coord.rollback_deployment().await.unwrap();
    assert_eq!(result.status, DeploymentStatus::RolledBack);

    let calls = rpc.get_rollback_calls().await;
    assert_eq!(calls.len(), 3);
}

#[tokio::test]
async fn test_rollback_partial_failure() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_rollback_failure(2, "rollback refused").await;
    let coord = make_coordinator(kv.clone(), rpc, 1);

    let mut record =
        DeploymentRecord::new("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms());
    record.status = DeploymentStatus::Completed;
    for node in &mut record.nodes {
        node.status = NodeDeployStatus::Healthy;
    }
    let json = serde_json::to_string(&record).unwrap();
    kv_write(&*kv, DEPLOY_CURRENT_KEY, &json).await;

    let result = coord.rollback_deployment().await.unwrap();
    assert_eq!(result.status, DeploymentStatus::Failed);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_rollback_rejects_non_eligible_status() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv, rpc, 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms())
        .await
        .unwrap();

    let result = coord.rollback_deployment().await;
    assert!(result.is_err());
    match result.unwrap_err() {
        DeployError::NotRollbackEligible { deploy_id, .. } => {
            assert_eq!(deploy_id, "deploy-1");
        }
        other => panic!("expected NotRollbackEligible, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_failover_recovery_resumes_deploying() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2, 3]).await;

    // Simulate partially-deployed state: node 2 healthy, nodes 1 & 3 pending
    let mut record =
        DeploymentRecord::new("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms());
    record.status = DeploymentStatus::Deploying;
    record.nodes[1].status = NodeDeployStatus::Healthy; // node 2
    let json = serde_json::to_string(&record).unwrap();
    kv_write(&*kv, DEPLOY_CURRENT_KEY, &json).await;

    let coord = make_coordinator(kv, rpc, 1);
    let result = coord.check_and_resume().await.unwrap();

    assert!(result.is_some());
    let resumed = result.unwrap();
    assert_eq!(resumed.status, DeploymentStatus::Completed);
    assert_eq!(resumed.count_healthy(), 3);
}

#[tokio::test]
async fn test_failover_recovery_no_deployment() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    let coord = make_coordinator(kv, rpc, 1);

    let result = coord.check_and_resume().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_failover_recovery_completed_not_resumed() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());

    let mut record =
        DeploymentRecord::new("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms());
    record.status = DeploymentStatus::Completed;
    let json = serde_json::to_string(&record).unwrap();
    kv_write(&*kv, DEPLOY_CURRENT_KEY, &json).await;

    let coord = make_coordinator(kv, rpc, 1);
    let result = coord.check_and_resume().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_failover_recovery_with_failed_node() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());

    let mut record =
        DeploymentRecord::new("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1, 2, 3], now_ms());
    record.status = DeploymentStatus::Deploying;
    record.nodes[1].status = NodeDeployStatus::Failed("previous failure".into());
    let json = serde_json::to_string(&record).unwrap();
    kv_write(&*kv, DEPLOY_CURRENT_KEY, &json).await;

    let coord = make_coordinator(kv, rpc, 1);
    let result = coord.check_and_resume().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_single_node_deployment() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1]).await;
    let coord = make_coordinator(kv, rpc.clone(), 1);

    coord
        .start_deployment("deploy-1".into(), test_artifact(), DeployStrategy::rolling(1), &[1], now_ms())
        .await
        .unwrap();

    let result = coord.run_deployment("deploy-1").await.unwrap();
    assert_eq!(result.status, DeploymentStatus::Completed);
    assert_eq!(result.count_healthy(), 1);

    let calls = rpc.get_upgrade_calls().await;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, 1);
}

#[tokio::test]
async fn test_history_pruning() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2, 3]).await;
    let coord = make_coordinator(kv.clone(), rpc, 1);

    for i in 0..55u32 {
        let deploy_id = format!("deploy-{i}");
        coord
            .start_deployment(
                deploy_id.clone(),
                test_artifact(),
                DeployStrategy::rolling(1),
                &[1, 2, 3],
                now_ms() + u64::from(i),
            )
            .await
            .unwrap();
        coord.run_deployment(&deploy_id).await.unwrap();
    }

    let scan = kv
        .scan(ScanRequest {
            prefix: DEPLOY_HISTORY_PREFIX.to_string(),
            limit_results: Some(100),
            continuation_token: None,
        })
        .await
        .unwrap();

    assert!(
        scan.entries.len() <= 50,
        "history should be pruned to MAX_DEPLOY_HISTORY (50), got {}",
        scan.entries.len()
    );
}

#[tokio::test]
async fn test_blob_artifact_deployment() {
    let kv = new_kv();
    let rpc = Arc::new(MockRpcClient::new());
    rpc.set_all_healthy(&[1, 2]).await;
    let coord = make_coordinator(kv, rpc.clone(), 1);

    let blob_hash = "a".repeat(64);
    let artifact = DeployArtifact::BlobHash(blob_hash.clone());

    coord
        .start_deployment("deploy-blob".into(), artifact, DeployStrategy::rolling(1), &[1, 2], now_ms())
        .await
        .unwrap();

    let result = coord.run_deployment("deploy-blob").await.unwrap();
    assert_eq!(result.status, DeploymentStatus::Completed);

    let calls = rpc.get_upgrade_calls().await;
    assert_eq!(calls.len(), 2);
    for (_, artifact_ref) in &calls {
        assert_eq!(artifact_ref, &blob_hash);
    }
}
