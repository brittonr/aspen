//! Worker registration, heartbeat, deregistration, and querying.

use std::collections::HashMap;

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::info;
use tracing::warn;

use super::DistributedWorkerCoordinator;
use super::types::WorkerFilter;
use super::types::WorkerInfo;
use super::types::WorkerStats;
use crate::registry::RegisterOptions;
use crate::registry::ServiceInstanceMetadata;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
    /// Register a worker with the coordinator.
    ///
    /// Uses optimistic check with re-validation under write lock to prevent
    /// TOCTOU race conditions that could exceed MAX_WORKERS.
    pub async fn register_worker(&self, info: WorkerInfo) -> Result<()> {
        assert!(info.max_concurrent > 0, "WORKER: max_concurrent must be > 0 for worker '{}'", info.worker_id);
        assert!(!info.worker_id.is_empty(), "WORKER: worker_id must not be empty");

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
        let key = verified::worker_stats_key(&info.worker_id);
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
        assert!(
            (0.0..=1.0).contains(&stats.load),
            "WORKER: heartbeat load must be in [0.0, 1.0], got {} for worker '{worker_id}'",
            stats.load
        );
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
            let key = verified::worker_stats_key(worker_id);
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
        let key = verified::worker_stats_key(worker_id);
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
}
