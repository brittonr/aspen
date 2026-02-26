//! Tests for VM-based job execution.

#![cfg(feature = "plugins-vm")]

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_jobs::DependencyFailurePolicy;
use aspen_jobs::DependencyState;
use aspen_jobs::HyperlightWorker;
use aspen_jobs::Job;
use aspen_jobs::JobId;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::VmJobPayload;
use aspen_jobs::Worker;
use chrono::Utc;

fn create_test_worker() -> HyperlightWorker {
    let blob_store = Arc::new(InMemoryBlobStore::new());
    HyperlightWorker::new(blob_store).unwrap()
}

#[tokio::test]
async fn test_blob_binary_payload() {
    // Create a job referencing a blob-stored binary
    let test_hash = "abc123def456";
    let test_size = 1024u64;
    let test_format = "elf";

    let job = JobSpec::with_blob_binary(test_hash, test_size, test_format).timeout(Duration::from_secs(1));

    assert_eq!(job.job_type, "vm_execute");
    assert!(job.config.tags.contains(&"requires_isolation".to_string()));

    // Verify payload serialization
    let payload: VmJobPayload = serde_json::from_value(job.payload).unwrap();
    match payload {
        VmJobPayload::BlobBinary { hash, size, format } => {
            assert_eq!(hash, test_hash);
            assert_eq!(size, test_size);
            assert_eq!(format, test_format);
        }
        _ => panic!("Expected BlobBinary payload"),
    }
}

#[tokio::test]
async fn test_nix_flake_payload() {
    let job = JobSpec::with_nix_flake("github:example/repo", "jobs.processor").timeout(Duration::from_secs(30));

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

    let job = JobSpec::with_nix_expr(nix_code).timeout(Duration::from_secs(10));

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
    let worker = create_test_worker();
    let job_types = worker.job_types();

    assert!(job_types.contains(&"vm_execute".to_string()));
    assert!(job_types.contains(&"sandboxed".to_string()));
}

#[tokio::test]
async fn test_payload_serialization_roundtrip() {
    let original = VmJobPayload::blob_binary("deadbeef", 4096, "elf");

    // Serialize to JSON
    let json = serde_json::to_value(&original).unwrap();

    // Deserialize back
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::BlobBinary {
                hash: h1,
                size: s1,
                format: f1,
            },
            VmJobPayload::BlobBinary {
                hash: h2,
                size: s2,
                format: f2,
            },
        ) => {
            assert_eq!(h1, h2);
            assert_eq!(s1, s2);
            assert_eq!(f1, f2);
        }
        _ => panic!("Payload types don't match"),
    }
}

#[tokio::test]
async fn test_nix_expression_payload_roundtrip() {
    let original = VmJobPayload::nix_flake("github:nixos/nixpkgs#hello", "packages.hello");

    let json = serde_json::to_value(&original).unwrap();
    let deserialized: VmJobPayload = serde_json::from_value(json).unwrap();

    match (original, deserialized) {
        (
            VmJobPayload::NixExpression {
                flake_url: f1,
                attribute: a1,
            },
            VmJobPayload::NixExpression {
                flake_url: f2,
                attribute: a2,
            },
        ) => {
            assert_eq!(f1, f2);
            assert_eq!(a1, a2);
        }
        _ => panic!("Payload types don't match"),
    }
}

#[tokio::test]
async fn test_job_with_isolation_flag() {
    let job = JobSpec::new("some_job").with_isolation(true);

    assert!(job.config.tags.contains(&"requires_isolation".to_string()));

    let job_no_isolation = JobSpec::new("other_job").with_isolation(false);

    assert!(!job_no_isolation.config.tags.contains(&"requires_isolation".to_string()));
}

// Integration test would require actual Hyperlight setup
#[tokio::test]
#[ignore = "Requires Hyperlight/KVM environment"]
async fn test_hyperlight_worker_execution() {
    let worker = create_test_worker();

    // Create a simple test job referencing a blob binary
    let job_spec = JobSpec::with_blob_binary("test_blob_hash", 1024, "elf").timeout(Duration::from_secs(1));

    let job = Job {
        id: JobId::new(),
        spec: job_spec,
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
    };

    let result = worker.execute(job).await;

    // Since we're using a test hash without actual blob data, expect failure
    assert!(!result.is_success());
}
