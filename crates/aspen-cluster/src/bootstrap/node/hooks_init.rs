//! Hook service initialization for cluster nodes.
//!
//! This module handles the HookService setup and event bridge spawning.

use std::sync::Arc;

use anyhow::Result;
use aspen_raft::StateMachineVariant;
use aspen_raft::log_subscriber::LogEntryPayload;
use aspen_raft::node::RaftNode;
use aspen_raft::ttl_cleanup::TtlCleanupConfig;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;

use crate::bootstrap::resources::HookResources;
use crate::config::NodeConfig;

/// Initialize hook service if enabled.
///
/// Creates the HookService from configuration and spawns the event bridge tasks
/// that subscribe to the broadcast channels. Returns HookResources for
/// inclusion in NodeHandle.
///
/// The event bridges convert events into HookEvents and dispatch them to registered
/// handlers. Handlers can be in-process closures, shell commands, or cross-cluster
/// forwarding (ForwardHandler not yet implemented).
///
/// Bridge types:
/// - Raft log bridge: Converts committed log entries into hook events
/// - Blob bridge: Converts blob store events (add, download, protect, etc.)
/// - Docs bridge: Converts docs sync events (sync started/completed, import/export)
/// - System events bridge: Monitors Raft metrics for LeaderElected and HealthChanged events
/// - Snapshot events bridge: Monitors snapshot creation and installation events
pub(super) async fn initialize_hook_service(
    config: &NodeConfig,
    log_broadcast: Option<&broadcast::Sender<LogEntryPayload>>,
    snapshot_broadcast: Option<&broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>>,
    blob_broadcast: Option<&broadcast::Sender<aspen_blob::BlobEvent>>,
    docs_broadcast: Option<&broadcast::Sender<aspen_docs::DocsEvent>>,
    raft_node: &Arc<RaftNode>,
    state_machine: &StateMachineVariant,
) -> Result<HookResources> {
    if !config.hooks.enabled {
        info!(node_id = config.node_id, "hook service disabled by configuration");
        return Ok(HookResources::disabled());
    }

    // Create hook service from configuration
    let hook_service = Arc::new(aspen_hooks::HookService::new(config.hooks.clone()));

    // Spawn raft log event bridge if log broadcast is available
    let event_bridge_cancel = if let Some(sender) = log_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::hooks_bridge::run_event_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "raft log event bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "raft log event bridge not started (log broadcast unavailable)");
        None
    };

    // Spawn blob event bridge if blob broadcast is available
    let blob_bridge_cancel = if let Some(sender) = blob_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::blob_bridge::run_blob_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "blob event bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "blob event bridge not started (blob broadcast unavailable)");
        None
    };

    // Spawn docs event bridge if docs broadcast is available
    let docs_bridge_cancel = if let Some(sender) = docs_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::docs_bridge::run_docs_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "docs event bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "docs event bridge not started (docs broadcast unavailable)");
        None
    };

    // Spawn system events bridge for LeaderElected and HealthChanged events
    let system_events_bridge_cancel = {
        let cancel = CancellationToken::new();
        let raft_node_clone = Arc::clone(raft_node);
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();
        let bridge_config = crate::system_events_bridge::SystemEventsBridgeConfig::default();

        tokio::spawn(async move {
            crate::system_events_bridge::run_system_events_bridge(
                raft_node_clone,
                service,
                node_id,
                bridge_config,
                cancel_clone,
            )
            .await;
        });

        info!(node_id = config.node_id, "system events bridge started");
        Some(cancel)
    };

    // Spawn snapshot events bridge if snapshot broadcast is available
    let snapshot_events_bridge_cancel = if let Some(sender) = snapshot_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::snapshot_events_bridge::run_snapshot_events_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "snapshot events bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "snapshot events bridge not started (snapshot broadcast unavailable)");
        None
    };

    // Spawn TTL events bridge if using Redb storage
    let ttl_events_bridge_cancel = if let StateMachineVariant::Redb(storage) = state_machine {
        let ttl_config = TtlCleanupConfig::default();
        let cancel = crate::ttl_events_bridge::spawn_ttl_events_bridge(
            Arc::clone(storage),
            ttl_config,
            Arc::clone(&hook_service),
            config.node_id,
        );
        info!(node_id = config.node_id, "TTL events bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "TTL events bridge not started (not using Redb storage)");
        None
    };

    info!(
        node_id = config.node_id,
        handler_count = config.hooks.handlers.len(),
        has_log_bridge = event_bridge_cancel.is_some(),
        has_blob_bridge = blob_bridge_cancel.is_some(),
        has_docs_bridge = docs_bridge_cancel.is_some(),
        has_system_bridge = system_events_bridge_cancel.is_some(),
        has_snapshot_bridge = snapshot_events_bridge_cancel.is_some(),
        has_ttl_bridge = ttl_events_bridge_cancel.is_some(),
        "hook service started"
    );

    Ok(HookResources {
        hook_service: Some(hook_service),
        event_bridge_cancel,
        blob_bridge_cancel,
        docs_bridge_cancel,
        system_events_bridge_cancel,
        ttl_events_bridge_cancel,
        snapshot_events_bridge_cancel,
    })
}
