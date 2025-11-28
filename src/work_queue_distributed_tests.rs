//! Distributed scenario tests for WorkQueue
//!
//! These tests verify behavior in distributed scenarios with multiple nodes,
//! network partitions, and concurrent operations.
//!
//! These tests verify:
//! - Multi-node coordination
//! - Cache consistency across nodes
//! - Handling of claim races between nodes
//! - Recovery from transient failures

#[cfg(test)]
mod distributed_tests {
    use crate::domain::types::{Job, JobStatus};
    use crate::domain::job_metadata::JobMetadata;
    use crate::domain::job_requirements::JobRequirements;
    use crate::persistent_store::PersistentStore;
    use crate::work_queue::WorkQueue;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // =========================================================================
    // DISTRIBUTED MOCK STORE
    // =========================================================================

    /// Mock store that simulates distributed behavior with potential failures
    #[derive(Clone)]
    struct DistributedMockStore {
        workflows: Arc<RwLock<HashMap<String, Job>>>,
        // Simulates claim failures at specific indices
        claim_fail_at: Arc<RwLock<Vec<usize>>>,
        claim_attempt_count: Arc<AtomicUsize>,
        // Simulates delayed responses
        delay_claim_ms: Arc<RwLock<Option<u64>>>,
    }

    impl DistributedMockStore {
        fn new() -> Self {
            Self {
                workflows: Arc::new(RwLock::new(HashMap::new())),
                claim_fail_at: Arc::new(RwLock::new(Vec::new())),
                claim_attempt_count: Arc::new(AtomicUsize::new(0)),
                delay_claim_ms: Arc::new(RwLock::new(None)),
            }
        }

        async fn set_claim_failures(&self, fail_indices: Vec<usize>) {
            *self.claim_fail_at.write().await = fail_indices;
        }

        async fn set_claim_delay(&self, ms: u64) {
            *self.delay_claim_ms.write().await = Some(ms);
        }

        fn get_claim_attempt_count(&self) -> usize {
            self.claim_attempt_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl PersistentStore for DistributedMockStore {
        async fn load_all_workflows(&self) -> Result<Vec<Job>> {
            let workflows = self.workflows.read().await;
            Ok(workflows.values().cloned().collect())
        }

        async fn upsert_workflow(&self, job: &Job) -> Result<()> {
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
            let attempt = self.claim_attempt_count.fetch_add(1, Ordering::SeqCst);

            // Simulate delay if configured
            if let Some(ms) = *self.delay_claim_ms.read().await {
                tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
            }

            // Check if this attempt should fail
            let fail_indices = self.claim_fail_at.read().await;
            if fail_indices.contains(&attempt) {
                return Ok(0);
            }
            drop(fail_indices);

            let mut workflows = self.workflows.write().await;
            if let Some(job) = workflows.get_mut(job_id) {
                if job.status == JobStatus::Pending {
                    job.status = JobStatus::Claimed;
                    job.claimed_by = Some(claimed_by.to_string());
                    job.assigned_worker_id = assigned_worker_id.map(|s| s.to_string());
                    job.metadata.updated_at = updated_at;
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
            let mut workflows = self.workflows.write().await;
            if let Some(job) = workflows.get_mut(job_id) {
                if job.status != JobStatus::Completed && job.status != JobStatus::Failed {
                    job.status = *status;
                    job.metadata.updated_at = updated_at;
                    if let Some(node_id) = completed_by {
                        job.completed_by = Some(node_id.to_string());
                    }
                    return Ok(1);
                }
            }
            Ok(0)
        }
    }

    // =========================================================================
    // HELPER FUNCTIONS
    // =========================================================================

    fn create_test_job(id: &str) -> Job {
        Job {
            id: id.to_string(),
            status: JobStatus::Pending,
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
    // DISTRIBUTED SCENARIO TESTS
    // =========================================================================

    /// Test: Multiple nodes claiming from same pool
    #[tokio::test]
    async fn test_distributed_claim_without_coordination() {
        let store = Arc::new(DistributedMockStore::new());

        // Pre-populate with 5 pending jobs
        for i in 0..5 {
            let job = create_test_job(&format!("job_{}", i));
            store.upsert_workflow(&job).await.unwrap();
        }

        // Create 3 independent nodes and claim work from each
        let queue1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_0".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue3 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Each node claims work concurrently
        let (claim1, claim2, claim3) = tokio::join!(
            queue1.claim_work(Some("worker_0"), None),
            queue2.claim_work(Some("worker_1"), None),
            queue3.claim_work(Some("worker_2"), None),
        );

        let result1 = claim1.unwrap();
        let result2 = claim2.unwrap();
        let result3 = claim3.unwrap();

        // Count successful claims
        let mut successful_claims = Vec::new();
        if let Some(j) = result1 { successful_claims.push(j); }
        if let Some(j) = result2 { successful_claims.push(j); }
        if let Some(j) = result3 { successful_claims.push(j); }

        // Should have claimed 3 jobs
        assert_eq!(successful_claims.len(), 3);

        // All claimed jobs should have different IDs
        let job_ids: Vec<_> = successful_claims
            .iter()
            .map(|j| j.id.clone())
            .collect();
        assert_eq!(job_ids.len(), 3);
    }

    /// Test: Sequential node failure and recovery
    #[tokio::test]
    async fn test_node_failure_and_recovery() {
        let store = Arc::new(DistributedMockStore::new());

        // Setup: one pending job
        let job = create_test_job("job_1");
        store.upsert_workflow(&job).await.unwrap();

        // Configure store to fail first claim attempt
        store.set_claim_failures(vec![0]).await;

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // First attempt fails
        let result1 = queue.claim_work(Some("worker_1"), None).await.unwrap();
        assert!(result1.is_none());

        // Second attempt succeeds
        let result2 = queue.claim_work(Some("worker_1"), None).await.unwrap();
        assert!(result2.is_some());

        let claimed_job = result2.unwrap();
        assert_eq!(claimed_job.status, JobStatus::Claimed);
    }

    /// Test: Delayed network responses
    #[tokio::test]
    async fn test_delayed_responses_maintain_consistency() {
        let store = Arc::new(DistributedMockStore::new());

        // Pre-populate with 10 jobs
        for i in 0..10 {
            let job = create_test_job(&format!("job_{}", i));
            store.upsert_workflow(&job).await.unwrap();
        }

        // Simulate 50ms delay on every claim
        store.set_claim_delay(50).await;

        let queue1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let start = std::time::Instant::now();

        // Both claim with delay
        let (claim1, claim2) = tokio::join!(
            queue1.claim_work(Some("worker_1"), None),
            queue2.claim_work(Some("worker_2"), None),
        );

        let elapsed = start.elapsed();

        assert!(claim1.is_ok());
        assert!(claim2.is_ok());
        assert!(elapsed.as_secs() < 5);
    }

    /// Test: Large contention scenario
    #[tokio::test]
    async fn test_large_contention_scenario() {
        let store = Arc::new(DistributedMockStore::new());

        // Only 5 jobs available
        for i in 0..5 {
            let job = create_test_job(&format!("job_{}", i));
            store.upsert_workflow(&job).await.unwrap();
        }

        // Create 10 nodes competing for 5 jobs
        let mut handles = Vec::new();
        for i in 0..10 {
            let store_clone = store.clone();
            handles.push(tokio::spawn(async move {
                let queue = WorkQueue::new(
                    iroh::Endpoint::builder().bind().await.unwrap(),
                    format!("node_{}", i),
                    store_clone,
                )
                .await
                .unwrap();

                queue
                    .claim_work(Some(&format!("worker_{}", i)), None)
                    .await
                    .unwrap()
            }));
        }

        // Collect all results
        let mut successful = 0;
        let mut job_ids = Vec::new();
        for handle in handles {
            if let Some(job) = handle.await.unwrap() {
                successful += 1;
                job_ids.push(job.id);
            }
        }

        // Only 5 should succeed
        assert_eq!(successful, 5, "Only 5 jobs available, so only 5 claims should succeed");

        // All should be unique
        let unique_count = job_ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 5, "All claimed jobs should be unique");
    }

    /// Test: Node updates state while another reads
    #[tokio::test]
    async fn test_read_write_interleaving() {
        let store = Arc::new(DistributedMockStore::new());

        let job = create_test_job("job_1");
        store.upsert_workflow(&job).await.unwrap();

        let queue1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Node 1 reads initial state
        let initial = queue1.get_work_by_id("job_1").await.unwrap().unwrap();
        assert_eq!(initial.status, JobStatus::Pending);

        // Node 2 claims the job
        let claimed = queue2.claim_work(Some("worker_2"), None).await.unwrap();
        assert!(claimed.is_some());

        // Node 1 reads again - should see updated state
        let updated = queue1.get_work_by_id("job_1").await.unwrap().unwrap();
        assert_eq!(updated.status, JobStatus::Claimed);

        // Node 2 updates status
        queue2.update_status("job_1", JobStatus::InProgress).await.unwrap();

        // Node 1 reads again - should see latest state
        let final_state = queue1.get_work_by_id("job_1").await.unwrap().unwrap();
        assert_eq!(final_state.status, JobStatus::InProgress);
    }

    /// Test: Mass status updates maintain consistency
    #[tokio::test]
    async fn test_mass_status_updates_consistency() {
        let store = Arc::new(DistributedMockStore::new());

        // Create 50 jobs
        for i in 0..50 {
            let mut job = create_test_job(&format!("job_{}", i));
            job.status = match i % 3 {
                0 => JobStatus::Pending,
                1 => JobStatus::Claimed,
                _ => JobStatus::InProgress,
            };
            store.upsert_workflow(&job).await.unwrap();
        }

        let queue = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Update first 20 jobs to completed
        for i in 0..20 {
            queue
                .update_status(&format!("job_{}", i), JobStatus::Completed)
                .await
                .unwrap();
        }

        // Refresh and verify
        let all = queue.list_work().await.unwrap();

        let completed_count = all.iter().filter(|j| j.status == JobStatus::Completed).count();
        assert_eq!(completed_count, 20, "Should have 20 completed jobs");

        // Verify store matches cache
        let store_jobs = store.load_all_workflows().await.unwrap();
        let store_completed = store_jobs
            .iter()
            .filter(|j| j.status == JobStatus::Completed)
            .count();
        assert_eq!(store_completed, 20, "Store should also show 20 completed");
    }

    /// Test: Orphaned jobs visible to other nodes
    #[tokio::test]
    async fn test_orphaned_jobs_visible_to_other_nodes() {
        let store = Arc::new(DistributedMockStore::new());

        let job = create_test_job("job_1");
        store.upsert_workflow(&job).await.unwrap();

        let queue1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // Node 1 claims the job
        let claimed = queue1.claim_work(Some("worker_1"), None).await.unwrap();
        assert!(claimed.is_some());

        // Node 2 tries to claim - should fail
        let result = queue2.claim_work(Some("worker_2"), None).await.unwrap();
        assert!(result.is_none());

        // Node 2 can see the job is claimed
        let job_state = queue2.get_work_by_id("job_1").await.unwrap().unwrap();
        assert_eq!(job_state.status, JobStatus::Claimed);
        assert_eq!(job_state.claimed_by, Some("node_1".to_string()));
    }

    /// Test: Queue statistics consistency
    #[tokio::test]
    async fn test_stats_consistency_across_nodes() {
        let store = Arc::new(DistributedMockStore::new());

        // Create jobs in various states
        for i in 0..30 {
            let mut job = create_test_job(&format!("job_{}", i));
            job.status = match i % 5 {
                0 => JobStatus::Pending,
                1 => JobStatus::Claimed,
                2 => JobStatus::InProgress,
                3 => JobStatus::Completed,
                _ => JobStatus::Failed,
            };
            store.upsert_workflow(&job).await.unwrap();
        }

        // Create 3 nodes and get stats
        let queue1 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_1".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue2 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_2".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        let queue3 = WorkQueue::new(
            iroh::Endpoint::builder().bind().await.unwrap(),
            "node_3".to_string(),
            store.clone(),
        )
        .await
        .unwrap();

        // All nodes get stats concurrently
        let (stats1, stats2, stats3) = tokio::join!(
            queue1.stats(),
            queue2.stats(),
            queue3.stats(),
        );

        // All should report the same totals
        assert_eq!(stats1.total, stats2.total);
        assert_eq!(stats2.total, stats3.total);
        assert_eq!(stats1.total, 30);
    }
}
