//! Integration tests for CloudHypervisorWorker.
//!
//! These tests verify the Cloud Hypervisor worker functionality:
//!
//! - Configuration validation
//! - Payload validation and bounds checking
//! - VM pool management
//! - Protocol message framing
//! - Worker lifecycle (startup/shutdown)
//!
//! # Test Categories
//!
//! 1. **Configuration** - Validation of worker and VM config
//! 2. **Payload Validation** - Tiger Style bounds on commands/args/env
//! 3. **Pool Operations** - Pool status and management
//! 4. **Protocol** - Message serialization and framing
//! 5. **Artifact Collection** - Glob patterns and size limits
//!
//! # Tiger Style
//!
//! - Bounded test timeouts
//! - Fixed limits on all resources
//! - No unbounded collections

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::store::InMemoryBlobStore;
use aspen_ci::workers::cloud_hypervisor::collect_artifacts;
use aspen_ci::workers::{CloudHypervisorPayload, CloudHypervisorWorker, CloudHypervisorWorkerConfig};
use aspen_constants::{CI_VM_DEFAULT_POOL_SIZE, CI_VM_MAX_EXECUTION_TIMEOUT_MS, MAX_CI_VMS_PER_NODE};
use aspen_jobs::Worker;
use tempfile::TempDir;
use tokio::fs;

// =============================================================================
// Configuration Tests
// =============================================================================

/// Test default configuration is valid.
#[test]
fn test_default_config_is_valid() {
    let config = CloudHypervisorWorkerConfig::default();
    assert!(config.validate().is_ok(), "default config should be valid");
}

/// Test configuration with explicit values.
#[test]
fn test_explicit_config_is_valid() {
    let config = CloudHypervisorWorkerConfig {
        node_id: 1,
        state_dir: PathBuf::from("/var/lib/aspen-ci"),
        pool_size: 2,
        max_vms: 8,
        kernel_path: PathBuf::new(),
        initrd_path: PathBuf::new(),
        vm_memory_mib: 4096,
        vm_vcpus: 2,
        ..Default::default()
    };
    assert!(config.validate().is_ok(), "explicit config should be valid");
}

/// Test pool size must not exceed max_vms.
#[test]
fn test_pool_size_bounded_by_max() {
    let config = CloudHypervisorWorkerConfig {
        pool_size: 100, // Larger than max_vms
        max_vms: 8,
        ..Default::default()
    };
    // Validation rejects pool_size > max_vms
    let result = config.validate();
    assert!(result.is_err(), "pool_size > max_vms should fail validation");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("pool_size") || err_msg.contains("max_vms"),
        "error should mention pool_size or max_vms: {}",
        err_msg
    );
}

/// Test max_vms respects constant limit.
#[test]
fn test_max_vms_constant() {
    let config = CloudHypervisorWorkerConfig {
        max_vms: MAX_CI_VMS_PER_NODE,
        ..Default::default()
    };
    assert!(config.validate().is_ok(), "max_vms at constant limit should be valid");
}

// =============================================================================
// Payload Validation Tests
// =============================================================================

/// Test valid payload passes validation.
#[test]
fn test_valid_payload() {
    let payload = CloudHypervisorPayload {
        job_name: Some("test-build".to_string()),
        command: "nix".to_string(),
        args: vec!["build".to_string(), "-L".to_string()],
        working_dir: ".".to_string(),
        env: HashMap::from([("NIX_PATH".to_string(), "/nix/var/nix/profiles".to_string())]),
        timeout_secs: 3600,
        artifacts: vec!["result/**".to_string()],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    assert!(payload.validate().is_ok(), "valid payload should pass");
}

/// Test empty command fails validation.
#[test]
fn test_empty_command_fails() {
    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "".to_string(),
        args: vec![],
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: 60,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "empty command should fail");
    assert!(result.unwrap_err().to_string().contains("empty"), "error should mention empty command");
}

/// Test command length limit.
#[test]
fn test_command_length_limit() {
    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "x".repeat(5000), // Over limit
        args: vec![],
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: 60,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "oversized command should fail");
    assert!(result.unwrap_err().to_string().contains("too long"), "error should mention length");
}

/// Test argument count limit.
#[test]
fn test_argument_count_limit() {
    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "echo".to_string(),
        args: vec!["arg".to_string(); 300], // Over limit of 256
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: 60,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "too many args should fail");
    assert!(result.unwrap_err().to_string().contains("many arguments"), "error should mention args");
}

/// Test individual argument length limit.
#[test]
fn test_argument_length_limit() {
    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "echo".to_string(),
        args: vec!["x".repeat(5000)], // One arg over limit
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: 60,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "oversized arg should fail");
    assert!(result.unwrap_err().to_string().contains("too long"), "error should mention length");
}

/// Test environment variable count limit.
#[test]
fn test_env_count_limit() {
    let mut env = HashMap::new();
    for i in 0..300 {
        env.insert(format!("VAR_{}", i), "value".to_string());
    }

    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "env".to_string(),
        args: vec![],
        working_dir: ".".to_string(),
        env,
        timeout_secs: 60,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "too many env vars should fail");
    assert!(result.unwrap_err().to_string().contains("environment"), "error should mention env");
}

/// Test timeout upper bound.
#[test]
fn test_timeout_limit() {
    let max_timeout_secs = CI_VM_MAX_EXECUTION_TIMEOUT_MS / 1000;

    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "sleep".to_string(),
        args: vec![],
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: max_timeout_secs + 1, // Over limit
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "timeout over limit should fail");
    assert!(result.unwrap_err().to_string().contains("timeout"), "error should mention timeout");
}

/// Test artifact pattern count limit.
#[test]
fn test_artifact_pattern_limit() {
    let payload = CloudHypervisorPayload {
        job_name: None,
        command: "make".to_string(),
        args: vec![],
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: 60,
        artifacts: vec!["*.txt".to_string(); 100], // Over limit of 64
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
    };
    let result = payload.validate();
    assert!(result.is_err(), "too many artifact patterns should fail");
    assert!(result.unwrap_err().to_string().contains("artifact"), "error should mention artifacts");
}

// =============================================================================
// Worker Creation Tests
// =============================================================================

/// Test worker creation with default config.
#[test]
fn test_worker_creation_default() {
    let config = CloudHypervisorWorkerConfig::default();
    let worker = CloudHypervisorWorker::new(config);
    assert!(worker.is_ok(), "worker should be created with default config");
}

/// Test worker creation with blob store.
#[test]
fn test_worker_creation_with_blob_store() {
    let config = CloudHypervisorWorkerConfig::default();
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = CloudHypervisorWorker::with_blob_store(config, Some(blob_store));
    assert!(worker.is_ok(), "worker should be created with blob store");
}

/// Test worker job types.
#[test]
fn test_worker_job_types() {
    let config = CloudHypervisorWorkerConfig::default();
    let worker = CloudHypervisorWorker::new(config).unwrap();

    let types = worker.job_types();
    assert!(types.contains(&"ci_vm".to_string()), "should handle ci_vm jobs");
    assert!(types.contains(&"cloud_hypervisor".to_string()), "should handle cloud_hypervisor jobs");
}

// =============================================================================
// Artifact Collection Tests
// =============================================================================

/// Test artifact collection with empty patterns.
#[tokio::test]
async fn test_artifact_collection_empty_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let result = collect_artifacts(temp_dir.path(), &[]).await.unwrap();

    assert!(result.artifacts.is_empty(), "should have no artifacts");
    assert!(result.unmatched_patterns.is_empty(), "should have no unmatched patterns");
    assert_eq!(result.total_size, 0, "should have zero total size");
}

/// Test artifact collection with basic file patterns.
#[tokio::test]
async fn test_artifact_collection_basic() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    fs::write(temp_dir.path().join("output.txt"), "hello world").await.unwrap();
    fs::write(temp_dir.path().join("result.json"), r#"{"status": "ok"}"#).await.unwrap();

    let result = collect_artifacts(temp_dir.path(), &["*.txt".to_string(), "*.json".to_string()]).await.unwrap();

    assert_eq!(result.artifacts.len(), 2, "should collect 2 artifacts");
    assert!(result.unmatched_patterns.is_empty(), "all patterns should match");
    assert!(result.total_size > 0, "should have non-zero total size");
}

/// Test artifact collection tracks unmatched patterns.
#[tokio::test]
async fn test_artifact_collection_unmatched() {
    let temp_dir = TempDir::new().unwrap();

    // Create only txt file
    fs::write(temp_dir.path().join("output.txt"), "hello").await.unwrap();

    let result = collect_artifacts(temp_dir.path(), &["*.txt".to_string(), "*.nonexistent".to_string()])
        .await
        .unwrap();

    assert_eq!(result.artifacts.len(), 1, "should collect 1 artifact");
    assert_eq!(result.unmatched_patterns.len(), 1, "should have 1 unmatched pattern");
    assert_eq!(result.unmatched_patterns[0], "*.nonexistent");
}

/// Test artifact collection with nested directories.
#[tokio::test]
async fn test_artifact_collection_nested() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure
    let nested = temp_dir.path().join("build").join("output");
    fs::create_dir_all(&nested).await.unwrap();
    fs::write(nested.join("artifact.bin"), vec![0u8; 100]).await.unwrap();

    let result = collect_artifacts(temp_dir.path(), &["**/artifact.bin".to_string()]).await.unwrap();

    assert_eq!(result.artifacts.len(), 1, "should collect nested artifact");
    assert_eq!(result.artifacts[0].relative_path, PathBuf::from("build/output/artifact.bin"));
}

/// Test artifact collection skips directories.
#[tokio::test]
async fn test_artifact_collection_skips_directories() {
    let temp_dir = TempDir::new().unwrap();

    // Create a directory with same name as pattern
    fs::create_dir_all(temp_dir.path().join("output")).await.unwrap();
    fs::write(temp_dir.path().join("output.txt"), "file").await.unwrap();

    let result = collect_artifacts(temp_dir.path(), &["output*".to_string()]).await.unwrap();

    // Should only get the file, not the directory
    assert_eq!(result.artifacts.len(), 1, "should only collect file");
}

// =============================================================================
// Pool Tests
// =============================================================================

/// Test pool status reporting.
#[tokio::test]
async fn test_pool_status() {
    let config = CloudHypervisorWorkerConfig {
        pool_size: 2,
        max_vms: 8,
        ..Default::default()
    };
    let worker = CloudHypervisorWorker::new(config).unwrap();
    let pool = worker.pool();

    let status = pool.status().await;
    assert_eq!(status.idle_vms, 0, "should start with no idle VMs");
    assert_eq!(status.total_vms, 0, "should start with no total VMs");
    assert_eq!(status.max_vms, 8, "should have correct max_vms");
    assert_eq!(status.target_pool_size, 2, "should have correct target pool size");
}

// =============================================================================
// Payload Serialization Tests
// =============================================================================

/// Test payload JSON serialization.
#[test]
fn test_payload_serialization() {
    let payload = CloudHypervisorPayload {
        job_name: Some("build-test".to_string()),
        command: "nix".to_string(),
        args: vec!["build".to_string()],
        working_dir: "/workspace/repo".to_string(),
        env: HashMap::from([("RUST_LOG".to_string(), "debug".to_string())]),
        timeout_secs: 3600,
        artifacts: vec!["result/**".to_string()],
        source_hash: Some("abc123".to_string()),
        checkout_dir: None,
        flake_attr: None,
    };

    let json = serde_json::to_string(&payload).unwrap();
    let parsed: CloudHypervisorPayload = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.command, "nix");
    assert_eq!(parsed.args, vec!["build"]);
    assert_eq!(parsed.job_name, Some("build-test".to_string()));
    assert_eq!(parsed.source_hash, Some("abc123".to_string()));
}

/// Test payload deserialization with defaults.
#[test]
fn test_payload_deserialization_defaults() {
    // Minimal JSON with only required field
    let json = r#"{"command": "echo"}"#;
    let payload: CloudHypervisorPayload = serde_json::from_str(json).unwrap();

    assert_eq!(payload.command, "echo");
    assert!(payload.args.is_empty(), "args should default to empty");
    assert_eq!(payload.working_dir, ".", "working_dir should default to current");
    assert!(payload.env.is_empty(), "env should default to empty");
    assert!(payload.timeout_secs > 0, "timeout should have default value");
    assert!(payload.artifacts.is_empty(), "artifacts should default to empty");
}

// =============================================================================
// Protocol Constants Tests
// =============================================================================

/// Test that we export the default pool size constant.
#[test]
fn test_pool_size_constant() {
    assert!(CI_VM_DEFAULT_POOL_SIZE > 0, "default pool size should be positive");
    assert!(CI_VM_DEFAULT_POOL_SIZE <= MAX_CI_VMS_PER_NODE, "default pool size should not exceed max VMs");
}

/// Test execution timeout constants.
#[test]
fn test_timeout_constants() {
    let max_timeout_secs = CI_VM_MAX_EXECUTION_TIMEOUT_MS / 1000;
    assert!(max_timeout_secs > 0, "max timeout should be positive");
    assert!(max_timeout_secs <= 24 * 3600, "max timeout should not exceed 24 hours");
}
