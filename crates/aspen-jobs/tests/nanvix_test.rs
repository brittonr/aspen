//! Tests for Nanvix micro-VM worker.

#![cfg(feature = "nanvix-executor")]

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_jobs::DependencyFailurePolicy;
use aspen_jobs::DependencyState;
use aspen_jobs::Job;
use aspen_jobs::JobId;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::NanvixWorker;
use aspen_jobs::VmJobPayload;
use aspen_jobs::Worker;
use chrono::Utc;

/// Create a test Job from a JobSpec with all required fields populated.
fn make_test_job(spec: JobSpec) -> Job {
    Job {
        id: JobId::new(),
        spec,
        status: JobStatus::Running,
        attempts: 0,
        last_error: None,
        result: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        scheduled_at: None,
        started_at: Some(Utc::now()),
        completed_at: None,
        worker_id: Some("test-worker".to_string()),
        next_retry_at: None,
        progress: None,
        progress_message: None,
        version: 1,
        dlq_metadata: None,
        dependency_state: DependencyState::Ready,
        blocked_by: vec![],
        blocking: vec![],
        dependency_failure_policy: DependencyFailurePolicy::FailCascade,
        execution_token: None,
    }
}

fn create_test_worker() -> (NanvixWorker, Arc<InMemoryBlobStore>) {
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = NanvixWorker::new(blob_store.clone()).unwrap();
    (worker, blob_store)
}

#[tokio::test]
async fn test_nanvix_worker_creation() {
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let result = NanvixWorker::new(blob_store);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_nanvix_worker_job_types() {
    let (worker, _blob_store) = create_test_worker();
    let job_types = worker.job_types();
    assert_eq!(job_types, vec!["nanvix_execute".to_string()]);
}

#[tokio::test]
async fn test_nanvix_workload_payload_serialization() {
    let original = VmJobPayload::nanvix_workload("deadbeefcafe1234", 4096, "javascript");

    let json = serde_json::to_value(&original).unwrap();
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::NanvixWorkload {
                hash: h1,
                size: s1,
                workload_type: w1,
            },
            VmJobPayload::NanvixWorkload {
                hash: h2,
                size: s2,
                workload_type: w2,
            },
        ) => {
            assert_eq!(h1, h2);
            assert_eq!(s1, s2);
            assert_eq!(w1, w2);
            assert_eq!(w1, "javascript");
        }
        _ => panic!("Expected NanvixWorkload payload"),
    }
}

#[tokio::test]
async fn test_nanvix_workload_payload_python() {
    let original = VmJobPayload::nanvix_workload("abc123def456", 8192, "python");

    let json = serde_json::to_value(&original).unwrap();
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::NanvixWorkload {
                hash: h1,
                size: s1,
                workload_type: w1,
            },
            VmJobPayload::NanvixWorkload {
                hash: h2,
                size: s2,
                workload_type: w2,
            },
        ) => {
            assert_eq!(h1, h2);
            assert_eq!(s1, s2);
            assert_eq!(w1, w2);
            assert_eq!(w1, "python");
        }
        _ => panic!("Expected NanvixWorkload payload"),
    }
}

#[tokio::test]
async fn test_nanvix_missing_blob() {
    let (worker, _blob_store) = create_test_worker();

    // Use a well-formed but nonexistent iroh-blobs hash (64 hex chars of zeros).
    let fake_hash = "0".repeat(64);
    let payload = VmJobPayload::nanvix_workload(&fake_hash, 1024, "javascript");
    let spec = JobSpec::new("nanvix_execute")
        .payload(payload)
        .unwrap()
        .with_isolation(true)
        .timeout(Duration::from_secs(5));
    let job = make_test_job(spec);

    let result = worker.execute(job).await;
    assert!(!result.is_success());

    if let aspen_jobs::JobResult::Failure(failure) = &result {
        assert!(
            failure.reason.contains("blob not found") || failure.reason.contains("not found"),
            "Expected failure about blob not found, got: {}",
            failure.reason
        );
    } else {
        panic!("Expected Failure result, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_nanvix_wrong_payload_type() {
    let (worker, _blob_store) = create_test_worker();

    // Submit a BlobBinary payload to the NanvixWorker.
    let payload = VmJobPayload::blob_binary("somehash", 1024, "elf");
    let spec = JobSpec::new("nanvix_execute")
        .payload(payload)
        .unwrap()
        .with_isolation(true)
        .timeout(Duration::from_secs(5));
    let job = make_test_job(spec);

    let result = worker.execute(job).await;
    assert!(!result.is_success());

    if let aspen_jobs::JobResult::Failure(failure) = &result {
        assert!(
            failure.reason.contains("NanvixWorker") || failure.reason.contains("NanvixWorkload"),
            "Expected failure about wrong payload type, got: {}",
            failure.reason
        );
    } else {
        panic!("Expected Failure result, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_nanvix_invalid_workload_type() {
    let (worker, blob_store) = create_test_worker();

    // Add some bytes to the blob store.
    let dummy_bytes: &[u8] = b"console.log('hello');";
    let add_result = blob_store.add_bytes(dummy_bytes).await.unwrap();
    let hash = add_result.blob_ref.hash.to_string();

    // Use an unknown workload_type.
    let payload = VmJobPayload::nanvix_workload(&hash, dummy_bytes.len() as u64, "ruby");
    let spec = JobSpec::new("nanvix_execute")
        .payload(payload)
        .unwrap()
        .with_isolation(true)
        .timeout(Duration::from_secs(5));
    let job = make_test_job(spec);

    let result = worker.execute(job).await;
    assert!(!result.is_success());

    if let aspen_jobs::JobResult::Failure(failure) = &result {
        assert!(
            failure.reason.contains("unknown workload_type"),
            "Expected failure about unknown workload_type, got: {}",
            failure.reason
        );
    } else {
        panic!("Expected Failure result, got: {:?}", result);
    }
}
