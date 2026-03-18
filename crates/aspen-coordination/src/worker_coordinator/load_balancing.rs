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
        // Tiger Style: job_type must not be empty
        debug_assert!(!job_type.is_empty(), "WORKER: job_type must not be empty");

        let workers = self.workers.read().await;

        // Filter to healthy, alive workers that can handle the job
        let eligible: Vec<_> = workers
            .values()
            .filter(|w| {
                w.health == HealthStatus::Healthy
                    && w.is_alive(self.config.heartbeat_timeout_ms)
                    && w.can_handle(job_type)
                    && w.is_pressure_ok(&self.config.pressure_thresholds)
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
                let eligible_count = eligible.len().min(u32::MAX as usize) as u32;
                let idx = (*counter % eligible_count) as usize;
                *counter = (*counter + 1) % eligible_count;
                eligible.get(idx).cloned()
            }
            LoadBalancingStrategy::LeastLoaded => eligible.into_iter().max_by(|a, b| {
                a.available_capacity().partial_cmp(&b.available_capacity()).unwrap_or(std::cmp::Ordering::Equal)
            }),
            LoadBalancingStrategy::Affinity => {
                if let Some(key) = affinity_key {
                    // Simple hash-based affinity
                    let hash = verified::simple_hash(key);
                    eligible.get((hash as usize) % eligible.len()).cloned()
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
                let hash = verified::simple_hash(key);
                eligible.get((hash as usize) % eligible.len()).cloned()
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::DistributedWorkerCoordinator;
    use super::super::types::WorkerCoordinatorConfig;
    use super::*;
    use crate::registry::HealthStatus;
    use crate::types::now_unix_ms;
    use crate::verified::worker::PressureThresholds;

    fn create_test_worker_with_pressure(
        id: &str,
        cpu_pressure: f32,
        memory_pressure: f32,
        io_pressure: f32,
        disk_build_free: f64,
        disk_store_free: f64,
    ) -> WorkerInfo {
        WorkerInfo {
            worker_id: id.to_string(),
            node_id: "n1".to_string(),
            peer_id: None,
            capabilities: vec!["test".to_string()],
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
            cpu_pressure_avg10: cpu_pressure,
            memory_pressure_avg10: memory_pressure,
            io_pressure_avg10: io_pressure,
            disk_free_build_pct: disk_build_free,
            disk_free_store_pct: disk_store_free,
            is_ready: true,
        }
    }

    #[tokio::test]
    async fn test_pressure_filtering_excludes_high_pressure_worker() {
        let store = aspen_testing::DeterministicKeyValueStore::new();
        let config = WorkerCoordinatorConfig {
            pressure_thresholds: PressureThresholds {
                cpu_psi_max: 50.0,
                memory_psi_max: 50.0,
                io_psi_max: 50.0,
                disk_free_build_min_pct: 10.0,
                disk_free_store_min_pct: 10.0,
            },
            ..Default::default()
        };
        let coordinator = DistributedWorkerCoordinator::with_config(store, config);

        // Add one healthy low-pressure worker
        let good_worker = create_test_worker_with_pressure("good", 10.0, 10.0, 10.0, 50.0, 50.0);
        // Add one worker with high CPU pressure (exceeds 50.0 threshold)
        let bad_worker = create_test_worker_with_pressure("bad", 80.0, 10.0, 10.0, 50.0, 50.0);

        {
            let mut workers = coordinator.workers.write().await;
            workers.insert(good_worker.worker_id.clone(), good_worker.clone());
            workers.insert(bad_worker.worker_id.clone(), bad_worker.clone());
        }

        // Select worker should only return the good worker (bad worker filtered out by pressure)
        let selected = coordinator.select_worker("test", None).await.unwrap();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().worker_id, "good");
    }

    #[tokio::test]
    async fn test_pressure_filtering_includes_worker_after_pressure_decreases() {
        let store = aspen_testing::DeterministicKeyValueStore::new();
        let config = WorkerCoordinatorConfig {
            pressure_thresholds: PressureThresholds {
                cpu_psi_max: 50.0,
                memory_psi_max: 50.0,
                io_psi_max: 50.0,
                disk_free_build_min_pct: 10.0,
                disk_free_store_min_pct: 10.0,
            },
            ..Default::default()
        };
        let coordinator = DistributedWorkerCoordinator::with_config(store, config);

        // Add worker with high CPU pressure initially
        let mut worker = create_test_worker_with_pressure("w1", 80.0, 10.0, 10.0, 50.0, 50.0);

        {
            let mut workers = coordinator.workers.write().await;
            workers.insert(worker.worker_id.clone(), worker.clone());
        }

        // Worker should be excluded due to high pressure
        let selected = coordinator.select_worker("test", None).await.unwrap();
        assert!(selected.is_none());

        // Update worker stats to have lower pressure (below threshold)
        worker.cpu_pressure_avg10 = 20.0;
        {
            let mut workers = coordinator.workers.write().await;
            workers.insert(worker.worker_id.clone(), worker.clone());
        }

        // Worker should now be eligible for selection
        let selected = coordinator.select_worker("test", None).await.unwrap();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().worker_id, "w1");
    }
}
