//! Health polling for nodes during deployment.
//!
//! After a node restarts, the coordinator polls its health until:
//! 1. It responds to GetHealth with healthy status
//! 2. It appears in Raft membership
//! 3. Its Raft log gap is below DEPLOY_LOG_GAP_THRESHOLD
//!
//! All three must pass within DEPLOY_HEALTH_TIMEOUT_SECS.

use std::time::Duration;

use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::warn;

use super::DeployError;
use super::DeploymentCoordinator;
use super::Result;
use crate::coordinator::rpc::NodeRpcClient;

impl<K: KeyValueStore + ?Sized, R: NodeRpcClient> DeploymentCoordinator<K, R> {
    /// Poll a node's health until it passes all checks or times out.
    ///
    /// Bounded by `health_timeout_secs`. Polls every `poll_interval_secs`.
    pub(super) async fn poll_node_health(&self, node_id: u64) -> Result<()> {
        let timeout = Duration::from_secs(self.health_timeout_secs);
        let interval = Duration::from_secs(self.poll_interval_secs);

        let start = tokio::time::Instant::now();
        let deadline = start + timeout;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(DeployError::HealthCheckTimeout {
                    node_id,
                    timeout_secs: self.health_timeout_secs,
                });
            }

            match self.rpc_client.check_health(node_id).await {
                Ok(true) => {
                    debug!(node_id, "health check passed");
                    return Ok(());
                }
                Ok(false) => {
                    debug!(node_id, "health check not yet passing, retrying");
                }
                Err(e) => {
                    warn!(node_id, error = %e, "health check error, retrying");
                }
            }

            tokio::time::sleep(interval).await;
        }
    }
}
