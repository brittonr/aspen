//! Integration tests for job lifecycle.

use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_jobs::{
    Job, JobId, JobManager, JobOutput, JobResult, JobSpec, JobStatus, Priority, RetryPolicy,
    Worker, WorkerPool,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Test worker that tracks execution.
struct TestWorker {
    executed: Arc<Mutex<Vec<String>>>,
}

impl TestWorker {
    fn new(executed: Arc<Mutex<Vec<String>>>) -> Self {
        Self { executed }
    }
}

#[async_trait]
impl Worker for TestWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let mut executed = self.executed.lock().await;
        executed.push(job.id.to_string());

        // Simulate work based on payload
        if let Some(fail) = job.spec.payload.get("fail") {
            if fail.as_bool().unwrap_or(false) {
                return JobResult::failure("simulated failure");
            }
        }

        JobResult::success(serde_json::json!({
            "message": format!("Job {} completed", job.id)
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["test".to_string()]
    }
}

#[tokio::test]
async fn test_job_submission_and_retrieval() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    // Initialize queues
    manager.initialize().await.unwrap();

    // Submit a job
    let spec = JobSpec::new("test")
        .payload(serde_json::json!({ "data": "test" }))
        .unwrap()
        .priority(Priority::High);

    let job_id = manager.submit(spec).await.unwrap();
    assert!(!job_id.to_string().is_empty());

    // Retrieve the job
    let job = manager.get_job(&job_id).await.unwrap();
    assert!(job.is_some());
    let job = job.unwrap();
    assert_eq!(job.id, job_id);
    assert_eq!(job.spec.job_type, "test");
    assert_eq!(job.status, JobStatus::Pending);
}

#[tokio::test]
async fn test_job_priority_ordering() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    manager.initialize().await.unwrap();

    // Submit jobs with different priorities
    let low = manager
        .submit(
            JobSpec::new("test")
                .priority(Priority::Low)
                .payload(serde_json::json!({"priority": "low"}))
                .unwrap(),
        )
        .await
        .unwrap();

    let high = manager
        .submit(
            JobSpec::new("test")
                .priority(Priority::High)
                .payload(serde_json::json!({"priority": "high"}))
                .unwrap(),
        )
        .await
        .unwrap();

    let normal = manager
        .submit(
            JobSpec::new("test")
                .priority(Priority::Normal)
                .payload(serde_json::json!({"priority": "normal"}))
                .unwrap(),
        )
        .await
        .unwrap();

    let critical = manager
        .submit(
            JobSpec::new("test")
                .priority(Priority::Critical)
                .payload(serde_json::json!({"priority": "critical"}))
                .unwrap(),
        )
        .await
        .unwrap();

    // Verify all jobs were created
    assert!(manager.get_job(&low).await.unwrap().is_some());
    assert!(manager.get_job(&high).await.unwrap().is_some());
    assert!(manager.get_job(&normal).await.unwrap().is_some());
    assert!(manager.get_job(&critical).await.unwrap().is_some());
}

#[tokio::test]
async fn test_job_cancellation() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    manager.initialize().await.unwrap();

    // Submit a job
    let job_id = manager
        .submit(JobSpec::new("test").payload(serde_json::json!({})).unwrap())
        .await
        .unwrap();

    // Cancel the job
    manager.cancel_job(&job_id).await.unwrap();

    // Verify job is cancelled
    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Cancelled);

    // Cannot cancel again
    assert!(manager.cancel_job(&job_id).await.is_err());
}

#[tokio::test]
async fn test_job_idempotency() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    manager.initialize().await.unwrap();

    // Submit a job with idempotency key
    let spec = JobSpec::new("test")
        .payload(serde_json::json!({"data": "test"}))
        .unwrap()
        .idempotency_key("unique-key-123");

    let job_id1 = manager.submit(spec.clone()).await.unwrap();

    // Submit again with same idempotency key
    let job_id2 = manager.submit(spec).await.unwrap();

    // Should return the same job ID
    assert_eq!(job_id1, job_id2);
}

#[tokio::test]
async fn test_job_scheduling() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    manager.initialize().await.unwrap();

    // Schedule a job for the future
    let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
    let spec = JobSpec::new("test")
        .payload(serde_json::json!({}))
        .unwrap()
        .schedule_at(future_time);

    let job_id = manager.submit(spec).await.unwrap();

    // Job should be in scheduled state
    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Scheduled);
    assert!(job.scheduled_at.is_some());
}

#[tokio::test]
async fn test_job_progress_update() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    manager.initialize().await.unwrap();

    // Submit a job
    let job_id = manager
        .submit(JobSpec::new("test").payload(serde_json::json!({})).unwrap())
        .await
        .unwrap();

    // Mark job as started
    manager
        .mark_started(&job_id, "worker-1".to_string())
        .await
        .unwrap();

    // Update progress
    manager
        .update_progress(&job_id, 50, Some("Half way done".to_string()))
        .await
        .unwrap();

    // Verify progress
    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.progress, Some(50));
    assert_eq!(job.progress_message, Some("Half way done".to_string()));
}

#[tokio::test]
async fn test_job_completion() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    manager.initialize().await.unwrap();

    // Submit a job
    let job_id = manager
        .submit(JobSpec::new("test").payload(serde_json::json!({})).unwrap())
        .await
        .unwrap();

    // Mark as started
    manager
        .mark_started(&job_id, "worker-1".to_string())
        .await
        .unwrap();

    // Mark as completed
    let result = JobResult::success(serde_json::json!({ "output": "done" }));
    manager.mark_completed(&job_id, result).await.unwrap();

    // Verify completion
    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    assert!(job.result.is_some());
    assert!(job.completed_at.is_some());
}

#[tokio::test]
async fn test_retry_policy() {
    let job = Job::from_spec(
        JobSpec::new("test")
            .payload(serde_json::json!({}))
            .unwrap()
            .retry_policy(RetryPolicy::exponential(3)),
    );

    // Calculate retry times
    let mut test_job = job.clone();
    test_job.attempts = 1;
    let retry1 = test_job.calculate_next_retry();
    assert!(retry1.is_some());

    test_job.attempts = 2;
    let retry2 = test_job.calculate_next_retry();
    assert!(retry2.is_some());

    test_job.attempts = 3;
    let retry3 = test_job.calculate_next_retry();
    assert!(retry3.is_none()); // Exceeded max attempts
}

#[tokio::test]
async fn test_worker_pool_basic() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let executed = Arc::new(Mutex::new(Vec::new()));

    let pool = WorkerPool::new(store.clone());

    // Register test worker
    let worker = TestWorker::new(executed.clone());
    pool.register_handler("test", worker).await.unwrap();

    // Start workers
    pool.start(2).await.unwrap();

    // Get worker info
    let info = pool.get_worker_info().await;
    assert_eq!(info.len(), 2);

    // Get stats
    let stats = pool.get_stats().await;
    assert_eq!(stats.total_workers, 2);
    assert_eq!(stats.idle_workers, 2);

    // Shutdown
    pool.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_worker_job_processing() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    let executed = Arc::new(Mutex::new(Vec::new()));

    // Initialize system
    manager.initialize().await.unwrap();

    // Submit a job
    let job_id = manager
        .submit(
            JobSpec::new("test")
                .payload(serde_json::json!({"test": "concurrent"}))
                .unwrap()
                .priority(Priority::Normal),
        )
        .await
        .unwrap();

    // Create pool with same manager
    let pool = WorkerPool::with_manager(manager.clone());

    // Register worker
    let worker = TestWorker::new(executed.clone());
    pool.register_handler("test", worker).await.unwrap();

    // Start multiple workers
    pool.start(3).await.unwrap();

    // Give workers time to process
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that job was executed exactly once
    let executed_jobs = executed.lock().await;
    assert_eq!(executed_jobs.len(), 1, "Job should be executed exactly once");
    assert_eq!(executed_jobs[0], job_id.to_string());

    // Verify job status
    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Completed);

    pool.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_job_retry_with_concurrent_workers() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    let executed = Arc::new(Mutex::new(Vec::new()));

    manager.initialize().await.unwrap();

    // Submit a job that will fail first time
    let job_id = manager
        .submit(
            JobSpec::new("test")
                .payload(serde_json::json!({"fail": true}))
                .unwrap()
                .priority(Priority::Normal)
                .retry_policy(RetryPolicy::fixed(2, Duration::from_millis(100))),
        )
        .await
        .unwrap();

    let pool = WorkerPool::with_manager(manager.clone());

    // Create a worker that fails on first attempt
    struct RetryTestWorker {
        executed: Arc<Mutex<Vec<String>>>,
        attempt_count: Arc<Mutex<u32>>,
    }

    impl RetryTestWorker {
        fn new(executed: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                executed,
                attempt_count: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl Worker for RetryTestWorker {
        async fn execute(&self, job: Job) -> JobResult {
            let mut executed = self.executed.lock().await;
            executed.push(job.id.to_string());

            let mut count = self.attempt_count.lock().await;
            *count += 1;

            // Fail on first attempt
            if *count == 1 {
                JobResult::failure("simulated failure for retry test")
            } else {
                JobResult::success(serde_json::json!({"attempt": *count}))
            }
        }

        fn job_types(&self) -> Vec<String> {
            vec!["test".to_string()]
        }
    }

    let worker = RetryTestWorker::new(executed.clone());
    pool.register_handler("test", worker).await.unwrap();

    // Start multiple workers
    pool.start(2).await.unwrap();

    // Give time for initial failure and retry
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Job should eventually succeed
    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Completed, "Job should complete after retry");
    assert!(job.attempts > 1, "Job should have multiple attempts");

    pool.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_job_dependencies() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    // Initialize queues
    manager.initialize().await.unwrap();

    // Submit job A (no dependencies)
    let spec_a = JobSpec::new("test")
        .payload(serde_json::json!({ "job": "A" }))
        .unwrap();
    let job_a_id = manager.submit(spec_a).await.unwrap();

    // Submit job B that depends on A
    let mut spec_b = JobSpec::new("test")
        .payload(serde_json::json!({ "job": "B" }))
        .unwrap();
    spec_b.config.dependencies.push(job_a_id.clone());
    let job_b_id = manager.submit(spec_b).await.unwrap();

    // Submit job C that depends on B
    let mut spec_c = JobSpec::new("test")
        .payload(serde_json::json!({ "job": "C" }))
        .unwrap();
    spec_c.config.dependencies.push(job_b_id.clone());
    let job_c_id = manager.submit(spec_c).await.unwrap();

    // Check job states
    let job_a = manager.get_job(&job_a_id).await.unwrap().unwrap();
    let job_b = manager.get_job(&job_b_id).await.unwrap().unwrap();
    let job_c = manager.get_job(&job_c_id).await.unwrap().unwrap();

    // Job A should be pending (ready to run)
    assert_eq!(job_a.status, JobStatus::Pending);
    assert!(job_a.dependency_state.is_ready());

    // Job B should be pending but blocked on A
    assert_eq!(job_b.status, JobStatus::Pending);
    assert!(!job_b.dependency_state.is_ready());

    // Job C should be pending but blocked on B
    assert_eq!(job_c.status, JobStatus::Pending);
    assert!(!job_c.dependency_state.is_ready());

    // Complete job A
    manager.mark_started(&job_a_id, "worker1".to_string()).await.unwrap();
    manager.mark_completed(&job_a_id, JobResult::success(serde_json::json!({"done": true}))).await.unwrap();

    // Now job B should be unblocked and ready
    let job_b = manager.get_job(&job_b_id).await.unwrap().unwrap();
    assert!(job_b.dependency_state.is_ready());

    // Job C should still be blocked
    let job_c = manager.get_job(&job_c_id).await.unwrap().unwrap();
    assert!(!job_c.dependency_state.is_ready());

    // Complete job B
    manager.mark_started(&job_b_id, "worker1".to_string()).await.unwrap();
    manager.mark_completed(&job_b_id, JobResult::success(serde_json::json!({"done": true}))).await.unwrap();

    // Now job C should be unblocked and ready
    let job_c = manager.get_job(&job_c_id).await.unwrap().unwrap();
    assert!(job_c.dependency_state.is_ready());
}

#[tokio::test]
async fn test_job_dependency_failure_cascade() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = JobManager::new(store.clone());

    // Initialize queues
    manager.initialize().await.unwrap();

    // Submit job A (no dependencies)
    let spec_a = JobSpec::new("test")
        .payload(serde_json::json!({ "job": "A" }))
        .unwrap();
    let job_a_id = manager.submit(spec_a).await.unwrap();

    // Submit job B that depends on A
    let mut spec_b = JobSpec::new("test")
        .payload(serde_json::json!({ "job": "B" }))
        .unwrap();
    spec_b.config.dependencies.push(job_a_id.clone());
    let job_b_id = manager.submit(spec_b).await.unwrap();

    // Mark job A as failed
    manager.mark_started(&job_a_id, "worker1".to_string()).await.unwrap();
    manager.mark_completed(&job_a_id, JobResult::failure("Job A failed")).await.unwrap();

    // Job B should be marked as failed due to cascade
    let job_b = manager.get_job(&job_b_id).await.unwrap().unwrap();
    assert_eq!(job_b.status, JobStatus::Failed);
    assert!(job_b.last_error.is_some());
    assert!(job_b.last_error.unwrap().contains("Dependency"));
}