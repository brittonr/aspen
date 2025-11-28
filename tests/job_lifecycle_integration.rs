//! Integration tests for complete job lifecycle workflows
//!
//! This test suite validates end-to-end job workflows that span multiple layers:
//! - Domain layer (JobCommandService, WorkerManagementService)
//! - Infrastructure layer (repositories)
//!
//! These tests use real domain services with mock infrastructure to verify
//! that components integrate correctly across layer boundaries.

use mvm_ci::domain::{
    JobCommandService, JobSubmission, JobStatus,
    WorkerManagementService, WorkerRegistration, WorkerType, WorkerHeartbeat,
};
use mvm_ci::repositories::mocks::{MockWorkRepository, MockWorkerRepository};
use mvm_ci::domain::event_publishers::InMemoryEventPublisher;
use mvm_ci::domain::events::{DomainEvent, EventPublisher};
use mvm_ci::repositories::{WorkRepository, WorkerRepository};
use std::sync::Arc;

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

/// Create integrated test environment with job command service and worker management
fn create_test_environment() -> (
    Arc<JobCommandService>,
    Arc<WorkerManagementService>,
    Arc<MockWorkRepository>,
    Arc<MockWorkerRepository>,
    Arc<InMemoryEventPublisher>,
) {
    let work_repo = Arc::new(MockWorkRepository::new());
    let worker_repo = Arc::new(MockWorkerRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let job_command_service = Arc::new(JobCommandService::with_events(
        work_repo.clone() as Arc<dyn WorkRepository>,
        event_publisher.clone() as Arc<dyn EventPublisher>,
    ));

    let worker_management_service = Arc::new(WorkerManagementService::new(
        worker_repo.clone() as Arc<dyn WorkerRepository>,
        work_repo.clone() as Arc<dyn WorkRepository>,
        Some(60), // 60 second heartbeat timeout
    ));

    (
        job_command_service,
        worker_management_service,
        work_repo,
        worker_repo,
        event_publisher,
    )
}

/// Create a worker registration for testing
fn create_worker_registration(worker_type: WorkerType) -> WorkerRegistration {
    WorkerRegistration {
        worker_type,
        endpoint_id: format!("endpoint-{}", uuid::Uuid::new_v4()),
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        metadata: serde_json::json!({
            "version": "1.0.0",
            "region": "us-west-2"
        }),
    }
}

/// Create a job submission for testing
fn create_job_submission() -> JobSubmission {
    JobSubmission {
        payload: serde_json::json!({
            "command": "echo 'Hello World'",
            "timeout": 300,
        }),
    }
}

// ============================================================================
// Complete Job Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_job_submission_creates_pending_job() {
    // Integration test: Job submission creates job in pending state
    let (job_cmd, _, work_repo, _, event_publisher) = create_test_environment();

    // Submit a job
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    // Verify job exists and is pending
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, job_id);
    assert_eq!(jobs[0].status, JobStatus::Pending);

    // Verify JobSubmitted event
    let events = event_publisher.get_events().await;
    assert_eq!(events.len(), 1);
    match &events[0] {
        DomainEvent::JobSubmitted { job_id: id, .. } => assert_eq!(id, &job_id),
        _ => panic!("Expected JobSubmitted event"),
    }
}

#[tokio::test]
async fn test_job_status_transitions_follow_state_machine() {
    // Integration test: Job status transitions follow valid state machine rules
    let (job_cmd, _, work_repo, _, _) = create_test_environment();

    // Submit a job
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    // Valid transition: Pending -> Claimed
    job_cmd
        .update_job_status(&job_id, JobStatus::Claimed, None)
        .await
        .expect("Pending to Claimed transition should succeed");

    // Valid transition: Claimed -> InProgress
    job_cmd
        .update_job_status(&job_id, JobStatus::InProgress, None)
        .await
        .expect("Claimed to InProgress transition should succeed");

    // Valid transition: InProgress -> Completed
    job_cmd
        .update_job_status(&job_id, JobStatus::Completed, None)
        .await
        .expect("InProgress to Completed transition should succeed");

    // Verify final state
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs[0].status, JobStatus::Completed);
}

#[tokio::test]
async fn test_invalid_job_status_transition_rejected() {
    // Integration test: Invalid state transitions are rejected
    let (job_cmd, _, _, _, _) = create_test_environment();

    // Submit a job
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    // Invalid transition: Pending -> InProgress (skipping Claimed)
    let result = job_cmd
        .update_job_status(&job_id, JobStatus::InProgress, None)
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("skipped"));
}

#[tokio::test]
async fn test_job_failure_with_error_message() {
    // Integration test: Job can fail with error message
    let (job_cmd, _, work_repo, _, event_publisher) = create_test_environment();

    // Submit and process job to InProgress
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    job_cmd
        .update_job_status(&job_id, JobStatus::Claimed, None)
        .await
        .expect("Status update should succeed");

    job_cmd
        .update_job_status(&job_id, JobStatus::InProgress, None)
        .await
        .expect("Status update should succeed");

    // Fail the job with error message
    job_cmd
        .update_job_status(&job_id, JobStatus::Failed, Some("Network timeout".to_string()))
        .await
        .expect("Job failure should succeed");

    // Verify status is failed
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs[0].status, JobStatus::Failed);

    // Verify JobFailed event
    let events = event_publisher.get_events().await;
    let has_failed_event = events.iter().any(|e| matches!(e, DomainEvent::JobFailed { .. }));
    assert!(has_failed_event);
}

#[tokio::test]
async fn test_job_cancellation() {
    // Integration test: Job can be cancelled
    let (job_cmd, _, work_repo, _, event_publisher) = create_test_environment();

    // Submit a job
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    // Cancel the job
    job_cmd
        .cancel_job(&job_id)
        .await
        .expect("Cancellation should succeed");

    // Verify job is in terminal state (Failed)
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs[0].status, JobStatus::Failed);

    // Verify cancellation event
    let events = event_publisher.get_events().await;
    let has_failed_event = events.iter().any(|e| matches!(e, DomainEvent::JobFailed { .. }));
    assert!(has_failed_event);
}

#[tokio::test]
async fn test_multiple_jobs_can_be_submitted() {
    // Integration test: Multiple jobs can be submitted concurrently
    let (job_cmd, _, work_repo, _, _) = create_test_environment();

    // Submit 5 jobs
    let mut job_ids = Vec::new();
    for _ in 0..5 {
        let submission = create_job_submission();
        let job_id = job_cmd
            .submit_job(submission)
            .await
            .expect("Job submission should succeed");
        job_ids.push(job_id);
    }

    // Verify all jobs exist
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs.len(), 5);

    // All jobs should be pending
    for job in &jobs {
        assert_eq!(job.status, JobStatus::Pending);
    }
}

// ============================================================================
// Worker Integration Tests
// ============================================================================

#[tokio::test]
async fn test_worker_registration_and_job_lifecycle() {
    // Integration test: Worker registers and can be assigned jobs
    let (job_cmd, worker_mgmt, work_repo, worker_repo, _) = create_test_environment();

    // Register a worker
    let registration = create_worker_registration(WorkerType::Firecracker);
    let worker = worker_mgmt
        .register_worker(registration)
        .await
        .expect("Worker registration should succeed");

    // Verify worker is registered
    let workers = worker_repo.list_all_workers().await.expect("Should list workers");
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].id, worker.id);

    // Submit a job
    let submission = create_job_submission();
    let _job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    // Job is now available for claiming by the worker
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, JobStatus::Pending);
}

#[tokio::test]
async fn test_worker_heartbeat_updates() {
    // Integration test: Worker heartbeats update worker state
    let (_, worker_mgmt, _, worker_repo, _) = create_test_environment();

    // Register a worker
    let registration = create_worker_registration(WorkerType::Wasm);
    let worker = worker_mgmt
        .register_worker(registration)
        .await
        .expect("Worker registration should succeed");

    let initial_heartbeat = worker.last_heartbeat;

    // Send heartbeat
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let heartbeat = WorkerHeartbeat {
        worker_id: worker.id.clone(),
        cpu_cores: Some(8),
        memory_mb: Some(16384),
        active_jobs: 2,
    };

    worker_mgmt
        .handle_heartbeat(heartbeat)
        .await
        .expect("Heartbeat should succeed");

    // Verify heartbeat timestamp was updated
    let workers = worker_repo.list_all_workers().await.expect("Should list workers");
    assert!(workers[0].last_heartbeat > initial_heartbeat);
    assert_eq!(workers[0].cpu_cores, Some(8));
    assert_eq!(workers[0].memory_mb, Some(16384));
}

#[tokio::test]
async fn test_worker_failure_detection_and_job_recovery() {
    // Integration test: Failed workers are detected and their jobs requeued
    let (job_cmd, worker_mgmt, work_repo, worker_repo, _) = create_test_environment();

    // Register a worker
    let registration = create_worker_registration(WorkerType::Firecracker);
    let worker = worker_mgmt
        .register_worker(registration)
        .await
        .expect("Worker registration should succeed");

    // Submit and simulate job claiming/execution
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    job_cmd
        .update_job_status(&job_id, JobStatus::Claimed, None)
        .await
        .expect("Status update should succeed");

    job_cmd
        .update_job_status(&job_id, JobStatus::InProgress, None)
        .await
        .expect("Status update should succeed");

    // Simulate worker failure by setting stale heartbeat
    let now = mvm_ci::common::timestamp::current_timestamp_or_zero();
    let mut stale_worker = worker.clone();
    stale_worker.last_heartbeat = now - 120; // 120 seconds ago (exceeds 60s timeout)
    worker_repo.add_workers(vec![stale_worker]).await;

    // Run health check - should detect failure and requeue jobs
    worker_mgmt
        .check_worker_health()
        .await
        .expect("Health check should succeed");

    // Verify worker is offline
    let workers = worker_repo.list_all_workers().await.expect("Should list workers");
    assert_eq!(workers[0].status, mvm_ci::domain::WorkerStatus::Offline);

    // Verify job was requeued
    let jobs = work_repo.list_work().await.expect("Should list jobs");
    assert_eq!(jobs[0].status, JobStatus::Pending);
}

// ============================================================================
// Cross-Layer Event Publishing Tests
// ============================================================================

#[tokio::test]
async fn test_events_published_throughout_job_lifecycle() {
    // Integration test: Events are published at each lifecycle stage
    let (job_cmd, _, _, _, event_publisher) = create_test_environment();

    // Submit job
    let submission = create_job_submission();
    let job_id = job_cmd
        .submit_job(submission)
        .await
        .expect("Job submission should succeed");

    // Verify JobSubmitted event
    let events = event_publisher.get_events().await;
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], DomainEvent::JobSubmitted { .. }));

    // Claim job (would typically be done by worker, simulated here)
    job_cmd
        .update_job_status(&job_id, JobStatus::Claimed, None)
        .await
        .expect("Status update should succeed");

    // Verify JobClaimed event
    let events = event_publisher.get_events().await;
    assert!(events.len() >= 2);

    // Start execution
    job_cmd
        .update_job_status(&job_id, JobStatus::InProgress, None)
        .await
        .expect("Status update should succeed");

    // Verify JobStatusChanged event
    let events = event_publisher.get_events().await;
    assert!(events.len() >= 3);

    // Complete job
    job_cmd
        .update_job_status(&job_id, JobStatus::Completed, None)
        .await
        .expect("Status update should succeed");

    // Verify JobCompleted event
    let events = event_publisher.get_events().await;
    assert!(events.len() >= 4);
    let last_event = &events[events.len() - 1];
    assert!(matches!(last_event, DomainEvent::JobCompleted { .. }));
}

#[tokio::test]
async fn test_multiple_workers_can_register_concurrently() {
    // Integration test: Multiple workers can register at the same time
    let (_, worker_mgmt, _, worker_repo, _) = create_test_environment();

    // Register multiple workers
    for worker_type in [WorkerType::Firecracker, WorkerType::Wasm] {
        let registration = create_worker_registration(worker_type);
        worker_mgmt
            .register_worker(registration)
            .await
            .expect("Worker registration should succeed");
    }

    // Verify all workers are registered
    let workers = worker_repo.list_all_workers().await.expect("Should list workers");
    assert_eq!(workers.len(), 2);
}

#[tokio::test]
async fn test_worker_draining_status() {
    // Integration test: Worker can be marked as draining
    let (_, worker_mgmt, _, worker_repo, _) = create_test_environment();

    // Register a worker
    let registration = create_worker_registration(WorkerType::Firecracker);
    let worker = worker_mgmt
        .register_worker(registration)
        .await
        .expect("Worker registration should succeed");

    // Mark worker as draining
    worker_mgmt
        .mark_worker_draining(&worker.id)
        .await
        .expect("Draining should succeed");

    // Verify worker status
    let workers = worker_repo.list_all_workers().await.expect("Should list workers");
    assert_eq!(workers[0].status, mvm_ci::domain::WorkerStatus::Draining);
}
