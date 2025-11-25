//! Scenario-based integration tests
//!
//! These tests verify end-to-end scenarios using real infrastructure components.
//! Tests use ControlPlaneFixture and TestWorkerFixture for realistic testing.

mod common;

use anyhow::Result;
use common::{ControlPlaneFixture, TestWorkerFixture};
use mvm_ci::domain::types::{JobStatus, WorkerType};
use std::time::Duration;

// ============================================================================
// Scenario 1: Worker Registration Flow
// ============================================================================

#[tokio::test]
async fn test_worker_registration_flow() -> Result<()> {
    // Start control plane on a unique port
    let control_plane = ControlPlaneFixture::new(3030).await?;
    let client = control_plane.client().await;

    // Create and register a WASM worker
    let mut wasm_worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;

    let worker_id = wasm_worker.register().await?;
    assert!(!worker_id.is_empty(), "Worker ID should not be empty");

    // Verify worker appears in registry
    let workers = client.list_workers().await?;
    assert_eq!(workers.len(), 1, "Should have 1 registered worker");
    assert_eq!(workers[0].id, worker_id);
    assert_eq!(workers[0].worker_type, WorkerType::Wasm);

    // Verify we can get worker details
    let worker_details = client.get_worker(&worker_id).await?;
    assert!(
        worker_details.is_some(),
        "Worker details should be retrievable"
    );

    let worker = worker_details.unwrap();
    assert_eq!(worker.worker_type, WorkerType::Wasm);
    assert!(worker.cpu_cores.is_some());
    assert!(worker.memory_mb.is_some());

    // Cleanup
    wasm_worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_multiple_worker_registration() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3031).await?;
    let client = control_plane.client().await;

    // Register multiple workers of different types
    let mut wasm_worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    let wasm_id = wasm_worker.register().await?;

    let mut fc_worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Firecracker,
    )
    .await?;
    let fc_id = fc_worker.register().await?;

    // Verify both workers are registered
    let workers = client.list_workers().await?;
    assert_eq!(workers.len(), 2, "Should have 2 registered workers");

    let worker_ids: Vec<String> = workers.iter().map(|w| w.id.clone()).collect();
    assert!(worker_ids.contains(&wasm_id));
    assert!(worker_ids.contains(&fc_id));

    // Verify we can distinguish by worker type
    let wasm_workers: Vec<_> = workers
        .iter()
        .filter(|w| w.worker_type == WorkerType::Wasm)
        .collect();
    let fc_workers: Vec<_> = workers
        .iter()
        .filter(|w| w.worker_type == WorkerType::Firecracker)
        .collect();

    assert_eq!(wasm_workers.len(), 1);
    assert_eq!(fc_workers.len(), 1);

    // Cleanup
    wasm_worker.shutdown().await;
    fc_worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

// ============================================================================
// Scenario 2: Heartbeat & Health Monitoring
// ============================================================================

#[tokio::test]
async fn test_worker_heartbeat_flow() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3032).await?;
    let client = control_plane.client().await;

    let mut worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    let worker_id = worker.register().await?;

    // Start sending heartbeats
    worker.start_heartbeat();

    // Wait for heartbeats to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify worker is showing as healthy/active
    let worker_details = client.get_worker(&worker_id).await?.unwrap();
    // TODO: Add timestamp verification once heartbeat timestamps are tracked

    // Stop heartbeats
    worker.stop_heartbeat();

    // Cleanup
    worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_heartbeat_with_active_jobs_count() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3033).await?;
    let client = control_plane.client().await;

    let mut worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    worker.register().await?;

    // Start heartbeat (active_jobs should be 0)
    worker.start_heartbeat();
    assert_eq!(worker.active_jobs(), 0);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // TODO: When we add job claiming, verify active_jobs count updates

    worker.stop_heartbeat();
    worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

// ============================================================================
// Scenario 3: Job Claiming with Worker Type Filtering
// ============================================================================

#[tokio::test]
async fn test_job_claiming_with_worker_type_filter() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3034).await?;
    let client = control_plane.client().await;

    // Register a WASM worker
    let mut wasm_worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    let wasm_id = wasm_worker.register().await?;

    // Submit a job that requires Firecracker
    // TODO: Implement job submission with worker type constraints
    // For now, submit a generic job
    let job_payload = serde_json::json!({"url": "https://example.com"});
    // client.publish_work(...)?;

    // WASM worker tries to claim work
    let claimed_job = wasm_worker.claim_work().await?;

    // If job was firecracker-only, WASM worker shouldn't claim it
    // If job accepts any worker, WASM worker should claim it
    // TODO: Add assertions once publish_work is implemented

    wasm_worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_job_claim_by_compatible_worker() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3035).await?;
    let client = control_plane.client().await;

    // Register WASM worker
    let mut wasm_worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    wasm_worker.register().await?;

    // Register Firecracker worker
    let mut fc_worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Firecracker,
    )
    .await?;
    fc_worker.register().await?;

    // TODO: Submit job with worker_type=WASM requirement
    // TODO: Verify only WASM worker can claim it
    // TODO: Verify Firecracker worker gets NO_CONTENT when claiming

    wasm_worker.shutdown().await;
    fc_worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

// ============================================================================
// Scenario 4: Orphaned Job Recovery
// ============================================================================

#[tokio::test]
async fn test_orphaned_job_recovery_when_worker_dies() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3036).await?;
    let client = control_plane.client().await;

    // Register worker and start heartbeat
    let mut worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    let worker_id = worker.register().await?;
    worker.start_heartbeat();

    // Simulate worker death (stop heartbeat and shutdown)
    worker.stop_heartbeat();
    worker.shutdown().await;

    // NOTE: Job submission and orphan recovery testing pending implementation

    control_plane.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_job_not_orphaned_if_heartbeat_continues() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3037).await?;
    let client = control_plane.client().await;

    let mut worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    worker.register().await?;
    worker.start_heartbeat();

    // NOTE: Job claiming and heartbeat persistence testing pending implementation

    worker.stop_heartbeat();
    worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

// ============================================================================
// Scenario 5: End-to-End Job Execution
// ============================================================================

#[tokio::test]
async fn test_end_to_end_job_execution() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3038).await?;
    let client = control_plane.client().await;

    // Start worker
    let mut worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    worker.register().await?;
    worker.start_heartbeat();

    // NOTE: End-to-end job execution testing pending implementation

    worker.stop_heartbeat();
    worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_job_failure_with_error_message() -> Result<()> {
    let control_plane = ControlPlaneFixture::new(3039).await?;
    let client = control_plane.client().await;

    let mut worker = TestWorkerFixture::new(
        control_plane.endpoint_ticket(),
        WorkerType::Wasm,
    )
    .await?;
    worker.register().await?;
    worker.start_heartbeat();

    // NOTE: Job failure and error handling testing pending implementation

    worker.stop_heartbeat();
    worker.shutdown().await;
    control_plane.shutdown().await?;

    Ok(())
}
