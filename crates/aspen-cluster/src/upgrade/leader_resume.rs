//! Leader transition detection for automatic deployment resume.
//!
//! Watches Raft metrics for leader state transitions and calls
//! `DeploymentCoordinator::check_and_resume()` when this node becomes leader.
//! Runs in a background task so it doesn't block leader initialization.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_deploy::DeploymentCoordinator;
use aspen_deploy::coordinator::rpc::NodeRpcClient;
use aspen_raft::types::AppTypeConfig;
use openraft::Raft;
use openraft::async_runtime::watch::WatchReceiver;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

/// Spawn a background task that watches for leader transitions and resumes
/// in-progress deployments.
///
/// When the node transitions to Leader state, spawns a task that calls
/// `check_and_resume()` on a `DeploymentCoordinator`. This handles the case
/// where the previous leader upgraded itself and restarted, leaving a
/// deployment in `Deploying` state in KV.
///
/// The resume task is spawned (not awaited) so it doesn't block the leader's
/// first heartbeat or normal Raft operations.
///
/// Returns a `CancellationToken` to stop the watcher.
pub fn spawn_deploy_resume_watcher<K, R>(
    raft: Arc<Raft<AppTypeConfig>>,
    kv: Arc<K>,
    rpc_client: Arc<R>,
    node_id: u64,
) -> CancellationToken
where
    K: KeyValueStore + ?Sized + 'static,
    R: NodeRpcClient + 'static,
{
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        deploy_resume_watcher_task(raft, kv, rpc_client, node_id, cancel_clone).await;
    });

    cancel
}

/// Background task that watches Raft metrics for leader transitions.
async fn deploy_resume_watcher_task<K, R>(
    raft: Arc<Raft<AppTypeConfig>>,
    kv: Arc<K>,
    rpc_client: Arc<R>,
    node_id: u64,
    cancel: CancellationToken,
) where
    K: KeyValueStore + ?Sized + 'static,
    R: NodeRpcClient + 'static,
{
    let mut rx = raft.metrics();
    // Check initial state
    let mut was_leader = {
        let metrics = rx.borrow_watched();
        metrics.state == openraft::ServerState::Leader
    };

    // If we're already leader at startup, check for in-progress deployments
    if was_leader {
        spawn_resume_task(kv.clone(), rpc_client.clone(), node_id);
    }

    loop {
        tokio::select! {
            result = rx.changed() => {
                match result {
                    Ok(()) => {
                        let is_leader = {
                            let metrics = rx.borrow_watched();
                            metrics.state == openraft::ServerState::Leader
                        };

                        // Detect transition TO leader
                        if is_leader && !was_leader {
                            info!(node_id, "node became leader, checking for in-progress deployments");
                            spawn_resume_task(kv.clone(), rpc_client.clone(), node_id);
                        }

                        was_leader = is_leader;
                    }
                    Err(_) => {
                        info!("deploy resume watcher exiting: Raft metrics channel closed");
                        break;
                    }
                }
            }
            _ = cancel.cancelled() => {
                info!("deploy resume watcher cancelled");
                break;
            }
        }
    }
}

/// Spawn a one-shot task that attempts to resume in-progress deployments.
fn spawn_resume_task<K, R>(kv: Arc<K>, rpc_client: Arc<R>, node_id: u64)
where
    K: KeyValueStore + ?Sized + 'static,
    R: NodeRpcClient + 'static,
{
    tokio::spawn(async move {
        let coordinator: DeploymentCoordinator<K, R, dyn aspen_core::ClusterController> =
            DeploymentCoordinator::new(kv, rpc_client, node_id);
        match coordinator.check_and_resume().await {
            Ok(Some(record)) => {
                info!(
                    deploy_id = %record.deploy_id,
                    status = ?record.status,
                    "resumed deployment {}", record.deploy_id
                );
            }
            Ok(None) => {
                info!(node_id, "no in-progress deployment to resume");
            }
            Err(e) => {
                error!(node_id, error = %e, "resume failed: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use aspen_deploy::DeployArtifact;
    use aspen_deploy::DeployStrategy;
    use aspen_deploy::DeploymentRecord;
    use aspen_deploy::DeploymentStatus;
    use aspen_deploy::NodeDeployStatus;
    use aspen_deploy::coordinator::rpc::RpcError;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    /// Mock RPC client that always reports healthy.
    struct MockHealthyRpcClient;

    #[async_trait::async_trait]
    impl NodeRpcClient for MockHealthyRpcClient {
        async fn send_upgrade(&self, _: u64, _: &str, _: &str, _: Option<&str>) -> Result<(), RpcError> {
            Ok(())
        }
        async fn send_rollback(&self, _: u64, _: &str) -> Result<(), RpcError> {
            Ok(())
        }
        async fn check_health(&self, _: u64) -> Result<bool, RpcError> {
            Ok(true)
        }
    }

    /// Test: check_and_resume with a Deploying record resumes pending nodes.
    #[tokio::test]
    async fn test_check_and_resume_with_deploying_record() {
        let kv = DeterministicKeyValueStore::new();
        let rpc_client = Arc::new(MockHealthyRpcClient);

        // Write a Deploying record with mixed node states
        let mut record = DeploymentRecord::new(
            "deploy-resume-1".into(),
            DeployArtifact::NixStorePath("/nix/store/abc-aspen".into()),
            DeployStrategy::rolling(1),
            &[1, 2, 3],
            1000,
        );
        record.status = DeploymentStatus::Deploying;
        // Node 1: already healthy (first upgraded), node 2: healthy, node 3: pending (leader, last)
        record.nodes[0].status = NodeDeployStatus::Healthy;
        record.nodes[1].status = NodeDeployStatus::Healthy;
        record.nodes[2].status = NodeDeployStatus::Pending;

        let value = serde_json::to_string(&record).unwrap();
        kv.write(aspen_core::WriteRequest::set("_sys:deploy:current", value)).await.unwrap();

        // Resume
        let coordinator: DeploymentCoordinator<_, _, dyn aspen_core::ClusterController> =
            DeploymentCoordinator::with_timeouts(kv.clone(), rpc_client, 1, 5, 1);
        let result = coordinator.check_and_resume().await;
        assert!(result.is_ok(), "check_and_resume should succeed: {:?}", result.err());

        let resumed = result.unwrap();
        assert!(resumed.is_some(), "should find an in-progress deployment");
        let record = resumed.unwrap();
        assert_eq!(record.deploy_id, "deploy-resume-1");
        // All nodes should be healthy after resume
        for node in &record.nodes {
            assert_eq!(node.status, NodeDeployStatus::Healthy, "node {} should be healthy after resume", node.node_id);
        }
    }

    /// Test: check_and_resume with no deployment returns None.
    #[tokio::test]
    async fn test_check_and_resume_no_deployment() {
        let kv = DeterministicKeyValueStore::new();
        let rpc_client = Arc::new(MockHealthyRpcClient);

        let coordinator: DeploymentCoordinator<_, _, dyn aspen_core::ClusterController> =
            DeploymentCoordinator::new(kv, rpc_client, 1);
        let result = coordinator.check_and_resume().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "should find no deployment");
    }

    /// Test: check_and_resume with Completed deployment returns None.
    #[tokio::test]
    async fn test_check_and_resume_completed_deployment() {
        let kv = DeterministicKeyValueStore::new();
        let rpc_client = Arc::new(MockHealthyRpcClient);

        let mut record = DeploymentRecord::new(
            "deploy-done".into(),
            DeployArtifact::NixStorePath("/nix/store/abc-aspen".into()),
            DeployStrategy::rolling(1),
            &[1, 2, 3],
            1000,
        );
        record.status = DeploymentStatus::Completed;

        let value = serde_json::to_string(&record).unwrap();
        kv.write(aspen_core::WriteRequest::set("_sys:deploy:current", value)).await.unwrap();

        let coordinator: DeploymentCoordinator<_, _, dyn aspen_core::ClusterController> =
            DeploymentCoordinator::new(kv, rpc_client, 1);
        let result = coordinator.check_and_resume().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "completed deployment should not be resumed");
    }
}
