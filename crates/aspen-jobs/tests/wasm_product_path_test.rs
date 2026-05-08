//! Product-path guardrails for WASM runtime-host jobs.
//!
//! These tests intentionally route WASM payloads through the job manager and
//! worker-pool orchestration path. Direct `WasmComponentWorker::execute` tests
//! remain useful unit coverage, but they are not runtime-host row evidence by
//! themselves.

#![cfg(feature = "plugins-wasm")]

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::RetryPolicy;
use aspen_jobs::WasmComponentWorker;
use aspen_jobs::WorkerPool;
use aspen_testing::DeterministicKeyValueStore;
use tokio::time::sleep;
use tokio::time::timeout;

const PRODUCT_PATH_TIMEOUT: Duration = Duration::from_secs(10);
const PRODUCT_PATH_POLL: Duration = Duration::from_millis(100);
const INVALID_WASM_BYTES: &[u8] = b"not-wasm-runtime-host-fixture";
const PRODUCT_PATH_MARKER: &str = "ASPEN_WASM_RUNTIME_HOST_EXECUTED";
const PRODUCT_PATH_GUARD_MARKER: &str = "ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD";
const FIXTURE_EXIT_CODE: i32 = 42;

fn runtime_host_fixture_wasm() -> Vec<u8> {
    vec![
        0x00,
        0x61,
        0x73,
        0x6d,
        0x01,
        0x00,
        0x00,
        0x00, // magic/version
        0x01,
        0x05,
        0x01,
        0x60,
        0x00,
        0x01,
        0x7f, // type: () -> i32
        0x03,
        0x02,
        0x01,
        0x00, // function section
        0x07,
        0x0b,
        0x01,
        0x07,
        b'e',
        b'x',
        b'e',
        b'c',
        b'u',
        b't',
        b'e',
        0x00,
        0x00, // export
        0x0a,
        0x06,
        0x01,
        0x04,
        0x00,
        0x41,
        FIXTURE_EXIT_CODE as u8,
        0x0b, // code
    ]
}

async fn run_wasm_job(component_bytes: &[u8]) -> (JobStatus, Option<aspen_jobs::JobResult>, Option<String>, u32) {
    let kv_store = DeterministicKeyValueStore::new();
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = WasmComponentWorker::new(kv_store.clone(), blob_store.clone()).unwrap();
    let manager = Arc::new(JobManager::new(kv_store));
    let pool = WorkerPool::with_manager(manager.clone());

    pool.register_handler("wasm_component", worker).await.unwrap();
    pool.start(1).await.unwrap();

    let blob = blob_store.add_bytes(component_bytes).await.unwrap();
    let payload = aspen_jobs::VmJobPayload::wasm_component_with_limits(
        blob.blob_ref.hash.to_string(),
        component_bytes.len() as u64,
        Some(1_000_000),
        Some(64 * 1024 * 1024),
    );
    let spec = JobSpec::new("wasm_component")
        .payload(payload)
        .unwrap()
        .retry_policy(RetryPolicy::none())
        .timeout(Duration::from_secs(5));

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
    .expect("wasm product-path job should reach a terminal state");

    pool.shutdown().await.unwrap();

    (job.status, job.result, job.last_error, job.attempts)
}

#[tokio::test]
async fn wasm_job_executes_declared_fixture_through_product_orchestration() {
    let (status, result, error, attempts) = run_wasm_job(&runtime_host_fixture_wasm()).await;

    assert_eq!(status, JobStatus::Completed, "unexpected error: {error:?}");
    let output = match result.expect("completed wasm job should have output") {
        aspen_jobs::JobResult::Success(output) => output.data,
        other => panic!("unexpected wasm job result: {other:?}"),
    };
    assert_eq!(output["abi"], "aspen:runtime-host/wasm-v1");
    assert_eq!(output["entrypoint"], "execute");
    assert_eq!(output["exit_code"], FIXTURE_EXIT_CODE);
    assert_eq!(output["marker"], PRODUCT_PATH_MARKER);
    assert!(attempts > 0, "{PRODUCT_PATH_MARKER}: job did not pass through worker-pool orchestration");
}

#[tokio::test]
async fn invalid_wasm_reaches_wasm_worker_through_product_orchestration() {
    let (status, _result, error, attempts) = run_wasm_job(INVALID_WASM_BYTES).await;

    assert!(matches!(status, JobStatus::Failed | JobStatus::DeadLetter));
    let failure = error.as_deref().unwrap_or_default();
    assert!(
        failure.contains("WASM magic"),
        "{PRODUCT_PATH_GUARD_MARKER}: expected wasm worker validation failure, got {failure:?}"
    );
    assert!(
        attempts > 0,
        "{PRODUCT_PATH_GUARD_MARKER}: invalid wasm job did not pass through worker-pool orchestration"
    );
}

#[test]
fn product_path_marker_distinguishes_guardrail_from_execution_evidence() {
    assert_eq!(PRODUCT_PATH_MARKER, "ASPEN_WASM_RUNTIME_HOST_EXECUTED");
    assert_eq!(PRODUCT_PATH_GUARD_MARKER, "ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD");
}
