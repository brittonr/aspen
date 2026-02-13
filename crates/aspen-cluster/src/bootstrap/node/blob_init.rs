//! Blob storage and replication initialization for cluster nodes.
//!
//! This module handles IrohBlobStore creation, blob replication manager setup,
//! and auto-announcement of blobs to DHT.

use std::sync::Arc;

#[cfg(feature = "blob")]
use aspen_blob::BlobEventBroadcaster;
#[cfg(feature = "blob")]
use aspen_blob::IrohBlobStore;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::bootstrap::resources::BlobReplicationResources;
use crate::IrohEndpointManager;
use crate::config::NodeConfig;

/// Initialize blob store if enabled.
#[cfg(feature = "blob")]
pub(super) async fn initialize_blob_store(
    config: &NodeConfig,
    data_dir: &std::path::Path,
    iroh_manager: &Arc<IrohEndpointManager>,
    broadcaster: Option<BlobEventBroadcaster>,
) -> Option<Arc<IrohBlobStore>> {
    if !config.blobs.enabled {
        info!(node_id = config.node_id, "blob store disabled by configuration");
        return None;
    }

    let blobs_dir = data_dir.join("blobs");
    if let Err(e) = std::fs::create_dir_all(&blobs_dir) {
        warn!(
            error = ?e,
            node_id = config.node_id,
            path = %blobs_dir.display(),
            "failed to create blobs directory, continuing without blob store"
        );
        return None;
    }

    match IrohBlobStore::new(&blobs_dir, iroh_manager.endpoint().clone()).await {
        Ok(store) => {
            let store = match broadcaster {
                Some(b) => store.with_broadcaster(b),
                None => store,
            };
            info!(
                node_id = config.node_id,
                path = %blobs_dir.display(),
                "blob store initialized"
            );
            Some(Arc::new(store))
        }
        Err(err) => {
            warn!(
                error = ?err,
                node_id = config.node_id,
                "failed to initialize blob store, continuing without it"
            );
            None
        }
    }
}

/// Initialize blob replication manager if replication is enabled.
///
/// Creates the `BlobReplicationManager` which coordinates blob replication
/// across cluster nodes. The manager is only created if:
/// - Blobs are enabled (`config.blobs.enabled`)
/// - Replication factor > 1 or auto-replication is enabled
///
/// # Arguments
/// * `config` - Node configuration with blob settings
/// * `blob_store` - The blob store for P2P transfers
/// * `kv_store` - The KV store for replica metadata (Raft-backed)
/// * `blob_events` - Receiver for blob events (from broadcaster)
/// * `shutdown` - Cancellation token for graceful shutdown
///
/// # Returns
/// `BlobReplicationResources` containing the manager and shutdown handles,
/// or disabled resources if replication is not configured.
#[cfg(feature = "blob")]
pub async fn initialize_blob_replication<KV>(
    config: &NodeConfig,
    blob_store: Option<Arc<IrohBlobStore>>,
    endpoint: Option<iroh::Endpoint>,
    kv_store: Arc<KV>,
    blob_events: Option<tokio::sync::broadcast::Receiver<aspen_blob::BlobEvent>>,
    shutdown: CancellationToken,
) -> BlobReplicationResources
where
    KV: aspen_core::traits::KeyValueStore + Send + Sync + 'static,
{
    // Check if blob replication should be enabled
    let Some(blob_store) = blob_store else {
        info!(node_id = config.node_id, "blob replication disabled: no blob store");
        return BlobReplicationResources::disabled();
    };

    let Some(endpoint) = endpoint else {
        info!(node_id = config.node_id, "blob replication disabled: no endpoint for RPC");
        return BlobReplicationResources::disabled();
    };

    let Some(blob_events) = blob_events else {
        info!(node_id = config.node_id, "blob replication disabled: no event broadcaster");
        return BlobReplicationResources::disabled();
    };

    // Only enable replication if:
    // - replication_factor > 1 (need multiple replicas), OR
    // - auto_replication is enabled (even with factor=1, we track replicas)
    if config.blobs.replication_factor <= 1 && !config.blobs.enable_auto_replication {
        info!(
            node_id = config.node_id,
            replication_factor = config.blobs.replication_factor,
            "blob replication disabled: replication_factor=1 and auto_replication=false"
        );
        return BlobReplicationResources::disabled();
    }

    // Build replication configuration from BlobConfig
    let replication_config = aspen_blob::ReplicationConfig {
        default_policy: aspen_blob::ReplicationPolicy {
            replication_factor: config.blobs.replication_factor.min(aspen_blob::MAX_REPLICATION_FACTOR),
            min_replicas: config.blobs.min_replicas.min(config.blobs.replication_factor),
            failure_domain_key: config.blobs.failure_domain_key.clone(),
            enable_quorum_writes: config.blobs.enable_quorum_writes,
        },
        node_id: config.node_id,
        auto_replicate: config.blobs.enable_auto_replication,
        repair_interval_secs: config.blobs.repair_interval_secs,
        repair_delay_secs: config.blobs.repair_delay_secs,
        max_concurrent: aspen_blob::MAX_CONCURRENT_REPLICATIONS,
    };

    // Create the trait adapters
    let metadata_store = Arc::new(aspen_blob::KvReplicaMetadataStore::new(kv_store));
    let blob_transfer = Arc::new(aspen_blob::IrohBlobTransfer::new(blob_store, endpoint));
    let placement = Arc::new(aspen_blob::WeightedPlacement);

    // Create child cancellation token for replication manager
    let replication_cancel = shutdown.child_token();

    // Spawn the replication manager
    match aspen_blob::BlobReplicationManager::spawn(
        replication_config.clone(),
        blob_events,
        metadata_store,
        blob_transfer,
        placement,
        replication_cancel.clone(),
    )
    .await
    {
        Ok((manager, task)) => {
            info!(
                node_id = config.node_id,
                replication_factor = replication_config.default_policy.replication_factor,
                min_replicas = replication_config.default_policy.min_replicas,
                auto_replicate = replication_config.auto_replicate,
                "blob replication manager started"
            );
            BlobReplicationResources {
                replication_manager: Some(manager),
                replication_cancel: Some(replication_cancel),
                replication_task: Some(task),
                topology_cancel: None, // Set later via wire_topology_watcher()
            }
        }
        Err(err) => {
            warn!(
                error = ?err,
                node_id = config.node_id,
                "failed to start blob replication manager, continuing without replication"
            );
            BlobReplicationResources::disabled()
        }
    }
}

/// Spawn background task for auto-announcing blobs to DHT.
///
/// This task announces local blobs to the DHT.
/// Runs in background and doesn't block node startup.
///
/// Note: We announce immediately rather than waiting for DHT bootstrap.
/// The DHT will learn about our blobs as soon as it's ready, and early
/// announcements are harmless (they just won't be routed until bootstrap).
#[cfg(feature = "blob")]
pub(super) fn spawn_blob_announcer(
    config: &NodeConfig,
    blob_store: &Option<Arc<IrohBlobStore>>,
    content_discovery: &Option<crate::content_discovery::ContentDiscoveryService>,
) {
    if content_discovery.is_none() || blob_store.is_none() {
        return;
    }

    let config_clone = config.clone();
    let blob_store_clone = blob_store.clone();
    let content_discovery_clone = content_discovery.clone();
    tokio::spawn(async move {
        // Announce immediately - no need to wait for DHT bootstrap.
        // Early announcements are harmless and get propagated once DHT is ready.
        auto_announce_local_blobs(&config_clone, blob_store_clone.as_ref(), content_discovery_clone.as_ref()).await;
    });
}

/// Perform auto-announce of local blobs if enabled.
///
/// This function is called after the node is fully initialized.
/// It scans the local blob store and announces all blobs to the DHT.
///
/// # Arguments
/// * `config` - Node configuration
/// * `blob_store` - Optional blob store reference
/// * `content_discovery` - Optional content discovery service
#[cfg(feature = "blob")]
pub async fn auto_announce_local_blobs(
    config: &NodeConfig,
    blob_store: Option<&Arc<IrohBlobStore>>,
    content_discovery: Option<&crate::content_discovery::ContentDiscoveryService>,
) {
    use aspen_blob::BlobStore;

    // Check if auto-announce is enabled
    if !config.content_discovery.auto_announce {
        return;
    }

    let Some(blob_store) = blob_store else {
        warn!(node_id = config.node_id, "auto_announce enabled but blob store not available");
        return;
    };

    let Some(discovery) = content_discovery else {
        warn!(node_id = config.node_id, "auto_announce enabled but content discovery service not available");
        return;
    };

    info!(node_id = config.node_id, "scanning local blobs for auto-announce");

    // List all local blobs
    match blob_store.list(10_000, None).await {
        Ok(result) => {
            if result.blobs.is_empty() {
                info!(node_id = config.node_id, "no local blobs to announce");
                return;
            }

            let blobs: Vec<_> = result.blobs.iter().map(|e| (e.hash, e.size, e.format)).collect();
            let count = blobs.len();

            info!(node_id = config.node_id, blob_count = count, "announcing local blobs to DHT");

            match discovery.announce_local_blobs(blobs).await {
                Ok(announced) => {
                    info!(node_id = config.node_id, announced, total = count, "auto-announce complete");
                }
                Err(err) => {
                    warn!(
                        node_id = config.node_id,
                        error = %err,
                        "failed to auto-announce local blobs"
                    );
                }
            }
        }
        Err(err) => {
            warn!(
                node_id = config.node_id,
                error = %err,
                "failed to list local blobs for auto-announce"
            );
        }
    }
}
