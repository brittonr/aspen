//! Worker trait and pool implementation for job execution.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::error::{JobError, Result};
use crate::job::{Job, JobResult};
use crate::manager::JobManager;

/// Trait for implementing job workers.
#[async_trait]
pub trait Worker: Send + Sync + 'static {
    /// Execute a job and return the result.
    async fn execute(&self, job: Job) -> JobResult;

    /// Called before the worker starts processing jobs.
    async fn on_start(&self) -> Result<()> {
        Ok(())
    }

    /// Called when the worker is shutting down.
    async fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Get the job types this worker can handle.
    /// If empty, handles all job types.
    fn job_types(&self) -> Vec<String> {
        vec![]
    }

    /// Check if the worker can handle a specific job type.
    fn can_handle(&self, job_type: &str) -> bool {
        let types = self.job_types();
        types.is_empty() || types.iter().any(|t| t == job_type)
    }
}

/// Status of a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    /// Worker is starting up.
    Starting,
    /// Worker is idle and ready for jobs.
    Idle,
    /// Worker is currently processing a job.
    Processing,
    /// Worker is shutting down.
    Stopping,
    /// Worker has stopped.
    Stopped,
    /// Worker has failed.
    Failed(String),
}

/// Information about a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Unique worker ID.
    pub id: String,
    /// Current status.
    pub status: WorkerStatus,
    /// Job types the worker handles.
    pub job_types: Vec<String>,
    /// Currently processing job ID.
    pub current_job: Option<String>,
    /// Total jobs processed.
    pub jobs_processed: u64,
    /// Total jobs failed.
    pub jobs_failed: u64,
    /// Worker started at.
    pub started_at: DateTime<Utc>,
    /// Last heartbeat.
    pub last_heartbeat: DateTime<Utc>,
}

/// Configuration for a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Worker ID (auto-generated if not provided).
    pub id: Option<String>,
    /// Maximum concurrent jobs.
    pub concurrency: usize,
    /// Heartbeat interval.
    pub heartbeat_interval: Duration,
    /// Graceful shutdown timeout.
    pub shutdown_timeout: Duration,
    /// Whether to dequeue jobs immediately or wait.
    pub poll_interval: Duration,
    /// Job types to handle (empty = all).
    pub job_types: Vec<String>,
    /// Visibility timeout for dequeued jobs.
    pub visibility_timeout: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            id: None,
            concurrency: 1,
            heartbeat_interval: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(60),
            poll_interval: Duration::from_millis(100),
            job_types: vec![],
            visibility_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Pool of workers for processing jobs.
pub struct WorkerPool<S: aspen_core::KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    workers: Arc<RwLock<HashMap<String, Arc<dyn Worker>>>>,
    worker_info: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    shutdown: Arc<RwLock<bool>>,
    concurrency_limiter: Arc<Semaphore>,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> WorkerPool<S> {
    /// Create a new worker pool.
    pub fn new(store: Arc<S>) -> Self {
        let manager = Arc::new(JobManager::new(store));
        Self::with_manager(manager)
    }

    /// Create a worker pool with an existing manager.
    pub fn with_manager(manager: Arc<JobManager<S>>) -> Self {
        Self {
            manager,
            workers: Arc::new(RwLock::new(HashMap::new())),
            worker_info: Arc::new(RwLock::new(HashMap::new())),
            handles: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::new(RwLock::new(false)),
            concurrency_limiter: Arc::new(Semaphore::new(10)), // Default max concurrent jobs
        }
    }

    /// Register a worker handler for specific job types.
    pub async fn register_handler<W: Worker>(&self, job_type: &str, worker: W) -> Result<()> {
        let mut workers = self.workers.write().await;
        workers.insert(job_type.to_string(), Arc::new(worker));
        info!(job_type, "registered worker handler");
        Ok(())
    }

    /// Start the worker pool with the specified number of workers.
    pub async fn start(&self, num_workers: usize) -> Result<()> {
        if *self.shutdown.read().await {
            return Err(JobError::WorkerRegistrationFailed {
                reason: "worker pool is shutting down".to_string(),
            });
        }

        let mut handles = self.handles.write().await;

        for i in 0..num_workers {
            let worker_id = format!("worker-{}", i);
            let worker_config = WorkerConfig {
                id: Some(worker_id.clone()),
                ..Default::default()
            };

            let handle = self.spawn_worker(worker_config).await?;
            handles.push(handle);
        }

        info!(num_workers, "worker pool started");
        Ok(())
    }

    /// Spawn a worker with custom configuration.
    pub async fn spawn_worker(&self, config: WorkerConfig) -> Result<JoinHandle<()>> {
        let worker_id = config.id.clone().unwrap_or_else(|| format!("worker-{}", uuid::Uuid::new_v4()));

        // Create worker info
        let info = WorkerInfo {
            id: worker_id.clone(),
            status: WorkerStatus::Starting,
            job_types: config.job_types.clone(),
            current_job: None,
            jobs_processed: 0,
            jobs_failed: 0,
            started_at: Utc::now(),
            last_heartbeat: Utc::now(),
        };

        self.worker_info.write().await.insert(worker_id.clone(), info);

        // Spawn worker task
        let manager = self.manager.clone();
        let workers = self.workers.clone();
        let worker_info = self.worker_info.clone();
        let shutdown = self.shutdown.clone();
        let concurrency_limiter = self.concurrency_limiter.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) =
                run_worker(worker_id.clone(), config, manager, workers, worker_info, shutdown, concurrency_limiter)
                    .await
            {
                error!(worker_id, error = ?e, "worker failed");
            }
        });

        Ok(handle)
    }

    /// Stop all workers gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        info!("shutting down worker pool");
        *self.shutdown.write().await = true;

        // Wait for all workers to finish
        let mut handles = self.handles.write().await;
        for handle in handles.drain(..) {
            let _ = handle.await;
        }

        info!("worker pool shutdown complete");
        Ok(())
    }

    /// Get information about all workers.
    pub async fn get_worker_info(&self) -> Vec<WorkerInfo> {
        self.worker_info.read().await.values().cloned().collect()
    }

    /// Get statistics about the worker pool.
    pub async fn get_stats(&self) -> WorkerPoolStats {
        let info = self.worker_info.read().await;

        let total = info.len();
        let idle = info.values().filter(|w| w.status == WorkerStatus::Idle).count();
        let processing = info.values().filter(|w| w.status == WorkerStatus::Processing).count();
        let failed = info.values().filter(|w| matches!(w.status, WorkerStatus::Failed(_))).count();

        let total_processed: u64 = info.values().map(|w| w.jobs_processed).sum();
        let total_failed: u64 = info.values().map(|w| w.jobs_failed).sum();

        WorkerPoolStats {
            total_workers: total,
            idle_workers: idle,
            processing_workers: processing,
            failed_workers: failed,
            total_jobs_processed: total_processed,
            total_jobs_failed: total_failed,
        }
    }
}

/// Statistics about the worker pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolStats {
    /// Total number of workers.
    pub total_workers: usize,
    /// Number of idle workers.
    pub idle_workers: usize,
    /// Number of workers processing jobs.
    pub processing_workers: usize,
    /// Number of failed workers.
    pub failed_workers: usize,
    /// Total jobs processed by all workers.
    pub total_jobs_processed: u64,
    /// Total jobs failed across all workers.
    pub total_jobs_failed: u64,
}

/// Run a worker loop.
async fn run_worker<S: aspen_core::KeyValueStore + ?Sized + 'static>(
    worker_id: String,
    config: WorkerConfig,
    manager: Arc<JobManager<S>>,
    workers: Arc<RwLock<HashMap<String, Arc<dyn Worker>>>>,
    worker_info: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    shutdown: Arc<RwLock<bool>>,
    concurrency_limiter: Arc<Semaphore>,
) -> Result<()> {
    info!(worker_id, "worker starting");

    // Update status to idle
    {
        let mut info = worker_info.write().await;
        if let Some(w) = info.get_mut(&worker_id) {
            w.status = WorkerStatus::Idle;
            w.last_heartbeat = Utc::now();
        }
    }

    // Worker loop
    loop {
        // Check shutdown
        if *shutdown.read().await {
            info!(worker_id, "worker shutting down");
            break;
        }

        // Try to dequeue jobs
        match manager.dequeue_jobs(&worker_id, 1, config.visibility_timeout).await {
            Ok(jobs) => {
                if jobs.is_empty() {
                    // No jobs available, wait before polling again
                    tokio::time::sleep(config.poll_interval).await;
                } else {
                    // Process the job
                    for (queue_item, job) in jobs {
                        // Acquire concurrency permit
                        let _permit = concurrency_limiter.acquire().await.unwrap();

                        // Update worker status
                        {
                            let mut info = worker_info.write().await;
                            if let Some(w) = info.get_mut(&worker_id) {
                                w.status = WorkerStatus::Processing;
                                w.current_job = Some(job.id.to_string());
                                w.last_heartbeat = Utc::now();
                            }
                        }

                        // Find appropriate worker handler
                        let handler = {
                            let workers = workers.read().await;
                            workers
                                .iter()
                                .find(|(_, w)| w.can_handle(&job.spec.job_type))
                                .map(|(_, w)| w.clone())
                                .or_else(|| workers.get(&job.spec.job_type).cloned())
                        };

                        if let Some(handler) = handler {
                            // Mark job as started
                            if let Err(e) = manager.mark_started(&job.id, worker_id.clone()).await {
                                error!(
                                    worker_id,
                                    job_id = %job.id,
                                    error = ?e,
                                    "failed to mark job as started"
                                );
                                continue;
                            }

                            // Execute job with timeout
                            let job_timeout = job.spec.config.timeout.unwrap_or(Duration::from_secs(300));

                            let job_id = job.id.clone();
                            let result = match timeout(job_timeout, handler.execute(job)).await {
                                Ok(result) => result,
                                Err(_) => {
                                    warn!(
                                        worker_id,
                                        job_id = %job_id,
                                        "job timed out"
                                    );
                                    JobResult::failure("job execution timed out")
                                }
                            };

                            // Process result
                            if result.is_success() {
                                // Acknowledge successful completion
                                if let Err(e) = manager.ack_job(&job_id, &queue_item.receipt_handle, result).await {
                                    error!(
                                        worker_id,
                                        job_id = %job_id,
                                        error = ?e,
                                        "failed to acknowledge job"
                                    );
                                }

                                // Update worker stats
                                let mut info = worker_info.write().await;
                                if let Some(w) = info.get_mut(&worker_id) {
                                    w.jobs_processed += 1;
                                }

                                info!(
                                    worker_id,
                                    job_id = %job_id,
                                    "job completed successfully"
                                );
                            } else {
                                // Handle failure
                                let error_msg = match &result {
                                    JobResult::Failure(f) => f.reason.clone(),
                                    _ => "unknown error".to_string(),
                                };

                                if let Err(e) =
                                    manager.nack_job(&job_id, &queue_item.receipt_handle, error_msg.clone()).await
                                {
                                    error!(
                                        worker_id,
                                        job_id = %job_id,
                                        error = ?e,
                                        "failed to nack job"
                                    );
                                }

                                // Update worker stats
                                let mut info = worker_info.write().await;
                                if let Some(w) = info.get_mut(&worker_id) {
                                    w.jobs_failed += 1;
                                }

                                warn!(
                                    worker_id,
                                    job_id = %job_id,
                                    error = error_msg,
                                    "job failed"
                                );
                            }
                        } else {
                            warn!(
                                worker_id,
                                job_id = %job.id,
                                job_type = job.spec.job_type,
                                "no handler found for job type"
                            );

                            // Nack the job since we can't handle it
                            if let Err(e) = manager
                                .nack_job(
                                    &job.id,
                                    &queue_item.receipt_handle,
                                    format!("no handler for job type: {}", job.spec.job_type),
                                )
                                .await
                            {
                                error!(
                                    worker_id,
                                    job_id = %job.id,
                                    error = ?e,
                                    "failed to nack unhandled job"
                                );
                            }
                        }

                        // Update worker status back to idle
                        {
                            let mut info = worker_info.write().await;
                            if let Some(w) = info.get_mut(&worker_id) {
                                w.status = WorkerStatus::Idle;
                                w.current_job = None;
                                w.last_heartbeat = Utc::now();
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!(
                    worker_id,
                    error = ?e,
                    "failed to dequeue jobs, retrying"
                );
                tokio::time::sleep(config.poll_interval).await;
            }
        }

        // Update heartbeat
        {
            let mut info = worker_info.write().await;
            if let Some(w) = info.get_mut(&worker_id) {
                w.last_heartbeat = Utc::now();
            }
        }
    }

    // Update status to stopped
    {
        let mut info = worker_info.write().await;
        if let Some(w) = info.get_mut(&worker_id) {
            w.status = WorkerStatus::Stopped;
        }
    }

    info!(worker_id, "worker stopped");
    Ok(())
}

/// A simple worker that logs the job.
pub struct LogWorker;

#[async_trait]
impl Worker for LogWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!(
            job_id = %job.id,
            job_type = %job.spec.job_type,
            "executing job"
        );

        // Simulate some work
        tokio::time::sleep(Duration::from_secs(1)).await;

        JobResult::success(serde_json::json!({
            "message": format!("Job {} completed", job.id)
        }))
    }
}
