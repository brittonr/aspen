//! Background task management and failover monitoring.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::DistributedWorkerCoordinator;

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
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
}
