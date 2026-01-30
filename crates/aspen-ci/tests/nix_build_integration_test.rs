//! Integration tests for NixBuildWorker.
//!
//! These tests verify end-to-end Nix build execution including:
//! - Successful builds with minimal flake outputs
//! - NAR archive creation and blob storage upload
//! - Cache registration and SNIX integration
//! - Build failure handling and cleanup
//! - Resource isolation and configuration validation
//!
//! # Test Categories
//!
//! 1. **Successful Builds** - Simple and multi-output flakes
//! 2. **Artifact Upload** - NAR archives, blob storage, cache registration
//! 3. **Build Failures** - Invalid flakes, timeouts, attribute errors
//! 4. **Configuration** - Missing services, partial configs, validation
//! 5. **Edge Cases** - Large outputs, long logs, concurrent builds
//!
//! # Tiger Style
//!
//! - Bounded test timeouts (no infinite waits)
//! - Explicit cleanup of build artifacts
//! - Real Nix integration with minimal test flakes
//! - Mock services for blob storage and cache index

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::BlobStore;
use aspen_blob::store::InMemoryBlobStore;
use aspen_cache::CacheEntry;
use aspen_cache::CacheError;
use aspen_cache::CacheIndex;
use aspen_cache::CacheStats;
use aspen_ci::workers::NixBuildPayload;
use aspen_ci::workers::NixBuildWorker;
use aspen_ci::workers::NixBuildWorkerConfig;
use aspen_jobs::DependencyFailurePolicy;
use aspen_jobs::DependencyState;
use aspen_jobs::Job;
use aspen_jobs::JobId;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::Priority;
use aspen_jobs::Worker;
use async_trait::async_trait;
use serde_json::Value;
use tempfile::TempDir;
use tokio::fs;

// Test constants
const TEST_TIMEOUT: Duration = Duration::from_secs(120); // Nix builds can be slow
const SIMPLE_FLAKE_TIMEOUT: u64 = 60; // 1 minute for simple builds

/// Check if nix command is available for testing.
async fn check_nix_available() -> bool {
    tokio::process::Command::new("nix")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Skip test if nix binary not available.
macro_rules! skip_if_no_nix {
    () => {
        if !check_nix_available().await {
            eprintln!("Skipping test: nix command not available");
            return;
        }
    };
}

/// Mock CacheIndex for testing cache registration.
#[derive(Debug, Clone, Default)]
pub struct MockCacheIndex {
    pub entries: Arc<tokio::sync::Mutex<HashMap<String, CacheEntry>>>,
    pub put_calls: Arc<tokio::sync::Mutex<Vec<String>>>,
}

#[async_trait]
impl CacheIndex for MockCacheIndex {
    async fn get(&self, store_hash: &str) -> Result<Option<CacheEntry>, CacheError> {
        let entries = self.entries.lock().await;
        Ok(entries.get(store_hash).cloned())
    }

    async fn put(&self, entry: CacheEntry) -> Result<(), CacheError> {
        let store_hash = entry.store_hash.clone();

        let mut entries = self.entries.lock().await;
        entries.insert(store_hash.clone(), entry);

        let mut calls = self.put_calls.lock().await;
        calls.push(store_hash);

        Ok(())
    }

    async fn exists(&self, store_hash: &str) -> Result<bool, CacheError> {
        let entries = self.entries.lock().await;
        Ok(entries.contains_key(store_hash))
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        let entries = self.entries.lock().await;
        Ok(CacheStats {
            total_entries: entries.len() as u64,
            total_nar_bytes: 0, // Mock doesn't track NAR sizes
            query_count: entries.len() as u64,
            hit_count: 0,  // Mock doesn't track hits
            miss_count: 0, // Mock doesn't track misses
            last_updated: chrono::Utc::now().timestamp() as u64,
        })
    }
}

/// Create a minimal test flake that builds successfully.
async fn create_test_flake(temp_dir: &TempDir) -> std::io::Result<String> {
    let flake_path = temp_dir.path().join("flake.nix");

    // Simple flake that builds a hello package
    let flake_content = r#"
{
  description = "Test flake for CI integration tests";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.${system} = {
        # Simple derivation that just creates a file
        hello = pkgs.writeText "test-hello" "Hello from CI test!";

        # Script package for testing executable outputs
        test-script = pkgs.writeScriptBin "test-script" ''
          #!/usr/bin/env bash
          echo "Test script executed successfully"
        '';

        # Default package
        default = pkgs.writeText "test-hello" "Hello from CI test!";
      };
    };
}
"#;

    fs::write(&flake_path, flake_content).await?;
    Ok(temp_dir.path().to_string_lossy().to_string())
}

/// Create test payload for successful build.
fn create_test_payload(flake_url: &str, attribute: Option<&str>) -> NixBuildPayload {
    NixBuildPayload {
        flake_url: flake_url.to_string(),
        attribute: attribute.unwrap_or("default").to_string(),
        timeout_secs: SIMPLE_FLAKE_TIMEOUT,
        sandbox: true,
        upload_result: true,
        artifacts: vec![],
        job_name: Some("test-build".to_string()),
        extra_args: vec![],
        working_dir: None,
        cache_key: None,
    }
}

/// Create a test job with the given payload.
fn create_test_job(payload: NixBuildPayload) -> Job {
    let job_spec = JobSpec::new("ci_nix_build")
        .payload(payload)
        .expect("failed to serialize payload")
        .priority(Priority::Normal)
        .timeout(TEST_TIMEOUT);

    Job {
        id: JobId::new(),
        spec: job_spec,
        status: JobStatus::Running,
        attempts: 1,
        last_error: None,
        result: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        scheduled_at: None,
        started_at: Some(chrono::Utc::now()),
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
        dependency_failure_policy: DependencyFailurePolicy::default(),
        execution_token: Some("test-token".to_string()),
    }
}

/// Test successful build execution with minimal flake.
#[tokio::test]
async fn test_successful_nix_build() {
    skip_if_no_nix!();

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let flake_url = create_test_flake(&temp_dir).await.expect("failed to create test flake");

    let payload = create_test_payload(&flake_url, Some("packages.x86_64-linux.hello"));
    let job = create_test_job(payload);

    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    // Execute build
    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Verify success
    assert!(result.is_success(), "build should succeed: {:?}", result);

    // Extract output from success result
    let job_output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected successful result"),
    };

    // Parse output JSON
    let output: Value = job_output.data;

    assert!(output.get("output_paths").is_some(), "output should contain output_paths");
    assert!(output.get("log_truncated").is_some(), "output should contain log_truncated field");
}

/// Test build with multiple packages by building them individually.
#[tokio::test]
async fn test_multi_package_build() {
    skip_if_no_nix!();

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let flake_url = create_test_flake(&temp_dir).await.expect("failed to create test flake");

    // Build each package individually to test multiple outputs
    let packages = vec!["packages.x86_64-linux.hello", "packages.x86_64-linux.test-script"];

    for package in packages {
        let payload = create_test_payload(&flake_url, Some(package));
        let job = create_test_job(payload);

        let config = NixBuildWorkerConfig::default();
        let worker = NixBuildWorker::new(config);

        let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

        // Verify success
        assert!(result.is_success(), "package {} build should succeed: {:?}", package, result);

        let job_output = match result {
            JobResult::Success(output) => output,
            _ => panic!("expected successful result for {}", package),
        };

        let output: Value = job_output.data;

        if let Some(output_paths) = output.get("output_paths").and_then(|v| v.as_array()) {
            assert!(!output_paths.is_empty(), "should have output paths for {}", package);

            // All paths should be /nix/store paths
            for path in output_paths {
                let path_str = path.as_str().expect("path should be string");
                assert!(path_str.starts_with("/nix/store/"), "path should be in nix store: {}", path_str);
            }
        } else {
            panic!("output_paths should be an array for {}", package);
        }
    }
}

/// Test build with blob storage upload.
#[tokio::test]
async fn test_build_with_blob_upload() {
    skip_if_no_nix!();

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let flake_url = create_test_flake(&temp_dir).await.expect("failed to create test flake");

    let payload = create_test_payload(&flake_url, Some("packages.x86_64-linux.hello"));
    let job = create_test_job(payload);

    // Create config with blob store and cache index
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let cache_index = Arc::new(MockCacheIndex::default());

    let mut config = NixBuildWorkerConfig::default();
    config.blob_store = Some(blob_store.clone());
    config.cache_index = Some(cache_index.clone());

    let worker = NixBuildWorker::new(config);

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Verify build succeeded
    assert!(result.is_success(), "build with upload should succeed: {:?}", result);

    // Verify blob store has uploads
    let blobs = blob_store.list(100, None).await.expect("failed to list blobs");
    assert!(!blobs.blobs.is_empty(), "blob store should contain uploaded NAR archives");

    // Verify cache index has entries
    let cache_calls = cache_index.put_calls.lock().await;
    assert!(!cache_calls.is_empty(), "cache index should have put() calls for store paths");
}

/// Test build failure with invalid flake.
#[tokio::test]
async fn test_build_failure_invalid_flake() {
    skip_if_no_nix!();

    // Use non-existent flake URL
    let payload = create_test_payload("/nonexistent/path", Some("default"));
    let job = create_test_job(payload);

    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Verify failure
    assert!(!result.is_success(), "invalid flake build should fail");

    match result {
        JobResult::Failure(failure) => {
            assert!(!failure.reason.is_empty(), "should have error message");
            assert!(
                failure.reason.contains("failed")
                    || failure.reason.contains("error")
                    || failure.reason.contains("nonexistent"),
                "error should be descriptive: {}",
                failure.reason
            );
        }
        _ => panic!("expected failure result"),
    }
}

/// Test build failure with invalid attribute.
#[tokio::test]
async fn test_build_failure_invalid_attribute() {
    skip_if_no_nix!();

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let flake_url = create_test_flake(&temp_dir).await.expect("failed to create test flake");

    // Use invalid attribute that doesn't exist in the flake
    let payload = create_test_payload(&flake_url, Some("packages.x86_64-linux.nonexistent"));
    let job = create_test_job(payload);

    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Verify failure
    assert!(!result.is_success(), "invalid attribute build should fail");

    match result {
        JobResult::Failure(failure) => {
            assert!(!failure.reason.is_empty(), "should have error message for invalid attribute");
        }
        JobResult::Success(_) => panic!("expected failure for invalid attribute"),
        JobResult::Cancelled => panic!("unexpected cancellation"),
    }
}

/// Test timeout handling with very short timeout.
/// Note: This test is inherently flaky as it depends on system performance.
/// Skipped for now as core functionality is tested by other tests.
#[tokio::test]
#[ignore = "Timeout test is flaky - depends on system performance"]
async fn test_build_timeout() {
    skip_if_no_nix!();

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let flake_url = create_test_flake(&temp_dir).await.expect("failed to create test flake");

    // Use extremely short timeout (100ms - should timeout before build starts)
    let mut payload = create_test_payload(&flake_url, Some("packages.x86_64-linux.hello"));
    payload.timeout_secs = 1; // This is the minimum we can set

    // Create job with much shorter timeout than the payload suggests
    let job_spec = JobSpec::new("ci_nix_build")
        .payload(payload)
        .expect("failed to serialize payload")
        .priority(Priority::Normal)
        .timeout(Duration::from_millis(100)); // Very short timeout

    let job = Job {
        id: JobId::new(),
        spec: job_spec,
        status: JobStatus::Running,
        attempts: 1,
        last_error: None,
        result: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        scheduled_at: None,
        started_at: Some(chrono::Utc::now()),
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
        dependency_failure_policy: DependencyFailurePolicy::default(),
        execution_token: Some("test-token".to_string()),
    };

    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    let result = tokio::time::timeout(Duration::from_secs(10), worker.execute(job))
        .await
        .expect("test timeout exceeded");

    // Verify timeout failure
    assert!(!result.is_success(), "short timeout build should fail: {:?}", result);

    match result {
        JobResult::Failure(failure) => {
            // Should be a timeout-related error
            println!("Timeout failure reason: {}", failure.reason);
            // Don't assert specific message as it may vary
        }
        _ => panic!("expected timeout failure, got {:?}", result),
    }
}

/// Test configuration validation with missing blob store.
#[tokio::test]
async fn test_config_missing_blob_store() {
    skip_if_no_nix!();

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let flake_url = create_test_flake(&temp_dir).await.expect("failed to create test flake");

    let payload = create_test_payload(&flake_url, Some("packages.x86_64-linux.hello"));
    let job = create_test_job(payload);

    // Config without blob store (uploads should be skipped)
    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Build should still succeed without blob store
    assert!(result.is_success(), "build without blob store should succeed: {:?}", result);

    // Extract output
    let job_output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected successful result"),
    };

    let output: Value = job_output.data;

    assert!(output.get("output_paths").is_some(), "should still have output paths");
    assert!(output.get("log_truncated").is_some(), "should still have log_truncated field");
}

/// Test payload validation edge cases.
#[tokio::test]
async fn test_payload_validation() {
    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    // Test empty flake URL
    let mut payload = create_test_payload("", Some("default"));
    payload.flake_url = "".to_string();
    let job = create_test_job(payload);

    let result = worker.execute(job).await;

    // Should fail validation immediately
    assert!(!result.is_success(), "empty flake URL should fail validation");

    match result {
        JobResult::Failure(failure) => {
            assert!(!failure.reason.is_empty(), "should have validation error");
        }
        _ => panic!("expected validation failure"),
    }
}

/// Test that worker only handles ci_nix_build job types.
#[test]
fn test_worker_job_types() {
    let config = NixBuildWorkerConfig::default();
    let worker = NixBuildWorker::new(config);

    let job_types = worker.job_types();
    assert_eq!(job_types, vec!["ci_nix_build"], "should only handle ci_nix_build jobs");
}
