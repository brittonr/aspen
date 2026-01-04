//! Membership watcher for TrustedPeersRegistry synchronization.
//!
//! Watches Raft metrics for membership changes and updates the TrustedPeersRegistry
//! to enable Iroh-native authentication. When cluster membership changes (nodes added,
//! removed, or promoted), the TrustedPeersRegistry is updated so that the
//! AuthenticatedRaftProtocolHandler can accept/reject connections based on current
//! cluster membership.
//!
//! # Architecture
//!
//! ```text
//! Raft Metrics Watch Channel
//!          |
//!          v
//!  membership_watcher task
//!          |
//!          v
//!  TrustedPeersRegistry.set_peers()
//!          |
//!          v
//!  AuthenticatedRaftProtocolHandler.accept() uses updated registry
//! ```
//!
//! # Tiger Style
//!
//! - Single background task per node (bounded resource)
//! - Explicit shutdown via CancellationToken
//! - Registry bounded by MAX_PEERS constant
//! - Reactive updates (no polling) via OpenRaft's watch channel

use std::sync::Arc;

use aspen_transport::TrustedPeersRegistry;
use iroh::PublicKey;
use openraft::Raft;
use openraft::async_runtime::watch::WatchReceiver;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::types::AppTypeConfig;

/// Spawn a background task that watches Raft membership and updates TrustedPeersRegistry.
///
/// The watcher monitors the Raft metrics channel for membership changes. When membership
/// changes (via init, add_learner, or change_membership), it extracts all PublicKeys from
/// the RaftMemberInfo and updates the TrustedPeersRegistry.
///
/// # Arguments
///
/// * `raft` - The Raft instance to watch for membership changes
/// * `trusted_peers` - The registry to update when membership changes
///
/// # Returns
///
/// A CancellationToken that can be used to stop the watcher gracefully.
///
/// # Example
///
/// ```ignore
/// let cancel = spawn_membership_watcher(raft.clone(), trusted_peers.clone());
///
/// // Later, when shutting down:
/// cancel.cancel();
/// ```
pub fn spawn_membership_watcher(
    raft: Arc<Raft<AppTypeConfig>>,
    trusted_peers: TrustedPeersRegistry,
) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        membership_watcher_task(raft, trusted_peers, cancel_clone).await;
    });

    cancel
}

/// Background task that watches Raft membership and updates TrustedPeersRegistry.
async fn membership_watcher_task(
    raft: Arc<Raft<AppTypeConfig>>,
    trusted_peers: TrustedPeersRegistry,
    cancel: CancellationToken,
) {
    // Get the metrics watch receiver
    let mut rx = raft.metrics();

    info!("membership watcher started");

    // Initial sync - populate with current membership
    // Extract PublicKeys while holding the borrow, then release before await
    let initial_keys = extract_public_keys_from_metrics(&rx.borrow_watched());
    sync_public_keys_to_registry(initial_keys, &trusted_peers).await;

    loop {
        tokio::select! {
            result = rx.changed() => {
                match result {
                    Ok(()) => {
                        // Extract PublicKeys while holding the borrow, then release before await
                        let keys = extract_public_keys_from_metrics(&rx.borrow_watched());
                        sync_public_keys_to_registry(keys, &trusted_peers).await;
                    }
                    Err(_) => {
                        // Sender dropped - Raft is shutting down
                        info!("membership watcher exiting: Raft metrics channel closed");
                        break;
                    }
                }
            }
            _ = cancel.cancelled() => {
                info!("membership watcher cancelled");
                break;
            }
        }
    }
}

/// Extract all PublicKeys from Raft metrics (synchronous, no await).
///
/// This function extracts the Iroh PublicKey from each RaftMemberInfo in the
/// current membership configuration (both voters and learners).
///
/// Called while holding the watch borrow, so must be synchronous.
fn extract_public_keys_from_metrics(metrics: &openraft::RaftMetrics<AppTypeConfig>) -> Vec<PublicKey> {
    let membership = metrics.membership_config.membership();

    // Collect all PublicKeys from all nodes (voters + learners)
    membership
        .nodes()
        .map(|(node_id, member_info)| {
            debug!(
                node_id = %node_id,
                public_key = %member_info.iroh_addr.id,
                "extracting PublicKey from membership"
            );
            member_info.iroh_addr.id
        })
        .collect()
}

/// Update the TrustedPeersRegistry with a new set of PublicKeys.
///
/// This is the async part of the sync operation, called after releasing
/// the metrics borrow.
async fn sync_public_keys_to_registry(public_keys: Vec<PublicKey>, trusted_peers: &TrustedPeersRegistry) {
    let count = public_keys.len();

    if count == 0 {
        warn!("membership is empty - trusted peers registry will be cleared");
    }

    // Update the registry with the new set
    trusted_peers.set_peers(public_keys).await;

    debug!(peer_count = count, "synchronized trusted peers registry with Raft membership");
}

#[cfg(test)]
mod tests {
    use iroh::SecretKey;

    use super::*;

    /// Generate a test PublicKey from a seed.
    fn test_public_key(seed: u8) -> PublicKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = seed;
        let secret = SecretKey::from_bytes(&key_bytes);
        secret.public()
    }

    #[tokio::test]
    async fn test_trusted_peers_registry_set_peers() {
        let registry = TrustedPeersRegistry::new();

        let pk1 = test_public_key(1);
        let pk2 = test_public_key(2);
        let pk3 = test_public_key(3);

        // Set initial peers
        registry.set_peers([pk1, pk2]).await;
        assert_eq!(registry.peer_count().await, 2);
        assert!(registry.is_trusted(&pk1).await);
        assert!(registry.is_trusted(&pk2).await);
        assert!(!registry.is_trusted(&pk3).await);

        // Replace with new set
        registry.set_peers([pk2, pk3]).await;
        assert_eq!(registry.peer_count().await, 2);
        assert!(!registry.is_trusted(&pk1).await);
        assert!(registry.is_trusted(&pk2).await);
        assert!(registry.is_trusted(&pk3).await);
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        // Spawn a task that waits for cancellation
        let handle = tokio::spawn(async move {
            cancel_clone.cancelled().await;
            true
        });

        // Cancel the token
        cancel.cancel();

        // Task should complete
        let result = handle.await.unwrap();
        assert!(result);
    }
}
