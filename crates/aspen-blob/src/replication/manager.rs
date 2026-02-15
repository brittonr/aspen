//! Blob Replication Manager.
//!
//! Coordinates blob replication across cluster nodes with:
//!
//! - Event-driven replication triggers (subscribes to BlobEvent::Added)
//! - Quorum writes (optional blocking until min_replicas confirmed)
//! - Background repair for under-replicated blobs
//! - Rate-limited concurrent replication tasks
//!
//! ## Architecture
//!
//! ```text
//! BlobReplicationManager
//!     |
//!     +-> Event Subscription (BlobEvent::Added)
//!     |   - Triggers replication on new blobs
//!     |
//!     +-> ReplicaTracker (via KV store)
//!     |   - Reads/writes replica metadata
//!     |   - Stored in Raft state machine
//!     |
//!     +-> PlacementStrategy
//!     |   - Selects target nodes
//!     |   - Considers failure domains
//!     |
//!     +-> Replication Tasks
//!     |   - Transfers blobs to targets
//!     |   - Updates tracker on success
//!     |
//!     +-> Repair Loop
//!         - Scans for under-replicated blobs
//!         - Triggers replication to restore
//! ```

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use iroh::PublicKey;
use iroh_blobs::Hash;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::events::BlobEvent;
use crate::events::BlobEventType;
use crate::replication::MAX_CONCURRENT_REPLICATIONS;
use crate::replication::MAX_REPAIR_BATCH_SIZE;
use crate::replication::MIN_REPAIR_INTERVAL_SECS;
use crate::replication::ReplicaSet;
use crate::replication::ReplicationPolicy;
use crate::replication::ReplicationRequest;
use crate::replication::ReplicationResult;
use crate::replication::ReplicationStatus;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the replication manager.
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Default replication policy for new blobs.
    pub default_policy: ReplicationPolicy,

    /// Our node ID.
    pub node_id: u64,

    /// Enable automatic replication on blob addition.
    pub auto_replicate: bool,

    /// Interval for repair checks (seconds).
    pub repair_interval_secs: u64,

    /// Delay before repairing under-replicated blobs (seconds).
    pub repair_delay_secs: u64,

    /// Maximum concurrent replication tasks.
    pub max_concurrent: u32,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            default_policy: ReplicationPolicy::default(),
            node_id: 0,
            auto_replicate: false,
            repair_interval_secs: 60,
            repair_delay_secs: 300,
            max_concurrent: MAX_CONCURRENT_REPLICATIONS,
        }
    }
}

// ============================================================================
// Node Info (for placement)
// ============================================================================

/// Information about a cluster node for replica placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID.
    pub node_id: u64,

    /// Iroh public key for P2P connection.
    pub public_key: PublicKey,

    /// Whether the node is currently healthy.
    pub is_healthy: bool,

    /// Tags for failure domain spreading (e.g., {"rack": "rack1"}).
    pub tags: HashMap<String, String>,

    /// Available disk space in bytes (for capacity-based placement).
    pub available_bytes: Option<u64>,

    /// Weight for placement (higher = more likely to be selected).
    pub weight: u32,
}

impl NodeInfo {
    /// Create a new NodeInfo.
    pub fn new(node_id: u64, public_key: PublicKey) -> Self {
        Self {
            node_id,
            public_key,
            is_healthy: true,
            tags: HashMap::new(),
            available_bytes: None,
            weight: 100,
        }
    }

    /// Set a tag value.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Get a tag value.
    pub fn get_tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(|s| s.as_str())
    }
}

// ============================================================================
// Placement Strategy
// ============================================================================

/// Strategy for selecting replica placement targets.
pub trait PlacementStrategy: Send + Sync {
    /// Select target nodes for replication.
    ///
    /// # Arguments
    /// * `hash` - Blob hash (used as placement seed)
    /// * `count` - Number of targets to select
    /// * `exclude` - Nodes to exclude (already have replica)
    /// * `nodes` - Available nodes to choose from
    /// * `failure_domain_key` - Optional key for failure domain spreading
    ///
    /// # Returns
    /// List of selected node IDs, may be fewer than `count` if not enough candidates.
    fn select_targets(
        &self,
        hash: &Hash,
        count: u32,
        exclude: &HashSet<u64>,
        nodes: &[NodeInfo],
        failure_domain_key: Option<&str>,
    ) -> Vec<u64>;
}

/// Default placement strategy using weighted random selection.
pub struct WeightedPlacement;

impl PlacementStrategy for WeightedPlacement {
    fn select_targets(
        &self,
        hash: &Hash,
        count: u32,
        exclude: &HashSet<u64>,
        nodes: &[NodeInfo],
        failure_domain_key: Option<&str>,
    ) -> Vec<u64> {
        // Filter to healthy candidates not already excluded
        let mut candidates: Vec<_> = nodes.iter().filter(|n| n.is_healthy && !exclude.contains(&n.node_id)).collect();

        if candidates.is_empty() {
            return Vec::new();
        }

        // Use hash bytes as deterministic seed
        let seed_bytes = hash.as_bytes();
        let mut seed = u64::from_le_bytes([
            seed_bytes[0],
            seed_bytes[1],
            seed_bytes[2],
            seed_bytes[3],
            seed_bytes[4],
            seed_bytes[5],
            seed_bytes[6],
            seed_bytes[7],
        ]);

        let mut selected = Vec::with_capacity(count as usize);
        let mut used_domains: HashSet<String> = HashSet::new();

        for i in 0..count {
            if candidates.is_empty() {
                break;
            }

            // Filter by failure domain if configured
            let eligible: Vec<_> = if let Some(domain_key) = failure_domain_key {
                candidates
                    .iter()
                    .filter(|n| n.get_tag(domain_key).map(|v| !used_domains.contains(v)).unwrap_or(true))
                    .cloned()
                    .collect()
            } else {
                candidates.clone()
            };

            // Fall back to all candidates if no eligible after domain filtering
            let pool = if eligible.is_empty() { &candidates } else { &eligible };

            if pool.is_empty() {
                break;
            }

            // Weighted random selection using straw algorithm
            let total_weight: u64 = pool.iter().map(|n| n.weight as u64).sum();
            if total_weight == 0 {
                break;
            }

            // Simple PRNG based on seed
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407 + i as u64);
            let target_weight = seed % total_weight;

            let mut cumulative = 0u64;
            let mut chosen_idx = 0;
            for (idx, node) in pool.iter().enumerate() {
                cumulative += node.weight as u64;
                if cumulative > target_weight {
                    chosen_idx = idx;
                    break;
                }
            }

            let chosen = pool[chosen_idx];
            selected.push(chosen.node_id);

            // Track used failure domain
            if let Some(domain_key) = failure_domain_key
                && let Some(domain_value) = chosen.get_tag(domain_key)
            {
                used_domains.insert(domain_value.to_string());
            }

            // Remove from candidates
            candidates.retain(|n| n.node_id != chosen.node_id);
        }

        selected
    }
}

// ============================================================================
// Commands
// ============================================================================

/// Commands sent to the replication manager.
enum ReplicationCommand {
    /// Replicate a blob to achieve policy.
    Replicate {
        request: ReplicationRequest,
        reply: Option<tokio::sync::oneshot::Sender<Result<ReplicationResult>>>,
    },

    /// Trigger repair check for a specific blob.
    RepairBlob { hash: Hash },

    /// Run a full repair cycle.
    RunRepairCycle,

    /// Update cluster topology (nodes available for placement).
    UpdateTopology { nodes: Vec<NodeInfo> },

    /// Get current replica status for a blob.
    GetStatus {
        hash: Hash,
        reply: tokio::sync::oneshot::Sender<Option<ReplicaSet>>,
    },
}

// ============================================================================
// Replication Manager
// ============================================================================

/// Handle for interacting with the replication manager.
#[derive(Clone)]
pub struct BlobReplicationManager {
    command_tx: mpsc::Sender<ReplicationCommand>,
    config: ReplicationConfig,
}

impl BlobReplicationManager {
    /// Create a new replication manager.
    ///
    /// # Arguments
    /// * `config` - Manager configuration
    /// * `blob_events` - Receiver for blob events (from BlobEventBroadcaster)
    /// * `cancel` - Cancellation token for shutdown
    ///
    /// # Type Parameters
    /// * `KV` - KeyValueStore implementation for replica metadata
    /// * `BS` - BlobStore implementation for blob transfers
    pub async fn spawn<KV, BS>(
        config: ReplicationConfig,
        blob_events: broadcast::Receiver<BlobEvent>,
        kv_store: Arc<KV>,
        blob_store: Arc<BS>,
        placement: Arc<dyn PlacementStrategy>,
        cancel: CancellationToken,
    ) -> Result<(Self, JoinHandle<()>)>
    where
        KV: ReplicaMetadataStore + 'static,
        BS: ReplicaBlobTransfer + 'static,
    {
        let (command_tx, command_rx) = mpsc::channel(256);

        let manager = Self {
            command_tx,
            config: config.clone(),
        };

        let task = tokio::spawn(run_manager(config, command_rx, blob_events, kv_store, blob_store, placement, cancel));

        Ok((manager, task))
    }

    /// Request replication of a blob.
    ///
    /// Returns immediately if not waiting for quorum, or blocks until
    /// min_replicas are confirmed if wait_for_ack is set.
    pub async fn replicate(&self, request: ReplicationRequest) -> Result<ReplicationResult> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(ReplicationCommand::Replicate {
                request,
                reply: Some(reply_tx),
            })
            .await
            .context("replication manager shut down")?;

        reply_rx.await.context("reply channel closed")?
    }

    /// Replicate a blob asynchronously (fire and forget).
    pub async fn replicate_async(&self, request: ReplicationRequest) -> Result<()> {
        self.command_tx
            .send(ReplicationCommand::Replicate { request, reply: None })
            .await
            .context("replication manager shut down")?;
        Ok(())
    }

    /// Trigger repair for a specific blob.
    pub async fn trigger_repair(&self, hash: &Hash) -> Result<()> {
        self.command_tx
            .send(ReplicationCommand::RepairBlob { hash: *hash })
            .await
            .context("replication manager shut down")?;
        Ok(())
    }

    /// Get the current replica status for a blob.
    pub async fn get_status(&self, hash: &Hash) -> Result<Option<ReplicaSet>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(ReplicationCommand::GetStatus {
                hash: *hash,
                reply: reply_tx,
            })
            .await
            .context("replication manager shut down")?;

        reply_rx.await.context("reply channel closed")
    }

    /// Update the cluster topology (available nodes for placement).
    pub async fn update_topology(&self, nodes: Vec<NodeInfo>) -> Result<()> {
        self.command_tx
            .send(ReplicationCommand::UpdateTopology { nodes })
            .await
            .context("replication manager shut down")?;
        Ok(())
    }

    /// Get the configuration.
    pub fn config(&self) -> &ReplicationConfig {
        &self.config
    }

    /// Run a full repair cycle.
    ///
    /// Scans for all under-replicated blobs and triggers repairs in priority order:
    /// 1. Critical blobs (0 replicas) - highest data loss risk
    /// 2. UnderReplicated blobs (below `min_replicas`)
    /// 3. Degraded blobs (below `replication_factor`)
    ///
    /// Each scan is limited to `MAX_REPAIR_BATCH_SIZE` (100) entries per category
    /// to prevent unbounded memory use. Repairs are rate-limited by
    /// `MAX_CONCURRENT_REPLICATIONS` (10) concurrent transfers.
    ///
    /// This is a fire-and-forget operation that returns immediately after
    /// initiating the repair cycle. Use `get_status()` to monitor individual
    /// blob repair progress.
    pub async fn run_repair_cycle(&self) -> Result<()> {
        self.command_tx
            .send(ReplicationCommand::RunRepairCycle)
            .await
            .context("replication manager shut down")?;
        Ok(())
    }
}

// ============================================================================
// Trait Abstractions (for dependency injection)
// ============================================================================

/// Trait for reading/writing replica metadata.
///
/// This is implemented by the actual KV store to allow the replication
/// manager to track replicas without tight coupling.
#[async_trait::async_trait]
pub trait ReplicaMetadataStore: Send + Sync {
    /// Get replica set for a blob.
    async fn get_replica_set(&self, hash: &Hash) -> Result<Option<ReplicaSet>>;

    /// Save replica set (creates or updates).
    async fn save_replica_set(&self, replicas: &ReplicaSet) -> Result<()>;

    /// Delete replica set.
    async fn delete_replica_set(&self, hash: &Hash) -> Result<()>;

    /// Scan for blobs matching a replication status.
    ///
    /// Returns hashes of blobs that match the given status.
    /// Limited to MAX_REPAIR_BATCH_SIZE entries.
    async fn scan_by_status(&self, status: ReplicationStatus) -> Result<Vec<Hash>>;
}

/// Trait for transferring blobs between nodes.
///
/// This is implemented by the blob store to allow the replication
/// manager to transfer blobs without tight coupling.
#[async_trait::async_trait]
pub trait ReplicaBlobTransfer: Send + Sync {
    /// Transfer a blob to a target node.
    ///
    /// Returns Ok(true) if the transfer succeeded.
    async fn transfer_to_node(&self, hash: &Hash, target: &NodeInfo) -> Result<bool>;

    /// Check if we have a blob locally.
    async fn has_locally(&self, hash: &Hash) -> Result<bool>;

    /// Get blob size.
    async fn get_size(&self, hash: &Hash) -> Result<Option<u64>>;
}

// ============================================================================
// Manager Event Loop
// ============================================================================

async fn run_manager<KV, BS>(
    config: ReplicationConfig,
    mut command_rx: mpsc::Receiver<ReplicationCommand>,
    mut blob_events: broadcast::Receiver<BlobEvent>,
    kv_store: Arc<KV>,
    blob_store: Arc<BS>,
    placement: Arc<dyn PlacementStrategy>,
    cancel: CancellationToken,
) where
    KV: ReplicaMetadataStore + 'static,
    BS: ReplicaBlobTransfer + 'static,
{
    info!(
        node_id = config.node_id,
        auto_replicate = config.auto_replicate,
        replication_factor = config.default_policy.replication_factor,
        "blob replication manager started"
    );

    // Shared state
    let nodes: Arc<RwLock<Vec<NodeInfo>>> = Arc::new(RwLock::new(Vec::new()));
    let replication_semaphore = Arc::new(Semaphore::new(config.max_concurrent as usize));

    // Repair timer
    let repair_interval = Duration::from_secs(config.repair_interval_secs.max(MIN_REPAIR_INTERVAL_SECS));
    let mut repair_timer = tokio::time::interval(repair_interval);
    repair_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("blob replication manager shutting down");
                break;
            }

            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplicationCommand::Replicate { request, reply } => {
                        let result = handle_replicate(
                            &config,
                            &kv_store,
                            &blob_store,
                            &placement,
                            &nodes,
                            &replication_semaphore,
                            request,
                        ).await;

                        if let Some(reply_tx) = reply {
                            let _ = reply_tx.send(result);
                        }
                    }

                    ReplicationCommand::RepairBlob { hash } => {
                        if let Err(e) = handle_repair_blob(
                            &config,
                            &kv_store,
                            &blob_store,
                            &placement,
                            &nodes,
                            &replication_semaphore,
                            &hash,
                        ).await {
                            warn!(hash = %hash.to_hex(), error = %e, "repair failed");
                        }
                    }

                    ReplicationCommand::RunRepairCycle => {
                        if let Err(e) = handle_repair_cycle(
                            &config,
                            &kv_store,
                            &blob_store,
                            &placement,
                            &nodes,
                            &replication_semaphore,
                        ).await {
                            warn!(error = %e, "repair cycle failed");
                        }
                    }

                    ReplicationCommand::UpdateTopology { nodes: new_nodes } => {
                        *nodes.write() = new_nodes;
                        debug!(count = nodes.read().len(), "updated cluster topology");
                    }

                    ReplicationCommand::GetStatus { hash, reply } => {
                        let result = kv_store.get_replica_set(&hash).await.ok().flatten();
                        let _ = reply.send(result);
                    }
                }
            }

            event_result = blob_events.recv() => {
                match event_result {
                    Ok(event) if event.event_type == BlobEventType::Added && config.auto_replicate => {
                        // Auto-replicate newly added blobs
                        let request = ReplicationRequest::new(
                            event.hash,
                            event.size_bytes,
                            Vec::new(), // Targets selected by placement strategy
                        );

                        let result = handle_replicate(
                            &config,
                            &kv_store,
                            &blob_store,
                            &placement,
                            &nodes,
                            &replication_semaphore,
                            request,
                        ).await;

                        if let Err(e) = result {
                            debug!(hash = %event.hash.to_hex(), error = %e, "auto-replicate failed");
                        }
                    }
                    Ok(_) => {
                        // Other events - ignore
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(count = n, "blob event receiver lagged, missed events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("blob event channel closed");
                    }
                }
            }

            _ = repair_timer.tick() => {
                // Periodic repair cycle
                if let Err(e) = handle_repair_cycle(
                    &config,
                    &kv_store,
                    &blob_store,
                    &placement,
                    &nodes,
                    &replication_semaphore,
                ).await {
                    debug!(error = %e, "periodic repair cycle failed");
                }
            }
        }
    }
}

// ============================================================================
// Command Handlers
// ============================================================================

async fn handle_replicate<KV, BS>(
    config: &ReplicationConfig,
    kv_store: &Arc<KV>,
    blob_store: &Arc<BS>,
    placement: &Arc<dyn PlacementStrategy>,
    nodes: &Arc<RwLock<Vec<NodeInfo>>>,
    semaphore: &Arc<Semaphore>,
    mut request: ReplicationRequest,
) -> Result<ReplicationResult>
where
    KV: ReplicaMetadataStore,
    BS: ReplicaBlobTransfer,
{
    let start = Instant::now();

    // Get or create replica set
    let mut replicas = kv_store.get_replica_set(&request.hash).await?.unwrap_or_else(|| {
        ReplicaSet::new(request.hash, request.size_bytes, config.default_policy.clone(), config.node_id)
    });

    // Check if already fully replicated
    if replicas.is_fully_replicated() {
        return Ok(ReplicationResult {
            hash: request.hash,
            successful: replicas.nodes.iter().cloned().collect(),
            failed: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Select targets if not specified
    if request.targets.is_empty() {
        let nodes_guard = nodes.read();
        let exclude: HashSet<u64> = replicas.nodes.iter().cloned().collect();
        let needed = replicas.replicas_needed();

        request.targets = placement.select_targets(
            &request.hash,
            needed,
            &exclude,
            &nodes_guard,
            replicas.policy.failure_domain_key.as_deref(),
        );
    }

    if request.targets.is_empty() {
        return Ok(ReplicationResult {
            hash: request.hash,
            successful: vec![config.node_id],
            failed: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Get node info for targets
    let target_infos: Vec<NodeInfo> = {
        let nodes_guard = nodes.read();
        request
            .targets
            .iter()
            .filter_map(|id| nodes_guard.iter().find(|n| n.node_id == *id).cloned())
            .collect()
    };

    // Transfer to targets (rate-limited)
    let mut successful = vec![config.node_id];
    let mut failed = Vec::new();

    for target in target_infos {
        let _permit = semaphore.acquire().await.context("semaphore closed")?;

        match blob_store.transfer_to_node(&request.hash, &target).await {
            Ok(true) => {
                successful.push(target.node_id);
                replicas.add_node(target.node_id);
                debug!(
                    hash = %request.hash.to_hex(),
                    target = target.node_id,
                    "blob transferred"
                );
            }
            Ok(false) => {
                failed.push((target.node_id, "transfer returned false".to_string()));
            }
            Err(e) => {
                failed.push((target.node_id, e.to_string()));
                warn!(
                    hash = %request.hash.to_hex(),
                    target = target.node_id,
                    error = %e,
                    "blob transfer failed"
                );
            }
        }
    }

    // Save updated replica set
    kv_store.save_replica_set(&replicas).await?;

    let result = ReplicationResult {
        hash: request.hash,
        successful,
        failed,
        duration_ms: start.elapsed().as_millis() as u64,
    };

    // Check quorum if enabled
    if request.wait_for_ack && !result.meets_quorum(replicas.policy.min_replicas) {
        anyhow::bail!(
            "quorum not reached: {} replicas, need {}",
            result.successful.len(),
            replicas.policy.min_replicas
        );
    }

    Ok(result)
}

async fn handle_repair_blob<KV, BS>(
    config: &ReplicationConfig,
    kv_store: &Arc<KV>,
    blob_store: &Arc<BS>,
    placement: &Arc<dyn PlacementStrategy>,
    nodes: &Arc<RwLock<Vec<NodeInfo>>>,
    semaphore: &Arc<Semaphore>,
    hash: &Hash,
) -> Result<()>
where
    KV: ReplicaMetadataStore,
    BS: ReplicaBlobTransfer,
{
    let replicas = kv_store.get_replica_set(hash).await?;
    let Some(replicas) = replicas else {
        return Ok(());
    };

    if replicas.is_fully_replicated() {
        return Ok(());
    }

    // Check if we have the blob locally
    if !blob_store.has_locally(hash).await? {
        debug!(hash = %hash.to_hex(), "cannot repair: blob not available locally");
        return Ok(());
    }

    let request = ReplicationRequest::new(*hash, replicas.size_bytes, Vec::new());

    handle_replicate(config, kv_store, blob_store, placement, nodes, semaphore, request).await?;

    Ok(())
}

async fn handle_repair_cycle<KV, BS>(
    config: &ReplicationConfig,
    kv_store: &Arc<KV>,
    blob_store: &Arc<BS>,
    placement: &Arc<dyn PlacementStrategy>,
    nodes: &Arc<RwLock<Vec<NodeInfo>>>,
    semaphore: &Arc<Semaphore>,
) -> Result<()>
where
    KV: ReplicaMetadataStore,
    BS: ReplicaBlobTransfer,
{
    // First handle critical (1 replica) blobs
    let critical = kv_store.scan_by_status(ReplicationStatus::Critical).await?;
    for hash in critical.into_iter().take(MAX_REPAIR_BATCH_SIZE as usize) {
        if let Err(e) = handle_repair_blob(config, kv_store, blob_store, placement, nodes, semaphore, &hash).await {
            warn!(hash = %hash.to_hex(), error = %e, "critical repair failed");
        }
    }

    // Then under-replicated blobs
    let under = kv_store.scan_by_status(ReplicationStatus::UnderReplicated).await?;
    for hash in under.into_iter().take(MAX_REPAIR_BATCH_SIZE as usize) {
        if let Err(e) = handle_repair_blob(config, kv_store, blob_store, placement, nodes, semaphore, &hash).await {
            debug!(hash = %hash.to_hex(), error = %e, "under-replicated repair failed");
        }
    }

    // Finally degraded blobs
    let degraded = kv_store.scan_by_status(ReplicationStatus::Degraded).await?;
    for hash in degraded.into_iter().take(MAX_REPAIR_BATCH_SIZE as usize) {
        if let Err(e) = handle_repair_blob(config, kv_store, blob_store, placement, nodes, semaphore, &hash).await {
            debug!(hash = %hash.to_hex(), error = %e, "degraded repair failed");
        }
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use iroh::SecretKey;
    use rand::SeedableRng;

    use super::*;

    fn make_test_key(seed: u8) -> PublicKey {
        // Use seed to create deterministic but valid keys
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        SecretKey::generate(&mut rng).public()
    }

    #[test]
    fn test_weighted_placement_basic() {
        let placement = WeightedPlacement;
        let hash = Hash::from_bytes([0x42; 32]);

        let nodes = vec![
            NodeInfo::new(1, make_test_key(1)),
            NodeInfo::new(2, make_test_key(2)),
            NodeInfo::new(3, make_test_key(3)),
        ];

        let exclude = HashSet::new();
        let targets = placement.select_targets(&hash, 2, &exclude, &nodes, None);

        assert_eq!(targets.len(), 2);
        // Should be deterministic for same hash
        let targets2 = placement.select_targets(&hash, 2, &exclude, &nodes, None);
        assert_eq!(targets, targets2);
    }

    #[test]
    fn test_weighted_placement_exclude() {
        let placement = WeightedPlacement;
        let hash = Hash::from_bytes([0x42; 32]);

        let nodes = vec![
            NodeInfo::new(1, make_test_key(1)),
            NodeInfo::new(2, make_test_key(2)),
            NodeInfo::new(3, make_test_key(3)),
        ];

        let mut exclude = HashSet::new();
        exclude.insert(1);
        exclude.insert(2);

        let targets = placement.select_targets(&hash, 2, &exclude, &nodes, None);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], 3);
    }

    #[test]
    fn test_weighted_placement_failure_domains() {
        let placement = WeightedPlacement;
        let hash = Hash::from_bytes([0x55; 32]);

        let nodes = vec![
            NodeInfo::new(1, make_test_key(1)).with_tag("rack", "rack1"),
            NodeInfo::new(2, make_test_key(2)).with_tag("rack", "rack1"),
            NodeInfo::new(3, make_test_key(3)).with_tag("rack", "rack2"),
            NodeInfo::new(4, make_test_key(4)).with_tag("rack", "rack2"),
        ];

        let exclude = HashSet::new();
        let targets = placement.select_targets(&hash, 2, &exclude, &nodes, Some("rack"));

        assert_eq!(targets.len(), 2);
        // Should be from different racks
        let rack1: HashSet<u64> = [1, 2].into_iter().collect();
        let rack2: HashSet<u64> = [3, 4].into_iter().collect();

        let first_rack = if rack1.contains(&targets[0]) { &rack1 } else { &rack2 };
        let second_rack = if rack1.contains(&targets[1]) { &rack1 } else { &rack2 };

        // They should be from different racks
        assert!(first_rack != second_rack || targets.len() < 2);
    }

    #[test]
    fn test_node_info_tags() {
        let key = make_test_key(1);
        let node = NodeInfo::new(1, key).with_tag("rack", "rack1").with_tag("zone", "us-east-1a");

        assert_eq!(node.get_tag("rack"), Some("rack1"));
        assert_eq!(node.get_tag("zone"), Some("us-east-1a"));
        assert_eq!(node.get_tag("missing"), None);
    }
}
