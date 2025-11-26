//! Work Item Cache - In-Memory Caching Layer
//!
//! This module provides a thread-safe in-memory cache for Jobs.
//! It handles:
//! - Fast local reads without hitting persistent storage
//! - Cache invalidation and refresh
//! - Thread-safe concurrent access via RwLock
//! - Statistics computation from cached data
//!
//! The cache serves as a performance optimization, with the persistent store
//! remaining the source of truth for distributed state.

use crate::domain::types::{Job, JobStatus, QueueStats};
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
    /// Internal cache storage (job_id → Job)
    cache: Arc<RwLock<HashMap<String, Job>>>,
}

impl WorkItemCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a cache from initial work items
    pub fn from_items(items: Vec<Job>) -> Self {
        let cache: HashMap<String, Job> = items
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect();

        Self {
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// Insert or update a work item in the cache
    pub async fn upsert(&self, job: Job) {
        let mut cache = self.cache.write().await;
        cache.insert(job.id.clone(), job);
    }

    /// Get a work item by job ID
    pub async fn get(&self, job_id: &str) -> Option<Job> {
        let cache = self.cache.read().await;
        cache.get(job_id).cloned()
    }

    /// Update a work item in-place via a closure
    ///
    /// Returns `true` if the item existed and was updated, `false` otherwise.
    pub async fn update<F>(&self, job_id: &str, updater: F) -> bool
    where
        F: FnOnce(&mut Job),
    {
        let mut cache = self.cache.write().await;
        if let Some(job) = cache.get_mut(job_id) {
            updater(job);
            true
        } else {
            false
        }
    }

    /// Replace the entire cache contents
    ///
    /// This is typically used when refreshing from persistent storage.
    pub async fn replace_all(&self, items: Vec<Job>) {
        let new_cache: HashMap<String, Job> = items
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect();

        let mut cache = self.cache.write().await;
        *cache = new_cache;
    }

    /// Get all work items as a vector
    pub async fn get_all(&self) -> Vec<Job> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get all work items as a HashMap (clone of internal state)
    #[allow(dead_code)] // Used in tests
    pub async fn get_all_map(&self) -> HashMap<String, Job> {
        let cache = self.cache.read().await;
        cache.clone()
    }

    /// Find the first work item matching a predicate
    #[allow(dead_code)] // Used in tests
    pub async fn find_first<F>(&self, predicate: F) -> Option<Job>
    where
        F: Fn(&Job) -> bool,
    {
        let cache = self.cache.read().await;
        cache.values().find(|item| predicate(item)).cloned()
    }

    /// Count work items matching a predicate
    pub async fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Job) -> bool,
    {
        let cache = self.cache.read().await;
        cache.values().filter(|item| predicate(item)).count()
    }

    /// Count work items by status
    pub async fn count_by_status(&self, status: JobStatus) -> usize {
        self.count(|item| item.status == status).await
    }

    /// Get the total number of cached items
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if the cache is empty
    #[allow(dead_code)] // Utility method for future use
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Compute statistics from cached work items
    ///
    /// This provides a fast way to get queue stats without querying
    /// persistent storage, but may be stale if cache hasn't been refreshed.
    pub async fn compute_stats(&self) -> QueueStats {
        let cache = self.cache.read().await;
        let jobs: Vec<Job> = cache.values().cloned().collect();
        QueueStats::from_jobs(&jobs)
    }

    /// Clear all items from the cache
    #[allow(dead_code)] // Utility method for future use
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
            .field("cache", &"<HashMap<String, Job>>")
            .finish()
    }
}
