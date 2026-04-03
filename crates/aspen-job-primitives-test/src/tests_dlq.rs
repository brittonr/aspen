//! Dead letter queue test.
//!
//! Scenario 1: Job with RetryPolicy::None fails → goes straight to DLQ.
//! Scenario 2: DLQ job can be redriven back to Pending.

use std::sync::Arc;
use std::time::Duration;

use aspen_jobs::JobManager;
use aspen_jobs::JobManagerConfig;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::RetryPolicy;

use crate::TestResult;
use crate::make_store;

pub async fn run() -> TestResult {
    test_job_moves_to_dlq_after_retries().await?;
    test_dlq_job_can_be_redriven().await?;
    Ok(())
}

/// Submit a job with no retries. Dequeue, start, nack → should go to DLQ.
async fn test_job_moves_to_dlq_after_retries() -> TestResult {
    let store = make_store();
    let config = JobManagerConfig {
        enable_deduplication: false,
        ..Default::default()
    };
    let manager = Arc::new(JobManager::with_config(store.clone(), config));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    // Submit job with no retries — first failure goes to DLQ
    let spec = JobSpec::new("failing-job").retry_policy(RetryPolicy::None);

    let job_id = manager.submit(spec).await.map_err(|e| format!("submit: {e}"))?;

    // Dequeue
    let items = manager
        .dequeue_jobs("test-worker", 1, Duration::from_secs(30))
        .await
        .map_err(|e| format!("dequeue: {e}"))?;

    if items.is_empty() {
        return Err("no jobs dequeued".to_string());
    }

    let (dequeued_item, _job) = &items[0];
    let receipt_handle = dequeued_item.receipt_handle.clone();

    // Start the job
    let execution_token = manager
        .mark_started(&job_id, "worker-0".to_string())
        .await
        .map_err(|e| format!("mark_started: {e}"))?;

    // Nack with no retries → should go to DLQ
    manager
        .nack_job(&job_id, &receipt_handle, &execution_token, "intentional failure".to_string())
        .await
        .map_err(|e| format!("nack: {e}"))?;

    // Verify job is in DLQ
    let job = manager.get_job(&job_id).await.map_err(|e| format!("get_job: {e}"))?.ok_or("job not found")?;

    if job.status != JobStatus::DeadLetter {
        return Err(format!("expected DeadLetter status, got {:?}", job.status));
    }

    let dlq_meta = job.dlq_metadata.as_ref().ok_or("missing DLQ metadata")?;
    println!("  DLQ: job moved to DLQ (reason={:?}, error={})", dlq_meta.reason, dlq_meta.final_error);

    Ok(())
}

/// Redrive a DLQ job back to Pending status.
async fn test_dlq_job_can_be_redriven() -> TestResult {
    let store = make_store();
    let config = JobManagerConfig {
        enable_deduplication: false,
        ..Default::default()
    };
    let manager = Arc::new(JobManager::with_config(store.clone(), config));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    // Submit and fail immediately (no retries)
    let spec = JobSpec::new("redrive-test").retry_policy(RetryPolicy::None);

    let job_id = manager.submit(spec).await.map_err(|e| format!("submit: {e}"))?;

    // Dequeue
    let items = manager
        .dequeue_jobs("test-worker", 1, Duration::from_secs(30))
        .await
        .map_err(|e| format!("dequeue: {e}"))?;

    if items.is_empty() {
        return Err("no jobs dequeued".to_string());
    }

    let (dequeued_item, _job) = &items[0];
    let receipt_handle = dequeued_item.receipt_handle.clone();

    // Start
    let execution_token = manager
        .mark_started(&job_id, "worker-0".to_string())
        .await
        .map_err(|e| format!("mark_started: {e}"))?;

    // Nack → DLQ
    manager
        .nack_job(&job_id, &receipt_handle, &execution_token, "fail".to_string())
        .await
        .map_err(|e| format!("nack: {e}"))?;

    let job = manager.get_job(&job_id).await.map_err(|e| format!("get_job: {e}"))?.ok_or("job not found")?;

    if job.status != JobStatus::DeadLetter {
        return Err(format!("expected DeadLetter, got {:?}", job.status));
    }

    // Redrive from DLQ
    manager.redrive_job(&job_id).await.map_err(|e| format!("redrive_job: {e}"))?;

    // Verify job is back to Pending
    let job = manager
        .get_job(&job_id)
        .await
        .map_err(|e| format!("get_job after redrive: {e}"))?
        .ok_or("job not found after redrive")?;

    if job.status != JobStatus::Pending {
        return Err(format!("expected Pending after redrive, got {:?}", job.status));
    }

    println!("  DLQ redrive: job correctly moved back to Pending");
    Ok(())
}
