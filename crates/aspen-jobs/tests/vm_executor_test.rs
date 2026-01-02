//! Tests for VM-based job execution.

#![cfg(feature = "vm-executor")]

use aspen_jobs::{
    HyperlightWorker, Job, JobSpec, JobStatus, VmJobPayload, Worker,
};
use std::time::Duration;

#[tokio::test]
async fn test_native_binary_payload() {
    // Create a simple test binary (echo command)
    let test_binary = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic number

    let job = JobSpec::with_native_binary(test_binary.clone())
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    assert_eq!(job.job_type, "vm_execute");
    assert!(job.config.tags.contains(&"requires_isolation".to_string()));

    // Verify payload serialization
    let payload: VmJobPayload = serde_json::from_value(job.payload).unwrap();
    match payload {
        VmJobPayload::NativeBinary { binary } => {
            assert_eq!(binary, test_binary);
        }
        _ => panic!("Expected NativeBinary payload"),
    }
}

#[tokio::test]
async fn test_nix_flake_payload() {
    let job = JobSpec::with_nix_flake("github:example/repo", "jobs.processor")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    assert_eq!(job.job_type, "vm_execute");

    let payload: VmJobPayload = serde_json::from_value(job.payload).unwrap();
    match payload {
        VmJobPayload::NixExpression { flake_url, attribute } => {
            assert_eq!(flake_url, "github:example/repo");
            assert_eq!(attribute, "jobs.processor");
        }
        _ => panic!("Expected NixExpression payload"),
    }
}

#[tokio::test]
async fn test_nix_derivation_payload() {
    let nix_code = r#"
    { pkgs ? import <nixpkgs> {} }:
    pkgs.writeScriptBin "test-job" ''
        echo "Hello from Nix"
    ''
    "#;

    let job = JobSpec::with_nix_expr(nix_code)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let payload: VmJobPayload = serde_json::from_value(job.payload).unwrap();
    match payload {
        VmJobPayload::NixDerivation { content } => {
            assert!(content.contains("Hello from Nix"));
        }
        _ => panic!("Expected NixDerivation payload"),
    }
}

#[tokio::test]
async fn test_hyperlight_worker_job_types() {
    let worker = HyperlightWorker::new().unwrap();
    let job_types = worker.job_types();

    assert!(job_types.contains(&"vm_execute".to_string()));
    assert!(job_types.contains(&"sandboxed".to_string()));
}

#[tokio::test]
async fn test_payload_serialization_roundtrip() {
    let original = VmJobPayload::native_binary(vec![1, 2, 3, 4]);

    // Serialize to JSON
    let json = serde_json::to_value(&original).unwrap();

    // Deserialize back
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::NativeBinary { binary: b1 },
            VmJobPayload::NativeBinary { binary: b2 },
        ) => {
            assert_eq!(b1, b2);
        }
        _ => panic!("Payload types don't match"),
    }
}

#[tokio::test]
async fn test_wasm_payload() {
    let wasm_module = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic number

    let payload = VmJobPayload::wasm_module(wasm_module.clone());

    match payload {
        VmJobPayload::WasmModule { module } => {
            assert_eq!(module, wasm_module);
        }
        _ => panic!("Expected WasmModule payload"),
    }
}

#[tokio::test]
async fn test_job_with_isolation_flag() {
    let job = JobSpec::new("some_job")
        .with_isolation(true)
        .build()
        .unwrap();

    assert!(job.config.tags.contains(&"requires_isolation".to_string()));

    let job_no_isolation = JobSpec::new("other_job")
        .with_isolation(false)
        .build()
        .unwrap();

    assert!(!job_no_isolation.config.tags.contains(&"requires_isolation".to_string()));
}

// Integration test would require actual Hyperlight setup
#[tokio::test]
#[ignore = "Requires Hyperlight/KVM environment"]
async fn test_hyperlight_worker_execution() {
    let worker = HyperlightWorker::new().unwrap();

    // Create a simple test job
    let job_spec = JobSpec::with_native_binary(vec![/* actual ELF binary */])
        .timeout(Duration::from_secs(1));

    let job = Job {
        id: aspen_jobs::JobId::new(),
        spec: job_spec.build().unwrap(),
        status: JobStatus::Running,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
        retry_count: 0,
        last_error: None,
        result: None,
        worker_id: Some("test-worker".to_string()),
        queue_item_id: None,
    };

    let result = worker.execute(job).await;

    // Since we're simulating, this should succeed
    assert!(result.is_success());
}