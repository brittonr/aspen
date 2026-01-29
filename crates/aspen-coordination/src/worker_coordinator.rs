//! Distributed worker coordinator for cross-node worker management.
//!
//! This module provides cluster-wide coordination for distributed workers,
//! enabling load balancing, work stealing, and worker group management.
//!
//! ## Features
//!
//! - Global worker registry and discovery
//! - Load-based job routing with pluggable strategies
//! - Work stealing for dynamic load rebalancing
//! - Worker group coordination for multi-worker tasks
//! - Automatic failover and job redistribution
//! - Health monitoring and capacity management
//!
//! ## Tiger Style
//!
//! - Fixed limits on workers and groups (MAX_WORKERS = 1024, MAX_GROUPS = 64)
//! - Bounded work stealing batches (MAX_STEAL_BATCH = 10)
//! - Fail-fast on invalid configurations
//! - All operations through Raft consensus for consistency

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_core::KeyValueStore;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::registry::HealthStatus;
use crate::registry::RegisterOptions;
use crate::registry::ServiceInstanceMetadata;
use crate::registry::ServiceRegistry;
use crate::types::now_unix_ms;

/// Maximum number of workers in the cluster.
const MAX_WORKERS: usize = 1024;

/// Maximum number of worker groups.
const MAX_GROUPS: usize = 64;

/// Maximum workers per group.
const MAX_WORKERS_PER_GROUP: usize = 32;

/// Maximum jobs to steal in one batch.
const MAX_STEAL_BATCH: usize = 10;

/// Steal hint key prefix.
const STEAL_HINT_PREFIX: &str = "__worker_coord:steal:";

/// Steal hint TTL in milliseconds (30 seconds).
/// Hints expire if not consumed within this window.
const STEAL_HINT_TTL_MS: u64 = 30_000;

/// Maximum steal hints per worker to prevent unbounded accumulation.
const MAX_STEAL_HINTS_PER_WORKER: usize = 10;

/// Maximum hints to cleanup per sweep.
const MAX_HINT_CLEANUP_BATCH: usize = 100;

/// Worker coordinator key prefix.
#[allow(dead_code)]
const COORDINATOR_PREFIX: &str = "__worker_coord:";

/// Worker stats key prefix.
const WORKER_STATS_PREFIX: &str = "__worker_stats:";

/// Worker group key prefix.
const WORKER_GROUP_PREFIX: &str = "__worker_group:";

/// Worker information stored in the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Unique worker identifier.
    pub worker_id: String,
    /// Node ID hosting this worker.
    pub node_id: String,
    /// Iroh peer ID for P2P communication.
    pub peer_id: Option<String>,
    /// Worker capabilities (job types it can handle).
    pub capabilities: Vec<String>,
    /// Current load (0.0 = idle, 1.0 = fully loaded).
    pub load: f32,
    /// Number of jobs currently processing.
    pub active_jobs: usize,
    /// Maximum concurrent jobs.
    pub max_concurrent: usize,
    /// Queue depth at this worker.
    pub queue_depth: usize,
    /// Worker health status.
    pub health: HealthStatus,
    /// Custom tags for routing.
    pub tags: Vec<String>,
    /// Last heartbeat timestamp.
    pub last_heartbeat_ms: u64,
    /// Worker started timestamp.
    pub started_at_ms: u64,
    /// Total jobs processed.
    pub total_processed: u64,
    /// Total jobs failed.
    pub total_failed: u64,
    /// Average job processing time in ms.
    pub avg_processing_time_ms: u64,
    /// Worker group memberships.
    pub groups: HashSet<String>,
}

impl WorkerInfo {
    /// Calculate available capacity (0.0 = no capacity, 1.0 = full capacity).
    pub fn available_capacity(&self) -> f32 {
        if self.health != HealthStatus::Healthy {
            return 0.0;
        }

        let capacity = 1.0 - self.load;
        capacity.clamp(0.0, 1.0)
    }

    /// Check if worker can handle a job type.
    pub fn can_handle(&self, job_type: &str) -> bool {
        self.capabilities.is_empty() || self.capabilities.contains(&job_type.to_string())
    }

    /// Check if worker is alive based on heartbeat.
    pub fn is_alive(&self, timeout_ms: u64) -> bool {
        now_unix_ms() - self.last_heartbeat_ms < timeout_ms
    }
}

/// Worker group for coordinated tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerGroup {
    /// Group identifier.
    pub group_id: String,
    /// Group description.
    pub description: String,
    /// Member worker IDs.
    pub members: HashSet<String>,
    /// Group leader worker ID (for coordination).
    pub leader: Option<String>,
    /// Required capabilities for group members.
    pub required_capabilities: Vec<String>,
    /// Minimum members needed for group to be active.
    pub min_members: usize,
    /// Maximum members allowed.
    pub max_members: usize,
    /// Group creation timestamp.
    pub created_at_ms: u64,
    /// Group state.
    pub state: GroupState,
}

/// State of a worker group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupState {
    /// Group is forming, waiting for members.
    Forming,
    /// Group is active and ready for tasks.
    Active,
    /// Group is executing a coordinated task.
    Executing,
    /// Group is disbanding.
    Disbanding,
}

/// A work stealing hint stored in the KV store.
///
/// Hints are coordination signals from the coordinator to workers,
/// indicating that a target worker should attempt to steal work
/// from a source worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealHint {
    /// Target worker ID (the worker that should steal).
    pub target_worker_id: String,
    /// Source worker ID (the worker to steal from).
    pub source_worker_id: String,
    /// Suggested batch size for stealing.
    pub batch_size: usize,
    /// When this hint was created (Unix ms).
    pub created_at_ms: u64,
    /// When this hint expires (Unix ms).
    pub expires_at_ms: u64,
    /// Round-robin index used for source selection (for debugging).
    pub source_index: usize,
}

impl StealHint {
    /// Create a new steal hint with TTL.
    pub fn new(target_worker_id: String, source_worker_id: String, batch_size: usize, source_index: usize) -> Self {
        let now = now_unix_ms();
        Self {
            target_worker_id,
            source_worker_id,
            batch_size,
            created_at_ms: now,
            expires_at_ms: now + STEAL_HINT_TTL_MS,
            source_index,
        }
    }

    /// Check if this hint has expired.
    pub fn is_expired(&self) -> bool {
        now_unix_ms() > self.expires_at_ms
    }

    /// Get remaining TTL in milliseconds.
    pub fn remaining_ttl_ms(&self) -> u64 {
        self.expires_at_ms.saturating_sub(now_unix_ms())
    }
}

/// Load balancing strategy for work distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Simple round-robin distribution.
    RoundRobin,
    /// Route to least loaded worker.
    LeastLoaded,
    /// Route based on worker affinity.
    Affinity,
    /// Consistent hashing for deterministic routing.
    ConsistentHash,
    /// Enable work stealing.
    WorkStealing,
}

/// Configuration for the distributed worker coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCoordinatorConfig {
    /// Load balancing strategy.
    pub strategy: LoadBalancingStrategy,
    /// Worker heartbeat timeout in milliseconds.
    pub heartbeat_timeout_ms: u64,
    /// Worker heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Enable work stealing.
    pub enable_work_stealing: bool,
    /// Work stealing check interval in milliseconds.
    pub steal_check_interval_ms: u64,
    /// Load threshold for stealing (steal if load < threshold).
    pub steal_load_threshold: f32,
    /// Queue depth threshold for stealing source.
    pub steal_queue_threshold: usize,
    /// Enable automatic failover.
    pub enable_failover: bool,
    /// Failover check interval in milliseconds.
    pub failover_check_interval_ms: u64,
    /// Maximum workers to track.
    pub max_workers: usize,
    /// Maximum groups to manage.
    pub max_groups: usize,
}

impl Default for WorkerCoordinatorConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::LeastLoaded,
            heartbeat_timeout_ms: 30_000,  // 30 seconds
            heartbeat_interval_ms: 10_000, // 10 seconds
            enable_work_stealing: true,
            steal_check_interval_ms: 5_000, // 5 seconds
            steal_load_threshold: 0.2,      // Steal if load < 20%
            steal_queue_threshold: 10,      // Source must have > 10 jobs
            enable_failover: true,
            failover_check_interval_ms: 15_000, // 15 seconds
            max_workers: MAX_WORKERS,
            max_groups: MAX_GROUPS,
        }
    }
}

/// Distributed worker coordinator for cluster-wide worker management.
#[derive(Clone)]
pub struct DistributedWorkerCoordinator<S: KeyValueStore + ?Sized> {
    /// Key-value store for coordination state.
    store: Arc<S>,
    /// Service registry for worker discovery.
    registry: ServiceRegistry<S>,
    /// Configuration.
    config: WorkerCoordinatorConfig,
    /// Local cache of worker info.
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    /// Worker groups.
    groups: Arc<RwLock<HashMap<String, WorkerGroup>>>,
    /// Round-robin counter for simple distribution.
    round_robin_counter: Arc<RwLock<usize>>,
    /// Round-robin counter for work stealing source selection.
    steal_source_counter: Arc<RwLock<usize>>,
    /// Background task handles.
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Shutdown signal.
    shutdown: Arc<tokio::sync::Notify>,
}

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
    /// Create a new distributed worker coordinator.
    pub fn new(store: Arc<S>) -> Self {
        Self::with_config(store, WorkerCoordinatorConfig::default())
    }

    /// Create a coordinator with custom configuration.
    pub fn with_config(store: Arc<S>, config: WorkerCoordinatorConfig) -> Self {
        let registry = ServiceRegistry::new(store.clone());

        Self {
            store,
            registry,
            config,
            workers: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counter: Arc::new(RwLock::new(0)),
            steal_source_counter: Arc::new(RwLock::new(0)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Register a worker with the coordinator.
    ///
    /// Uses optimistic check with re-validation under write lock to prevent
    /// TOCTOU race conditions that could exceed MAX_WORKERS.
    pub async fn register_worker(&self, info: WorkerInfo) -> Result<()> {
        // Early validation (optimistic, may have false positives from concurrent registrations)
        {
            let workers = self.workers.read().await;
            if workers.len() >= self.config.max_workers {
                bail!("maximum worker limit {} reached", self.config.max_workers);
            }
        }

        // Register in service registry (no local state changed yet)
        let metadata = ServiceInstanceMetadata {
            version: "1.0.0".to_string(),
            tags: info.tags.clone(),
            weight: (info.max_concurrent as u32).max(1),
            custom: HashMap::from([
                ("capabilities".to_string(), info.capabilities.join(",")),
                ("node_id".to_string(), info.node_id.clone()),
            ]),
        };

        let (_token, _deadline) = self
            .registry
            .register("distributed-worker", &info.worker_id, &info.node_id, metadata, RegisterOptions {
                ttl_ms: Some(self.config.heartbeat_timeout_ms),
                initial_status: Some(info.health),
                lease_id: None,
            })
            .await?;

        // Store worker info in KV store (no local state changed yet)
        let key = format!("{}{}", WORKER_STATS_PREFIX, info.worker_id);
        let value = serde_json::to_string(&info)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value,
                },
            })
            .await?;

        // Final insert with re-validation (TOCTOU protection)
        let worker_id = info.worker_id.clone();
        let node_id = info.node_id.clone();

        let mut workers = self.workers.write().await;

        // Re-check limit under write lock - if we've hit the limit and this worker
        // isn't already registered (idempotent re-registration is OK), reject
        if workers.len() >= self.config.max_workers && !workers.contains_key(&worker_id) {
            // Rollback: cleanup KV store (fire-and-forget with logging)
            if let Err(e) = self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key },
                })
                .await
            {
                warn!(
                    error = %e,
                    worker_id = %worker_id,
                    "failed to rollback worker registration from KV store"
                );
            }
            bail!("maximum worker limit {} reached during registration", self.config.max_workers);
        }

        workers.insert(worker_id.clone(), info);

        info!(
            worker_id = %worker_id,
            node_id = %node_id,
            "worker registered with coordinator"
        );

        Ok(())
    }

    /// Update worker heartbeat and stats.
    pub async fn heartbeat(&self, worker_id: &str, stats: WorkerStats) -> Result<()> {
        // Update in registry
        if let Some(instance) = self.registry.get_instance("distributed-worker", worker_id).await? {
            self.registry.heartbeat("distributed-worker", worker_id, instance.fencing_token).await?;
        }

        // Update worker info
        let mut workers = self.workers.write().await;
        if let Some(info) = workers.get_mut(worker_id) {
            info.last_heartbeat_ms = now_unix_ms();
            info.load = stats.load;
            info.active_jobs = stats.active_jobs;
            info.queue_depth = stats.queue_depth;
            info.total_processed = stats.total_processed;
            info.total_failed = stats.total_failed;
            info.avg_processing_time_ms = stats.avg_processing_time_ms;
            info.health = stats.health;

            // Update in KV store
            let key = format!("{}{}", WORKER_STATS_PREFIX, worker_id);
            let value = serde_json::to_string(&info)?;

            self.store
                .write(WriteRequest {
                    command: WriteCommand::Set { key, value },
                })
                .await?;
        } else {
            bail!("worker {} not found", worker_id);
        }

        Ok(())
    }

    /// Deregister a worker.
    pub async fn deregister_worker(&self, worker_id: &str) -> Result<()> {
        // Get instance to get fencing token
        let instance = self.registry.get_instance("distributed-worker", worker_id).await?;

        // Remove from registry
        if let Some(inst) = instance {
            self.registry.deregister("distributed-worker", worker_id, inst.fencing_token).await?;
        }

        // Remove from KV store
        let key = format!("{}{}", WORKER_STATS_PREFIX, worker_id);
        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await?;

        // Remove from local cache
        let mut workers = self.workers.write().await;
        workers.remove(worker_id);

        // Remove from all groups
        let mut groups = self.groups.write().await;
        for group in groups.values_mut() {
            group.members.remove(worker_id);
            if group.leader.as_deref() == Some(worker_id) {
                // Elect new leader if needed
                group.leader = group.members.iter().next().cloned();
            }
        }

        info!(worker_id, "worker deregistered from coordinator");
        Ok(())
    }

    /// Select a worker for a job based on the configured strategy.
    pub async fn select_worker(&self, job_type: &str, affinity_key: Option<&str>) -> Result<Option<WorkerInfo>> {
        let workers = self.workers.read().await;

        // Filter to healthy, alive workers that can handle the job
        let eligible: Vec<_> = workers
            .values()
            .filter(|w| {
                w.health == HealthStatus::Healthy
                    && w.is_alive(self.config.heartbeat_timeout_ms)
                    && w.can_handle(job_type)
            })
            .cloned()
            .collect();

        if eligible.is_empty() {
            return Ok(None);
        }

        // Select based on strategy
        let selected = match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let mut counter = self.round_robin_counter.write().await;
                let idx = *counter % eligible.len();
                *counter = (*counter + 1) % eligible.len();
                eligible.get(idx).cloned()
            }
            LoadBalancingStrategy::LeastLoaded => eligible.into_iter().max_by(|a, b| {
                a.available_capacity().partial_cmp(&b.available_capacity()).unwrap_or(std::cmp::Ordering::Equal)
            }),
            LoadBalancingStrategy::Affinity => {
                if let Some(key) = affinity_key {
                    // Simple hash-based affinity
                    let hash = simple_hash(key) as usize;
                    eligible.get(hash % eligible.len()).cloned()
                } else {
                    // Fallback to least loaded
                    eligible.into_iter().max_by(|a, b| {
                        a.available_capacity().partial_cmp(&b.available_capacity()).unwrap_or(std::cmp::Ordering::Equal)
                    })
                }
            }
            LoadBalancingStrategy::ConsistentHash => {
                // Simplified consistent hash
                let key = affinity_key.unwrap_or(job_type);
                let hash = simple_hash(key) as usize;
                eligible.get(hash % eligible.len()).cloned()
            }
            LoadBalancingStrategy::WorkStealing => {
                // For work stealing, prefer least loaded but consider queue depth
                eligible.into_iter().min_by_key(|w| (w.queue_depth, (w.load * 1000.0) as u32))
            }
        };

        Ok(selected)
    }

    /// Get all workers matching a filter.
    pub async fn get_workers(&self, filter: WorkerFilter) -> Result<Vec<WorkerInfo>> {
        let workers = self.workers.read().await;

        let filtered: Vec<_> = workers
            .values()
            .filter(|w| {
                // Check health
                if let Some(health) = filter.health
                    && w.health != health
                {
                    return false;
                }

                // Check capabilities
                if let Some(ref cap) = filter.capability
                    && !w.can_handle(cap)
                {
                    return false;
                }

                // Check node
                if let Some(ref node) = filter.node_id
                    && w.node_id != *node
                {
                    return false;
                }

                // Check tags
                if let Some(ref tags) = filter.tags
                    && !tags.iter().all(|t| w.tags.contains(t))
                {
                    return false;
                }

                // Check load threshold
                if let Some(max_load) = filter.max_load
                    && w.load > max_load
                {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Find workers that can steal work (low load).
    pub async fn find_steal_targets(&self) -> Result<Vec<WorkerInfo>> {
        let workers = self.workers.read().await;

        let targets: Vec<_> = workers
            .values()
            .filter(|w| {
                w.health == HealthStatus::Healthy
                    && w.is_alive(self.config.heartbeat_timeout_ms)
                    && w.load < self.config.steal_load_threshold
                    && w.active_jobs < w.max_concurrent
            })
            .cloned()
            .collect();

        Ok(targets)
    }

    /// Find workers that are good sources for work stealing (high load).
    pub async fn find_steal_sources(&self) -> Result<Vec<WorkerInfo>> {
        let workers = self.workers.read().await;

        let sources: Vec<_> = workers
            .values()
            .filter(|w| {
                w.health == HealthStatus::Healthy
                    && w.is_alive(self.config.heartbeat_timeout_ms)
                    && w.queue_depth > self.config.steal_queue_threshold
            })
            .cloned()
            .collect();

        Ok(sources)
    }

    /// Create a new worker group.
    ///
    /// Uses optimistic check with re-validation under write lock to prevent
    /// TOCTOU race conditions. Lock ordering: workers before groups.
    pub async fn create_group(&self, group: WorkerGroup) -> Result<()> {
        // Early validation (optimistic, may have false positives from concurrent creations)
        {
            let groups = self.groups.read().await;
            if groups.len() >= self.config.max_groups {
                bail!("maximum group limit {} reached", self.config.max_groups);
            }
        }

        // Validate member count (stateless check)
        if group.members.len() > MAX_WORKERS_PER_GROUP {
            bail!("group exceeds maximum member limit {}", MAX_WORKERS_PER_GROUP);
        }

        // Store in KV (no local state changed yet)
        let key = format!("{}{}", WORKER_GROUP_PREFIX, group.group_id);
        let value = serde_json::to_string(&group)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value,
                },
            })
            .await?;

        // Acquire locks in canonical order: workers FIRST, then groups
        let mut workers = self.workers.write().await;
        let mut groups = self.groups.write().await;

        // Re-check limit under write lock (TOCTOU protection)
        if groups.len() >= self.config.max_groups && !groups.contains_key(&group.group_id) {
            // Rollback: cleanup KV store (fire-and-forget with logging)
            if let Err(e) = self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key },
                })
                .await
            {
                warn!(
                    error = %e,
                    group_id = %group.group_id,
                    "failed to rollback group creation from KV store"
                );
            }
            bail!("maximum group limit {} reached during creation", self.config.max_groups);
        }

        // Update worker memberships
        for member_id in &group.members {
            if let Some(worker) = workers.get_mut(member_id) {
                worker.groups.insert(group.group_id.clone());
            }
        }

        groups.insert(group.group_id.clone(), group);

        Ok(())
    }

    /// Get a worker group by ID.
    pub async fn get_group(&self, group_id: &str) -> Result<Option<WorkerGroup>> {
        let groups = self.groups.read().await;
        Ok(groups.get(group_id).cloned())
    }

    /// Add a worker to a group.
    ///
    /// Uses persist-first pattern to avoid cache/KV divergence.
    /// Lock ordering: workers before groups (no locks across await).
    pub async fn add_to_group(&self, group_id: &str, worker_id: &str) -> Result<()> {
        // Phase 1: Read-only validation and prepare update
        let updated_group = {
            let groups = self.groups.read().await;
            let group = groups.get(group_id).ok_or_else(|| anyhow::anyhow!("group {} not found", group_id))?;

            if group.members.len() >= group.max_members {
                bail!("group {} is at maximum capacity", group_id);
            }

            let mut updated = group.clone();
            updated.members.insert(worker_id.to_string());
            updated
        };

        // Phase 2: Persist FIRST (no locks held)
        let key = format!("{}{}", WORKER_GROUP_PREFIX, group_id);
        let value = serde_json::to_string(&updated_group)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        // Phase 3: Update cache SECOND with canonical lock order (workers first)
        let mut workers = self.workers.write().await;
        let mut groups = self.groups.write().await;

        // Re-validate group exists (may have been deleted concurrently)
        if let Some(group) = groups.get_mut(group_id) {
            group.members.insert(worker_id.to_string());
        }

        if let Some(worker) = workers.get_mut(worker_id) {
            worker.groups.insert(group_id.to_string());
        }

        Ok(())
    }

    /// Remove a worker from a group.
    ///
    /// Uses persist-first pattern to avoid cache/KV divergence.
    /// Lock ordering: workers before groups (no locks across await).
    pub async fn remove_from_group(&self, group_id: &str, worker_id: &str) -> Result<()> {
        // Phase 1: Read-only validation and prepare update
        let updated_group = {
            let groups = self.groups.read().await;
            let group = groups.get(group_id).ok_or_else(|| anyhow::anyhow!("group {} not found", group_id))?;

            let mut updated = group.clone();
            updated.members.remove(worker_id);

            // Update leader if needed
            if updated.leader.as_deref() == Some(worker_id) {
                updated.leader = updated.members.iter().next().cloned();
            }

            updated
        };

        // Phase 2: Persist FIRST (no locks held)
        let key = format!("{}{}", WORKER_GROUP_PREFIX, group_id);
        let value = serde_json::to_string(&updated_group)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        // Phase 3: Update cache SECOND with canonical lock order (workers first)
        let mut workers = self.workers.write().await;
        let mut groups = self.groups.write().await;

        // Re-validate group exists (may have been deleted concurrently)
        if let Some(group) = groups.get_mut(group_id) {
            group.members.remove(worker_id);
            if group.leader.as_deref() == Some(worker_id) {
                group.leader = group.members.iter().next().cloned();
            }
        }

        if let Some(worker) = workers.get_mut(worker_id) {
            worker.groups.remove(group_id);
        }

        Ok(())
    }

    /// Start background tasks for monitoring and work stealing.
    ///
    /// The coordinator must be wrapped in `Arc` to enable spawning background tasks.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let mut tasks = self.tasks.write().await;

        // Start failover monitor
        if self.config.enable_failover {
            let coordinator = self.clone();
            let handle = tokio::spawn(async move {
                coordinator.failover_monitor().await;
            });
            tasks.push(handle);
        }

        // Start work stealing monitor
        if self.config.enable_work_stealing {
            let coordinator = self.clone();
            let handle = tokio::spawn(async move {
                coordinator.work_stealing_monitor().await;
            });
            tasks.push(handle);
        }

        info!("distributed worker coordinator started");
        Ok(())
    }

    /// Stop the coordinator and its background tasks.
    pub async fn stop(&self) -> Result<()> {
        self.shutdown.notify_waiters();

        let mut tasks = self.tasks.write().await;
        for handle in tasks.drain(..) {
            let _ = handle.await;
        }

        info!("distributed worker coordinator stopped");
        Ok(())
    }

    /// Background task for failover monitoring.
    async fn failover_monitor(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_millis(self.config.failover_check_interval_ms));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_and_handle_failures().await {
                        error!("failover check failed: {}", e);
                    }
                }
                _ = self.shutdown.notified() => {
                    debug!("failover monitor shutting down");
                    break;
                }
            }
        }
    }

    /// Check for failed workers and handle redistribution.
    async fn check_and_handle_failures(&self) -> Result<()> {
        let mut workers = self.workers.write().await;

        let failed_workers: Vec<_> = workers
            .iter()
            .filter(|(_, w)| !w.is_alive(self.config.heartbeat_timeout_ms))
            .map(|(id, _)| id.clone())
            .collect();

        for worker_id in failed_workers {
            warn!(worker_id, "worker detected as failed, removing");

            // Remove from groups
            let mut groups = self.groups.write().await;
            for group in groups.values_mut() {
                group.members.remove(&worker_id);
                if group.leader.as_deref() == Some(&worker_id) {
                    group.leader = group.members.iter().next().cloned();
                }
            }
            drop(groups);

            workers.remove(&worker_id);

            // Note: Job redistribution would be handled by the job manager
            // when it detects the worker is no longer available
        }

        Ok(())
    }

    /// Background task for work stealing monitor.
    async fn work_stealing_monitor(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_millis(self.config.steal_check_interval_ms));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.coordinate_work_stealing().await {
                        error!("work stealing coordination failed: {}", e);
                    }
                }
                _ = self.shutdown.notified() => {
                    debug!("work stealing monitor shutting down");
                    break;
                }
            }
        }
    }

    /// Coordinate work stealing between workers.
    ///
    /// Identifies target workers (low load) and source workers (high queue depth),
    /// then creates steal hints for consumption by the job manager.
    ///
    /// Uses round-robin source selection to distribute stealing load evenly
    /// across all available sources.
    async fn coordinate_work_stealing(&self) -> Result<()> {
        let targets = self.find_steal_targets().await?;
        let sources = self.find_steal_sources().await?;

        if targets.is_empty() || sources.is_empty() {
            return Ok(());
        }

        // Sort sources by queue depth (descending) for deterministic selection
        let mut sorted_sources = sources;
        sorted_sources.sort_by(|a, b| b.queue_depth.cmp(&a.queue_depth));

        // Get and update round-robin counter
        let mut counter = self.steal_source_counter.write().await;
        let mut hints_created = 0usize;

        // Limit targets per coordination round (Tiger Style)
        const MAX_TARGETS_PER_ROUND: usize = 5;

        for target in targets.iter().take(MAX_TARGETS_PER_ROUND) {
            // Round-robin through sources
            let source_index = *counter % sorted_sources.len();
            *counter = (*counter + 1) % sorted_sources.len();

            let source = &sorted_sources[source_index];

            // Skip if target and source are the same worker
            if target.worker_id == source.worker_id {
                continue;
            }

            debug!(
                target = %target.worker_id,
                source = %source.worker_id,
                source_depth = source.queue_depth,
                source_index = source_index,
                "coordinating work stealing"
            );

            // Create typed steal hint with TTL
            let hint =
                StealHint::new(target.worker_id.clone(), source.worker_id.clone(), MAX_STEAL_BATCH, source_index);

            // Store hint with composite key
            let key = format!("{}{}:{}", STEAL_HINT_PREFIX, target.worker_id, source.worker_id);
            let value = serde_json::to_string(&hint)?;

            self.store
                .write(WriteRequest {
                    command: WriteCommand::Set { key, value },
                })
                .await?;

            hints_created += 1;
        }

        // Cleanup expired hints at the end of each coordination round
        let cleaned = self.cleanup_expired_hints().await?;
        if cleaned > 0 || hints_created > 0 {
            debug!(hints_created = hints_created, hints_cleaned = cleaned, "work stealing coordination completed");
        }

        Ok(())
    }

    /// Get pending steal hints for a worker.
    ///
    /// Returns all non-expired steal hints where the given worker is the target.
    /// The job manager should call this periodically to check for stealing opportunities.
    pub async fn get_steal_hints(&self, worker_id: &str) -> Result<Vec<StealHint>> {
        // Scan for hints targeting this worker
        let prefix = format!("{}{}:", STEAL_HINT_PREFIX, worker_id);

        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix,
                limit: Some(MAX_STEAL_HINTS_PER_WORKER as u32),
                continuation_token: None,
            })
            .await?;

        let mut hints = Vec::new();
        for entry in scan_result.entries {
            match serde_json::from_str::<StealHint>(&entry.value) {
                Ok(hint) => {
                    // Only include non-expired hints
                    if !hint.is_expired() {
                        hints.push(hint);
                    }
                }
                Err(e) => {
                    // Log and skip malformed entries
                    warn!(
                        key = %entry.key,
                        error = %e,
                        "failed to parse steal hint, skipping"
                    );
                }
            }
        }

        Ok(hints)
    }

    /// Consume (acknowledge and delete) a steal hint.
    ///
    /// Call this after successfully stealing work from the source worker,
    /// or to dismiss a hint that is no longer actionable.
    ///
    /// This operation is idempotent - calling it on an already-consumed or
    /// non-existent hint returns success.
    pub async fn consume_steal_hint(&self, target_id: &str, source_id: &str) -> Result<()> {
        let key = format!("{}{}:{}", STEAL_HINT_PREFIX, target_id, source_id);

        // Delete is idempotent - succeeds even if key doesn't exist
        let _ = self
            .store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await;

        debug!(target = target_id, source = source_id, "steal hint consumed");

        Ok(())
    }

    /// Check if a specific steal hint exists and is valid.
    pub async fn has_steal_hint(&self, target_id: &str, source_id: &str) -> Result<bool> {
        let key = format!("{}{}:{}", STEAL_HINT_PREFIX, target_id, source_id);

        match self.store.read(aspen_core::ReadRequest::new(key)).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    match serde_json::from_str::<StealHint>(&kv.value) {
                        Ok(hint) => Ok(!hint.is_expired()),
                        Err(_) => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// Clean up expired steal hints.
    ///
    /// Scans for all steal hints and removes those that have passed their
    /// expiration deadline.
    async fn cleanup_expired_hints(&self) -> Result<usize> {
        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: STEAL_HINT_PREFIX.to_string(),
                limit: Some(MAX_HINT_CLEANUP_BATCH as u32),
                continuation_token: None,
            })
            .await?;

        let mut deleted = 0usize;

        for entry in scan_result.entries {
            let should_delete = match serde_json::from_str::<StealHint>(&entry.value) {
                Ok(hint) => hint.is_expired(),
                Err(_) => {
                    // Malformed entry - delete it
                    warn!(key = %entry.key, "deleting malformed steal hint");
                    true
                }
            };

            if should_delete {
                let _ = self
                    .store
                    .write(WriteRequest {
                        command: WriteCommand::Delete { key: entry.key },
                    })
                    .await;
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// Force cleanup of all steal hints (for testing or shutdown).
    pub async fn clear_all_steal_hints(&self) -> Result<usize> {
        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: STEAL_HINT_PREFIX.to_string(),
                limit: Some(MAX_HINT_CLEANUP_BATCH as u32),
                continuation_token: None,
            })
            .await?;

        let mut deleted = 0usize;

        for entry in scan_result.entries {
            let _ = self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key: entry.key },
                })
                .await;
            deleted += 1;
        }

        if deleted > 0 {
            info!(count = deleted, "cleared all steal hints");
        }

        Ok(deleted)
    }
}

/// Filter for querying workers.
#[derive(Debug, Clone, Default)]
pub struct WorkerFilter {
    /// Filter by health status.
    pub health: Option<HealthStatus>,
    /// Filter by capability.
    pub capability: Option<String>,
    /// Filter by node ID.
    pub node_id: Option<String>,
    /// Filter by tags.
    pub tags: Option<Vec<String>>,
    /// Filter by maximum load.
    pub max_load: Option<f32>,
}

/// Worker statistics for heartbeat updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    /// Current load.
    pub load: f32,
    /// Active jobs.
    pub active_jobs: usize,
    /// Queue depth.
    pub queue_depth: usize,
    /// Total processed.
    pub total_processed: u64,
    /// Total failed.
    pub total_failed: u64,
    /// Average processing time.
    pub avg_processing_time_ms: u64,
    /// Health status.
    pub health: HealthStatus,
}

/// Simple hash function for consistent hashing.
fn simple_hash(s: &str) -> u64 {
    let mut hash = 0u64;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_capacity() {
        let mut worker = WorkerInfo {
            worker_id: "w1".to_string(),
            node_id: "n1".to_string(),
            peer_id: None,
            capabilities: vec![],
            load: 0.3,
            active_jobs: 3,
            max_concurrent: 10,
            queue_depth: 5,
            health: HealthStatus::Healthy,
            tags: vec![],
            last_heartbeat_ms: now_unix_ms(),
            started_at_ms: now_unix_ms(),
            total_processed: 100,
            total_failed: 2,
            avg_processing_time_ms: 50,
            groups: HashSet::new(),
        };

        assert_eq!(worker.available_capacity(), 0.7);

        worker.health = HealthStatus::Unhealthy;
        assert_eq!(worker.available_capacity(), 0.0);
    }

    #[test]
    fn test_worker_can_handle() {
        let worker = WorkerInfo {
            worker_id: "w1".to_string(),
            node_id: "n1".to_string(),
            peer_id: None,
            capabilities: vec!["email".to_string(), "sms".to_string()],
            load: 0.5,
            active_jobs: 5,
            max_concurrent: 10,
            queue_depth: 0,
            health: HealthStatus::Healthy,
            tags: vec![],
            last_heartbeat_ms: now_unix_ms(),
            started_at_ms: now_unix_ms(),
            total_processed: 0,
            total_failed: 0,
            avg_processing_time_ms: 0,
            groups: HashSet::new(),
        };

        assert!(worker.can_handle("email"));
        assert!(worker.can_handle("sms"));
        assert!(!worker.can_handle("push"));
    }
}
