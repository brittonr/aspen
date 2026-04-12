//! Startup probe for peer-enforced expungement.
//!
//! Nodes that were removed while offline can restart with stale local trust
//! metadata. This task asks a current peer for the local node's current-epoch
//! share. Healthy members receive a share; removed members receive
//! `TrustResponse::Expunged`, which permanently marks the node as expunged.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::node::PeerExpungementProbeOutcome;
use crate::node::RaftNode;

const STARTUP_PROBE_DELAY: Duration = Duration::from_secs(3);
const STARTUP_PROBE_INTERVAL: Duration = Duration::from_secs(5);
const STARTUP_PROBE_ATTEMPTS: u32 = 12;

/// Spawn a bounded background task that probes peers for expungement.
pub fn spawn_trust_peer_probe(node: Arc<RaftNode>) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        trust_peer_probe_task(node, cancel_clone).await;
    });

    cancel
}

async fn trust_peer_probe_task(node: Arc<RaftNode>, cancel: CancellationToken) {
    tokio::select! {
        _ = cancel.cancelled() => {
            info!(node_id = node.node_id().0, "trust peer probe cancelled before first attempt");
            return;
        }
        _ = tokio::time::sleep(STARTUP_PROBE_DELAY) => {}
    }

    for attempt in 1..=STARTUP_PROBE_ATTEMPTS {
        match node.probe_for_peer_expungement().await {
            Ok(PeerExpungementProbeOutcome::NotApplicable) => return,
            Ok(PeerExpungementProbeOutcome::Healthy) => return,
            Ok(PeerExpungementProbeOutcome::Expunged { epoch }) => {
                info!(node_id = node.node_id().0, attempt, epoch, "peer probe marked node as expunged");
                return;
            }
            Ok(PeerExpungementProbeOutcome::RetryNeeded) => {
                if attempt == STARTUP_PROBE_ATTEMPTS {
                    warn!(
                        node_id = node.node_id().0,
                        attempts = STARTUP_PROBE_ATTEMPTS,
                        "peer expungement probe exhausted retries"
                    );
                    return;
                }
            }
            Err(error) => {
                if attempt == STARTUP_PROBE_ATTEMPTS {
                    warn!(
                        node_id = node.node_id().0,
                        attempts = STARTUP_PROBE_ATTEMPTS,
                        error,
                        "peer expungement probe failed"
                    );
                    return;
                }
                warn!(node_id = node.node_id().0, attempt, error, "peer expungement probe attempt failed");
            }
        }

        tokio::select! {
            _ = cancel.cancelled() => {
                info!(node_id = node.node_id().0, attempt, "trust peer probe cancelled");
                return;
            }
            _ = tokio::time::sleep(STARTUP_PROBE_INTERVAL) => {}
        }
    }
}
