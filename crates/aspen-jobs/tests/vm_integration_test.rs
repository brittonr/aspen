//! Integration tests for VM-based job execution with Hyperlight.

#![cfg(all(feature = "plugins-vm", target_os = "linux"))]

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_jobs::HyperlightWorker;
use aspen_jobs::Job;
use aspen_jobs::JobSpec;
use aspen_jobs::VmJobPayload;
use aspen_jobs::Worker;

fn create_test_worker() -> Result<HyperlightWorker, String> {
    let blob_store = Arc::new(InMemoryBlobStore::new());
    HyperlightWorker::new(blob_store).map_err(|e| e.to_string())
}

/// Test that we can create a HyperlightWorker.
#[tokio::test]
async fn test_create_hyperlight_worker() {
    // This might fail if KVM is not available
    match create_test_worker() {
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

/// Test blob binary job creation.
#[tokio::test]
async fn test_blob_binary_job_creation() {
    let test_hash = "abc123def456deadbeef";
    let test_size = 4096u64;

    let job_spec = JobSpec::with_blob_binary(test_hash, test_size, "elf")
        .timeout(Duration::from_secs(1))
        .priority(aspen_jobs::Priority::High)
        .tag("test");

    assert_eq!(job_spec.job_type, "vm_execute");
    assert!(job_spec.config.tags.contains(&"requires_isolation".to_string()));
    assert!(job_spec.config.tags.contains(&"test".to_string()));

    // Verify the payload
    let payload: VmJobPayload = serde_json::from_value(job_spec.payload).unwrap();
    match payload {
        VmJobPayload::BlobBinary { hash, size, format } => {
            assert_eq!(hash, test_hash);
            assert_eq!(size, test_size);
            assert_eq!(format, "elf");
        }
        _ => panic!("Expected BlobBinary payload"),
    }
}

/// Test Nix flake job creation.
#[tokio::test]
async fn test_nix_flake_job_creation() {
    let job_spec = JobSpec::with_nix_flake("github:example/repo", "packages.worker").timeout(Duration::from_secs(30));

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

    let job_spec = JobSpec::with_nix_expr(nix_code).timeout(Duration::from_secs(10));

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
async fn test_vm_execution_with_blob_binary() {
    let worker = match create_test_worker() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    // Reference a test blob (would need to be uploaded to blob store first)
    let spec = JobSpec::with_blob_binary("test_blob_hash", 64, "elf").timeout(Duration::from_secs(1));

    let job = Job::from_spec(spec);

    let result = worker.execute(job).await;

    // Since we're using a test hash without actual blob data, expect failure
    assert!(!result.is_success(), "Expected failure with test blob hash - blob doesn't exist");
}

/// Test job with isolation flag.
#[tokio::test]
async fn test_isolation_flag() {
    let job_spec = JobSpec::new("test_job").with_isolation(true);

    assert!(job_spec.config.tags.contains(&"requires_isolation".to_string()));

    let job_spec_no_isolation = JobSpec::new("test_job").with_isolation(false);

    assert!(!job_spec_no_isolation.config.tags.contains(&"requires_isolation".to_string()));
}

/// Test all payload types serialize correctly.
#[tokio::test]
async fn test_all_payload_types_serialization() {
    // BlobBinary
    let blob = VmJobPayload::blob_binary("hash123", 1024, "elf");
    let json = serde_json::to_value(&blob).unwrap();
    assert_eq!(json["type"], "BlobBinary");

    // NixExpression
    let nix_flake = VmJobPayload::nix_flake("github:test/repo", "attr");
    let json = serde_json::to_value(&nix_flake).unwrap();
    assert_eq!(json["type"], "NixExpression");

    // NixDerivation
    let nix_deriv = VmJobPayload::nix_derivation("{ pkgs }: pkgs.hello");
    let json = serde_json::to_value(&nix_deriv).unwrap();
    assert_eq!(json["type"], "NixDerivation");
}
