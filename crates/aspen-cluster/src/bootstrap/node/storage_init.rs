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
///
/// Applies per-node election timeout jitter based on node_id to prevent
/// split-votes when multiple nodes start elections simultaneously. openraft
/// randomizes the timeout once at engine creation and never re-randomizes,
/// so without jitter, nodes with similar startup timing persistently split
/// votes (especially in VMs with synchronized scheduling).
pub(super) fn create_raft_config_and_broadcast(config: &NodeConfig) -> (Arc<RaftConfig>, StorageBroadcasts) {
    // Derive deterministic per-node jitter from node_id using Knuth's
    // multiplicative hash. Plain `node_id % range` gives tiny jitter for
    // small IDs (1, 2, 3 → 1ms, 2ms, 3ms). The multiplicative hash spreads
    // small IDs across the full jitter range.
    // With default 1500-3000ms range, jitter adds 0-500ms based on node_id.
    let timeout_range = config.election_timeout_max_ms.saturating_sub(config.election_timeout_min_ms);
    let jitter = if timeout_range > 0 {
        let jitter_range = timeout_range / 3;
        if jitter_range > 0 {
            // Knuth multiplicative hash: spread node_id across jitter range.
            // 2654435761 is the golden ratio × 2^32, truncated.
            let hash = config.node_id.wrapping_mul(2_654_435_761);
            hash % jitter_range
        } else {
            0
        }
    } else {
        0
    };

    let election_min = config.election_timeout_min_ms.saturating_add(jitter);
    let election_max = config.election_timeout_max_ms.saturating_add(jitter);

    if jitter > 0 {
        info!(
            node_id = config.node_id,
            jitter_ms = jitter,
            election_min,
            election_max,
            "applied per-node election timeout jitter to prevent split-votes"
        );
    }

    let raft_config = Arc::new(RaftConfig {
        cluster_name: config.cookie.clone(),
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: election_min,
        election_timeout_max: election_max,
        replication_lag_threshold: 10000,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(10_000),
        max_in_snapshot_log_to_keep: 1_000,
        enable_tick: true,
        install_snapshot_timeout: aspen_raft::constants::SNAPSHOT_INSTALL_TIMEOUT_MS,
        ..RaftConfig::default()
    });

    let (log_broadcast, snapshot_broadcast) = if config.hooks.is_enabled || config.docs.is_enabled {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE as usize);
        let (snapshot_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE as usize);
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

async fn create_in_memory_raft_instance(
    config: &NodeConfig,
    raft_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Result<(Arc<Raft<AppTypeConfig>>, StateMachineVariant, Option<CancellationToken>)> {
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

fn open_shared_redb_storage(
    config: &NodeConfig,
    data_dir: &std::path::Path,
    broadcasts: &StorageBroadcasts,
) -> Result<(std::path::PathBuf, Arc<SharedRedbStorage>)> {
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

    Ok((db_path, shared_storage))
}

#[cfg(feature = "trust")]
fn ensure_storage_not_expunged(storage: &SharedRedbStorage, db_path: &std::path::Path) -> Result<()> {
    if !storage.is_expunged().map_err(|e| anyhow::anyhow!("failed to check expungement: {e}"))? {
        return Ok(());
    }

    let metadata = storage
        .load_expunged()
        .map_err(|e| anyhow::anyhow!("failed to load expungement metadata: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("is_expunged returned true but load_expunged returned None"))?;
    Err(anyhow::anyhow!(
        "THIS NODE HAS BEEN PERMANENTLY EXPUNGED from the cluster at epoch {}. \
         Removed by node {}. To rejoin, wipe the data directory ({}) and restart.",
        metadata.epoch,
        metadata.removed_by,
        db_path.parent().unwrap_or(db_path).display()
    ))
}

#[cfg(not(feature = "trust"))]
fn ensure_storage_not_expunged(_storage: &SharedRedbStorage, _db_path: &std::path::Path) -> Result<()> {
    Ok(())
}

async fn create_redb_raft_instance(
    config: &NodeConfig,
    raft_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
    data_dir: &std::path::Path,
    broadcasts: &StorageBroadcasts,
) -> Result<(Arc<Raft<AppTypeConfig>>, StateMachineVariant, Option<CancellationToken>)> {
    let (db_path, shared_storage) = open_shared_redb_storage(config, data_dir, broadcasts)?;
    ensure_storage_not_expunged(shared_storage.as_ref(), &db_path)?;

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
        StorageBackend::InMemory => create_in_memory_raft_instance(config, raft_config, network_factory).await,
        StorageBackend::Redb => {
            create_redb_raft_instance(config, raft_config, network_factory, data_dir, broadcasts).await
        }
    }
}

#[cfg(test)]
mod tests {
    use aspen_raft_types::MIN_SNAPSHOT_LOG_THRESHOLD;

    /// Regression: LogsSinceLast(100) triggered a snapshot race that panicked
    /// the Raft core (`snapshot.submitted > apply_progress.submitted`).
    /// The configured threshold in this module must respect the safety floor.
    #[test]
    fn test_snapshot_threshold_respects_safety_floor() {
        // This is the value used in create_raft_config_and_broadcast() above.
        // If you change LogsSinceLast(N), update this constant AND ensure N >= MIN.
        let configured_threshold: u64 = 10_000;
        assert!(
            configured_threshold >= MIN_SNAPSHOT_LOG_THRESHOLD,
            "snapshot threshold {configured_threshold} is below safety floor \
             {MIN_SNAPSHOT_LOG_THRESHOLD}; see napkin 2026-02-26 snapshot race"
        );
    }
}
