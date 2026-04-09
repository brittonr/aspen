//! Named wait helpers for common test readiness conditions.
//!
//! Each helper wraps [`wait_until`] with a descriptive condition name
//! and a sensible default poll interval. Callers get clear timeout messages:
//!
//! > timed out after 30s waiting for: replication converged
//!
//! For helpers that need to return a value (leader ID, key value), callers
//! should use [`wait_until`] directly with their own state capture.

use std::time::Duration;

use crate::harness::WaitError;
use crate::harness::wait_until;

/// Default poll interval for leader checks.
const LEADER_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Default poll interval for replication checks.
const REPLICATION_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Default poll interval for health checks.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Default poll interval for job completion checks.
const JOB_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Wait for all nodes to converge on the same applied log index.
///
/// `get_indices` should return the applied log index for each connected node.
/// Succeeds when all indices are equal and non-empty.
pub async fn wait_for_replication<F, Fut>(timeout: Duration, mut get_indices: F) -> Result<(), WaitError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<Vec<u64>, String>> + Send,
{
    wait_until("replication converged", timeout, REPLICATION_POLL_INTERVAL, || {
        let fut = get_indices();
        async {
            match fut.await {
                Ok(indices) if indices.is_empty() => Ok(false),
                Ok(indices) => Ok(indices.iter().all(|&i| i == indices[0])),
                Err(msg) => Err(msg),
            }
        }
    })
    .await
}

/// Wait for the cluster to have at least `expected_nodes` healthy members.
///
/// `check_health` should return the count of currently healthy nodes.
pub async fn wait_for_cluster_health<F, Fut>(
    expected_nodes: u32,
    timeout: Duration,
    mut check_health: F,
) -> Result<(), WaitError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<u32, String>> + Send,
{
    wait_until(&format!("cluster health ({expected_nodes} nodes)"), timeout, HEALTH_POLL_INTERVAL, || {
        let fut = check_health();
        async move {
            match fut.await {
                Ok(healthy) => Ok(healthy >= expected_nodes),
                Err(msg) => Err(msg),
            }
        }
    })
    .await
}

/// Wait for a job to reach a terminal state.
///
/// `check_done` should return `Ok(true)` when the job is finished
/// (success or failure), `Ok(false)` when still running.
pub async fn wait_for_job_completion<F, Fut>(job_id: &str, timeout: Duration, check_done: F) -> Result<(), WaitError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<bool, String>> + Send,
{
    wait_until(&format!("job {job_id} completed"), timeout, JOB_POLL_INTERVAL, check_done).await
}

/// Wait for a key to appear in the KV store.
///
/// `key_exists` should return `Ok(true)` when the key is present.
pub async fn wait_for_key_present<F, Fut>(key: &str, timeout: Duration, key_exists: F) -> Result<(), WaitError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<bool, String>> + Send,
{
    wait_until(&format!("key '{key}' present"), timeout, REPLICATION_POLL_INTERVAL, key_exists).await
}

/// Wait for a leader to be elected on any node.
///
/// `has_leader` should return `Ok(true)` when a leader is known.
/// To get the leader ID, use [`wait_until`] directly with your own capture.
pub async fn wait_for_leader<F, Fut>(timeout: Duration, has_leader: F) -> Result<(), WaitError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<bool, String>> + Send,
{
    wait_until("leader elected", timeout, LEADER_POLL_INTERVAL, has_leader).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replication_all_synced() {
        let result = wait_for_replication(Duration::from_secs(1), || async { Ok(vec![5, 5, 5]) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn replication_empty_indices_times_out() {
        let result = wait_for_replication(Duration::from_millis(50), || async { Ok(vec![]) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cluster_health_met() {
        let result = wait_for_cluster_health(3, Duration::from_secs(1), || async { Ok(3) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cluster_health_insufficient_times_out() {
        let result = wait_for_cluster_health(3, Duration::from_millis(50), || async { Ok(2) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn job_completion_immediate() {
        let result = wait_for_job_completion("job-123", Duration::from_secs(1), || async { Ok(true) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn job_completion_timeout_includes_id() {
        let result = wait_for_job_completion("job-456", Duration::from_millis(50), || async { Ok(false) }).await;
        match result {
            Err(WaitError::TimedOut { condition, .. }) => assert!(condition.contains("job-456")),
            other => panic!("expected TimedOut, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn key_present_immediate() {
        let result = wait_for_key_present("my-key", Duration::from_secs(1), || async { Ok(true) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn leader_immediate() {
        let result = wait_for_leader(Duration::from_secs(1), || async { Ok(true) }).await;
        assert!(result.is_ok());
    }
}
