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
use crate::repositories::JobAssignment;

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

    pub async fn initialize(&self) -> Result<()> {
        // Schema for workflows
        self.hiqlite
            .execute(
                r#"
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                data TEXT,
                compatible_worker_types TEXT
            )
            "#,
                params!(),
            )
            .await?;

        // Schema for job assignments (infrastructure concern)
        self.hiqlite
            .execute(
                r#"
            CREATE TABLE IF NOT EXISTS job_assignments (
                job_id TEXT PRIMARY KEY,
                claimed_by_node TEXT,
                assigned_worker_id TEXT,
                completed_by_node TEXT,
                claimed_at INTEGER,
                started_at INTEGER,
                completed_at INTEGER,
                FOREIGN KEY (job_id) REFERENCES workflows(id)
            )
            "#,
                params!(),
            )
            .await?;
        Ok(())
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
                "SELECT id, status, created_at, updated_at, started_at, error_message, retry_count, data, compatible_worker_types FROM workflows",
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
            }
        }).collect();

        Ok(jobs)
    }

    async fn upsert_workflow(&self, job: &Job, assignment: &JobAssignment) -> Result<()> {
        let status_str = status_to_string(&job.status);
        let payload_str = serde_json::to_string(&job.payload)?;
        let compatible_worker_types_str = if job.requirements.compatible_worker_types.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&job.requirements.compatible_worker_types)?)
        };

        self.hiqlite
            .execute(
                "INSERT OR REPLACE INTO workflows (id, status, created_at, updated_at, started_at, error_message, retry_count, data, compatible_worker_types) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                params!(
                    job.id.clone(),
                    status_str,
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
        
        self.upsert_job_assignment(assignment).await?;

        Ok(())
    }

    async fn upsert_job_assignment(&self, assignment: &JobAssignment) -> Result<()> {
        self.hiqlite
            .execute(
                "INSERT OR REPLACE INTO job_assignments (job_id, claimed_by_node, assigned_worker_id, completed_by_node, claimed_at, started_at, completed_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                params!(
                    assignment.job_id.clone(),
                    assignment.claimed_by_node.clone(),
                    assignment.assigned_worker_id.clone(),
                    assignment.completed_by_node.clone(),
                    assignment.claimed_at,
                    assignment.started_at,
                    assignment.completed_at
                ),
            )
            .await?;
        Ok(())
    }

    async fn claim_workflow(
        &self,
        job_id: &str,
        updated_at: i64,
    ) -> Result<usize> {
        // Atomic claim operation with optimistic locking
        // Only succeeds if status is currently 'pending'
        let rows_affected = self.hiqlite
            .execute(
                "UPDATE workflows SET status = $1, updated_at = $2 WHERE id = $3 AND status = 'pending'",
                params!("claimed", updated_at, job_id),
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
                    "UPDATE workflows SET status = $1, updated_at = $2 WHERE id = $3 AND (status NOT IN ('completed', 'failed') OR status = $1)",
                    params!(status_str, updated_at, job_id),
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

        if rows_affected > 0 && (*status == JobStatus::Completed || *status == JobStatus::Failed) {
            self.hiqlite.execute(
                "UPDATE job_assignments SET completed_by_node = $1, completed_at = $2 WHERE job_id = $3",
                params!(completed_by, updated_at, job_id)
            ).await?;
        }

        Ok(rows_affected)
    }

    async fn find_job_assignment_by_id(&self, job_id: &str) -> Result<Option<JobAssignment>> {
        let result = self.hiqlite
            .query_as(
                "SELECT job_id, claimed_by_node, assigned_worker_id, completed_by_node, claimed_at, started_at, completed_at FROM job_assignments WHERE job_id = $1",
                params!(job_id),
            )
            .await?;
        Ok(result.into_iter().next())
    }

    async fn load_all_job_assignments(&self) -> Result<Vec<JobAssignment>> {
        let result = self.hiqlite
            .query_as(
                "SELECT job_id, claimed_by_node, assigned_worker_id, completed_by_node, claimed_at, started_at, completed_at FROM job_assignments",
                params!(),
            )
            .await?;
        Ok(result)
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
