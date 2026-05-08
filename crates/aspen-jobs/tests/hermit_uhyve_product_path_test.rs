//! Product-path guardrails for Hermit/Uhyve runtime-host jobs.
//!
//! These tests route Hermit/Uhyve payloads through `JobManager` and
//! `WorkerPool`. The non-ignored tests use a fake Uhyve command only to prove
//! Aspen orchestration, payload validation, receipt shaping, and guardrails; a
//! real Uhyve proof remains gated until a Hermit image and capable host are
//! available.

#![cfg(feature = "plugins-vm")]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_jobs::HERMIT_UHYVE_JOB_TYPE;
use aspen_jobs::HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER;
use aspen_jobs::HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD_MARKER;
use aspen_jobs::HermitUhyveJobPayload;
use aspen_jobs::HermitUhyveWorker;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::RetryPolicy;
use aspen_jobs::WorkerPool;
use aspen_testing::DeterministicKeyValueStore;
use tokio::time::sleep;
use tokio::time::timeout;

const PRODUCT_PATH_TIMEOUT: Duration = Duration::from_secs(10);
const PRODUCT_PATH_POLL: Duration = Duration::from_millis(100);
const JOB_TIMEOUT: Duration = Duration::from_secs(5);
const HERMIT_ARTIFACT_HASH: &str = "sha256:hermit-uhyve-runtime-host-fixture";
const FAKE_HERMIT_IMAGE: &[u8] = b"fake-hermit-image-for-product-path-guard";

fn write_fake_uhyve(dir: &Path, exit_success: bool) -> std::path::PathBuf {
    let path = dir.join("fake-uhyve");
    let exit = if exit_success { 0 } else { 7 };
    std::fs::write(
        &path,
        format!(
            "#!/usr/bin/env sh\nset -eu\ntest -f \"$1\"\nprintf '%s image=%s\\n' '{marker}' \"$1\"\nprintf 'SECRET_TOKEN=should-redact\\n'\nexit {exit}\n",
            marker = HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER,
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

async fn run_hermit_job(
    image_bytes: &[u8],
    payload_mutator: impl FnOnce(&mut HermitUhyveJobPayload),
    uhyve_command: std::path::PathBuf,
) -> (JobStatus, Option<JobResult>, Option<String>, u32) {
    let kv_store = DeterministicKeyValueStore::new();
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = HermitUhyveWorker::with_uhyve_command(blob_store.clone(), uhyve_command);
    let manager = Arc::new(JobManager::new(kv_store));
    let pool = WorkerPool::with_manager(manager.clone());

    pool.register_handler(HERMIT_UHYVE_JOB_TYPE, worker).await.unwrap();
    pool.start(1).await.unwrap();

    let blob = blob_store.add_bytes(image_bytes).await.unwrap();
    let mut payload = HermitUhyveJobPayload::blob_image(
        blob.blob_ref.hash.to_string(),
        image_bytes.len() as u64,
        HERMIT_ARTIFACT_HASH,
    );
    payload.timeout_secs = Some(2);
    payload.serial_log_limit_bytes = Some(1024);
    payload_mutator(&mut payload);

    let spec = JobSpec::new(HERMIT_UHYVE_JOB_TYPE)
        .payload(payload)
        .unwrap()
        .retry_policy(RetryPolicy::none())
        .timeout(JOB_TIMEOUT);
    let job_id = manager.submit(spec).await.unwrap();
    let job = timeout(PRODUCT_PATH_TIMEOUT, async {
        loop {
            let job = manager.get_job(&job_id).await.unwrap().expect("submitted job exists");
            if matches!(job.status, JobStatus::Completed | JobStatus::Failed | JobStatus::DeadLetter) {
                break job;
            }
            sleep(PRODUCT_PATH_POLL).await;
        }
    })
    .await
    .expect("Hermit/Uhyve product-path job should reach a terminal state");

    pool.shutdown().await.unwrap();
    (job.status, job.result, job.last_error, job.attempts)
}

#[tokio::test]
async fn hermit_uhyve_worker_wraps_fake_runner_receipt_through_product_orchestration() {
    let dir = tempfile::tempdir().unwrap();
    let fake_uhyve = write_fake_uhyve(dir.path(), true);
    let (status, result, error, attempts) = run_hermit_job(FAKE_HERMIT_IMAGE, |_| {}, fake_uhyve).await;

    assert_eq!(status, JobStatus::Completed, "unexpected error: {error:?}");
    assert!(attempts > 0, "job did not pass through worker-pool orchestration");
    let output = match result.expect("completed Hermit/Uhyve job should have output") {
        JobResult::Success(output) => output.data,
        other => panic!("unexpected Hermit/Uhyve job result: {other:?}"),
    };
    assert_eq!(output["abi"], "aspen:runtime-host/hermit-uhyve-v1");
    assert_eq!(output["engine"], "uhyve");
    assert_eq!(output["artifact_hash"], HERMIT_ARTIFACT_HASH);
    assert_eq!(output["lifecycle_state"], "completed");
    assert_eq!(output["exit_status"], 0);
    assert_eq!(output["marker"], HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER);
    assert!(
        output["serial_stdout"]
            .as_str()
            .unwrap_or_default()
            .contains(HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER),
        "fake runner did not preserve marker in bounded serial output: {output:?}"
    );
    assert!(
        !output.to_string().contains("should-redact"),
        "receipt leaked secret-like serial output: {output:?}"
    );
}

#[tokio::test]
async fn hermit_uhyve_invalid_artifact_identity_reaches_product_worker_before_failure() {
    let dir = tempfile::tempdir().unwrap();
    let fake_uhyve = write_fake_uhyve(dir.path(), true);
    let (status, _result, error, attempts) = run_hermit_job(
        FAKE_HERMIT_IMAGE,
        |payload| payload.artifact_hash = "mutable-hermit-image".to_string(),
        fake_uhyve,
    )
    .await;

    assert!(matches!(status, JobStatus::Failed | JobStatus::DeadLetter));
    assert!(
        attempts > 0,
        "{HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD_MARKER}: invalid job did not reach orchestration"
    );
    let failure = error.as_deref().unwrap_or_default();
    assert!(
        failure.contains("immutable sha256"),
        "{HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD_MARKER}: expected immutable identity failure, got {failure:?}"
    );
}

#[tokio::test]
async fn hermit_uhyve_nonzero_exit_is_product_path_failure_not_execution_proof() {
    let dir = tempfile::tempdir().unwrap();
    let fake_uhyve = write_fake_uhyve(dir.path(), false);
    let (status, _result, error, attempts) = run_hermit_job(FAKE_HERMIT_IMAGE, |_| {}, fake_uhyve).await;

    assert!(matches!(status, JobStatus::Failed | JobStatus::DeadLetter));
    assert!(
        attempts > 0,
        "{HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD_MARKER}: failed job did not reach orchestration"
    );
    let failure = error.as_deref().unwrap_or_default();
    assert!(
        failure.contains("Uhyve exited unsuccessfully"),
        "{HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD_MARKER}: expected Uhyve exit failure, got {failure:?}"
    );
}

#[tokio::test]
#[ignore = "Requires real uhyve, a virtualization-capable host, and ASPEN_HERMIT_UHYVE_IMAGE"]
async fn hermit_uhyve_executes_declared_fixture_through_product_orchestration() {
    let image = std::env::var("ASPEN_HERMIT_UHYVE_IMAGE")
        .expect("ASPEN_HERMIT_UHYVE_IMAGE must point at a Hermit image for gated proof");
    let uhyve = std::env::var_os("ASPEN_UHYVE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("uhyve"));
    let image_bytes = std::fs::read(&image).expect("Hermit fixture image should be readable");
    let (status, result, error, attempts) = run_hermit_job(&image_bytes, |_| {}, uhyve).await;

    assert_eq!(status, JobStatus::Completed, "unexpected error: {error:?}");
    assert!(attempts > 0, "job did not pass through worker-pool orchestration");
    let output = match result.expect("completed Hermit/Uhyve job should have output") {
        JobResult::Success(output) => output.data,
        other => panic!("unexpected Hermit/Uhyve job result: {other:?}"),
    };
    assert_eq!(output["marker"], HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER);
    assert!(
        output["serial_stdout"]
            .as_str()
            .unwrap_or_default()
            .contains(HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER)
            || output["serial_stderr"]
                .as_str()
                .unwrap_or_default()
                .contains(HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER),
        "real Hermit/Uhyve proof did not emit marker: {output:?}"
    );
}
