//! Cached Work Query Service - Cache Layer
//!
//! This module provides a caching decorator for WorkQueryService.
//! It implements the Decorator pattern to add caching behavior.
//!
//! Responsibilities:
//! - Cache query results in WorkItemCache
//! - Refresh cache when version changes
//! - Delegate to inner WorkQueryService on cache miss
//! - Provide cache statistics and metrics
//!
//! This is the Infrastructure/Cache layer in clean architecture.

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn};

use crate::work::query_service::WorkQueryService;
use crate::work_item_cache::WorkItemCache;
use crate::domain::types::{Job, JobStatus, QueueStats};

/// Cached work query service - decorates WorkQueryService with caching
///
/// This service wraps WorkQueryService and adds caching behavior.
/// It maintains a local cache and refreshes it when the cache version changes.
pub struct CachedWorkQueryService {
    /// Inner query service (does actual queries)
    inner: WorkQueryService,
    /// In-memory cache for fast reads
    cache: WorkItemCache,
    /// Last known cache version
    last_cache_sync: Arc<AtomicU64>,
}

impl CachedWorkQueryService {
    /// Create a new CachedWorkQueryService
    pub async fn new(inner: WorkQueryService) -> Result<Self> {
        // Initialize cache by loading all work items
        let work_items = inner.list_work().await?;
        let cache = WorkItemCache::from_items(work_items.clone());

        info!(count = work_items.len(), "Initialized query cache");

        Ok(Self {
            inner,
            cache,
            last_cache_sync: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Refresh cache from the inner query service
    async fn refresh_cache(&self) -> Result<()> {
        debug!("Refreshing cache from query service");
        let work_items = self.inner.list_work().await?;
        self.cache.replace_all(work_items).await;
        Ok(())
    }

    /// Selectively refresh cache if needed
    ///
    /// This checks if the cache version has changed and refreshes if needed.
    /// For now, we always refresh on read operations to ensure consistency.
    /// In the future, this could be optimized with version tracking.
    async fn refresh_cache_if_needed(&self, current_version: u64) -> Result<()> {
        let last_sync = self.last_cache_sync.load(Ordering::SeqCst);

        if current_version > last_sync {
            self.refresh_cache().await?;
            self.last_cache_sync.store(current_version, Ordering::SeqCst);
        }

        Ok(())
    }

    /// List all work items (with caching)
    pub async fn list_work(&self, cache_version: u64) -> Result<Vec<Job>> {
        self.refresh_cache_if_needed(cache_version).await?;
        Ok(self.cache.get_all().await)
    }

    /// Get a specific work item by ID (with caching)
    pub async fn get_work_by_id(&self, job_id: &str, cache_version: u64) -> Result<Option<Job>> {
        // Check cache first
        if let Some(job) = self.cache.get(job_id).await {
            // Verify cache is fresh
            let last_sync = self.last_cache_sync.load(Ordering::SeqCst);
            if cache_version <= last_sync {
                return Ok(Some(job));
            }
        }

        // Cache miss or stale - refresh and try again
        self.refresh_cache().await?;
        self.last_cache_sync.store(cache_version, Ordering::SeqCst);
        Ok(self.cache.get(job_id).await)
    }

    /// List work items filtered by status (with caching)
    pub async fn list_work_by_status(
        &self,
        status: JobStatus,
        cache_version: u64,
    ) -> Result<Vec<Job>> {
        self.refresh_cache_if_needed(cache_version).await?;

        let all_items = self.cache.get_all().await;
        Ok(all_items
            .into_iter()
            .filter(|item| item.status == status)
            .collect())
    }

    /// List work items claimed by a specific worker (with caching)
    pub async fn list_work_by_worker(
        &self,
        worker_id: &str,
        cache_version: u64,
    ) -> Result<Vec<Job>> {
        self.refresh_cache_if_needed(cache_version).await?;

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

    /// List work items with pagination (with caching)
    pub async fn list_work_paginated(
        &self,
        offset: usize,
        limit: usize,
        cache_version: u64,
    ) -> Result<Vec<Job>> {
        self.refresh_cache_if_needed(cache_version).await?;

        let mut all_items = self.cache.get_all().await;
        all_items.sort_by(|a, b| b.updated_at().cmp(&a.updated_at()));

        Ok(all_items
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }

    /// List work items filtered by status AND worker (with caching)
    pub async fn list_work_by_status_and_worker(
        &self,
        status: JobStatus,
        worker_id: &str,
        cache_version: u64,
    ) -> Result<Vec<Job>> {
        self.refresh_cache_if_needed(cache_version).await?;

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

    /// Get work queue statistics (with caching)
    pub async fn stats(&self, cache_version: u64) -> QueueStats {
        // Try to refresh cache
        if let Err(e) = self.refresh_cache_if_needed(cache_version).await {
            warn!("Failed to refresh cache for stats: {}", e);
        }

        // Compute stats from cache
        self.cache.compute_stats().await
    }

    /// Force a cache refresh (useful for debugging and recovery)
    pub async fn force_cache_refresh(&self) -> Result<()> {
        info!("Forcing cache refresh");
        self.refresh_cache().await
    }
}

impl Clone for CachedWorkQueryService {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            cache: self.cache.clone(),
            last_cache_sync: Arc::clone(&self.last_cache_sync),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::repository::WorkRepositoryImpl;
    use crate::work::command_service::WorkCommandService;
    use crate::hiqlite_persistent_store::HiqlitePersistentStore;
    use crate::hiqlite::HiqliteService;

    async fn create_test_service() -> (WorkCommandService, CachedWorkQueryService) {
        let hiqlite = Arc::new(
            HiqliteService::new(None)
                .await
                .expect("Failed to create HiqliteService"),
        );
        let store = Arc::new(HiqlitePersistentStore::new(hiqlite.clone()));
        let repository = Arc::new(WorkRepositoryImpl::new(store.clone()));

        let command_service = WorkCommandService::new("test-node".to_string(), store);
        let query_service = WorkQueryService::new(repository);
        let cached_query_service = CachedWorkQueryService::new(query_service)
            .await
            .expect("Failed to create cached query service");

        (command_service, cached_query_service)
    }

    #[tokio::test]
    async fn test_list_work_with_cache() {
        let (cmd, query) = create_test_service().await;

        // Publish some work
        let payload = serde_json::json!({"task": "test"});
        cmd.publish_work("test-job-1".to_string(), payload).await.unwrap();

        // List work (should use cache)
        let version = cmd.get_cache_version();
        let result = query.list_work(version).await;
        assert!(result.is_ok());
        let jobs = result.unwrap();
        assert!(!jobs.is_empty());
    }

    #[tokio::test]
    async fn test_get_work_by_id_with_cache() {
        let (cmd, query) = create_test_service().await;

        // Publish a job
        let payload = serde_json::json!({"task": "test"});
        cmd.publish_work("test-job-2".to_string(), payload).await.unwrap();

        // Get by ID (should use cache)
        let version = cmd.get_cache_version();
        let result = query.get_work_by_id("test-job-2", version).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_cache_refresh_on_version_change() {
        let (cmd, query) = create_test_service().await;

        // Get initial version
        let v1 = cmd.get_cache_version();

        // Publish a job (increments version)
        let payload = serde_json::json!({"task": "test"});
        cmd.publish_work("test-job-3".to_string(), payload).await.unwrap();

        let v2 = cmd.get_cache_version();
        assert!(v2 > v1);

        // Query should refresh cache due to version change
        let result = query.list_work(v2).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stats_with_cache() {
        let (cmd, query) = create_test_service().await;

        // Publish some jobs
        let payload = serde_json::json!({"task": "test"});
        cmd.publish_work("test-job-4".to_string(), payload.clone()).await.unwrap();
        cmd.publish_work("test-job-5".to_string(), payload).await.unwrap();

        // Get stats
        let version = cmd.get_cache_version();
        let stats = query.stats(version).await;
        assert!(stats.total >= 2);
    }
}
