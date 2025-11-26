//! Worker repository for managing worker lifecycle and state
//!
//! This module provides trait-based abstractions for worker management,
//! enabling dependency injection and testability in the domain layer.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::types::{Worker, WorkerHeartbeat, WorkerRegistration, WorkerStats, WorkerStatus};
use crate::hiqlite_service::HiqliteService;
use crate::params;

/// Repository abstraction for worker operations
///
/// Wraps HiqliteService to provide a testable interface for worker lifecycle operations.
/// All methods use domain types to maintain proper layer separation.
#[async_trait]
pub trait WorkerRepository: Send + Sync {
    /// Register a new worker in the cluster
    ///
    /// Creates a new worker record with a unique ID and initial heartbeat timestamp.
    ///
    /// # Arguments
    /// * `registration` - Worker registration information
    ///
    /// # Returns
    /// The newly created Worker record with generated ID
    async fn register(&self, registration: WorkerRegistration) -> Result<Worker>;

    /// Record a heartbeat from a worker
    ///
    /// Updates the worker's last_heartbeat timestamp and optionally updates
    /// resource information (CPU cores, memory, active jobs).
    ///
    /// # Arguments
    /// * `heartbeat` - Heartbeat update information
    async fn heartbeat(&self, heartbeat: WorkerHeartbeat) -> Result<()>;

    /// Find a specific worker by ID
    ///
    /// # Arguments
    /// * `worker_id` - The worker identifier to look up
    ///
    /// # Returns
    /// The worker if found, None otherwise
    async fn find_by_id(&self, worker_id: &str) -> Result<Option<Worker>>;

    /// List all online workers
    ///
    /// Returns workers with status = 'online'
    async fn list_online_workers(&self) -> Result<Vec<Worker>>;

    /// List all workers regardless of status
    async fn list_all_workers(&self) -> Result<Vec<Worker>>;

    /// Mark a worker as offline
    ///
    /// Called by the health monitor when a worker fails heartbeat checks.
    ///
    /// # Arguments
    /// * `worker_id` - The worker to mark offline
    async fn mark_offline(&self, worker_id: &str) -> Result<()>;

    /// Mark a worker as draining
    ///
    /// Used for graceful shutdown - worker stops accepting new jobs.
    ///
    /// # Arguments
    /// * `worker_id` - The worker to mark as draining
    async fn mark_draining(&self, worker_id: &str) -> Result<()>;

    /// Increment the active job count for a worker
    ///
    /// Called when a worker claims a job.
    ///
    /// # Arguments
    /// * `worker_id` - The worker that claimed the job
    async fn increment_job_count(&self, worker_id: &str) -> Result<()>;

    /// Decrement the active job count for a worker
    ///
    /// Called when a worker completes or fails a job.
    ///
    /// # Arguments
    /// * `worker_id` - The worker that finished the job
    async fn decrement_job_count(&self, worker_id: &str) -> Result<()>;

    /// Get aggregate statistics for the worker pool
    async fn get_stats(&self) -> Result<WorkerStats>;
}

/// Helper struct for database row deserialization
#[derive(Debug, serde::Deserialize)]
struct WorkerRow {
    id: String,
    worker_type: String,
    status: String,
    endpoint_id: String,
    registered_at: i64,
    last_heartbeat: i64,
    cpu_cores: Option<i64>,
    memory_mb: Option<i64>,
    active_jobs: i64,
    total_jobs_completed: i64,
    metadata: Option<String>,
}

impl From<hiqlite::Row<'static>> for WorkerRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            id: row.get("id"),
            worker_type: row.get("worker_type"),
            status: row.get("status"),
            endpoint_id: row.get("endpoint_id"),
            registered_at: row.get("registered_at"),
            last_heartbeat: row.get("last_heartbeat"),
            cpu_cores: row.get("cpu_cores"),
            memory_mb: row.get("memory_mb"),
            active_jobs: row.get("active_jobs"),
            total_jobs_completed: row.get("total_jobs_completed"),
            metadata: row.get("metadata"),
        }
    }
}

/// Helper to convert WorkerRow to domain Worker
fn row_to_worker(row: WorkerRow) -> Result<Worker> {
    let metadata = match row.metadata {
        Some(s) => serde_json::from_str(&s)?,
        None => serde_json::json!({}),
    };

    let worker_type = row.worker_type.parse()?;
    let status = row.status.parse()?;

    Ok(Worker {
        id: row.id,
        worker_type,
        status,
        endpoint_id: row.endpoint_id,
        registered_at: row.registered_at,
        last_heartbeat: row.last_heartbeat,
        cpu_cores: row.cpu_cores.map(|v| v as u32),
        memory_mb: row.memory_mb.map(|v| v as u64),
        active_jobs: row.active_jobs as u32,
        total_jobs_completed: row.total_jobs_completed as u64,
        metadata,
    })
}

/// Hiqlite-based implementation of WorkerRepository
///
/// Stores worker state in the distributed Hiqlite database with strong consistency.
pub struct HiqliteWorkerRepository {
    hiqlite: Arc<HiqliteService>,
}

impl HiqliteWorkerRepository {
    pub fn new(hiqlite: Arc<HiqliteService>) -> Self {
        Self { hiqlite }
    }
}

#[async_trait]
impl WorkerRepository for HiqliteWorkerRepository {
    async fn register(&self, registration: WorkerRegistration) -> Result<Worker> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Generate unique worker ID
        let worker_id = format!("worker-{}-{}", registration.worker_type, uuid::Uuid::new_v4());

        let metadata_str = serde_json::to_string(&registration.metadata)?;

        // Insert worker record
        self.hiqlite
            .execute(
                "INSERT INTO workers (
                    id, worker_type, status, endpoint_id,
                    registered_at, last_heartbeat,
                    cpu_cores, memory_mb,
                    active_jobs, total_jobs_completed, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                params![
                    &worker_id,
                    registration.worker_type.to_string(),
                    WorkerStatus::Online.to_string(),
                    &registration.endpoint_id,
                    now,
                    now,
                    registration.cpu_cores.map(|v| v as i64),
                    registration.memory_mb.map(|v| v as i64),
                    0_i64,  // active_jobs
                    0_i64,  // total_jobs_completed
                    &metadata_str,
                ],
            )
            .await?;

        tracing::info!(
            worker_id = %worker_id,
            worker_type = %registration.worker_type,
            endpoint_id = %registration.endpoint_id,
            "Worker registered"
        );

        // Return the created worker
        Ok(Worker {
            id: worker_id,
            worker_type: registration.worker_type,
            status: WorkerStatus::Online,
            endpoint_id: registration.endpoint_id,
            registered_at: now,
            last_heartbeat: now,
            cpu_cores: registration.cpu_cores,
            memory_mb: registration.memory_mb,
            active_jobs: 0,
            total_jobs_completed: 0,
            metadata: registration.metadata,
        })
    }

    async fn heartbeat(&self, heartbeat: WorkerHeartbeat) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.hiqlite
            .execute(
                "UPDATE workers
                 SET last_heartbeat = $1,
                     active_jobs = $2,
                     cpu_cores = COALESCE($3, cpu_cores),
                     memory_mb = COALESCE($4, memory_mb)
                 WHERE id = $5",
                params![
                    now,
                    heartbeat.active_jobs as i64,
                    heartbeat.cpu_cores.map(|v| v as i64),
                    heartbeat.memory_mb.map(|v| v as i64),
                    &heartbeat.worker_id,
                ],
            )
            .await?;

        tracing::debug!(
            worker_id = %heartbeat.worker_id,
            active_jobs = heartbeat.active_jobs,
            "Worker heartbeat recorded"
        );

        Ok(())
    }

    async fn find_by_id(&self, worker_id: &str) -> Result<Option<Worker>> {
        let rows: Vec<WorkerRow> = self
            .hiqlite
            .query_as(
                "SELECT * FROM workers WHERE id = $1",
                params![worker_id],
            )
            .await?;

        // Get the first row if it exists
        match rows.into_iter().next() {
            Some(row) => {
                let worker = row_to_worker(row)?;
                Ok(Some(worker))
            }
            None => Ok(None),
        }
    }

    async fn list_online_workers(&self) -> Result<Vec<Worker>> {
        let rows: Vec<WorkerRow> = self
            .hiqlite
            .query_as(
                "SELECT * FROM workers WHERE status = $1 ORDER BY registered_at DESC",
                params![WorkerStatus::Online.to_string()],
            )
            .await?;

        let workers = rows
            .into_iter()
            .map(row_to_worker)
            .collect::<Result<Vec<_>>>()?;

        Ok(workers)
    }

    async fn list_all_workers(&self) -> Result<Vec<Worker>> {
        let rows: Vec<WorkerRow> = self
            .hiqlite
            .query_as(
                "SELECT * FROM workers ORDER BY registered_at DESC",
                params![],
            )
            .await?;

        let workers = rows
            .into_iter()
            .map(row_to_worker)
            .collect::<Result<Vec<_>>>()?;

        Ok(workers)
    }

    async fn mark_offline(&self, worker_id: &str) -> Result<()> {
        self.hiqlite
            .execute(
                "UPDATE workers SET status = $1 WHERE id = $2",
                params![WorkerStatus::Offline.to_string(), worker_id],
            )
            .await?;

        tracing::info!(worker_id = %worker_id, "Worker marked offline");
        Ok(())
    }

    async fn mark_draining(&self, worker_id: &str) -> Result<()> {
        self.hiqlite
            .execute(
                "UPDATE workers SET status = $1 WHERE id = $2",
                params![WorkerStatus::Draining.to_string(), worker_id],
            )
            .await?;

        tracing::info!(worker_id = %worker_id, "Worker marked draining");
        Ok(())
    }

    async fn increment_job_count(&self, worker_id: &str) -> Result<()> {
        self.hiqlite
            .execute(
                "UPDATE workers SET active_jobs = active_jobs + 1 WHERE id = $1",
                params![worker_id],
            )
            .await?;

        Ok(())
    }

    async fn decrement_job_count(&self, worker_id: &str) -> Result<()> {
        self.hiqlite
            .execute(
                "UPDATE workers
                 SET active_jobs = MAX(0, active_jobs - 1),
                     total_jobs_completed = total_jobs_completed + 1
                 WHERE id = $1",
                params![worker_id],
            )
            .await?;

        Ok(())
    }

    async fn get_stats(&self) -> Result<WorkerStats> {
        let workers = self.list_all_workers().await?;
        Ok(WorkerStats::from_workers(&workers))
    }
}
