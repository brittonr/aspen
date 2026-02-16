//! Storage initialization for cluster nodes.
//!
//! This module handles the creation of Raft storage backends and
//! broadcast channels for log and snapshot events.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_raft::StateMachineVariant;
use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
use aspen_raft::log_subscriber::LogEntryPayload;
use aspen_raft::storage::InMemoryLogStore;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::storage::StorageBackend;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::ttl_cleanup::TtlCleanupConfig;
use aspen_raft::ttl_cleanup::spawn_redb_ttl_cleanup_task;
use aspen_raft::types::AppTypeConfig;
use openraft::Config as RaftConfig;
use openraft::Raft;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::IrpcRaftNetworkFactory;
use crate::config::NodeConfig;

/// Broadcast channels for storage events.
pub(super) struct StorageBroadcasts {
    /// Log entry broadcast for hooks and docs export.
    pub log: Option<broadcast::Sender<LogEntryPayload>>,
    /// Snapshot event broadcast for hooks.
    pub snapshot: Option<broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>>,
}

/// Create Raft configuration and broadcast channels.
///
/// Returns (raft_config, broadcasts).
pub(super) fn create_raft_config_and_broadcast(config: &NodeConfig) -> (Arc<RaftConfig>, StorageBroadcasts) {
    let raft_config = Arc::new(RaftConfig {
        cluster_name: config.cookie.clone(),
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: config.election_timeout_min_ms,
        election_timeout_max: config.election_timeout_max_ms,
        replication_lag_threshold: 10000,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(100),
        max_in_snapshot_log_to_keep: 100,
        enable_tick: true,
        install_snapshot_timeout: aspen_raft::constants::SNAPSHOT_INSTALL_TIMEOUT_MS,
        ..RaftConfig::default()
    });

    let (log_broadcast, snapshot_broadcast) = if config.hooks.is_enabled || config.docs.is_enabled {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let (snapshot_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        info!(
            node_id = config.node_id,
            buffer_size = LOG_BROADCAST_BUFFER_SIZE,
            hooks_enabled = config.hooks.is_enabled,
            docs_enabled = config.docs.is_enabled,
            "created broadcast channels for hooks/docs"
        );
        (Some(log_sender), Some(snapshot_sender))
    } else {
        (None, None)
    };

    (raft_config, StorageBroadcasts {
        log: log_broadcast,
        snapshot: snapshot_broadcast,
    })
}

/// Create Raft instance with appropriate storage backend.
///
/// Returns (raft, state_machine_variant, ttl_cleanup_cancel).
pub(super) async fn create_raft_instance(
    config: &NodeConfig,
    raft_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
    data_dir: &std::path::Path,
    broadcasts: &StorageBroadcasts,
) -> Result<(Arc<Raft<AppTypeConfig>>, StateMachineVariant, Option<CancellationToken>)> {
    match config.storage_backend {
        StorageBackend::InMemory => {
            let log_store = Arc::new(InMemoryLogStore::default());
            let state_machine = InMemoryStateMachine::new();
            let raft = Arc::new(
                Raft::new(
                    config.node_id.into(),
                    raft_config,
                    network_factory.as_ref().clone(),
                    log_store.as_ref().clone(),
                    state_machine.clone(),
                )
                .await
                .context("failed to create in-memory Raft instance")?,
            );
            Ok((raft, StateMachineVariant::InMemory(state_machine), None))
        }
        StorageBackend::Redb => {
            let db_path = data_dir.join(format!("node_{}_shared.redb", config.node_id));
            let shared_storage = Arc::new(
                SharedRedbStorage::with_broadcasts(
                    &db_path,
                    broadcasts.log.clone(),
                    broadcasts.snapshot.clone(),
                    &config.node_id.to_string(),
                )
                .map_err(|e| anyhow::anyhow!("failed to open shared redb storage: {}", e))?,
            );

            info!(
                node_id = config.node_id,
                path = %db_path.display(),
                "created shared redb storage (single-fsync mode)"
            );

            let raft = Arc::new(
                Raft::new(
                    config.node_id.into(),
                    raft_config,
                    network_factory.as_ref().clone(),
                    shared_storage.as_ref().clone(),
                    shared_storage.as_ref().clone(),
                )
                .await
                .context("failed to create Redb-backed Raft instance")?,
            );

            let ttl_cancel = spawn_redb_ttl_cleanup_task(shared_storage.clone(), TtlCleanupConfig::default());
            info!(node_id = config.node_id, "Redb TTL cleanup task started");
            Ok((raft, StateMachineVariant::Redb(shared_storage), Some(ttl_cancel)))
        }
    }
}
