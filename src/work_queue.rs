//! Work Queue with hiqlite-backed distributed state
//!
//! This module provides a distributed work queue where:
//! - Jobs are published and persisted to hiqlite (Raft-replicated SQLite)
//! - Workers claim jobs with strong consistency guarantees
//! - State is replicated across all nodes via Raft consensus
//! - Local cache provides fast reads with hiqlite as source of truth

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::hiqlite_service::HiqliteService;
use crate::params;

/// Work item representing a job in the distributed queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    /// Unique job identifier
    pub job_id: String,
    /// Job status
    pub status: WorkStatus,
    /// Node that claimed this work (if any)
    pub claimed_by: Option<String>,
    /// Node that completed this work (if any)
    pub completed_by: Option<String>,
    /// Timestamp when job was created
    pub created_at: i64,
    /// Timestamp when job was last updated
    pub updated_at: i64,
    /// Job payload (JSON)
    pub payload: serde_json::Value,
}

/// Work status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkStatus {
    /// Job is available for claiming
    Pending,
    /// Job has been claimed by a worker
    Claimed,
    /// Job is being executed
    InProgress,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
}

/// Work Queue service with hiqlite-backed persistence
#[derive(Clone)]
pub struct WorkQueue {
    /// Local cache of work items (for fast reads)
    cache: Arc<RwLock<HashMap<String, WorkItem>>>,
    /// Our node identifier
    node_id: String,
    /// Hiqlite service for distributed state
    hiqlite: HiqliteService,
}

impl std::fmt::Debug for WorkQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkQueue")
            .field("node_id", &self.node_id)
            .field("hiqlite", &"<HiqliteService>")
            .finish()
    }
}

impl WorkQueue {
    /// Create a new work queue with hiqlite backend
    pub async fn new(
        _endpoint: iroh::Endpoint,
        node_id: String,
        hiqlite: HiqliteService,
    ) -> Result<Self> {
        tracing::info!(
            node_id = %node_id,
            "Work queue initialized with hiqlite backend"
        );

        // Load existing workflows from hiqlite into cache
        let cache = Self::load_from_hiqlite(&hiqlite).await?;

        Ok(Self {
            cache: Arc::new(RwLock::new(cache)),
            node_id,
            hiqlite,
        })
    }

    /// Load all workflows from hiqlite into local cache
    async fn load_from_hiqlite(hiqlite: &HiqliteService) -> Result<HashMap<String, WorkItem>> {
        #[derive(Debug, serde::Deserialize)]
        struct WorkflowRow {
            id: String,
            status: String,
            claimed_by: Option<String>,
            completed_by: Option<String>,
            created_at: i64,
            updated_at: i64,
            data: Option<String>,
        }

        impl From<hiqlite::Row<'static>> for WorkflowRow {
            fn from(mut row: hiqlite::Row<'static>) -> Self {
                Self {
                    id: row.get("id"),
                    status: row.get("status"),
                    claimed_by: row.get("claimed_by"),
                    completed_by: row.get("completed_by"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    data: row.get("data"),
                }
            }
        }

        let rows: Vec<WorkflowRow> = hiqlite
            .query_as("SELECT id, status, claimed_by, completed_by, created_at, updated_at, data FROM workflows", params!())
            .await?;

        let mut cache = HashMap::new();
        for row in rows {
            let status = match row.status.as_str() {
                "pending" => WorkStatus::Pending,
                "claimed" => WorkStatus::Claimed,
                "in_progress" => WorkStatus::InProgress,
                "completed" => WorkStatus::Completed,
                "failed" => WorkStatus::Failed,
                _ => WorkStatus::Pending,
            };

            let payload = row.data
                .and_then(|d| serde_json::from_str(&d).ok())
                .unwrap_or(serde_json::Value::Null);

            let work_item = WorkItem {
                job_id: row.id.clone(),
                status,
                claimed_by: row.claimed_by,
                completed_by: row.completed_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
                payload,
            };

            cache.insert(row.id, work_item);
        }

        tracing::info!(count = cache.len(), "Loaded workflows from hiqlite");
        Ok(cache)
    }

    /// Publish a new work item to the queue
    pub async fn publish_work(&self, job_id: String, payload: serde_json::Value) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        let work_item = WorkItem {
            job_id: job_id.clone(),
            status: WorkStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: now,
            updated_at: now,
            payload: payload.clone(),
        };

        // Persist to hiqlite first (strong consistency)
        let payload_str = serde_json::to_string(&payload)?;
        self.hiqlite
            .execute(
                "INSERT OR REPLACE INTO workflows (id, status, claimed_by, completed_by, created_at, updated_at, data) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                params!(job_id.clone(), "pending", None::<String>, None::<String>, now, now, payload_str),
            )
            .await?;

        // Update local cache
        self.cache.write().await.insert(job_id.clone(), work_item);

        tracing::info!(job_id = %job_id, "Work item published to hiqlite");
        Ok(())
    }

    /// Claim available work from the queue
    pub async fn claim_work(&self) -> Result<Option<WorkItem>> {
        let now = chrono::Utc::now().timestamp();

        // Refresh cache from hiqlite to get latest state from distributed database
        let fresh_cache = Self::load_from_hiqlite(&self.hiqlite).await?;

        // Update our cache with fresh data
        {
            let mut cache = self.cache.write().await;
            *cache = fresh_cache;
        }

        // Find first pending work item in cache
        let mut cache = self.cache.write().await;

        for (job_id, work_item) in cache.iter_mut() {
            if work_item.status == WorkStatus::Pending {
                // Claim it in hiqlite first (atomic operation via Raft)
                let rows_affected = self.hiqlite
                    .execute(
                        "UPDATE workflows SET status = $1, claimed_by = $2, updated_at = $3 WHERE id = $4 AND status = 'pending'",
                        params!("claimed", self.node_id.clone(), now, job_id.clone()),
                    )
                    .await?;

                // If hiqlite update succeeded, update local cache
                if rows_affected > 0 {
                    work_item.status = WorkStatus::Claimed;
                    work_item.claimed_by = Some(self.node_id.clone());
                    work_item.updated_at = now;

                    tracing::info!(
                        job_id = %job_id,
                        node_id = %self.node_id,
                        "Work item claimed via hiqlite"
                    );

                    return Ok(Some(work_item.clone()));
                } else {
                    // Another node claimed it first - refresh from hiqlite
                    tracing::debug!(job_id = %job_id, "Claim race detected - another node claimed first");
                }
            }
        }

        Ok(None)
    }

    /// Update work item status
    pub async fn update_status(&self, job_id: &str, status: WorkStatus) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let status_str = match status {
            WorkStatus::Pending => "pending",
            WorkStatus::Claimed => "claimed",
            WorkStatus::InProgress => "in_progress",
            WorkStatus::Completed => "completed",
            WorkStatus::Failed => "failed",
        };

        // If job is completing or failing, record which node completed it
        if status == WorkStatus::Completed || status == WorkStatus::Failed {
            // State machine guard: prevent regression from terminal states
            // Allow idempotent updates (completed→completed or failed→failed)
            let rows_affected = self.hiqlite
                .execute(
                    "UPDATE workflows SET status = $1, completed_by = $2, updated_at = $3 WHERE id = $4 AND (status NOT IN ('completed', 'failed') OR status = $1)",
                    params!(status_str, self.node_id.clone(), now, job_id),
                )
                .await?;

            if rows_affected == 0 {
                tracing::debug!(job_id = %job_id, new_status = ?status, "Status update rejected by state machine (job may already be in terminal state)");
                return Ok(());
            }

            // Update local cache
            let mut cache = self.cache.write().await;
            if let Some(work_item) = cache.get_mut(job_id) {
                work_item.status = status.clone();
                work_item.completed_by = Some(self.node_id.clone());
                work_item.updated_at = now;
            }
        } else {
            // For other status updates, don't touch completed_by
            // State machine guard: prevent any updates to terminal states
            let rows_affected = self.hiqlite
                .execute(
                    "UPDATE workflows SET status = $1, updated_at = $2 WHERE id = $3 AND status NOT IN ('completed', 'failed')",
                    params!(status_str, now, job_id),
                )
                .await?;

            if rows_affected == 0 {
                tracing::debug!(job_id = %job_id, new_status = ?status, "Status update rejected by state machine (job is in terminal state)");
                return Ok(());
            }

            // Update local cache
            let mut cache = self.cache.write().await;
            if let Some(work_item) = cache.get_mut(job_id) {
                work_item.status = status.clone();
                work_item.updated_at = now;
            }
        }

        tracing::info!(job_id = %job_id, status = ?status, node_id = %self.node_id, "Work status updated in hiqlite");

        Ok(())
    }

    /// List all work items (refreshed from hiqlite)
    pub async fn list_work(&self) -> Result<Vec<WorkItem>> {
        // Always refresh from hiqlite to get distributed state
        let fresh_cache = Self::load_from_hiqlite(&self.hiqlite).await?;

        // Update our local cache with fresh data
        *self.cache.write().await = fresh_cache.clone();

        Ok(fresh_cache.values().cloned().collect())
    }

    /// Get a placeholder ticket (iroh-docs integration pending)
    pub fn get_ticket(&self) -> String {
        format!("work-queue://{}", self.node_id)
    }

    /// Get work queue statistics (refreshed from hiqlite)
    pub async fn stats(&self) -> WorkQueueStats {
        // Refresh from hiqlite to get distributed state
        let fresh_cache = match Self::load_from_hiqlite(&self.hiqlite).await {
            Ok(cache) => cache,
            Err(e) => {
                tracing::warn!("Failed to refresh cache from hiqlite: {}", e);
                // Fall back to local cache
                return self.stats_from_cache().await;
            }
        };

        // Update our local cache
        *self.cache.write().await = fresh_cache.clone();

        let mut stats = WorkQueueStats::default();

        for work_item in fresh_cache.values() {
            match work_item.status {
                WorkStatus::Pending => stats.pending += 1,
                WorkStatus::Claimed => stats.claimed += 1,
                WorkStatus::InProgress => stats.in_progress += 1,
                WorkStatus::Completed => stats.completed += 1,
                WorkStatus::Failed => stats.failed += 1,
            }
        }

        stats
    }

    /// Get stats from local cache only (fallback)
    async fn stats_from_cache(&self) -> WorkQueueStats {
        let cache = self.cache.read().await;
        let mut stats = WorkQueueStats::default();

        for work_item in cache.values() {
            match work_item.status {
                WorkStatus::Pending => stats.pending += 1,
                WorkStatus::Claimed => stats.claimed += 1,
                WorkStatus::InProgress => stats.in_progress += 1,
                WorkStatus::Completed => stats.completed += 1,
                WorkStatus::Failed => stats.failed += 1,
            }
        }

        stats
    }
}

/// Work queue statistics
#[derive(Debug, Default, Serialize)]
pub struct WorkQueueStats {
    pub pending: usize,
    pub claimed: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
}
