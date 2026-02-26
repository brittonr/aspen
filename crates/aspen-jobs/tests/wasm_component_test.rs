//! Tests for WASM Component Model worker.

#![cfg(feature = "plugins-wasm")]

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
use aspen_jobs::VmJobPayload;
use aspen_jobs::WasmComponentWorker;
use aspen_jobs::Worker;
use aspen_testing::DeterministicKeyValueStore;
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

fn create_test_worker() -> (WasmComponentWorker, Arc<InMemoryBlobStore>) {
    let kv_store = DeterministicKeyValueStore::new();
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = WasmComponentWorker::new(kv_store, blob_store.clone()).unwrap();
    (worker, blob_store)
}

#[tokio::test]
async fn test_wasm_component_worker_creation() {
    let kv_store = DeterministicKeyValueStore::new();
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let result = WasmComponentWorker::new(kv_store, blob_store);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_wasm_component_worker_job_types() {
    let (worker, _blob_store) = create_test_worker();
    let job_types = worker.job_types();
    assert_eq!(job_types, vec!["wasm_component".to_string()]);
}

#[tokio::test]
async fn test_wasm_component_payload_serialization() {
    let original = VmJobPayload::wasm_component("deadbeefcafe1234", 8192);

    let json = serde_json::to_value(&original).unwrap();
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::WasmComponent {
                hash: h1,
                size: s1,
                fuel_limit: f1,
                memory_limit: m1,
            },
            VmJobPayload::WasmComponent {
                hash: h2,
                size: s2,
                fuel_limit: f2,
                memory_limit: m2,
            },
        ) => {
            assert_eq!(h1, h2);
            assert_eq!(s1, s2);
            assert_eq!(f1, f2);
            assert_eq!(m1, m2);
            // Default constructor should have None limits.
            assert_eq!(f1, None);
            assert_eq!(m1, None);
        }
        _ => panic!("Expected WasmComponent payload"),
    }
}

#[tokio::test]
async fn test_wasm_component_payload_with_limits() {
    let fuel = Some(500_000_000u64);
    let memory = Some(512 * 1024 * 1024u64);
    let original = VmJobPayload::wasm_component_with_limits("abc123", 4096, fuel, memory);

    let json = serde_json::to_value(&original).unwrap();
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::WasmComponent {
                hash: h1,
                size: s1,
                fuel_limit: f1,
                memory_limit: m1,
            },
            VmJobPayload::WasmComponent {
                hash: h2,
                size: s2,
                fuel_limit: f2,
                memory_limit: m2,
            },
        ) => {
            assert_eq!(h1, h2);
            assert_eq!(s1, s2);
            assert_eq!(f1, f2);
            assert_eq!(m1, m2);
            assert_eq!(f1, fuel);
            assert_eq!(m1, memory);
        }
        _ => panic!("Expected WasmComponent payload"),
    }
}

#[tokio::test]
async fn test_wasm_component_invalid_magic_bytes() {
    let (worker, blob_store) = create_test_worker();

    // Add invalid bytes (not valid WASM magic) to the blob store.
    let invalid_bytes: &[u8] = &[0x00, 0x00, 0x00, 0x00];
    let add_result = blob_store.add_bytes(invalid_bytes).await.unwrap();
    let hash = add_result.blob_ref.hash.to_string();

    // Build a WasmComponent payload referencing the invalid blob.
    let payload = VmJobPayload::wasm_component(&hash, invalid_bytes.len() as u64);
    let spec = JobSpec::new("wasm_component")
        .payload(payload)
        .unwrap()
        .with_isolation(true)
        .timeout(Duration::from_secs(5));
    let job = make_test_job(spec);

    let result = worker.execute(job).await;
    assert!(!result.is_success());

    // The failure reason should mention WASM magic bytes.
    if let aspen_jobs::JobResult::Failure(failure) = &result {
        assert!(failure.reason.contains("WASM magic"), "Expected failure about WASM magic, got: {}", failure.reason);
    } else {
        panic!("Expected Failure result, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_wasm_component_missing_blob() {
    let (worker, _blob_store) = create_test_worker();

    // Use a well-formed but nonexistent iroh-blobs hash (64 hex chars of zeros).
    let fake_hash = "0".repeat(64);
    let payload = VmJobPayload::wasm_component(&fake_hash, 1024);
    let spec = JobSpec::new("wasm_component")
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
async fn test_wasm_component_wrong_payload_type() {
    let (worker, _blob_store) = create_test_worker();

    // Submit a BlobBinary payload to the WasmComponentWorker.
    let payload = VmJobPayload::blob_binary("somehash", 1024, "elf");
    let spec = JobSpec::new("wasm_component")
        .payload(payload)
        .unwrap()
        .with_isolation(true)
        .timeout(Duration::from_secs(5));
    let job = make_test_job(spec);

    let result = worker.execute(job).await;
    assert!(!result.is_success());

    if let aspen_jobs::JobResult::Failure(failure) = &result {
        assert!(
            failure.reason.contains("WasmComponent"),
            "Expected failure about wrong payload type, got: {}",
            failure.reason
        );
    } else {
        panic!("Expected Failure result, got: {:?}", result);
    }
}
