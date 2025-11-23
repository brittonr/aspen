//! Work Item Cache - In-Memory Caching Layer
//!
//! This module provides a thread-safe in-memory cache for WorkItems.
//! It handles:
//! - Fast local reads without hitting persistent storage
//! - Cache invalidation and refresh
//! - Thread-safe concurrent access via RwLock
//! - Statistics computation from cached data
//!
//! The cache serves as a performance optimization, with the persistent store
//! remaining the source of truth for distributed state.

use crate::work_queue::{WorkItem, WorkStatus, WorkQueueStats};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe in-memory cache for work items
///
/// This cache provides fast local access to work items without requiring
/// queries to the distributed persistent store. It must be manually refreshed
/// to stay in sync with distributed state.
#[derive(Clone)]
pub struct WorkItemCache {
    /// Internal cache storage (job_id → WorkItem)
    cache: Arc<RwLock<HashMap<String, WorkItem>>>,
}

impl WorkItemCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a cache from initial work items
    pub fn from_items(items: Vec<WorkItem>) -> Self {
        let cache: HashMap<String, WorkItem> = items
            .into_iter()
            .map(|item| (item.job_id.clone(), item))
            .collect();

        Self {
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// Insert or update a work item in the cache
    pub async fn upsert(&self, work_item: WorkItem) {
        let mut cache = self.cache.write().await;
        cache.insert(work_item.job_id.clone(), work_item);
    }

    /// Get a work item by job ID
    pub async fn get(&self, job_id: &str) -> Option<WorkItem> {
        let cache = self.cache.read().await;
        cache.get(job_id).cloned()
    }

    /// Update a work item in-place via a closure
    ///
    /// Returns `true` if the item existed and was updated, `false` otherwise.
    pub async fn update<F>(&self, job_id: &str, updater: F) -> bool
    where
        F: FnOnce(&mut WorkItem),
    {
        let mut cache = self.cache.write().await;
        if let Some(work_item) = cache.get_mut(job_id) {
            updater(work_item);
            true
        } else {
            false
        }
    }

    /// Replace the entire cache contents
    ///
    /// This is typically used when refreshing from persistent storage.
    pub async fn replace_all(&self, items: Vec<WorkItem>) {
        let new_cache: HashMap<String, WorkItem> = items
            .into_iter()
            .map(|item| (item.job_id.clone(), item))
            .collect();

        let mut cache = self.cache.write().await;
        *cache = new_cache;
    }

    /// Get all work items as a vector
    pub async fn get_all(&self) -> Vec<WorkItem> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get all work items as a HashMap (clone of internal state)
    pub async fn get_all_map(&self) -> HashMap<String, WorkItem> {
        let cache = self.cache.read().await;
        cache.clone()
    }

    /// Find the first work item matching a predicate
    pub async fn find_first<F>(&self, predicate: F) -> Option<WorkItem>
    where
        F: Fn(&WorkItem) -> bool,
    {
        let cache = self.cache.read().await;
        cache.values().find(|item| predicate(item)).cloned()
    }

    /// Count work items matching a predicate
    pub async fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&WorkItem) -> bool,
    {
        let cache = self.cache.read().await;
        cache.values().filter(|item| predicate(item)).count()
    }

    /// Count work items by status
    pub async fn count_by_status(&self, status: WorkStatus) -> usize {
        self.count(|item| item.status == status).await
    }

    /// Get the total number of cached items
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if the cache is empty
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Compute statistics from cached work items
    ///
    /// This provides a fast way to get queue stats without querying
    /// persistent storage, but may be stale if cache hasn't been refreshed.
    pub async fn compute_stats(&self) -> WorkQueueStats {
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

    /// Clear all items from the cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

impl Default for WorkItemCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WorkItemCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkItemCache")
            .field("cache", &"<HashMap<String, WorkItem>>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_item(job_id: &str, status: WorkStatus) -> WorkItem {
        WorkItem {
            job_id: job_id.to_string(),
            status,
            claimed_by: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            payload: json!({}),
        }
    }

    #[tokio::test]
    async fn test_new_cache_is_empty() {
        let cache = WorkItemCache::new();
        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let cache = WorkItemCache::new();
        let item = create_test_item("job1", WorkStatus::Pending);

        cache.upsert(item.clone()).await;

        let retrieved = cache.get("job1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().job_id, "job1");
    }

    #[tokio::test]
    async fn test_update_in_place() {
        let cache = WorkItemCache::new();
        let item = create_test_item("job1", WorkStatus::Pending);
        cache.upsert(item).await;

        let updated = cache
            .update("job1", |item| {
                item.status = WorkStatus::Claimed;
            })
            .await;

        assert!(updated);

        let retrieved = cache.get("job1").await.unwrap();
        assert_eq!(retrieved.status, WorkStatus::Claimed);
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let cache = WorkItemCache::new();

        let updated = cache
            .update("nonexistent", |item| {
                item.status = WorkStatus::Claimed;
            })
            .await;

        assert!(!updated);
    }

    #[tokio::test]
    async fn test_replace_all() {
        let cache = WorkItemCache::new();
        cache.upsert(create_test_item("old1", WorkStatus::Pending)).await;

        let new_items = vec![
            create_test_item("new1", WorkStatus::Pending),
            create_test_item("new2", WorkStatus::Claimed),
        ];

        cache.replace_all(new_items).await;

        assert_eq!(cache.len().await, 2);
        assert!(cache.get("new1").await.is_some());
        assert!(cache.get("new2").await.is_some());
        assert!(cache.get("old1").await.is_none());
    }

    #[tokio::test]
    async fn test_find_first() {
        let cache = WorkItemCache::new();
        cache.upsert(create_test_item("job1", WorkStatus::Completed)).await;
        cache.upsert(create_test_item("job2", WorkStatus::Pending)).await;
        cache.upsert(create_test_item("job3", WorkStatus::Pending)).await;

        let found = cache
            .find_first(|item| item.status == WorkStatus::Pending)
            .await;

        assert!(found.is_some());
        let found_item = found.unwrap();
        assert!(found_item.job_id == "job2" || found_item.job_id == "job3");
    }

    #[tokio::test]
    async fn test_count_by_status() {
        let cache = WorkItemCache::new();
        cache.upsert(create_test_item("job1", WorkStatus::Pending)).await;
        cache.upsert(create_test_item("job2", WorkStatus::Pending)).await;
        cache.upsert(create_test_item("job3", WorkStatus::Claimed)).await;

        assert_eq!(cache.count_by_status(WorkStatus::Pending).await, 2);
        assert_eq!(cache.count_by_status(WorkStatus::Claimed).await, 1);
        assert_eq!(cache.count_by_status(WorkStatus::Completed).await, 0);
    }

    #[tokio::test]
    async fn test_compute_stats() {
        let cache = WorkItemCache::new();
        cache.upsert(create_test_item("job1", WorkStatus::Pending)).await;
        cache.upsert(create_test_item("job2", WorkStatus::Pending)).await;
        cache.upsert(create_test_item("job3", WorkStatus::Claimed)).await;
        cache.upsert(create_test_item("job4", WorkStatus::InProgress)).await;
        cache.upsert(create_test_item("job5", WorkStatus::Completed)).await;
        cache.upsert(create_test_item("job6", WorkStatus::Failed)).await;

        let stats = cache.compute_stats().await;

        assert_eq!(stats.pending, 2);
        assert_eq!(stats.claimed, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
    }

    #[tokio::test]
    async fn test_from_items() {
        let items = vec![
            create_test_item("job1", WorkStatus::Pending),
            create_test_item("job2", WorkStatus::Claimed),
        ];

        let cache = WorkItemCache::from_items(items);

        assert_eq!(cache.len().await, 2);
        assert!(cache.get("job1").await.is_some());
        assert!(cache.get("job2").await.is_some());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = WorkItemCache::new();
        cache.upsert(create_test_item("job1", WorkStatus::Pending)).await;
        cache.upsert(create_test_item("job2", WorkStatus::Claimed)).await;

        assert_eq!(cache.len().await, 2);

        cache.clear().await;

        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }
}
