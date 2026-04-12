//! Startup probe for peer-enforced expungement.
//!
//! Nodes that were removed while offline can restart with stale local trust
//! metadata. Startup must not continue until the node either confirms that
//! enough peers still treat it as a current member or persists an expungement
//! marker returned by a peer.

use std::sync::Arc;
use std::time::Duration;

use tracing::info;
use tracing::warn;

use crate::node::PeerExpungementProbeOutcome;
use crate::node::RaftNode;

const STARTUP_PROBE_INTERVAL: Duration = Duration::from_secs(5);
const STARTUP_PROBE_ATTEMPTS: u32 = 12;

/// Block startup until peer expungement probing either clears the node or
/// persists expungement.
pub async fn ensure_startup_trust_peer_probe(node: Arc<RaftNode>) -> Result<(), String> {
    for attempt in 1..=STARTUP_PROBE_ATTEMPTS {
        match node.probe_for_peer_expungement().await {
            Ok(PeerExpungementProbeOutcome::NotApplicable) => return Ok(()),
            Ok(PeerExpungementProbeOutcome::Healthy) => {
                info!(node_id = node.node_id().0, attempt, "startup trust peer probe cleared node");
                return Ok(());
            }
            Ok(PeerExpungementProbeOutcome::Expunged { epoch }) => {
                return Err(format!(
                    "startup trust peer probe marked node {} as expunged at epoch {epoch}",
                    node.node_id().0
                ));
            }
            Ok(PeerExpungementProbeOutcome::RetryNeeded) => {
                if attempt == STARTUP_PROBE_ATTEMPTS {
                    return Err(format!(
                        "startup trust peer probe could not clear node {} after {} attempts",
                        node.node_id().0,
                        STARTUP_PROBE_ATTEMPTS
                    ));
                }
                warn!(
                    node_id = node.node_id().0,
                    attempt,
                    attempts = STARTUP_PROBE_ATTEMPTS,
                    "startup trust peer probe needs another attempt"
                );
            }
            Err(error) => {
                if attempt == STARTUP_PROBE_ATTEMPTS {
                    return Err(format!(
                        "startup trust peer probe failed for node {} after {} attempts: {error}",
                        node.node_id().0,
                        STARTUP_PROBE_ATTEMPTS
                    ));
                }
                warn!(node_id = node.node_id().0, attempt, error, "startup trust peer probe attempt failed");
            }
        }

        tokio::time::sleep(STARTUP_PROBE_INTERVAL).await;
    }

    Err(format!("startup trust peer probe exhausted attempts for node {}", node.node_id().0))
}
