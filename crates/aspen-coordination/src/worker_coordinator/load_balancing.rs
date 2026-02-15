//! Load balancing strategies for worker selection.

use anyhow::Result;
use aspen_traits::KeyValueStore;

use super::DistributedWorkerCoordinator;
use super::types::LoadBalancingStrategy;
use super::types::WorkerInfo;
use crate::registry::HealthStatus;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
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
                    let hash = verified::simple_hash(key) as usize;
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
                let hash = verified::simple_hash(key) as usize;
                eligible.get(hash % eligible.len()).cloned()
            }
            LoadBalancingStrategy::WorkStealing => {
                // For work stealing, prefer least loaded but consider queue depth
                eligible.into_iter().min_by_key(|w| (w.queue_depth, (w.load * 1000.0) as u32))
            }
        };

        if let Some(ref w) = selected {
            debug_assert!(
                w.health == HealthStatus::Healthy,
                "WORKER: selected worker '{}' must be healthy",
                w.worker_id
            );
            debug_assert!(
                w.is_alive(self.config.heartbeat_timeout_ms),
                "WORKER: selected worker '{}' must be alive",
                w.worker_id
            );
        }

        Ok(selected)
    }
}
