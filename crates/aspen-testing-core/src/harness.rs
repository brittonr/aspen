//! Shared test harness facade for cross-layer cluster testing.
//!
//! Defines [`TestCluster`] — the common interface for cluster lifecycle,
//! readiness waits, and KV operations across test layers (in-memory router,
//! madsim simulation, patchbay namespace, real Iroh network).
//!
//! Layer-specific crates implement `TestCluster` so suites can be promoted
//! between layers without rewriting bootstrap logic.

use std::time::Duration;

use async_trait::async_trait;

/// Outcome of a single wait attempt.
#[derive(Debug, Clone)]
pub enum WaitOutcome {
    /// Condition was met.
    Ready,
    /// Timed out before the condition was met.
    TimedOut {
        /// Human-readable description of what was being waited on.
        condition: String,
        /// How long we waited.
        elapsed: Duration,
    },
}

impl WaitOutcome {
    /// Returns `Ok(())` if ready, or an error describing the timeout.
    pub fn into_result(self) -> Result<(), WaitError> {
        match self {
            Self::Ready => Ok(()),
            Self::TimedOut { condition, elapsed } => Err(WaitError::TimedOut { condition, elapsed }),
        }
    }
}

/// Error returned by wait helpers.
#[derive(Debug, Clone)]
pub enum WaitError {
    /// The condition was not met within the timeout.
    TimedOut { condition: String, elapsed: Duration },
    /// The cluster reported an error while polling.
    ClusterError(String),
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TimedOut { condition, elapsed } => {
                write!(f, "timed out after {elapsed:?} waiting for: {condition}")
            }
            Self::ClusterError(msg) => write!(f, "cluster error during wait: {msg}"),
        }
    }
}

impl std::error::Error for WaitError {}

/// Common interface for test cluster lifecycle and operations.
///
/// Each test layer (router, madsim, patchbay, real-network) provides its own
/// implementation. Suites written against `TestCluster` can move between
/// layers without changing their core logic.
///
/// # Provided methods
///
/// [`wait_until`] builds bounded poll loops from a closure, so layer
/// implementations only need to expose the primitive operations and callers
/// get consistent timeout behavior for free.
#[async_trait]
pub trait TestCluster: Send + Sync {
    /// Wait for a leader to be elected. Returns the leader's node ID.
    async fn wait_for_leader(&self, timeout: Duration) -> Result<u64, WaitError>;

    /// Wait for all voter nodes to reach the same applied log index.
    async fn wait_for_replication(&self, timeout: Duration) -> Result<(), WaitError>;

    /// Write a key-value pair through the cluster leader.
    async fn write_kv(&self, key: &str, value: &str) -> Result<(), WaitError>;

    /// Read a value from the cluster. Returns `None` if the key does not exist.
    async fn read_kv(&self, key: &str) -> Result<Option<String>, WaitError>;

    /// Number of nodes in the cluster.
    fn node_count(&self) -> u32;
}

/// Run a bounded poll loop until `check` returns `true` or `timeout` elapses.
///
/// `condition` is a human-readable label used in timeout error messages.
/// `poll_interval` controls how often `check` is called.
pub async fn wait_until<F, Fut>(
    condition: &str,
    timeout: Duration,
    poll_interval: Duration,
    mut check: F,
) -> Result<(), WaitError>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<bool, String>> + Send,
{
    #[allow(unknown_lints)]
    #[allow(ambient_clock, reason = "test harness wait boundary reads monotonic time")]
    fn current_instant() -> tokio::time::Instant {
        tokio::time::Instant::now()
    }

    debug_assert!(!timeout.is_zero(), "wait_until timeout must be positive");
    debug_assert!(!poll_interval.is_zero(), "poll interval must be positive");

    let start = current_instant();
    let deadline = start + timeout;

    while current_instant() < deadline {
        match check().await {
            Ok(true) => return Ok(()),
            Ok(false) => tokio::time::sleep(poll_interval).await,
            Err(msg) => return Err(WaitError::ClusterError(msg)),
        }
    }

    Err(WaitError::TimedOut {
        condition: condition.to_string(),
        elapsed: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_until_immediate_success() {
        let result =
            wait_until("always true", Duration::from_secs(1), Duration::from_millis(10), || async { Ok(true) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn wait_until_times_out() {
        let result =
            wait_until("never true", Duration::from_millis(50), Duration::from_millis(10), || async { Ok(false) })
                .await;
        match result {
            Err(WaitError::TimedOut { condition, .. }) => {
                assert_eq!(condition, "never true");
            }
            other => panic!("expected TimedOut, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn wait_until_cluster_error() {
        let result = wait_until("broken", Duration::from_secs(1), Duration::from_millis(10), || async {
            Err("node crashed".to_string())
        })
        .await;
        match result {
            Err(WaitError::ClusterError(msg)) => {
                assert_eq!(msg, "node crashed");
            }
            other => panic!("expected ClusterError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn wait_until_succeeds_after_retries() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = wait_until("third try", Duration::from_secs(1), Duration::from_millis(10), move || {
            let c = counter_clone.clone();
            async move {
                let val = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(val >= 2)
            }
        })
        .await;
        assert!(result.is_ok());
        assert!(counter.load(std::sync::atomic::Ordering::SeqCst) >= 3);
    }

    #[tokio::test]
    async fn wait_outcome_into_result() {
        assert!(WaitOutcome::Ready.into_result().is_ok());

        let timed_out_outcome = WaitOutcome::TimedOut {
            condition: "test".to_string(),
            elapsed: Duration::from_secs(5),
        };
        let err = timed_out_outcome.into_result().unwrap_err();
        assert!(err.to_string().contains("test"));
    }

    #[tokio::test]
    async fn wait_error_display_includes_elapsed() {
        let err = WaitError::TimedOut {
            condition: "leader".to_string(),
            elapsed: Duration::from_secs(10),
        };
        let msg = err.to_string();
        assert!(msg.contains("10s"), "message should show elapsed: {msg}");
        assert!(msg.contains("leader"), "message should show condition: {msg}");
    }

    #[tokio::test]
    async fn wait_error_cluster_error_display() {
        let err = WaitError::ClusterError("connection refused".into());
        assert!(err.to_string().contains("connection refused"));
    }
}
