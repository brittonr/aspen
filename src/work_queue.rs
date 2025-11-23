//! Work Queue - Thin Coordinator
//!
//! This module provides a distributed work queue that coordinates:
//! - WorkItemCache: In-memory caching layer for fast reads
//! - PersistentStore: Distributed storage with strong consistency
//! - WorkStateMachine: Business logic for state transitions
//!
//! WorkQueue is now a thin orchestrator with no embedded business logic.

use anyhow::Result;
use std::sync::Arc;
use crate::persistent_store::PersistentStore;
use crate::work_item_cache::WorkItemCache;
use crate::work_state_machine::WorkStateMachine;
use crate::domain::types::{Job, JobStatus, QueueStats};

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

        let job = Job {
            id: job_id.clone(),
            status: JobStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: now,
            updated_at: now,
            payload: payload.clone(),
        };

        // Persist to store first (strong consistency via underlying implementation)
        self.store.upsert_workflow(&job).await?;

        // Update local cache
        self.cache.upsert(job).await;

        tracing::info!(job_id = %job_id, "Work item published to persistent store");
        Ok(())
    }

    /// Claim available work from the queue
    pub async fn claim_work(&self) -> Result<Option<Job>> {
        let now = chrono::Utc::now().timestamp();

        // Refresh cache from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Count pending jobs for debugging
        let pending_count = self.cache.count_by_status(JobStatus::Pending).await;
        let total_count = self.cache.len().await;

        tracing::info!(
            pending_count = pending_count,
            total_count = total_count,
            node_id = %self.node_id,
            "Attempting to claim work"
        );

        // Try to claim first pending work item
        let all_items = self.cache.get_all().await;
        for job in all_items {
            if job.status == JobStatus::Pending {
                tracing::info!(
                    job_id = %job.id,
                    node_id = %self.node_id,
                    "Attempting to claim pending job"
                );

                // Claim it via persistent store (atomic operation)
                let rows_affected = self.store
                    .claim_workflow(&job.id, &self.node_id, now)
                    .await?;

                tracing::info!(
                    job_id = %job.id,
                    rows_affected = rows_affected,
                    "Claim operation result"
                );

                // If store update succeeded, update local cache and return
                if rows_affected > 0 {
                    let claimed = self.cache.update(&job.id, |item| {
                        item.status = JobStatus::Claimed;
                        item.claimed_by = Some(self.node_id.clone());
                        item.updated_at = now;
                    }).await;

                    if claimed {
                        let claimed_item = self.cache.get(&job.id).await.unwrap();
                        tracing::info!(
                            job_id = %claimed_item.id,
                            node_id = %self.node_id,
                            "Work item claimed successfully"
                        );
                        return Ok(Some(claimed_item));
                    }
                } else {
                    // Another node claimed it first
                    tracing::warn!(job_id = %job.id, "Claim race detected - another node claimed first");
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
    pub async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
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
        self.cache.update(job_id, |job| {
            job.status = status;
            job.updated_at = now;
            if let Some(completed_by_node) = completed_by {
                job.completed_by = Some(completed_by_node.to_string());
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
    pub async fn list_work(&self) -> Result<Vec<Job>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Return all cached items
        Ok(self.cache.get_all().await)
    }

    /// Get a specific work item by ID
    ///
    /// This method performs data access filtering at the infrastructure layer,
    /// avoiding the need to load all work items.
    pub async fn get_work_by_id(&self, job_id: &str) -> Result<Option<Job>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Get from cache
        Ok(self.cache.get(job_id).await)
    }

    /// List work items filtered by status
    ///
    /// This method performs data access filtering at the infrastructure layer,
    /// avoiding the need to load and filter all work items in the domain layer.
    pub async fn list_work_by_status(&self, status: JobStatus) -> Result<Vec<Job>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Filter by status from cache
        let all_items = self.cache.get_all().await;
        Ok(all_items
            .into_iter()
            .filter(|item| item.status == status)
            .collect())
    }

    /// List work items claimed by a specific worker
    ///
    /// This method performs data access filtering at the infrastructure layer.
    pub async fn list_work_by_worker(&self, worker_id: &str) -> Result<Vec<Job>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Filter by worker from cache
        let all_items = self.cache.get_all().await;
        Ok(all_items
            .into_iter()
            .filter(|item| {
                item.claimed_by
                    .as_ref()
                    .map(|id| id == worker_id)
                    .unwrap_or(false)
            })
            .collect())
    }

    /// List work items with pagination (ordered by update time, most recent first)
    ///
    /// This method performs pagination at the infrastructure layer,
    /// avoiding the need to load all items when only a subset is needed.
    pub async fn list_work_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Get all items and sort by updated_at (most recent first)
        let mut all_items = self.cache.get_all().await;
        all_items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply pagination
        Ok(all_items
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }

    /// List work items filtered by status AND worker (composite query)
    ///
    /// This method performs multiple filters at the infrastructure layer.
    pub async fn list_work_by_status_and_worker(
        &self,
        status: JobStatus,
        worker_id: &str,
    ) -> Result<Vec<Job>> {
        // Refresh from persistent store to get latest distributed state
        self.refresh_cache().await?;

        // Filter by both status and worker from cache
        let all_items = self.cache.get_all().await;
        Ok(all_items
            .into_iter()
            .filter(|item| {
                item.status == status
                    && item
                        .claimed_by
                        .as_ref()
                        .map(|id| id == worker_id)
                        .unwrap_or(false)
            })
            .collect())
    }

    /// Get a placeholder ticket (iroh-docs integration pending)
    pub fn get_ticket(&self) -> String {
        format!("work-queue://{}", self.node_id)
    }

    /// Get work queue statistics (refreshed from persistent store)
    pub async fn stats(&self) -> QueueStats {
        // Try to refresh from persistent store to get distributed state
        if let Err(e) = self.refresh_cache().await {
            tracing::warn!("Failed to refresh cache from persistent store: {}", e);
            // Fall back to stale cache (better than nothing)
        }

        // Compute stats from cache (using dedicated cache method)
        self.cache.compute_stats().await
    }
}
