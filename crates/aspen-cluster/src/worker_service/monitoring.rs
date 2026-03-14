//! Worker health monitoring and metadata updates.

use std::sync::Arc;
use std::time::Duration;

use aspen_coordination::verified::worker_stats_key;
use aspen_core::KeyValueStore;
use aspen_core::WriteRequest;
use aspen_jobs::WorkerMetadata;
use aspen_jobs::WorkerPoolStats;
use tracing::info;
use tracing::warn;

use super::types::Result;
use super::types::WorkerService;

impl WorkerService {
    /// Update worker metadata for P2P affinity routing.
    pub(super) async fn update_worker_metadata(&self) -> Result<()> {
        let metadata = WorkerMetadata {
            id: format!("node-{}", self.node_id),
            node_id: self.iroh_node_id,
            tags: self.config.tags.clone(),
            region: None,        // Could be configured later
            load: 0.0,           // Will be updated by monitoring
            local_blobs: vec![], // Could query blob store
            latencies: Default::default(),
            local_shards: vec![], // Will be populated when sharding is enabled
        };

        self.affinity_manager.update_worker_metadata(metadata).await;

        Ok(())
    }

    /// Write worker stats (with PSI pressure) to KV for a single worker.
    async fn write_worker_stats_to_kv(
        store: &Arc<dyn KeyValueStore>,
        worker_id: &str,
        node_id: u64,
        load: f32,
        pool_stats: &WorkerPoolStats,
    ) {
        let pressure = aspen_jobs::read_psi_pressure();
        let disk =
            aspen_jobs::read_disk_free(Some(std::path::Path::new("/tmp")), Some(std::path::Path::new("/nix/store")));

        let stats = serde_json::json!({
            "worker_id": worker_id,
            "node_id": format!("node-{}", node_id),
            "load": load,
            "idle_workers": pool_stats.idle_workers,
            "processing_workers": pool_stats.processing_workers,
            "total_processed": pool_stats.total_jobs_processed,
            "total_failed": pool_stats.total_jobs_failed,
            "cpu_pressure_avg10": pressure.cpu_avg10,
            "memory_pressure_avg10": pressure.memory_avg10,
            "io_pressure_avg10": pressure.io_avg10,
            "disk_free_build_pct": disk.build_dir_free_pct,
            "disk_free_store_pct": disk.store_dir_free_pct,
        });

        let key = worker_stats_key(worker_id);
        if let Ok(value) = serde_json::to_string(&stats) {
            if let Err(e) = store.write(WriteRequest::set(key, value)).await {
                warn!(worker_id, error = %e, "failed to write worker stats to KV");
            }
        }
    }

    /// Start monitoring task for worker health and metrics.
    pub(super) fn start_monitoring(&mut self) {
        let pool = self.pool.clone();
        let affinity_manager = self.affinity_manager.clone();
        let store = self.store.clone();
        let node_id = self.node_id;
        let iroh_node_id = self.iroh_node_id;
        let tags = self.config.tags.clone();
        let worker_count = self.config.worker_count;
        let shutdown = self.shutdown.clone();

        let handle = self.task_tracker.spawn(async move {
            // Write initial stats immediately, then every 10 seconds.
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Get worker stats
                        let stats = pool.get_stats().await;

                        // Calculate load (ratio of processing to total workers)
                        let load = if stats.total_workers > 0 {
                            stats.processing_workers as f32 / stats.total_workers as f32
                        } else {
                            0.0
                        };

                        // Update metadata with current load
                        let metadata = WorkerMetadata {
                            id: format!("node-{}", node_id),
                            node_id: iroh_node_id,
                            tags: tags.clone(),
                            region: None,
                            load,
                            local_blobs: vec![],
                            latencies: Default::default(),
                            local_shards: vec![],
                        };

                        affinity_manager.update_worker_metadata(metadata).await;

                        // Write per-worker stats to KV with PSI data
                        for i in 0..worker_count {
                            let worker_id = format!("node-{}-worker-{}", node_id, i);
                            Self::write_worker_stats_to_kv(
                                &store, &worker_id, node_id, load, &stats,
                            )
                            .await;
                        }

                        // Log stats
                        info!(
                            node_id,
                            idle = stats.idle_workers,
                            processing = stats.processing_workers,
                            failed = stats.failed_workers,
                            jobs_processed = stats.total_jobs_processed,
                            jobs_failed = stats.total_jobs_failed,
                            load = format!("{:.2}%", load * 100.0),
                            "worker service stats"
                        );
                    }
                    _ = shutdown.notified() => {
                        info!(node_id, "worker monitoring task shutting down");
                        break;
                    }
                }
            }
        });

        self.monitor_handle = Some(handle);
    }
}
