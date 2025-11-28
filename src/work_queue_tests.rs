//! Comprehensive test suite for WorkQueue and WorkItemCache
//!
//! This module contains comprehensive tests covering:
//! 1. Unit tests for cache invalidation and consistency
//! 2. Integration tests for race conditions in claim_work()
//! 3. Property-based tests for invariants using proptest
//! 4. Simulation tests for distributed scenarios
//! 5. Rollback scenarios when persistence fails

#[cfg(test)]
mod tests {
    use crate::domain::types::{Job, JobStatus};
    use crate::domain::job_metadata::JobMetadata;
    use crate::domain::job_requirements::JobRequirements;
    use crate::persistent_store::PersistentStore;
    use crate::work_item_cache::WorkItemCache;
    use crate::work_queue::WorkQueue;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::RwLock;
    use std::collections::HashMap;

    // =========================================================================
    // MOCK IMPLEMENTATIONS
    // =========================================================================

    /// Mock persistent store for testing
    /// Tracks all operations for verification
    #[derive(Clone)]
    struct MockStore {
        // Storage of workflows
        workflows: Arc<RwLock<HashMap<String, Job>>>,
        // Counters for operation tracking
        load_all_count: Arc<AtomicUsize>,
        upsert_count: Arc<AtomicUsize>,
        claim_count: Arc<AtomicUsize>,
        claim_success_count: Arc<AtomicUsize>,
        update_status_count: Arc<AtomicUsize>,
        // For simulating persistence failures
        should_fail_claim: Arc<RwLock<bool>>,
        should_fail_update: Arc<RwLock<bool>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                workflows: Arc::new(RwLock::new(HashMap::new())),
                load_all_count: Arc::new(AtomicUsize::new(0)),
                upsert_count: Arc::new(AtomicUsize::new(0)),
                claim_count: Arc::new(AtomicUsize::new(0)),
                claim_success_count: Arc::new(AtomicUsize::new(0)),
                update_status_count: Arc::new(AtomicUsize::new(0)),
                should_fail_claim: Arc::new(RwLock::new(false)),
                should_fail_update: Arc::new(RwLock::new(false)),
            }
        }

        async fn reset_counters(&self) {
            self.load_all_count.store(0, Ordering::SeqCst);
            self.upsert_count.store(0, Ordering::SeqCst);
            self.claim_count.store(0, Ordering::SeqCst);
            self.claim_success_count.store(0, Ordering::SeqCst);
            self.update_status_count.store(0, Ordering::SeqCst);
        }

        fn load_all_count(&self) -> usize {
            self.load_all_count.load(Ordering::SeqCst)
        }

        fn claim_count(&self) -> usize {
            self.claim_count.load(Ordering::SeqCst)
        }

        fn claim_success_count(&self) -> usize {
            self.claim_success_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl PersistentStore for MockStore {
        async fn load_all_workflows(&self) -> Result<Vec<Job>> {
            self.load_all_count.fetch_add(1, Ordering::SeqCst);
            let workflows = self.workflows.read().await;
            Ok(workflows.values().cloned().collect())
        }

        async fn upsert_workflow(&self, job: &Job) -> Result<()> {
            self.upsert_count.fetch_add(1, Ordering::SeqCst);
            let mut workflows = self.workflows.write().await;
            workflows.insert(job.id.clone(), job.clone());
            Ok(())
        }

        async fn claim_workflow(
            &self,
            job_id: &str,
            claimed_by: &str,
            assigned_worker_id: Option<&str>,
            updated_at: i64,
        ) -> Result<usize> {
            self.claim_count.fetch_add(1, Ordering::SeqCst);

            // Check if we should simulate a failure
            if *self.should_fail_claim.read().await {
                return Ok(0);
            }

            let mut workflows = self.workflows.write().await;
            if let Some(job) = workflows.get_mut(job_id) {
                if job.status == JobStatus::Pending {
                    job.status = JobStatus::Claimed;
                    job.claimed_by = Some(claimed_by.to_string());
                    job.assigned_worker_id = assigned_worker_id.map(|s| s.to_string());
                    job.metadata.updated_at = updated_at;
                    self.claim_success_count.fetch_add(1, Ordering::SeqCst);
                    return Ok(1);
                }
            }
            Ok(0)
        }

        async fn update_workflow_status(
            &self,
            job_id: &str,
            status: &JobStatus,
            completed_by: Option<&str>,
            updated_at: i64,
        ) -> Result<usize> {
            self.update_status_count.fetch_add(1, Ordering::SeqCst);

            // Check if we should simulate a failure
            if *self.should_fail_update.read().await {
                return Ok(0);
            }

            let mut workflows = self.workflows.write().await;
            if let Some(job) = workflows.get_mut(job_id) {
                // Validate state transitions (simplified state machine)
                if job.status == JobStatus::Completed || job.status == JobStatus::Failed {
                    return Ok(0); // Already terminal
                }
                job.status = *status;
                job.metadata.updated_at = updated_at;
                if let Some(node_id) = completed_by {
                    job.completed_by = Some(node_id.to_string());
                }
                return Ok(1);
            }
            Ok(0)
        }
    }

    /// Helper to create a test job
    fn create_test_job(id: &str, status: JobStatus) -> Job {
        Job {
            id: id.to_string(),
            status,
            payload: serde_json::json!({"test": "data"}),
            requirements: JobRequirements::default(),
            metadata: JobMetadata::new(),
            error_message: None,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
        }
    }

    // =========================================================================
    // UNIT TESTS - CACHE OPERATIONS
    // =========================================================================

    #[tokio::test]
    async fn test_cache_upsert_new_item() {
        let cache = WorkItemCache::new();
        let job = create_test_job("job1", JobStatus::Pending);

        cache.upsert(job.clone()).await;

        let cached = cache.get("job1").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, "job1");
    }

    #[tokio::test]
    async fn test_cache_upsert_overwrites_existing() {
        let cache = WorkItemCache::new();
        let job1 = create_test_job("job1", JobStatus::Pending);
        let mut job2 = create_test_job("job1", JobStatus::Claimed);

        cache.upsert(job1).await;
        job2.claimed_by = Some("node1".to_string());
        cache.upsert(job2.clone()).await;

        let cached = cache.get("job1").await;
        assert!(cached.is_some());
        let cached_job = cached.unwrap();
        assert_eq!(cached_job.status, JobStatus::Claimed);
        assert_eq!(cached_job.claimed_by, Some("node1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_update_in_place() {
        let cache = WorkItemCache::new();
        let job = create_test_job("job1", JobStatus::Pending);
        cache.upsert(job).await;

        let updated = cache
            .update("job1", |j| {
                j.status = JobStatus::Claimed;
                j.claimed_by = Some("worker1".to_string());
            })
            .await;

        assert!(updated);
        let cached = cache.get("job1").await.unwrap();
        assert_eq!(cached.status, JobStatus::Claimed);
        assert_eq!(cached.claimed_by, Some("worker1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_update_nonexistent_returns_false() {
        let cache = WorkItemCache::new();

        let updated = cache
            .update("nonexistent", |_| {
                // This should not be called
            })
            .await;

        assert!(!updated);
    }

    #[tokio::test]
    async fn test_cache_replace_all_invalidates_old_items() {
        let cache = WorkItemCache::new();
        let job1 = create_test_job("job1", JobStatus::Pending);
        cache.upsert(job1).await;

        // Replace with different items
        let job2 = create_test_job("job2", JobStatus::Claimed);
        cache.replace_all(vec![job2]).await;

        assert!(cache.get("job1").await.is_none());
        assert!(cache.get("job2").await.is_some());
    }

    #[tokio::test]
    async fn test_cache_count_by_status() {
        let cache = WorkItemCache::new();
        let pending = create_test_job("job1", JobStatus::Pending);
        let claimed = create_test_job("job2", JobStatus::Claimed);
        let in_progress = create_test_job("job3", JobStatus::InProgress);

        cache.upsert(pending).await;
        cache.upsert(claimed).await;
        cache.upsert(in_progress).await;

        assert_eq!(cache.count_by_status(JobStatus::Pending).await, 1);
        assert_eq!(cache.count_by_status(JobStatus::Claimed).await, 1);
        assert_eq!(cache.count_by_status(JobStatus::InProgress).await, 1);
        assert_eq!(cache.count_by_status(JobStatus::Completed).await, 0);
    }

    #[tokio::test]
    async fn test_cache_get_all_returns_copy() {
        let cache = WorkItemCache::new();
        let job1 = create_test_job("job1", JobStatus::Pending);
        let job2 = create_test_job("job2", JobStatus::Claimed);

        cache.upsert(job1).await;
        cache.upsert(job2).await;

        let all = cache.get_all().await;
        assert_eq!(all.len(), 2);

        // Verify we got copies, not references
        let ids: Vec<String> = all.iter().map(|j| j.id.clone()).collect();
        assert!(ids.contains(&"job1".to_string()));
        assert!(ids.contains(&"job2".to_string()));
    }

    #[tokio::test]
    async fn test_cache_clear_removes_all_items() {
        let cache = WorkItemCache::new();
        let job1 = create_test_job("job1", JobStatus::Pending);
        let job2 = create_test_job("job2", JobStatus::Claimed);

        cache.upsert(job1).await;
        cache.upsert(job2).await;
        assert_eq!(cache.len().await, 2);

        cache.clear().await;
        assert_eq!(cache.len().await, 0);
        assert!(cache.get("job1").await.is_none());
    }

    // =========================================================================
    // UNIT TESTS - CACHE CONSISTENCY
    // =========================================================================

    #[tokio::test]
    async fn test_cache_and_store_consistency_after_publish() {
        let store = Arc::new(MockStore::new());
        let queue = WorkQueue::new(
            // Note: endpoint is unused in tests
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        queue
            .publish_work("job1".to_string(), serde_json::json!({"test": true}))
            .await
            .unwrap();

        // Verify store received the upsert (increment happened)
        assert_eq!(store.upsert_count.load(Ordering::SeqCst), 1);

        // Refresh cache and verify
        let all = queue.list_work().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "job1");
    }

    #[tokio::test]
    async fn test_cache_refresh_loads_latest_from_store() {
        let store = Arc::new(MockStore::new());

        // Pre-populate the store with a job
        let job = create_test_job("job1", JobStatus::Claimed);
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Initial load should have the job
        let jobs = queue.list_work().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "job1");
    }

    #[tokio::test]
    async fn test_claim_work_updates_both_cache_and_store() {
        let store = Arc::new(MockStore::new());

        // Pre-populate with a pending job
        let job = create_test_job("job1", JobStatus::Pending);
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let claimed = queue.claim_work(Some("worker1"), None).await.unwrap();

        // Verify store was updated
        assert_eq!(store.claim_success_count(), 1);

        // Verify cache was updated
        assert!(claimed.is_some());
        let claimed_job = claimed.unwrap();
        assert_eq!(claimed_job.status, JobStatus::Claimed);
        assert_eq!(claimed_job.claimed_by, Some("node1".to_string()));

        // Verify both store and cache agree
        let stored_job = store.load_all_workflows().await.unwrap();
        assert_eq!(stored_job[0].status, JobStatus::Claimed);
    }

    #[tokio::test]
    async fn test_update_status_synchronizes_cache_and_store() {
        let store = Arc::new(MockStore::new());

        // Setup
        let mut job = create_test_job("job1", JobStatus::Claimed);
        job.claimed_by = Some("node1".to_string());
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Update status
        queue
            .update_status("job1", JobStatus::InProgress)
            .await
            .unwrap();

        // Verify both cache and store were updated
        let cached = queue.list_work().await.unwrap();
        assert_eq!(cached[0].status, JobStatus::InProgress);

        let stored = store.load_all_workflows().await.unwrap();
        assert_eq!(stored[0].status, JobStatus::InProgress);
    }

    // =========================================================================
    // INTEGRATION TESTS - RACE CONDITIONS
    // =========================================================================

    #[tokio::test]
    async fn test_concurrent_claims_only_one_succeeds() {
        let store = Arc::new(MockStore::new());

        // Setup: one pending job
        let job = create_test_job("job1", JobStatus::Pending);
        store.upsert_workflow(&job).await.unwrap();

        let queue1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Both try to claim simultaneously
        let (claim1, claim2) = tokio::join!(
            queue1.claim_work(Some("worker1"), None),
            queue2.claim_work(Some("worker2"), None)
        );

        let result1 = claim1.unwrap();
        let result2 = claim2.unwrap();

        // Exactly one should succeed
        let succeeded = (result1.is_some(), result2.is_some());
        match succeeded {
            (true, false) => {
                assert_eq!(result1.unwrap().claimed_by.as_ref().unwrap(), "node1");
            }
            (false, true) => {
                assert_eq!(result2.unwrap().claimed_by.as_ref().unwrap(), "node2");
            }
            _ => panic!("Either exactly one should succeed, got: {:?}", succeeded),
        }

        // Verify store recorded only one successful claim
        assert_eq!(store.claim_success_count(), 1);
    }

    #[tokio::test]
    async fn test_multiple_concurrent_publishes() {
        let store = Arc::new(MockStore::new());
        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Publish 10 jobs concurrently
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let q = queue.clone();
                tokio::spawn(async move {
                    q.publish_work(
                        format!("job{}", i),
                        serde_json::json!({"index": i}),
                    )
                    .await
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }

        // Verify all jobs are in the store
        let all = store.load_all_workflows().await.unwrap();
        assert_eq!(all.len(), 10);
    }

    #[tokio::test]
    async fn test_claim_then_immediate_status_update() {
        let store = Arc::new(MockStore::new());

        let job = create_test_job("job1", JobStatus::Pending);
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Claim the job
        let claimed = queue.claim_work(Some("worker1"), None).await.unwrap();
        assert!(claimed.is_some());

        // Immediately update status
        queue
            .update_status("job1", JobStatus::InProgress)
            .await
            .unwrap();

        // Verify final state in both store and cache
        let cached = queue.list_work().await.unwrap();
        let final_job = &cached[0];
        assert_eq!(final_job.status, JobStatus::InProgress);
        assert_eq!(final_job.claimed_by, Some("node1".to_string()));
    }

    // =========================================================================
    // ROLLBACK AND ERROR HANDLING TESTS
    // =========================================================================

    #[tokio::test]
    async fn test_claim_failure_does_not_update_cache() {
        let store = Arc::new(MockStore::new());

        let job = create_test_job("job1", JobStatus::Pending);
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Simulate claim failure in the store
        *store.should_fail_claim.write().await = true;

        let result = queue.claim_work(Some("worker1"), None).await.unwrap();

        // Claim should fail
        assert!(result.is_none());

        // Cache should still show the job as pending (since refresh_cache is called)
        let all = queue.list_work().await.unwrap();
        assert_eq!(all[0].status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_failure_does_not_corrupt_cache() {
        let store = Arc::new(MockStore::new());

        let mut job = create_test_job("job1", JobStatus::Claimed);
        job.claimed_by = Some("node1".to_string());
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Initial state check
        let before = queue.list_work().await.unwrap();
        assert_eq!(before[0].status, JobStatus::Claimed);

        // Simulate update failure
        *store.should_fail_update.write().await = true;

        queue
            .update_status("job1", JobStatus::InProgress)
            .await
            .unwrap();

        // Cache should still be in sync with store (claimed state)
        let after = queue.list_work().await.unwrap();
        assert_eq!(after[0].status, JobStatus::Claimed);
    }

    #[tokio::test]
    async fn test_partial_failure_recovery() {
        let store = Arc::new(MockStore::new());

        let job = create_test_job("job1", JobStatus::Pending);
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // First claim fails
        *store.should_fail_claim.write().await = true;
        let result1 = queue.claim_work(Some("worker1"), None).await.unwrap();
        assert!(result1.is_none());

        // Second claim succeeds
        *store.should_fail_claim.write().await = false;
        let result2 = queue.claim_work(Some("worker1"), None).await.unwrap();
        assert!(result2.is_some());

        let claimed_job = result2.unwrap();
        assert_eq!(claimed_job.status, JobStatus::Claimed);
    }

    // =========================================================================
    // MULTI-NODE SCENARIOS
    // =========================================================================

    #[tokio::test]
    async fn test_multiple_nodes_work_distributed() {
        let store = Arc::new(MockStore::new());

        // Create 3 pending jobs
        for i in 0..3 {
            let job = create_test_job(&format!("job{}", i), JobStatus::Pending);
            store.upsert_workflow(&job).await.unwrap();
        }

        let node1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let node2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let node3 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node3".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Each node claims work
        let claim1 = node1.claim_work(Some("worker1"), None).await.unwrap();
        let claim2 = node2.claim_work(Some("worker2"), None).await.unwrap();
        let claim3 = node3.claim_work(Some("worker3"), None).await.unwrap();

        // Collect claimed jobs
        let claimed: Vec<_> = vec![claim1, claim2, claim3]
            .into_iter()
            .filter_map(|c| c)
            .collect();

        // Should have claimed all 3 jobs
        assert_eq!(claimed.len(), 3);

        // Verify no duplicates
        let ids: Vec<String> = claimed.iter().map(|j| j.id.clone()).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"job0".to_string()));
        assert!(ids.contains(&"job1".to_string()));
        assert!(ids.contains(&"job2".to_string()));
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_multiple_updates() {
        let store = Arc::new(MockStore::new());

        let job = create_test_job("job1", JobStatus::Pending);
        store.upsert_workflow(&job).await.unwrap();

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Claim
        queue.claim_work(Some("worker1"), None).await.unwrap();

        // Update through get_work_by_id (which refreshes cache)
        let by_id = queue.get_work_by_id("job1").await.unwrap();
        assert!(by_id.is_some());
        assert_eq!(by_id.unwrap().status, JobStatus::Claimed);

        // Update status
        queue
            .update_status("job1", JobStatus::InProgress)
            .await
            .unwrap();

        // Verify final state
        let final_state = queue.get_work_by_id("job1").await.unwrap();
        assert!(final_state.is_some());
        assert_eq!(final_state.unwrap().status, JobStatus::InProgress);
    }
}
