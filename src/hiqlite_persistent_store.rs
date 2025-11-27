//! Hiqlite Persistent Store Adapter
//!
//! This module provides a concrete implementation of the `PersistentStore` trait
//! backed by HiqliteService (Raft-replicated SQLite). It translates domain-level
//! operations into SQL queries while maintaining strong consistency guarantees.

use anyhow::Result;
use async_trait::async_trait;
use crate::hiqlite::HiqliteService;
use crate::persistent_store::PersistentStore;
use crate::domain::types::{Job, JobStatus};
use crate::domain::job_metadata::JobMetadata;
use crate::domain::job_requirements::JobRequirements;
use crate::params;

/// Hiqlite-backed implementation of PersistentStore
///
/// This adapter wraps HiqliteService and provides domain-level operations
/// on workflow data. All operations are strongly consistent via Raft consensus.
#[derive(Clone)]
pub struct HiqlitePersistentStore {
    hiqlite: HiqliteService,
}

impl HiqlitePersistentStore {
    /// Create a new Hiqlite persistent store
    pub fn new(hiqlite: HiqliteService) -> Self {
        Self { hiqlite }
    }
}

#[async_trait]
impl PersistentStore for HiqlitePersistentStore {
    async fn load_all_workflows(&self) -> Result<Vec<Job>> {
        // Define row structure matching database schema
        #[derive(Debug, serde::Deserialize)]
        struct WorkflowRow {
            id: String,
            status: String,
            claimed_by: Option<String>,
            assigned_worker_id: Option<String>,
            completed_by: Option<String>,
            created_at: i64,
            updated_at: i64,
            started_at: Option<i64>,
            error_message: Option<String>,
            retry_count: i64,
            data: Option<String>,
            compatible_worker_types: Option<String>,
        }

        impl From<hiqlite::Row<'static>> for WorkflowRow {
            fn from(mut row: hiqlite::Row<'static>) -> Self {
                Self {
                    id: row.get("id"),
                    status: row.get("status"),
                    claimed_by: row.get("claimed_by"),
                    assigned_worker_id: row.get("assigned_worker_id"),
                    completed_by: row.get("completed_by"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    started_at: row.get("started_at"),
                    error_message: row.get("error_message"),
                    retry_count: row.get("retry_count"),
                    data: row.get("data"),
                    compatible_worker_types: row.get("compatible_worker_types"),
                }
            }
        }

        // Query all workflows from hiqlite
        let rows: Vec<WorkflowRow> = self.hiqlite
            .query_as(
                "SELECT id, status, claimed_by, assigned_worker_id, completed_by, created_at, updated_at, started_at, error_message, retry_count, data, compatible_worker_types FROM workflows",
                params!()
            )
            .await?;

        // Transform database rows into domain Jobs
        let jobs = rows.into_iter().map(|row| {
            let status = match row.status.as_str() {
                "pending" => JobStatus::Pending,
                "claimed" => JobStatus::Claimed,
                "in_progress" => JobStatus::InProgress,
                "completed" => JobStatus::Completed,
                "failed" => JobStatus::Failed,
                _ => JobStatus::Pending, // Default to pending for unknown statuses
            };

            let payload = row.data
                .and_then(|d| serde_json::from_str(&d).ok())
                .unwrap_or(serde_json::Value::Null);

            let compatible_worker_types = row.compatible_worker_types
                .and_then(|types_str| serde_json::from_str(&types_str).ok())
                .unwrap_or_else(Vec::new);

            let requirements = JobRequirements {
                compatible_worker_types,
                isolation_level: None,
                memory_mb: None,
                vcpus: None,
            };

            let metadata = JobMetadata {
                created_at: row.created_at,
                updated_at: row.updated_at,
                started_at: row.started_at,
                completed_at: None, // Not yet tracked in DB
                retry_count: row.retry_count as u32,
            };

            Job {
                id: row.id,
                status,
                payload,
                requirements,
                metadata,
                error_message: row.error_message,
                claimed_by: row.claimed_by,
                assigned_worker_id: row.assigned_worker_id,
                completed_by: row.completed_by,
            }
        }).collect();

        Ok(jobs)
    }

    async fn upsert_workflow(&self, job: &Job) -> Result<()> {
        let status_str = status_to_string(&job.status);
        let payload_str = serde_json::to_string(&job.payload)?;
        let compatible_worker_types_str = if job.requirements.compatible_worker_types.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&job.requirements.compatible_worker_types)?)
        };

        self.hiqlite
            .execute(
                "INSERT OR REPLACE INTO workflows (id, status, claimed_by, assigned_worker_id, completed_by, created_at, updated_at, started_at, error_message, retry_count, data, compatible_worker_types) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                params!(
                    job.id.clone(),
                    status_str,
                    job.claimed_by.clone(),
                    job.assigned_worker_id.clone(),
                    job.completed_by.clone(),
                    job.metadata.created_at,
                    job.metadata.updated_at,
                    job.metadata.started_at,
                    job.error_message.clone(),
                    job.metadata.retry_count as i64,
                    payload_str,
                    compatible_worker_types_str
                ),
            )
            .await?;

        Ok(())
    }

    async fn claim_workflow(
        &self,
        job_id: &str,
        claimed_by: &str,
        assigned_worker_id: Option<&str>,
        updated_at: i64,
    ) -> Result<usize> {
        // Atomic claim operation with optimistic locking
        // Only succeeds if status is currently 'pending'
        let rows_affected = self.hiqlite
            .execute(
                "UPDATE workflows SET status = $1, claimed_by = $2, assigned_worker_id = $3, updated_at = $4 WHERE id = $5 AND status = 'pending'",
                params!("claimed", claimed_by, assigned_worker_id, updated_at, job_id),
            )
            .await?;

        Ok(rows_affected)
    }

    async fn update_workflow_status(
        &self,
        job_id: &str,
        status: &JobStatus,
        completed_by: Option<&str>,
        updated_at: i64,
    ) -> Result<usize> {
        let status_str = status_to_string(status);

        // Handle different status transitions
        let rows_affected = if *status == JobStatus::Completed || *status == JobStatus::Failed {
            // Terminal states: set completed_by
            // State machine guard: prevent regression from terminal states
            // Allow idempotent updates (completed→completed or failed→failed)
            self.hiqlite
                .execute(
                    "UPDATE workflows SET status = $1, completed_by = $2, updated_at = $3 WHERE id = $4 AND (status NOT IN ('completed', 'failed') OR status = $1)",
                    params!(status_str, completed_by, updated_at, job_id),
                )
                .await?
        } else if *status == JobStatus::InProgress {
            // InProgress: set started_at if not already set
            // State machine guard: prevent updates to terminal states
            self.hiqlite
                .execute(
                    "UPDATE workflows SET status = $1, started_at = COALESCE(started_at, $2), updated_at = $2 WHERE id = $3 AND status NOT IN ('completed', 'failed')",
                    params!(status_str, updated_at, job_id),
                )
                .await?
        } else {
            // Other non-terminal states: just update status and timestamp
            // State machine guard: prevent any updates to terminal states
            self.hiqlite
                .execute(
                    "UPDATE workflows SET status = $1, updated_at = $2 WHERE id = $3 AND status NOT IN ('completed', 'failed')",
                    params!(status_str, updated_at, job_id),
                )
                .await?
        };

        Ok(rows_affected)
    }
}

/// Convert JobStatus enum to database string representation
fn status_to_string(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Pending => "pending",
        JobStatus::Claimed => "claimed",
        JobStatus::InProgress => "in_progress",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    }
}
