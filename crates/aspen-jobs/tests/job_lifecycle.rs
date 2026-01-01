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