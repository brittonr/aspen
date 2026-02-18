//! Tests for Nanvix micro-VM worker.

#![cfg(feature = "plugins-nanvix")]

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
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteRequest;
use aspen_testing::DeterministicKeyValueStore;
use aspen_traits::KeyValueStore;
use base64::Engine;
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
    let kv_store = DeterministicKeyValueStore::new();
    let worker = NanvixWorker::new(blob_store.clone(), kv_store).unwrap();
    (worker, blob_store)
}

fn create_test_worker_with_kv() -> (NanvixWorker, Arc<InMemoryBlobStore>, Arc<DeterministicKeyValueStore>) {
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let kv_store = DeterministicKeyValueStore::new();
    let worker = NanvixWorker::new(blob_store.clone(), kv_store.clone()).unwrap();
    (worker, blob_store, kv_store)
}

#[tokio::test]
async fn test_nanvix_worker_creation() {
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let kv_store = DeterministicKeyValueStore::new();
    let result = NanvixWorker::new(blob_store, kv_store);
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

/// End-to-end test: pre-populate KV with an input file, run a JS workload
/// that reads the input and writes an output file, verify the output makes
/// it back to KV.
///
/// This exercises the full workspace materialization -> sandbox execution ->
/// sync-back flow. The guest accesses the host filesystem through Nanvix's
/// default syscall interception (forwarding to host libc).
#[tokio::test]
#[ignore = "Requires KVM + QuickJS binary in ~/.cache/nanvix-registry"]
async fn test_nanvix_workspace_e2e_js_file_io() {
    let (worker, blob_store, kv_store) = create_test_worker_with_kv();

    let job_id = "e2e-js-test";

    // Pre-populate KV with an input file the JS workload will read.
    let input_content = b"hello from KV store";
    let encoded = base64::engine::general_purpose::STANDARD.encode(input_content);
    kv_store
        .write(WriteRequest::set(&format!("nanvix/workspaces/{}/input.txt", job_id), &encoded))
        .await
        .unwrap();

    // JS workload that:
    // 1. Derives its workspace directory from its own path (scriptArgs)
    // 2. Reads input.txt from the workspace
    // 3. Writes output.txt with the reversed content
    let js_code = r#"
import * as std from 'std';
import * as os from 'os';

// scriptArgs[0] is the module flag '-m', scriptArgs[1] is the script path
var scriptPath = scriptArgs[scriptArgs.length - 1];
// Derive workspace directory (parent of the script file)
var lastSlash = scriptPath.lastIndexOf('/');
var workspaceDir = scriptPath.substring(0, lastSlash);

// Read input file from workspace
var inputPath = workspaceDir + '/input.txt';
var inputFile = std.open(inputPath, 'r');
if (!inputFile) {
    console.log('ERROR: could not open ' + inputPath);
    std.exit(1);
}
var content = inputFile.readAsString();
inputFile.close();
console.log('Read input: ' + content);

// Write reversed content to output file
var reversed = content.split('').reverse().join('');
var outputPath = workspaceDir + '/output.txt';
var outputFile = std.open(outputPath, 'w');
outputFile.puts(reversed);
outputFile.close();
console.log('Wrote output: ' + reversed);
"#;

    // Add the JS workload to the blob store.
    let add_result = blob_store.add_bytes(js_code.as_bytes()).await.unwrap();
    let hash = add_result.blob_ref.hash.to_string();
    let size = js_code.len() as u64;

    // Build and execute the job.
    let payload = VmJobPayload::nanvix_workload(&hash, size, "javascript");
    let spec = JobSpec::new("nanvix_execute")
        .payload(payload)
        .unwrap()
        .with_isolation(true)
        .timeout(Duration::from_secs(30));
    let mut job = make_test_job(spec);
    job.id = JobId::from_string(job_id.to_string());

    let result = worker.execute(job).await;

    // Verify execution succeeded.
    assert!(result.is_success(), "Expected success, got: {:?}", result);

    // Verify the output file was synced back to KV.
    let output_key = format!("nanvix/workspaces/{}/output.txt", job_id);
    let read_result = kv_store.read(ReadRequest::new(&output_key)).await.unwrap();
    let kv_entry = read_result.kv.expect("output.txt should exist in KV after sync");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&kv_entry.value)
        .expect("output.txt value should be valid base64");
    let output_str = String::from_utf8(decoded).expect("output should be valid UTF-8");

    // The workload reverses "hello from KV store"
    let expected: String = "hello from KV store".chars().rev().collect();
    assert_eq!(output_str, expected, "Guest should have written reversed input to output.txt");

    // Verify console output mentions what happened.
    if let aspen_jobs::JobResult::Success(success) = &result {
        let data = &success.data;
        let console = data["console_output"].as_str().unwrap_or("");
        assert!(console.contains("Read input"), "Console should show the script read the input, got: {}", console);
        let files_mat = data["files_materialized"].as_u64().unwrap_or(0);
        assert_eq!(files_mat, 1, "Should have materialized 1 file (input.txt)");
        let files_written = data["files_written"].as_u64().unwrap_or(0);
        assert!(files_written >= 1, "Should have synced at least 1 file (output.txt), got: {}", files_written);
    }
}
