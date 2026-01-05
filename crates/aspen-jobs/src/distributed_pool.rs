//! Distributed worker pool with cross-node coordination.
//!
//! This module extends the basic WorkerPool with distributed coordination
//! capabilities including work stealing, cross-node job migration, and
//! worker group support.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use aspen_coordination::DistributedWorkerCoordinator;
use aspen_coordination::GroupState;
use aspen_coordination::HealthStatus;
use aspen_coordination::Priority as RoutingPriority;
use aspen_coordination::RoutingContext;
use aspen_coordination::WorkerCoordinatorConfig;
use aspen_coordination::WorkerFilter;
use aspen_coordination::WorkerGroup;
use aspen_coordination::WorkerInfo;
use aspen_coordination::WorkerStats;
use aspen_core::KeyValueStore;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::JobId;
use crate::manager::JobManager;
use crate::types::Priority;
use crate::worker::Worker;
use crate::worker::WorkerConfig;
use crate::worker::WorkerPool;
use crate::worker::WorkerStatus;

/// Configuration for distributed worker pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedPoolConfig {
    /// Base worker configuration.
    pub worker_config: WorkerConfig,
    /// Coordinator configuration.
    pub coordinator_config: WorkerCoordinatorConfig,
    /// Node identifier.
    pub node_id: String,
    /// Iroh peer ID for P2P communication.
    pub peer_id: Option<String>,
    /// Enable cross-node job migration.
    pub enable_migration: bool,
    /// Enable participation in work stealing.
    pub enable_work_stealing: bool,
    /// Work steal check interval.
    pub steal_check_interval: Duration,
    /// Maximum jobs to migrate at once.
    pub max_migration_batch: usize,
    /// Job types this pool specializes in.
    pub specializations: Vec<String>,
    /// Tags for worker routing.
    pub tags: Vec<String>,
}

impl Default for DistributedPoolConfig {
    fn default() -> Self {
        Self {
            worker_config: WorkerConfig::default(),
            coordinator_config: WorkerCoordinatorConfig::default(),
            node_id: format!("node-{}", uuid::Uuid::new_v4()),
            peer_id: None,
            enable_migration: true,
            enable_work_stealing: true,
            steal_check_interval: Duration::from_secs(5),
            max_migration_batch: 10,
            specializations: vec![],
            tags: vec![],
        }
    }
}

/// A distributed worker pool that coordinates across nodes.
pub struct DistributedWorkerPool<S: KeyValueStore + ?Sized> {
    /// Base worker pool.
    pool: Arc<WorkerPool<S>>,
    /// Job manager.
    manager: Arc<JobManager<S>>,
    /// Distributed coordinator.
    coordinator: Arc<DistributedWorkerCoordinator<S>>,
    /// Configuration.
    config: DistributedPoolConfig,
    /// Worker registrations.
    registrations: Arc<RwLock<HashMap<String, WorkerRegistration>>>,
    /// Active worker groups.
    groups: Arc<RwLock<HashMap<String, WorkerGroupHandle>>>,
    /// Background task handles.
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Shutdown signal.
    shutdown: Arc<tokio::sync::Notify>,
}

/// Registration info for a worker in the distributed system.
#[derive(Debug, Clone)]
struct WorkerRegistration {
    worker_id: String,
    #[allow(dead_code)] // Used for registration tracking
    registered_at: chrono::DateTime<Utc>,
    #[allow(dead_code)] // Used for heartbeat monitoring
    last_heartbeat: chrono::DateTime<Utc>,
}

/// Handle to a worker group for coordinated tasks.
#[derive(Clone)]
pub struct WorkerGroupHandle {
    group_id: String,
    members: HashSet<String>,
    barrier: Arc<tokio::sync::Barrier>,
    broadcast: tokio::sync::broadcast::Sender<GroupMessage>,
}

/// Messages for worker group coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupMessage {
    /// Start processing phase.
    StartPhase {
        /// Name of the phase to start.
        phase: String,
    },
    /// Phase completed by a member.
    PhaseComplete {
        /// ID of the worker that completed.
        worker_id: String,
        /// Name of the completed phase.
        phase: String,
    },
    /// Broadcast data to group.
    Broadcast {
        /// Data to broadcast.
        data: Vec<u8>,
    },
    /// Group task completed.
    TaskComplete,
}

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerPool<S> {
    /// Create a new distributed worker pool.
    pub fn new(store: Arc<S>, config: DistributedPoolConfig) -> Self {
        let manager = Arc::new(JobManager::new(store.clone()));
        let pool = Arc::new(WorkerPool::with_manager(manager.clone()));
        let coordinator = Arc::new(DistributedWorkerCoordinator::with_config(store, config.coordinator_config.clone()));

        Self {
            pool,
            manager,
            coordinator,
            config,
            registrations: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Register a worker handler.
    pub async fn register_handler<W: Worker>(&self, job_type: &str, worker: W) -> Result<()> {
        self.pool.register_handler(job_type, worker).await
    }

    /// Start the distributed pool with the specified number of local workers.
    pub async fn start(&self, num_workers: usize) -> Result<()> {
        info!(
            node_id = %self.config.node_id,
            num_workers,
            "starting distributed worker pool"
        );

        // Start the coordinator
        self.coordinator.clone().start().await.map_err(|e| JobError::WorkerRegistrationFailed {
            reason: format!("failed to start coordinator: {}", e),
        })?;

        // Start local workers
        for i in 0..num_workers {
            let worker_id = format!("{}-worker-{}", self.config.node_id, i);

            // Create worker config
            let mut worker_config = self.config.worker_config.clone();
            worker_config.id = Some(worker_id.clone());
            worker_config.job_types = self.config.specializations.clone();

            // Spawn worker
            let _handle = self.pool.spawn_worker(worker_config).await?;

            // Register with coordinator
            let worker_info = WorkerInfo {
                worker_id: worker_id.clone(),
                node_id: self.config.node_id.clone(),
                peer_id: self.config.peer_id.clone(),
                capabilities: self.config.specializations.clone(),
                load: 0.0,
                active_jobs: 0,
                max_concurrent: self.config.worker_config.concurrency,
                queue_depth: 0,
                health: HealthStatus::Healthy,
                tags: self.config.tags.clone(),
                last_heartbeat_ms: aspen_coordination::now_unix_ms(),
                started_at_ms: aspen_coordination::now_unix_ms(),
                total_processed: 0,
                total_failed: 0,
                avg_processing_time_ms: 0,
                groups: HashSet::new(),
            };

            self.coordinator
                .register_worker(worker_info)
                .await
                .map_err(|e| JobError::WorkerRegistrationFailed {
                    reason: format!("failed to register worker: {}", e),
                })?;

            // Track registration
            let mut registrations = self.registrations.write().await;
            registrations.insert(worker_id.clone(), WorkerRegistration {
                worker_id,
                registered_at: Utc::now(),
                last_heartbeat: Utc::now(),
            });
        }

        // Start background tasks
        self.start_background_tasks().await?;

        info!(
            node_id = %self.config.node_id,
            "distributed worker pool started"
        );
        Ok(())
    }

    /// Start background tasks for coordination.
    async fn start_background_tasks(&self) -> Result<()> {
        let mut tasks = self.tasks.write().await;

        // Heartbeat task
        let pool = self.pool.clone();
        let coordinator = self.coordinator.clone();
        let registrations = self.registrations.clone();
        let shutdown = self.shutdown.clone();
        let node_id = self.config.node_id.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Send heartbeats for all workers
                        let worker_info = pool.get_worker_info().await;
                        let regs = registrations.read().await;

                        for info in worker_info {
                            if let Some(_reg) = regs.get(&info.id) {
                                let stats = WorkerStats {
                                    load: info.jobs_processed as f32 / 100.0,  // Simplified
                                    active_jobs: if info.status == WorkerStatus::Processing { 1 } else { 0 },
                                    queue_depth: 0,  // Would need queue depth tracking
                                    total_processed: info.jobs_processed,
                                    total_failed: info.jobs_failed,
                                    avg_processing_time_ms: 50,  // Would need timing tracking
                                    health: match info.status {
                                        WorkerStatus::Failed(_) => HealthStatus::Unhealthy,
                                        _ => HealthStatus::Healthy,
                                    },
                                };

                                if let Err(e) = coordinator.heartbeat(&info.id, stats).await {
                                    warn!(
                                        worker_id = %info.id,
                                        error = %e,
                                        "failed to send heartbeat"
                                    );
                                }
                            }
                        }
                    }
                    _ = shutdown.notified() => {
                        debug!(node_id, "heartbeat task shutting down");
                        break;
                    }
                }
            }
        });
        tasks.push(handle);

        // Work stealing task
        if self.config.enable_work_stealing {
            let manager = self.manager.clone();
            let coordinator = self.coordinator.clone();
            let node_id = self.config.node_id.clone();
            let shutdown = self.shutdown.clone();
            let interval = self.config.steal_check_interval;

            let handle = tokio::spawn(async move {
                let mut check_interval = tokio::time::interval(interval);

                loop {
                    tokio::select! {
                        _ = check_interval.tick() => {
                            if let Err(e) = perform_work_stealing(
                                &manager,
                                &coordinator,
                                &node_id,
                            ).await {
                                error!(
                                    node_id,
                                    error = %e,
                                    "work stealing failed"
                                );
                            }
                        }
                        _ = shutdown.notified() => {
                            debug!(node_id, "work stealing task shutting down");
                            break;
                        }
                    }
                }
            });
            tasks.push(handle);
        }

        Ok(())
    }

    /// Get a distributed job router that uses the coordinator for job placement.
    pub fn get_router(&self) -> DistributedJobRouter<S> {
        DistributedJobRouter {
            manager: self.manager.clone(),
            coordinator: self.coordinator.clone(),
            config: self.config.clone(),
        }
    }

    /// Create a worker group for coordinated tasks.
    pub async fn create_worker_group(
        &self,
        group_id: String,
        required_capabilities: Vec<String>,
        min_members: usize,
    ) -> Result<WorkerGroupHandle> {
        // Find eligible workers
        let filter = WorkerFilter {
            health: Some(HealthStatus::Healthy),
            capability: required_capabilities.first().cloned(),
            node_id: Some(self.config.node_id.clone()),
            tags: None,
            max_load: Some(0.8),
        };

        let workers = self.coordinator.get_workers(filter).await?;

        if workers.len() < min_members {
            return Err(JobError::WorkerRegistrationFailed {
                reason: format!("insufficient workers: need {}, have {}", min_members, workers.len()),
            });
        }

        // Create group in coordinator
        let members: HashSet<_> = workers.iter().take(min_members).map(|w| w.worker_id.clone()).collect();

        let group = WorkerGroup {
            group_id: group_id.clone(),
            description: "Coordinated task group".to_string(),
            members: members.clone(),
            leader: members.iter().next().cloned(),
            required_capabilities,
            min_members,
            max_members: min_members * 2,
            created_at_ms: aspen_coordination::now_unix_ms(),
            state: GroupState::Active,
        };

        self.coordinator.create_group(group).await?;

        // Create local group handle
        let (tx, _) = tokio::sync::broadcast::channel(100);
        let handle = WorkerGroupHandle {
            group_id: group_id.clone(),
            members,
            barrier: Arc::new(tokio::sync::Barrier::new(min_members)),
            broadcast: tx,
        };

        let mut groups = self.groups.write().await;
        groups.insert(group_id, handle.clone());

        Ok(handle)
    }

    /// Submit a job to a specific worker group.
    pub async fn submit_to_group(&self, group_id: &str, job_specs: Vec<crate::JobSpec>) -> Result<Vec<JobId>> {
        let groups = self.groups.read().await;
        let group = groups.get(group_id).ok_or_else(|| JobError::InvalidJobSpec {
            reason: format!("group {} not found", group_id),
        })?;

        if job_specs.len() != group.members.len() {
            return Err(JobError::InvalidJobSpec {
                reason: format!("job count {} doesn't match group size {}", job_specs.len(), group.members.len()),
            });
        }

        // Submit jobs with worker affinity
        let mut job_ids = Vec::new();
        for (spec, worker_id) in job_specs.into_iter().zip(group.members.iter()) {
            // Add worker affinity metadata
            let mut spec = spec;
            spec.metadata.insert("group_id".to_string(), group_id.to_string());
            spec.metadata.insert("worker_affinity".to_string(), worker_id.clone());

            let job_id = self.manager.submit(spec).await?;
            job_ids.push(job_id);
        }

        // Notify group members
        let _ = group.broadcast.send(GroupMessage::StartPhase {
            phase: "processing".to_string(),
        });

        Ok(job_ids)
    }

    /// Wait for all workers in a group to reach a barrier.
    pub async fn group_barrier(&self, group_id: &str) -> Result<()> {
        let groups = self.groups.read().await;
        let group = groups.get(group_id).ok_or_else(|| JobError::InvalidJobSpec {
            reason: format!("group {} not found", group_id),
        })?;

        group.barrier.wait().await;
        Ok(())
    }

    /// Shutdown the distributed pool.
    pub async fn shutdown(&self) -> Result<()> {
        info!(node_id = %self.config.node_id, "shutting down distributed pool");

        // Signal shutdown
        self.shutdown.notify_waiters();

        // Wait for background tasks
        let mut tasks = self.tasks.write().await;
        for handle in tasks.drain(..) {
            let _ = handle.await;
        }

        // Deregister all workers
        let registrations = self.registrations.read().await;
        for reg in registrations.values() {
            if let Err(e) = self.coordinator.deregister_worker(&reg.worker_id).await {
                warn!(
                    worker_id = %reg.worker_id,
                    error = %e,
                    "failed to deregister worker"
                );
            }
        }

        // Shutdown coordinator and pool
        self.coordinator.stop().await?;
        self.pool.shutdown().await?;

        info!(node_id = %self.config.node_id, "distributed pool shut down");
        Ok(())
    }
}

/// Router that uses distributed coordinator for job placement.
pub struct DistributedJobRouter<S: KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    coordinator: Arc<DistributedWorkerCoordinator<S>>,
    #[allow(dead_code)] // Reserved for future routing configuration
    config: DistributedPoolConfig,
}

impl<S: KeyValueStore + ?Sized + 'static> DistributedJobRouter<S> {
    /// Route a job to the best available worker across the cluster.
    pub async fn route_job(&self, spec: crate::JobSpec) -> Result<JobId> {
        // Build routing context
        let context = RoutingContext {
            affinity_key: spec.metadata.get("affinity_key").cloned(),
            required_tags: spec
                .metadata
                .get("required_tags")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            preferred_node: spec.metadata.get("preferred_node").cloned(),
            priority: match spec.config.priority {
                Priority::Critical => Some(RoutingPriority::Critical),
                Priority::High => Some(RoutingPriority::High),
                Priority::Normal => Some(RoutingPriority::Normal),
                Priority::Low => Some(RoutingPriority::Low),
            },
            estimated_size: None,
            metadata: spec.metadata.clone(),
        };

        // Select worker
        let worker =
            self.coordinator.select_worker(&spec.job_type, context.affinity_key.as_deref()).await?.ok_or_else(
                || JobError::NoWorkersAvailable {
                    job_type: spec.job_type.clone(),
                },
            )?;

        debug!(
            job_type = %spec.job_type,
            worker_id = %worker.worker_id,
            node_id = %worker.node_id,
            load = worker.load,
            "routed job to worker"
        );

        // Add routing metadata
        let mut spec = spec;
        spec.metadata.insert("routed_worker".to_string(), worker.worker_id);
        spec.metadata.insert("routed_node".to_string(), worker.node_id);

        // Submit job
        self.manager.submit(spec).await
    }

    /// Get cluster-wide job statistics.
    pub async fn get_cluster_stats(&self) -> Result<ClusterJobStats> {
        let workers = self.coordinator.get_workers(WorkerFilter::default()).await?;

        let total_workers = workers.len();
        let healthy_workers = workers.iter().filter(|w| w.health == HealthStatus::Healthy).count();
        let total_capacity: usize = workers.iter().map(|w| w.max_concurrent).sum();
        let total_active: usize = workers.iter().map(|w| w.active_jobs).sum();
        let total_queued: usize = workers.iter().map(|w| w.queue_depth).sum();
        let avg_load = if !workers.is_empty() {
            workers.iter().map(|w| w.load).sum::<f32>() / workers.len() as f32
        } else {
            0.0
        };

        let nodes: HashSet<_> = workers.iter().map(|w| w.node_id.clone()).collect();

        Ok(ClusterJobStats {
            total_workers,
            healthy_workers,
            total_capacity,
            total_active,
            total_queued,
            avg_load,
            nodes: nodes.len(),
        })
    }
}

/// Cluster-wide job statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterJobStats {
    /// Total number of workers across the cluster.
    pub total_workers: usize,
    /// Number of healthy workers.
    pub healthy_workers: usize,
    /// Total capacity across all workers.
    pub total_capacity: usize,
    /// Total active jobs currently processing.
    pub total_active: usize,
    /// Total jobs queued for processing.
    pub total_queued: usize,
    /// Average load across workers (0.0-1.0).
    pub avg_load: f32,
    /// Number of nodes in the cluster.
    pub nodes: usize,
}

/// Perform work stealing from overloaded workers.
async fn perform_work_stealing<S: KeyValueStore + ?Sized + 'static>(
    _manager: &JobManager<S>,
    coordinator: &DistributedWorkerCoordinator<S>,
    node_id: &str,
) -> Result<()> {
    // Find local workers that can steal
    let targets = coordinator.find_steal_targets().await?;
    let local_targets: Vec<_> = targets.into_iter().filter(|w| w.node_id == node_id).collect();

    if local_targets.is_empty() {
        return Ok(());
    }

    // Find overloaded sources
    let sources = coordinator.find_steal_sources().await?;

    for target in local_targets.iter().take(2) {
        // Limit stealing rounds
        if let Some(source) = sources
            .iter()
            .filter(|s| s.node_id != node_id) // Prefer cross-node stealing
            .max_by_key(|s| s.queue_depth)
        {
            debug!(
                target = %target.worker_id,
                source = %source.worker_id,
                "attempting work stealing"
            );

            // Note: Actual job stealing would require:
            // 1. Query source node for stealable jobs
            // 2. Transfer jobs to local queue
            // 3. Update job metadata with new assignment
            // This is a simplified placeholder

            info!(
                target = %target.worker_id,
                source = %source.worker_id,
                "work stealing opportunity identified"
            );
        }
    }

    Ok(())
}

impl WorkerGroupHandle {
    /// Get the group ID.
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    /// Get group members.
    pub fn members(&self) -> &HashSet<String> {
        &self.members
    }

    /// Wait for all members to reach the barrier.
    pub async fn barrier(&self) {
        self.barrier.wait().await;
    }

    /// Broadcast a message to all group members.
    pub fn broadcast(&self, message: GroupMessage) -> Result<()> {
        self.broadcast.send(message).map_err(|_| JobError::WorkerCommunicationFailed {
            reason: "failed to broadcast to group".to_string(),
        })?;
        Ok(())
    }

    /// Subscribe to group messages.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<GroupMessage> {
        self.broadcast.subscribe()
    }
}

/// Extension trait for Job to support distributed features.
pub trait DistributedJobExt {
    /// Set worker affinity for the job.
    fn with_affinity(self, key: String) -> Self;

    /// Set required node for the job.
    fn with_node(self, node_id: String) -> Self;

    /// Set required tags for worker selection.
    fn with_tags(self, tags: Vec<String>) -> Self;
}

impl DistributedJobExt for crate::JobSpec {
    fn with_affinity(mut self, key: String) -> Self {
        self.metadata.insert("affinity_key".to_string(), key);
        self
    }

    fn with_node(mut self, node_id: String) -> Self {
        self.metadata.insert("preferred_node".to_string(), node_id);
        self
    }

    fn with_tags(mut self, tags: Vec<String>) -> Self {
        if let Ok(tags_json) = serde_json::to_string(&tags) {
            self.metadata.insert("required_tags".to_string(), tags_json);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distributed_job_extensions() {
        let mut spec = crate::JobSpec::new("test_job");
        spec.metadata.insert("affinity_key".to_string(), "user123".to_string());
        spec = spec.with_node("node1".to_string());
        spec = spec.with_tags(vec!["gpu".to_string(), "ml".to_string()]);

        assert_eq!(spec.metadata.get("affinity_key"), Some(&"user123".to_string()));
        assert_eq!(spec.metadata.get("preferred_node"), Some(&"node1".to_string()));

        let tags: Vec<String> = serde_json::from_str(spec.metadata.get("required_tags").unwrap()).unwrap();
        assert_eq!(tags, vec!["gpu", "ml"]);
    }
}
