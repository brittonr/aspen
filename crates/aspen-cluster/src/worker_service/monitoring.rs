//! Worker health monitoring and metadata updates.

use std::time::Duration;

use aspen_jobs::WorkerMetadata;
use tracing::info;

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

    /// Start monitoring task for worker health and metrics.
    pub(super) fn start_monitoring(&mut self) {
        let pool = self.pool.clone();
        let affinity_manager = self.affinity_manager.clone();
        let node_id = self.node_id;
        let iroh_node_id = self.iroh_node_id;
        let tags = self.config.tags.clone();
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

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
                            local_blobs: vec![], // Could query blob store periodically
                            latencies: Default::default(),
                            local_shards: vec![], // Will be populated when sharding is enabled
                        };

                        affinity_manager.update_worker_metadata(metadata).await;

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
