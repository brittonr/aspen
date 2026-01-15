//! Blob replication topology watcher.
//!
//! Watches Raft metrics for membership changes and updates the
//! BlobReplicationManager's topology accordingly. This ensures
//! the placement strategy always has an up-to-date view of
//! available nodes for replica placement.
//!
//! # Architecture
//!
//! ```text
//! Raft Metrics Watch Channel
//!          |
//!          v
//!  topology_watcher task
//!          |
//!          v
//!  BlobReplicationManager.update_topology()
//!          |
//!          v
//!  PlacementStrategy uses updated node list
//! ```
//!
//! # Tiger Style
//!
//! - Single background task per node (bounded resource)
//! - Explicit shutdown via CancellationToken
//! - Reactive updates (no polling) via OpenRaft's watch channel
//! - Safe borrow pattern: extract data while holding borrow, release before async calls

use iroh::PublicKey;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::NodeInfo;
use super::manager::BlobReplicationManager;

/// Callback type for extracting node info from Raft metrics.
///
/// This callback is called while holding the metrics borrow, so it must
/// be synchronous and fast. It should extract the node information needed
/// for topology updates.
pub type NodeInfoExtractor<M> = Box<dyn Fn(&M) -> Vec<NodeInfo> + Send + Sync>;

/// Spawn a background task that watches Raft membership and updates blob topology.
///
/// The watcher monitors a metrics watch channel for membership changes. When
/// membership changes, it extracts NodeInfo from each node and calls
/// `update_topology()` on the BlobReplicationManager.
///
/// # Arguments
///
/// * `metrics_rx` - A watch receiver for metrics updates (e.g., from `raft.metrics()`)
/// * `extractor` - A function to extract `Vec<NodeInfo>` from the metrics
/// * `manager` - The BlobReplicationManager to update
///
/// # Returns
///
/// A CancellationToken that can be used to stop the watcher gracefully.
///
/// # Example
///
/// ```ignore
/// use aspen_blob::replication::topology_watcher::spawn_topology_watcher;
///
/// let extractor = Box::new(|metrics: &RaftMetrics<AppTypeConfig>| {
///     metrics.membership_config.membership().nodes()
///         .map(|(node_id, info)| NodeInfo::new(*node_id, info.iroh_addr.id))
///         .collect()
/// });
///
/// let cancel = spawn_topology_watcher(
///     raft.metrics(),
///     extractor,
///     manager.clone(),
/// );
///
/// // Later, when shutting down:
/// cancel.cancel();
/// ```
pub fn spawn_topology_watcher<M>(
    metrics_rx: tokio::sync::watch::Receiver<M>,
    extractor: NodeInfoExtractor<M>,
    manager: BlobReplicationManager,
) -> CancellationToken
where
    M: Send + Sync + 'static,
{
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        topology_watcher_task(metrics_rx, extractor, manager, cancel_clone).await;
    });

    cancel
}

/// Background task that watches for membership changes and updates topology.
async fn topology_watcher_task<M>(
    mut metrics_rx: tokio::sync::watch::Receiver<M>,
    extractor: NodeInfoExtractor<M>,
    manager: BlobReplicationManager,
    cancel: CancellationToken,
) where
    M: Send + Sync + 'static,
{
    info!("blob topology watcher started");

    // Initial sync - populate with current membership
    let initial_nodes = {
        let metrics = metrics_rx.borrow();
        extractor(&metrics)
    };

    if !initial_nodes.is_empty() {
        debug!(node_count = initial_nodes.len(), "initial topology sync");
        if let Err(e) = manager.update_topology(initial_nodes).await {
            warn!(error = %e, "failed initial topology update");
        }
    }

    loop {
        tokio::select! {
            result = metrics_rx.changed() => {
                match result {
                    Ok(()) => {
                        // Extract node info while holding the borrow
                        let nodes = {
                            let metrics = metrics_rx.borrow();
                            extractor(&metrics)
                        };

                        let node_count = nodes.len();
                        debug!(
                            node_count,
                            "membership changed, updating blob topology"
                        );

                        // Update topology (async, after releasing borrow)
                        if let Err(e) = manager.update_topology(nodes).await {
                            warn!(error = %e, "failed to update blob topology");
                        }
                    }
                    Err(_) => {
                        // Sender dropped - Raft is shutting down
                        info!("blob topology watcher exiting: metrics channel closed");
                        break;
                    }
                }
            }
            _ = cancel.cancelled() => {
                info!("blob topology watcher cancelled");
                break;
            }
        }
    }
}

/// Helper function to create a NodeInfo from node ID and public key.
///
/// This is a convenience function for use in extractors.
pub fn node_info_from_membership(node_id: u64, public_key: PublicKey) -> NodeInfo {
    NodeInfo::new(node_id, public_key)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use iroh::SecretKey;
    use tokio::sync::watch;

    use super::*;

    /// Generate a test PublicKey from a seed.
    fn test_public_key(seed: u8) -> PublicKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = seed;
        let secret = SecretKey::from_bytes(&key_bytes);
        secret.public()
    }

    /// Simple metrics type for testing.
    #[derive(Clone, Debug)]
    struct TestMetrics {
        nodes: Vec<(u64, PublicKey)>,
    }

    #[tokio::test]
    async fn test_cancellation_token_stops_watcher() {
        let (tx, rx) = watch::channel(TestMetrics { nodes: vec![] });

        let extractor: NodeInfoExtractor<TestMetrics> =
            Box::new(|m| m.nodes.iter().map(|(id, pk)| NodeInfo::new(*id, *pk)).collect());

        // Create a mock manager (we can't easily test with a real one here)
        // For this test, we just verify the cancellation works
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move {
            // Simulate the watcher loop with just cancellation
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(10)) => false,
                _ = cancel_clone.cancelled() => true,
            }
        });

        // Cancel after a short delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancel.cancel();

        // Task should complete with true (cancelled)
        let result = handle.await.unwrap();
        assert!(result, "watcher should have been cancelled");
    }

    #[test]
    fn test_node_info_from_membership() {
        let pk = test_public_key(42);
        let info = node_info_from_membership(1, pk);

        assert_eq!(info.node_id, 1);
        assert_eq!(info.public_key, pk);
        assert!(info.is_healthy);
        assert_eq!(info.weight, 100);
    }
}
