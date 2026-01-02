//! Integration tests for VM-based job execution with Hyperlight.

#![cfg(all(feature = "vm-executor", target_os = "linux"))]

use aspen_jobs::{
    HyperlightWorker, Job, JobSpec, VmJobPayload, Worker,
};
use std::time::Duration;

/// Test that we can create a HyperlightWorker.
#[tokio::test]
async fn test_create_hyperlight_worker() {
    // This might fail if KVM is not available
    match HyperlightWorker::new() {
        Ok(worker) => {
            let job_types = worker.job_types();
            assert!(job_types.contains(&"vm_execute".to_string()));
            assert!(job_types.contains(&"sandboxed".to_string()));
        }
        Err(e) => {
            eprintln!("Warning: Could not create HyperlightWorker: {}", e);
            eprintln!("This test requires KVM support on Linux");
        }
    }
}

/// Test payload serialization with native binary.
#[tokio::test]
async fn test_native_binary_job_creation() {
    let test_binary = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF header

    let job_spec = JobSpec::with_native_binary(test_binary.clone())
        .timeout(Duration::from_secs(1))
        .priority(aspen_jobs::Priority::High)
        .tag("test");

    assert_eq!(job_spec.job_type, "vm_execute");
    assert!(job_spec.config.tags.contains(&"requires_isolation".to_string()));
    assert!(job_spec.config.tags.contains(&"test".to_string()));

    // Verify the payload
    let payload: VmJobPayload = serde_json::from_value(job_spec.payload).unwrap();
    match payload {
        VmJobPayload::NativeBinary { binary } => {
            assert_eq!(binary, test_binary);
        }
        _ => panic!("Expected NativeBinary payload"),
    }
}

/// Test Nix flake job creation.
#[tokio::test]
async fn test_nix_flake_job_creation() {
    let job_spec = JobSpec::with_nix_flake("github:example/repo", "packages.worker")
        .timeout(Duration::from_secs(30));

    assert_eq!(job_spec.job_type, "vm_execute");

    let payload: VmJobPayload = serde_json::from_value(job_spec.payload).unwrap();
    match payload {
        VmJobPayload::NixExpression { flake_url, attribute } => {
            assert_eq!(flake_url, "github:example/repo");
            assert_eq!(attribute, "packages.worker");
        }
        _ => panic!("Expected NixExpression payload"),
    }
}

/// Test inline Nix expression job creation.
#[tokio::test]
async fn test_nix_derivation_job_creation() {
    let nix_code = r#"
    { pkgs ? import <nixpkgs> {} }:
    pkgs.writeScriptBin "test" ''
        echo "test"
    ''
    "#;

    let job_spec = JobSpec::with_nix_expr(nix_code)
        .timeout(Duration::from_secs(10));

    let payload: VmJobPayload = serde_json::from_value(job_spec.payload).unwrap();
    match payload {
        VmJobPayload::NixDerivation { content } => {
            assert!(content.contains("echo"));
        }
        _ => panic!("Expected NixDerivation payload"),
    }
}

/// Integration test for VM execution (requires KVM).
#[tokio::test]
#[ignore = "Requires KVM and Hyperlight setup"]
async fn test_vm_execution_with_stub_binary() {
    let worker = match HyperlightWorker::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    // Create a minimal ELF stub
    let stub_binary = create_minimal_elf_stub();

    let spec = JobSpec::with_native_binary(stub_binary)
        .timeout(Duration::from_secs(1));

    let job = Job::from_spec(spec);

    let result = worker.execute(job).await;

    // Even with a stub, we should get some result
    match result {
        aspen_jobs::JobResult::Success(_) => {
            // Success is ok
        }
        aspen_jobs::JobResult::Failure(f) => {
            // Failure is expected with a stub binary
            assert!(f.reason.contains("execution") || f.reason.contains("Guest"));
        }
        aspen_jobs::JobResult::Cancelled => {
            panic!("Job should not be cancelled");
        }
    }
}

/// Helper to create a minimal valid ELF binary stub.
fn create_minimal_elf_stub() -> Vec<u8> {
    // This is a minimal ELF header that should be recognized as valid
    // but won't actually execute properly
    let mut elf = vec![
        0x7f, 0x45, 0x4c, 0x46, // ELF magic
        0x02, // 64-bit
        0x01, // Little endian
        0x01, // ELF version
        0x00, // System V ABI
    ];

    // Pad to minimum ELF header size
    elf.resize(64, 0);

    elf
}

/// Test WASM module payload.
#[tokio::test]
async fn test_wasm_module_payload() {
    let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic

    let payload = VmJobPayload::wasm_module(wasm_bytes.clone());

    // Serialize and deserialize
    let json = serde_json::to_value(&payload).unwrap();
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match deserialized {
        VmJobPayload::WasmModule { module } => {
            assert_eq!(module, wasm_bytes);
        }
        _ => panic!("Expected WasmModule payload"),
    }
}

/// Test job with isolation flag.
#[tokio::test]
async fn test_isolation_flag() {
    let job_spec = JobSpec::new("test_job")
        .with_isolation(true);

    assert!(job_spec.config.tags.contains(&"requires_isolation".to_string()));

    let job_spec_no_isolation = JobSpec::new("test_job")
        .with_isolation(false);

    assert!(!job_spec_no_isolation.config.tags.contains(&"requires_isolation".to_string()));
}