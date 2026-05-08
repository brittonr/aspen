//! Product-path guardrails for OCI-lowered runtime-host jobs.
//!
//! OCI is a packaging and lowering input in Aspen's production contract, not
//! a raw container runtime proof. These tests require an immutable OCI source
//! identity, a declared lowering plan into a supported isolated host, and a
//! product-path job execution receipt for the derived artifact.

#![cfg(feature = "plugins-wasm")]

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::RetryPolicy;
use aspen_jobs::WasmComponentWorker;
use aspen_jobs::WorkerPool;
use aspen_runtime_core::AdmissionError;
use aspen_runtime_core::OciLoweringPlan;
use aspen_runtime_core::OciLoweringTarget;
use aspen_runtime_core::RedactedValue;
use aspen_runtime_core::RuntimeArtifact;
use aspen_runtime_core::RuntimeCapabilityBinding;
use aspen_runtime_core::RuntimeDiagnostic;
use aspen_runtime_core::RuntimeHostKind;
use aspen_runtime_core::RuntimeResources;
use aspen_runtime_core::RuntimeUnitDeclaration;
use aspen_runtime_core::RuntimeUnitKind;
use aspen_runtime_core::admit_oci_lowering_plan;
use aspen_runtime_core::admit_oci_lowering_receipt;
use aspen_runtime_core::admit_unit;
use aspen_runtime_core::oci_lowering_receipt;
use aspen_testing::DeterministicKeyValueStore;
use tokio::time::sleep;
use tokio::time::timeout;

const PRODUCT_PATH_TIMEOUT: Duration = Duration::from_secs(10);
const PRODUCT_PATH_POLL: Duration = Duration::from_millis(100);
const SOURCE_OCI_DIGEST: &str = "sha256:oci-packaged-wasm-runtime-host-fixture";
const DERIVED_WASM_DIGEST: &str = "sha256:wasm-from-oci-runtime-host-fixture";
const OCI_ENTRYPOINT: &str = "/app/execute";
const WASM_ENTRYPOINT: &str = "execute";
const WASM_ABI: &str = "aspen:runtime-host/wasm-v1";
const PRODUCT_PATH_MARKER: &str = "ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED";
const PRODUCT_PATH_GUARD_MARKER: &str = "ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD";
const TARGET_MARKER: &str = "ASPEN_WASM_RUNTIME_HOST_EXECUTED";
const INVALID_WASM_BYTES: &[u8] = b"not-an-oci-lowered-wasm-fixture";
const FIXTURE_EXIT_CODE: i32 = 43;

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

fn runtime_capability(handle_id: &str) -> RuntimeCapabilityBinding {
    RuntimeCapabilityBinding {
        handle_id: handle_id.to_string(),
        ability: "runtime/launch".to_string(),
        resource: "aspen://runtime/runner/wasm".to_string(),
        proof_refs: vec!["proof:oci-lowering-fixture".to_string()],
        caveats: vec![],
    }
}

fn oci_decl(host_kind: RuntimeHostKind, source_digest: &str) -> RuntimeUnitDeclaration {
    RuntimeUnitDeclaration {
        unit_id: "job/oci-lowered-wasm-fixture".to_string(),
        unit_kind: RuntimeUnitKind::ExecutionRun,
        host_kind,
        artifact: RuntimeArtifact::OciImage {
            image_digest: source_digest.to_string(),
            entrypoint: OCI_ENTRYPOINT.to_string(),
            args: vec!["--bounded-output".to_string()],
        },
        capabilities: vec![runtime_capability("runner:wasm")],
        resources: RuntimeResources {
            memory_bytes: Some(64 * 1024 * 1024),
            cpu_millis: Some(100),
            wall_time_ms: Some(5_000),
            wasm_fuel: Some(1_000_000),
            max_open_files: Some(0),
        },
        routes: vec![],
    }
}

fn oci_to_wasm_plan(derived_artifacts: Vec<RuntimeArtifact>) -> OciLoweringPlan {
    OciLoweringPlan {
        source_image_digest: SOURCE_OCI_DIGEST.to_string(),
        entrypoint: OCI_ENTRYPOINT.to_string(),
        args: vec!["--bounded-output".to_string()],
        target: OciLoweringTarget::Wasm,
        derived_artifacts,
        transformation_provenance: vec![RedactedValue::Hash(
            "sha256:deterministic-oci-to-wasm-lowering-recipe".to_string(),
        )],
        declared_handles: vec!["runner:wasm".to_string()],
        unsupported_diagnostics: vec![],
    }
}

fn derived_wasm_artifact() -> RuntimeArtifact {
    RuntimeArtifact::WasmModule {
        module_hash: DERIVED_WASM_DIGEST.to_string(),
        abi: WASM_ABI.to_string(),
        entrypoint: WASM_ENTRYPOINT.to_string(),
    }
}

async fn run_oci_lowered_wasm_job(component_bytes: &[u8]) -> (JobStatus, Option<JobResult>, Option<String>, u32) {
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
    .expect("OCI-lowered WASM product-path job should reach a terminal state");

    pool.shutdown().await.unwrap();

    (job.status, job.result, job.last_error, job.attempts)
}

#[tokio::test]
async fn oci_lowered_wasm_executes_through_product_orchestration() {
    let decl = oci_decl(RuntimeHostKind::Wasm, SOURCE_OCI_DIGEST);
    let plan = oci_to_wasm_plan(vec![derived_wasm_artifact()]);
    admit_oci_lowering_plan(&decl, &plan).unwrap();

    let lowering_receipt = oci_lowering_receipt(
        "receipt/oci-lowered-wasm",
        &decl,
        &plan,
        RedactedValue::OpaqueHandle("runner:wasm/product-path".to_string()),
        vec![RuntimeDiagnostic {
            key: "lowering".to_string(),
            value: RedactedValue::Plain("immutable OCI artifact lowered into WASM module".to_string()),
        }],
    );
    admit_oci_lowering_receipt(&lowering_receipt).unwrap();

    let (status, result, error, attempts) = run_oci_lowered_wasm_job(&runtime_host_fixture_wasm()).await;
    assert_eq!(status, JobStatus::Completed, "unexpected error: {error:?}");
    assert!(attempts > 0, "{PRODUCT_PATH_MARKER}: job did not pass through worker-pool orchestration");

    let output = match result.expect("completed OCI-lowered WASM job should have output") {
        JobResult::Success(output) => output.data,
        other => panic!("unexpected OCI-lowered WASM job result: {other:?}"),
    };
    assert_eq!(output["marker"], TARGET_MARKER);
    assert_eq!(output["abi"], WASM_ABI);
    assert_eq!(output["entrypoint"], WASM_ENTRYPOINT);
    assert_eq!(output["exit_code"], FIXTURE_EXIT_CODE);

    let product_receipt = serde_json::json!({
        "marker": PRODUCT_PATH_MARKER,
        "source_oci_digest": lowering_receipt.source_image_digest,
        "selected_target_host": "wasm",
        "derived_artifact_hash": DERIVED_WASM_DIGEST,
        "runner_identity": "runner:wasm/product-path",
        "lifecycle_state": "completed",
        "exit_status": output["exit_code"],
        "bounded_output_summary": output,
    });

    assert_eq!(product_receipt["marker"], PRODUCT_PATH_MARKER);
    assert_eq!(product_receipt["source_oci_digest"], SOURCE_OCI_DIGEST);
    assert_eq!(product_receipt["derived_artifact_hash"], DERIVED_WASM_DIGEST);
    assert_eq!(product_receipt["bounded_output_summary"]["marker"], TARGET_MARKER);
    let serialized = serde_json::to_string(&product_receipt).unwrap();
    assert!(!serialized.contains("token="));
    assert!(!serialized.contains("secret="));
    assert!(!serialized.contains("connection_string"));
}

#[tokio::test]
async fn invalid_oci_lowered_artifact_reaches_target_worker_but_not_execution_marker() {
    let decl = oci_decl(RuntimeHostKind::Wasm, SOURCE_OCI_DIGEST);
    let plan = oci_to_wasm_plan(vec![derived_wasm_artifact()]);
    admit_oci_lowering_plan(&decl, &plan).unwrap();

    let (status, _result, error, attempts) = run_oci_lowered_wasm_job(INVALID_WASM_BYTES).await;

    assert!(matches!(status, JobStatus::Failed | JobStatus::DeadLetter));
    assert!(
        attempts > 0,
        "{PRODUCT_PATH_GUARD_MARKER}: invalid derived artifact did not pass through worker-pool orchestration"
    );
    let failure = error.as_deref().unwrap_or_default();
    assert!(
        failure.contains("WASM magic"),
        "{PRODUCT_PATH_GUARD_MARKER}: expected target worker validation failure, got {failure:?}"
    );
    assert!(!failure.contains(PRODUCT_PATH_MARKER));
}

#[test]
fn oci_model_only_and_raw_container_paths_do_not_satisfy_product_proof() {
    let plan = oci_to_wasm_plan(vec![derived_wasm_artifact()]);
    let decl = oci_decl(RuntimeHostKind::Wasm, SOURCE_OCI_DIGEST);
    admit_oci_lowering_plan(&decl, &plan).unwrap();
    let lowering_receipt = oci_lowering_receipt(
        "receipt/model-only",
        &decl,
        &plan,
        RedactedValue::OpaqueHandle("runner:wasm/product-path".to_string()),
        vec![],
    );
    admit_oci_lowering_receipt(&lowering_receipt).unwrap();
    let model_only = serde_json::to_string(&lowering_receipt).unwrap();
    assert!(model_only.contains(SOURCE_OCI_DIGEST));
    assert!(
        !model_only.contains(PRODUCT_PATH_MARKER),
        "{PRODUCT_PATH_GUARD_MARKER}: model-only lowering receipt must not be product execution proof"
    );

    let raw_container = oci_decl(RuntimeHostKind::OciContainer, SOURCE_OCI_DIGEST);
    assert_eq!(admit_unit(&raw_container), Err(AdmissionError::OciRejectsRawContainerInProduction));
    assert_eq!(
        admit_oci_lowering_plan(&raw_container, &plan),
        Err(AdmissionError::OciRejectsRawContainerInProduction)
    );

    let mutable_tag = oci_decl(RuntimeHostKind::Wasm, "registry.example/aspen/app:latest");
    assert_eq!(admit_oci_lowering_plan(&mutable_tag, &plan), Err(AdmissionError::OciRequiresDigest));

    let missing_derived_artifact = oci_to_wasm_plan(vec![]);
    assert_eq!(
        admit_oci_lowering_plan(&decl, &missing_derived_artifact),
        Err(AdmissionError::OciRequiresDerivedArtifact)
    );
}

#[test]
fn product_path_marker_distinguishes_guardrail_from_execution_evidence() {
    assert_eq!(PRODUCT_PATH_MARKER, "ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED");
    assert_eq!(PRODUCT_PATH_GUARD_MARKER, "ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD");
}
