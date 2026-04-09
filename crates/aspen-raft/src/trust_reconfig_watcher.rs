//! Trust reconfiguration watcher.
//!
//! Watches Raft leadership changes and restarts trust rotation when a node
//! becomes leader while the committed membership is ahead of the stored trust epoch.

use std::sync::Arc;

use openraft::async_runtime::watch::WatchReceiver;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::node::RaftNode;

/// Spawn a background watcher that restarts pending trust reconfiguration on leadership gain.
pub fn spawn_trust_reconfig_watcher(node: Arc<RaftNode>) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        trust_reconfig_watcher_task(node, cancel_clone).await;
    });

    cancel
}

async fn trust_reconfig_watcher_task(node: Arc<RaftNode>, cancel: CancellationToken) {
    let mut rx = node.raft().metrics();
    let mut was_leader = false;
    let mut last_attempted_epoch = 0u64;

    loop {
        tokio::select! {
            result = rx.changed() => {
                match result {
                    Ok(()) => {
                        let metrics = rx.borrow_watched().clone();
                        let is_leader = metrics.current_leader == Some(node.node_id());
                        let became_leader = is_leader && !was_leader;
                        was_leader = is_leader;

                        if !is_leader {
                            continue;
                        }

                        let pending = match node.pending_trust_reconfiguration(&metrics) {
                            Ok(value) => value,
                            Err(error) => {
                                warn!(error, "failed to inspect pending trust reconfiguration state");
                                continue;
                            }
                        };

                        let Some((old_members, new_members, target_epoch)) = pending else {
                            continue;
                        };

                        if !became_leader && target_epoch <= last_attempted_epoch {
                            continue;
                        }

                        last_attempted_epoch = target_epoch;
                        info!(epoch = target_epoch, "leader detected pending trust reconfiguration; restarting");
                        if let Err(error) = node
                            .rotate_trust_after_membership_change(old_members, new_members, target_epoch)
                            .await
                        {
                            warn!(epoch = target_epoch, error, "failed to restart pending trust reconfiguration");
                        }
                    }
                    Err(_) => {
                        info!("trust reconfiguration watcher exiting: Raft metrics channel closed");
                        break;
                    }
                }
            }
            _ = cancel.cancelled() => {
                info!("trust reconfiguration watcher cancelled");
                break;
            }
        }
    }
}
