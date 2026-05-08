//! Product-path guardrails for Hyperlight runtime-host jobs.
//!
//! These tests intentionally route Hyperlight payloads through the job manager
//! and worker-pool orchestration path. Direct `HyperlightWorker::execute` tests
//! remain useful unit coverage, but they are not runtime-host row evidence by
//! themselves.

#![cfg(feature = "plugins-vm")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_jobs::HyperlightWorker;
use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::RetryPolicy;
use aspen_jobs::WorkerPool;
use aspen_testing::DeterministicKeyValueStore;
use tokio::time::sleep;
use tokio::time::timeout;

const PRODUCT_PATH_TIMEOUT: Duration = Duration::from_secs(30);
const PRODUCT_PATH_POLL: Duration = Duration::from_millis(100);
const JOB_TIMEOUT: Duration = Duration::from_secs(10);
const PRODUCT_PATH_MARKER: &str = "ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED";
const PRODUCT_PATH_GUARD_MARKER: &str = "ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD";
const HYPERLIGHT_ABI: &str = "aspen:runtime-host/hyperlight-v1";
const INVALID_HYPERLIGHT_BYTES: &[u8] = b"not-a-hyperlight-guest";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn default_guest_binary_path() -> PathBuf {
    repo_root().join("examples/vm-jobs/echo-worker/target/x86_64-hyperlight-none/release/echo-worker")
}

fn configured_guest_binary_path() -> PathBuf {
    std::env::var_os("ASPEN_HYPERLIGHT_GUEST_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(default_guest_binary_path)
}

fn build_guest_fixture_if_missing() -> PathBuf {
    let guest_binary = configured_guest_binary_path();
    if guest_binary.exists() {
        return guest_binary;
    }

    let guest_dir = repo_root().join("examples/vm-jobs/echo-worker");
    let status = Command::new("cargo")
        .current_dir(&guest_dir)
        .env(
            "RUSTFLAGS",
            "-Zunstable-options --cfg=hyperlight --check-cfg=cfg(hyperlight) -Clink-args=-eentrypoint",
        )
        .args(["hyperlight", "build", "--release"])
        .status()
        .expect("failed to spawn cargo hyperlight build for Hyperlight guest fixture");
    assert!(status.success(), "cargo hyperlight build for Hyperlight guest fixture failed: {status}");
    assert!(guest_binary.exists(), "Hyperlight guest fixture was not built at {}", guest_binary.display());
    guest_binary
}

async fn run_hyperlight_job(
    binary_bytes: &[u8],
    input: Option<serde_json::Value>,
) -> (JobStatus, Option<aspen_jobs::JobResult>, Option<String>, u32) {
    let kv_store = DeterministicKeyValueStore::new();
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = HyperlightWorker::new(blob_store.clone()).unwrap();
    let manager = Arc::new(JobManager::new(kv_store));
    let pool = WorkerPool::with_manager(manager.clone());

    pool.register_handler("vm_execute", worker).await.unwrap();
    pool.start(1).await.unwrap();

    let blob = blob_store.add_bytes(binary_bytes).await.unwrap();
    let mut spec = JobSpec::with_blob_binary(
        &blob.blob_ref.hash.to_string(),
        binary_bytes.len() as u64,
        "x86_64-hyperlight-none-elf",
    )
    .retry_policy(RetryPolicy::none())
    .timeout(JOB_TIMEOUT);
    if let Some(input) = input {
        spec.payload["input"] = input;
    }

    let job_id = manager.submit(spec).await.unwrap();
    let job = timeout(PRODUCT_PATH_TIMEOUT, async {
        loop {
            let job = manager.get_job(&job_id).await.unwrap().expect("submitted job exists");
            if matches!(job.status, JobStatus::Failed | JobStatus::Completed | JobStatus::DeadLetter) {
                break job;
            }
            sleep(PRODUCT_PATH_POLL).await;
        }
    })
    .await
    .expect("Hyperlight product-path job should reach a terminal state");

    pool.shutdown().await.unwrap();

    (job.status, job.result, job.last_error, job.attempts)
}

#[tokio::test]
#[ignore = "Requires cargo-hyperlight and a Hyperlight/KVM-capable host"]
async fn hyperlight_job_executes_declared_fixture_through_product_orchestration() {
    let guest_binary = build_guest_fixture_if_missing();
    let binary_bytes = std::fs::read(&guest_binary).expect("Hyperlight guest fixture should be readable");
    let (status, result, error, attempts) =
        run_hyperlight_job(&binary_bytes, Some(serde_json::Value::String(PRODUCT_PATH_MARKER.to_string()))).await;

    assert_eq!(status, JobStatus::Completed, "unexpected error: {error:?}");
    let output = match result.expect("completed Hyperlight job should have output") {
        aspen_jobs::JobResult::Success(output) => output.data,
        other => panic!("unexpected Hyperlight job result: {other:?}"),
    };
    assert_eq!(output["abi"], HYPERLIGHT_ABI);
    assert_eq!(output["entrypoint"], "execute");
    assert_eq!(output["marker"], PRODUCT_PATH_MARKER);
    assert!(
        output["raw_output"].as_str().unwrap_or_default().contains(PRODUCT_PATH_MARKER),
        "{PRODUCT_PATH_MARKER}: guest output did not echo the proof marker: {output:?}"
    );
    assert!(attempts > 0, "{PRODUCT_PATH_MARKER}: job did not pass through worker-pool orchestration");
}

#[tokio::test]
async fn invalid_hyperlight_reaches_worker_through_product_orchestration() {
    let (status, _result, error, attempts) = run_hyperlight_job(INVALID_HYPERLIGHT_BYTES, None).await;

    assert!(matches!(status, JobStatus::Failed | JobStatus::DeadLetter));
    let failure = error.as_deref().unwrap_or_default();
    assert!(
        failure.contains("Failed to create sandbox") || failure.contains("VM execution failed"),
        "{PRODUCT_PATH_GUARD_MARKER}: expected Hyperlight worker validation failure, got {failure:?}"
    );
    assert!(
        attempts > 0,
        "{PRODUCT_PATH_GUARD_MARKER}: invalid Hyperlight job did not pass through worker-pool orchestration"
    );
}

#[test]
fn product_path_marker_distinguishes_guardrail_from_execution_evidence() {
    assert_eq!(PRODUCT_PATH_MARKER, "ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED");
    assert_eq!(PRODUCT_PATH_GUARD_MARKER, "ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD");
    assert_ne!(PRODUCT_PATH_MARKER, PRODUCT_PATH_GUARD_MARKER);
}
