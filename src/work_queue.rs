//! Work Queue - Thin Coordinator
//!
//! This module provides a distributed work queue that coordinates:
//! - WorkItemCache: In-memory caching layer for fast reads
//! - PersistentStore: Distributed storage with strong consistency
//! - WorkStateMachine: Business logic for state transitions
//!
//! WorkQueue is now a thin orchestrator with no embedded business logic.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::persistent_store::PersistentStore;
use crate::work_item_cache::WorkItemCache;
use crate::work_state_machine::WorkStateMachine;

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

/// Work Queue - Coordinates cache, persistence, and state machine
#[derive(Clone)]
pub struct WorkQueue {
    /// In-memory cache for fast local reads
    cache: WorkItemCache,
    /// Our node identifier
    node_id: String,
    /// Persistent store for distributed state (trait allows swapping implementations)
    store: Arc<dyn PersistentStore>,
}

impl std::fmt::Debug for WorkQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkQueue")
            .field("node_id", &self.node_id)
            .field("cache", &self.cache)
            .field("store", &"<dyn PersistentStore>")
            .finish()
    }
}

impl WorkQueue {
    /// Create a new work queue with persistent store backend
    pub async fn new(
        _endpoint: iroh::Endpoint,
        node_id: String,
        store: Arc<dyn PersistentStore>,
    ) -> Result<Self> {
        tracing::info!(
            node_id = %node_id,
            "Work queue initialized with persistent store"
        );

        // Load existing workflows from persistent store into cache
        let work_items = store.load_all_workflows().await?;
        let cache = WorkItemCache::from_items(work_items.clone());

        tracing::info!(count = work_items.len(), "Loaded workflows from persistent store");

        Ok(Self {
            cache,
            node_id,
            store,
        })
    }

    /// Refresh cache from persistent store
    async fn refresh_cache(&self) -> Result<()> {
        let work_items = self.store.load_all_workflows().await?;
        self.cache.replace_all(work_items).await;
        Ok(())
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

        // Persist to store first (strong consistency via underlying implementation)
        self.store.upsert_workflow(&work_item).await?;

        // Update local cache
        self.cache.upsert(work_item).await;

        tracing::info!(job_id = %job_id, "Work item published to persistent store");
        Ok(())
    }

    /// Claim available work from the queue
    pub async fn claim_work(&self) -> Result<Option<WorkItem>> {
        let now = chrono::Utc::now().timestamp();

        // Refresh cache from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Count pending jobs for debugging
        let pending_count = self.cache.count_by_status(WorkStatus::Pending).await;
        let total_count = self.cache.len().await;

        tracing::info!(
            pending_count = pending_count,
            total_count = total_count,
            node_id = %self.node_id,
            "Attempting to claim work"
        );

        // Try to claim first pending work item
        let all_items = self.cache.get_all().await;
        for work_item in all_items {
            if work_item.status == WorkStatus::Pending {
                tracing::info!(
                    job_id = %work_item.job_id,
                    node_id = %self.node_id,
                    "Attempting to claim pending job"
                );

                // Claim it via persistent store (atomic operation)
                let rows_affected = self.store
                    .claim_workflow(&work_item.job_id, &self.node_id, now)
                    .await?;

                tracing::info!(
                    job_id = %work_item.job_id,
                    rows_affected = rows_affected,
                    "Claim operation result"
                );

                // If store update succeeded, update local cache and return
                if rows_affected > 0 {
                    let claimed = self.cache.update(&work_item.job_id, |item| {
                        item.status = WorkStatus::Claimed;
                        item.claimed_by = Some(self.node_id.clone());
                        item.updated_at = now;
                    }).await;

                    if claimed {
                        let claimed_item = self.cache.get(&work_item.job_id).await.unwrap();
                        tracing::info!(
                            job_id = %claimed_item.job_id,
                            node_id = %self.node_id,
                            "Work item claimed successfully"
                        );
                        return Ok(Some(claimed_item));
                    }
                } else {
                    // Another node claimed it first
                    tracing::warn!(job_id = %work_item.job_id, "Claim race detected - another node claimed first");
                }
            }
        }

        tracing::info!(
            pending_count = pending_count,
            "No work claimed - returning None"
        );
        Ok(None)
    }

    /// Update work item status
    pub async fn update_status(&self, job_id: &str, status: WorkStatus) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        // Determine completed_by using state machine logic
        let completed_by = if WorkStateMachine::requires_completed_by(&status) {
            Some(self.node_id.as_str())
        } else {
            None
        };

        // Update via persistent store (includes state machine guards)
        let rows_affected = self.store
            .update_workflow_status(job_id, &status, completed_by, now)
            .await?;

        if rows_affected == 0 {
            tracing::debug!(
                job_id = %job_id,
                new_status = ?status,
                "Status update rejected by state machine (job may already be in terminal state)"
            );
            return Ok(());
        }

        // Update local cache
        self.cache.update(job_id, |work_item| {
            work_item.status = status.clone();
            work_item.updated_at = now;
            if let Some(completed_by_node) = completed_by {
                work_item.completed_by = Some(completed_by_node.to_string());
            }
        }).await;

        tracing::info!(
            job_id = %job_id,
            status = ?status,
            node_id = %self.node_id,
            "Work status updated in persistent store"
        );

        Ok(())
    }

    /// List all work items (refreshed from persistent store)
    pub async fn list_work(&self) -> Result<Vec<WorkItem>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Return all cached items
        Ok(self.cache.get_all().await)
    }

    /// Get a placeholder ticket (iroh-docs integration pending)
    pub fn get_ticket(&self) -> String {
        format!("work-queue://{}", self.node_id)
    }

    /// Get work queue statistics (refreshed from persistent store)
    pub async fn stats(&self) -> WorkQueueStats {
        // Try to refresh from persistent store to get distributed state
        if let Err(e) = self.refresh_cache().await {
            tracing::warn!("Failed to refresh cache from persistent store: {}", e);
            // Fall back to stale cache (better than nothing)
        }

        // Compute stats from cache (using dedicated cache method)
        self.cache.compute_stats().await
    }
}

/// Work queue statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WorkQueueStats {
    pub pending: usize,
    pub claimed: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
}
