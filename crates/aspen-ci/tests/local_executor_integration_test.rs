//! Integration tests for LocalExecutorWorker.
//!
//! These tests verify end-to-end command execution including:
//! - Successful shell command execution
//! - Nix build command execution with proper flag injection
//! - Working directory management and workspace isolation
//! - Environment variable handling
//! - Timeout enforcement
//! - Artifact collection
//! - Error handling and exit codes
//!
//! # Test Categories
//!
//! 1. **Successful Execution** - Simple commands, multi-step commands
//! 2. **Workspace Management** - Directory creation, cleanup, isolation
//! 3. **Nix Integration** - Flag injection, offline mode, flake config
//! 4. **Failure Handling** - Non-zero exit, timeouts, missing commands
//! 5. **Artifact Collection** - Glob patterns, blob upload
//!
//! # Tiger Style
//!
//! - Bounded test timeouts (no infinite waits)
//! - Explicit cleanup of test workspaces
//! - Real process execution (no mocks for core functionality)
//! - Mock services only for blob storage

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::prelude::*;
use aspen_blob::store::InMemoryBlobStore;
use aspen_ci::workers::LocalExecutorPayload;
use aspen_ci::workers::LocalExecutorWorker;
use aspen_ci::workers::LocalExecutorWorkerConfig;
use aspen_jobs::DependencyFailurePolicy;
use aspen_jobs::DependencyState;
use aspen_jobs::Job;
use aspen_jobs::JobId;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::Priority;
use aspen_jobs::Worker;
use serde_json::Value;
use tempfile::TempDir;

// Test constants
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const COMMAND_TIMEOUT: u64 = 10; // seconds

/// Extract stdout content from job output data.
///
/// Handles both old format (direct string) and new OutputRef format.
fn extract_stdout(data: &Value) -> &str {
    // Try new OutputRef format first: {"type": "inline", "content": "..."}
    if let Some(stdout_obj) = data.get("stdout").and_then(|v| v.as_object()) {
        if let Some(content) = stdout_obj.get("content").and_then(|v| v.as_str()) {
            return content;
        }
    }
    // Fall back to old string format
    data.get("stdout").and_then(|v| v.as_str()).unwrap_or("")
}

/// Extract stderr content from job output data.
///
/// Handles both old format (direct string) and new OutputRef format.
fn extract_stderr(data: &Value) -> &str {
    // Try new OutputRef format first: {"type": "inline", "content": "..."}
    if let Some(stderr_obj) = data.get("stderr").and_then(|v| v.as_object()) {
        if let Some(content) = stderr_obj.get("content").and_then(|v| v.as_str()) {
            return content;
        }
    }
    // Fall back to old string format
    data.get("stderr").and_then(|v| v.as_str()).unwrap_or("")
}

/// Create a test worker with a temp directory workspace.
fn create_test_worker(temp_dir: &TempDir) -> LocalExecutorWorker {
    let config = LocalExecutorWorkerConfig {
        workspace_dir: temp_dir.path().to_path_buf(),
        should_cleanup_workspaces: true,
        ..Default::default()
    };
    LocalExecutorWorker::new(config)
}

/// Create a test worker with blob store.
fn create_test_worker_with_blobs(temp_dir: &TempDir, blob_store: Arc<dyn BlobStore>) -> LocalExecutorWorker {
    let config = LocalExecutorWorkerConfig {
        workspace_dir: temp_dir.path().to_path_buf(),
        should_cleanup_workspaces: true,
        ..Default::default()
    };
    LocalExecutorWorker::with_blob_store(config, blob_store)
}

/// Create test payload for a shell command.
fn create_shell_payload(command: &str, args: Vec<&str>) -> LocalExecutorPayload {
    LocalExecutorPayload {
        job_name: Some("test-job".to_string()),
        command: command.to_string(),
        args: args.into_iter().map(String::from).collect(),
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: COMMAND_TIMEOUT,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
        run_id: None,
        cached_execution: false,
    }
}

/// Create a test job with the given payload and job type.
fn create_test_job(payload: LocalExecutorPayload, job_type: &str) -> Job {
    let job_spec = JobSpec::new(job_type)
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

// =============================================================================
// Successful Execution Tests
// =============================================================================

/// Test successful execution of a simple echo command.
#[tokio::test]
async fn test_successful_echo_command() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("echo", vec!["Hello, World!"]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "echo should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stdout = extract_stdout(&data);
    assert!(stdout.contains("Hello, World!"), "stdout should contain greeting: {}", stdout);
}

/// Test successful execution with exit code 0.
#[tokio::test]
async fn test_exit_code_success() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("true", vec![]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "true command should succeed");

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let exit_code = data.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
    assert_eq!(exit_code, 0, "exit code should be 0");
}

/// Test execution with environment variables.
#[tokio::test]
async fn test_environment_variables() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let mut payload = create_shell_payload("sh", vec!["-c", "echo $MY_VAR"]);
    payload.env.insert("MY_VAR".to_string(), "test_value_123".to_string());

    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "env var test should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stdout = extract_stdout(&data);
    assert!(stdout.contains("test_value_123"), "stdout should contain env var value: {}", stdout);
}

/// Test execution creates files in workspace.
#[tokio::test]
async fn test_workspace_file_creation() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("sh", vec!["-c", "echo 'test content' > output.txt && cat output.txt"]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "file creation should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stdout = extract_stdout(&data);
    assert!(stdout.contains("test content"), "should output file contents: {}", stdout);
}

// =============================================================================
// Failure Handling Tests
// =============================================================================

/// Test non-zero exit code handling.
#[tokio::test]
async fn test_nonzero_exit_code() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("sh", vec!["-c", "exit 42"]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Non-zero exit is still a completed execution, check the exit code
    let output = match result {
        JobResult::Success(output) => output,
        JobResult::Failure(f) => {
            // Verify failure contains exit code info
            assert!(
                f.reason.contains("42") || f.reason.contains("exit"),
                "failure should mention exit code: {}",
                f.reason
            );
            return;
        }
        _ => panic!("unexpected result: {:?}", result),
    };

    let data: Value = output.data;
    let exit_code = data.get("exit_code").and_then(|v| v.as_i64());
    if let Some(code) = exit_code {
        assert_eq!(code, 42, "exit code should be 42");
    }
}

/// Test command not found handling.
#[tokio::test]
async fn test_command_not_found() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("nonexistent_command_xyz123", vec![]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Should fail - command doesn't exist
    match result {
        JobResult::Failure(f) => {
            assert!(!f.reason.is_empty(), "should have error message");
        }
        JobResult::Success(output) => {
            // Some systems return non-zero exit code instead of spawn failure
            let data: Value = output.data;
            let exit_code = data.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
            assert_ne!(exit_code, 0, "should have non-zero exit code for missing command");
        }
        _ => panic!("unexpected result: {:?}", result),
    }
}

/// Test stderr capture.
#[tokio::test]
async fn test_stderr_capture() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("sh", vec!["-c", "echo 'error message' >&2"]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "stderr test should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stderr = extract_stderr(&data);
    assert!(stderr.contains("error message"), "stderr should contain error: {}", stderr);
}

// =============================================================================
// Payload Validation Tests
// =============================================================================

/// Test empty command validation.
#[tokio::test]
async fn test_empty_command_validation() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = LocalExecutorPayload {
        job_name: Some("test".to_string()),
        command: "".to_string(),
        args: vec![],
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: COMMAND_TIMEOUT,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
        run_id: None,
        cached_execution: false,
    };
    let job = create_test_job(payload, "shell_command");

    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(
                f.reason.contains("empty") || f.reason.contains("command"),
                "should mention empty command: {}",
                f.reason
            );
        }
        _ => panic!("expected validation failure for empty command"),
    }
}

/// Test command too long validation.
#[tokio::test]
async fn test_command_too_long_validation() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = LocalExecutorPayload {
        job_name: Some("test".to_string()),
        command: "x".repeat(5000), // MAX_COMMAND_LENGTH is 4096
        args: vec![],
        working_dir: ".".to_string(),
        env: HashMap::new(),
        timeout_secs: COMMAND_TIMEOUT,
        artifacts: vec![],
        source_hash: None,
        checkout_dir: None,
        flake_attr: None,
        run_id: None,
        cached_execution: false,
    };
    let job = create_test_job(payload, "shell_command");

    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(
                f.reason.contains("too long") || f.reason.contains("command"),
                "should mention command too long: {}",
                f.reason
            );
        }
        _ => panic!("expected validation failure for long command"),
    }
}

// =============================================================================
// Nix Integration Tests
// =============================================================================

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

/// Test nix command with flag injection.
#[tokio::test]
async fn test_nix_flag_injection() {
    if !check_nix_available().await {
        eprintln!("Skipping test: nix command not available");
        return;
    }

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    // Use ci_nix_build job type which triggers flag injection
    // The --help flag should work with or without extra flags
    let payload = create_shell_payload("nix", vec!["--help"]);
    let job = create_test_job(payload, "ci_nix_build");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    // Should succeed (nix --help exits 0)
    assert!(result.is_success(), "nix --help should succeed: {:?}", result);
}

/// Test nix eval command execution.
#[tokio::test]
async fn test_nix_eval_command() {
    if !check_nix_available().await {
        eprintln!("Skipping test: nix command not available");
        return;
    }

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("nix", vec!["eval", "--expr", "1 + 1"]);
    let job = create_test_job(payload, "ci_nix_build");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "nix eval should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stdout = extract_stdout(&data);
    assert!(stdout.contains("2"), "nix eval 1+1 should output 2: {}", stdout);
}

// =============================================================================
// Worker Configuration Tests
// =============================================================================

/// Test worker job types.
#[test]
fn test_worker_job_types() {
    let config = LocalExecutorWorkerConfig::default();
    let worker = LocalExecutorWorker::new(config);

    let job_types = worker.job_types();

    assert!(job_types.contains(&"shell_command".to_string()), "should handle shell_command");
    assert!(job_types.contains(&"local_executor".to_string()), "should handle local_executor");
    // Shell executor must NOT claim types belonging to dedicated executors
    assert!(!job_types.contains(&"ci_nix_build".to_string()), "ci_nix_build belongs to NixBuildWorker");
    assert!(!job_types.contains(&"ci_vm".to_string()), "ci_vm belongs to VM workers");
    assert_eq!(job_types.len(), 2);
}

/// Test default configuration.
#[test]
fn test_default_config() {
    let config = LocalExecutorWorkerConfig::default();

    assert_eq!(config.workspace_dir, std::path::PathBuf::from("/workspace"));
    assert!(config.should_cleanup_workspaces);
}

/// Test worker with blob store.
#[tokio::test]
async fn test_worker_with_blob_store() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = create_test_worker_with_blobs(&temp_dir, blob_store);

    let payload = create_shell_payload("echo", vec!["test"]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "execution with blob store should succeed");
}

// =============================================================================
// Artifact Collection Tests
// =============================================================================

/// Test artifact glob pattern collection.
#[tokio::test]
async fn test_artifact_collection() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let blob_store = Arc::new(InMemoryBlobStore::new());
    let worker = create_test_worker_with_blobs(&temp_dir, blob_store.clone());

    let mut payload = create_shell_payload("sh", vec!["-c", "echo 'artifact content' > artifact.txt"]);
    payload.artifacts = vec!["*.txt".to_string()];

    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "artifact collection should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;

    // Check if artifacts were collected
    if let Some(artifacts) = data.get("artifacts") {
        // May have uploaded artifacts
        println!("Artifacts field: {:?}", artifacts);
    }
}

// =============================================================================
// Timeout Tests
// =============================================================================

/// Test timeout with very short timeout.
#[tokio::test]
async fn test_execution_timeout() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let mut payload = create_shell_payload("sleep", vec!["30"]);
    payload.timeout_secs = 1; // 1 second timeout

    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(Duration::from_secs(10), worker.execute(job))
        .await
        .expect("test timeout exceeded");

    // Should fail due to timeout
    match result {
        JobResult::Failure(f) => {
            assert!(
                f.reason.to_lowercase().contains("timeout") || f.reason.to_lowercase().contains("timed out"),
                "should mention timeout: {}",
                f.reason
            );
        }
        _ => panic!("expected timeout failure, got: {:?}", result),
    }
}

// =============================================================================
// Working Directory Tests
// =============================================================================

/// Test execution with pwd shows workspace directory.
#[tokio::test]
async fn test_working_directory_is_in_workspace() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("pwd", vec![]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "pwd should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stdout = extract_stdout(&data);

    // Working directory should be under the temp_dir workspace
    let workspace_prefix = temp_dir.path().to_string_lossy();
    assert!(
        stdout.contains(&*workspace_prefix),
        "working directory should be under workspace: got {} expected prefix {}",
        stdout,
        workspace_prefix
    );
}

/// Test execution can create and use subdirectories.
#[tokio::test]
async fn test_subdirectory_creation_and_use() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    // Create subdir, write file, and read it back - all in one command
    let payload = create_shell_payload("sh", vec![
        "-c",
        "mkdir -p subdir && echo 'in subdir' > subdir/marker.txt && cat subdir/marker.txt",
    ]);
    let job = create_test_job(payload, "shell_command");

    let result = tokio::time::timeout(TEST_TIMEOUT, worker.execute(job)).await.expect("test timeout exceeded");

    assert!(result.is_success(), "subdirectory operations should succeed: {:?}", result);

    let output = match result {
        JobResult::Success(output) => output,
        _ => panic!("expected success"),
    };

    let data: Value = output.data;
    let stdout = extract_stdout(&data);
    assert!(stdout.contains("in subdir"), "should read file from subdir: {}", stdout);
}

// =============================================================================
// Concurrent Execution Tests
// =============================================================================

/// Test multiple concurrent job executions.
#[tokio::test]
async fn test_concurrent_execution() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = Arc::new(create_test_worker(&temp_dir));

    let mut handles = vec![];

    for i in 0..3 {
        let worker = worker.clone();
        let payload = create_shell_payload("sh", vec!["-c", &format!("echo 'job {}'", i)]);
        let job = create_test_job(payload, "shell_command");

        let handle = tokio::spawn(async move { worker.execute(job).await });
        handles.push(handle);
    }

    // Wait for all jobs
    let results: Vec<_> =
        n0_future::join_all(handles).await.into_iter().map(|r| r.expect("task should not panic")).collect();

    // All should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_success(), "job {} should succeed: {:?}", i, result);
    }
}

// ── Log bridge integration tests ────────────────────────────────────────────

/// Minimal in-memory KV store for testing CI log streaming.
///
/// Captures all writes for later assertion. Only the `write` method is needed
/// for the log bridge; read/delete/scan are stub implementations.
mod mock_kv {
    use std::collections::HashMap;
    use std::sync::Arc;

    use aspen_core::DeleteRequest;
    use aspen_core::DeleteResult;
    use aspen_core::KeyValueStore;
    use aspen_core::KeyValueStoreError;
    use aspen_core::{KvDelete, KvRead, KvScan, KvWrite};
    use aspen_core::KeyValueWithRevision;
    use aspen_core::ReadRequest;
    use aspen_core::ReadResult;
    use aspen_core::ScanRequest;
    use aspen_core::ScanResult;
    use aspen_core::WriteCommand;
    use aspen_core::WriteRequest;
    use aspen_core::WriteResult;
    use tokio::sync::RwLock;

    #[derive(Debug, Default, Clone)]
    pub struct MockKvStore {
        pub data: Arc<RwLock<HashMap<String, String>>>,
    }

    impl MockKvStore {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait::async_trait]
    impl KvRead for MockKvStore {
        async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
            let data = self.data.read().await;
            let kv = data.get(&request.key).map(|v| KeyValueWithRevision {
                key: request.key.clone(),
                value: v.clone(),
                version: 1,
                create_revision: 1,
                mod_revision: 1,
            });
            Ok(ReadResult { kv })
        }
    }

    #[async_trait::async_trait]
    impl KvWrite for MockKvStore {
        async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
            let mut data = self.data.write().await;
            match &request.command {
                WriteCommand::Set { key, value } => {
                    data.insert(key.clone(), value.clone());
                }
                _ => {} // Only Set is used by log_bridge
            }
            Ok(WriteResult::default())
        }
    }

    #[async_trait::async_trait]
    impl KvDelete for MockKvStore {
        async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
            let mut data = self.data.write().await;
            let is_deleted = data.remove(&request.key).is_some();
            Ok(DeleteResult {
                key: request.key,
                is_deleted,
            })
        }
    }

    #[async_trait::async_trait]
    impl KvScan for MockKvStore {
        async fn scan(&self, _request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            Ok(ScanResult {
                entries: vec![],
                result_count: 0,
                is_truncated: false,
                continuation_token: None,
            })
        }
    }

    impl KeyValueStore for MockKvStore {}
}

/// Test 3.5: Shell job with mock KV store produces log chunks with correct key format.
///
/// Verifies that when a KV store and run_id are provided, the shell executor
/// spawns the log_bridge and writes `_ci:logs:{run_id}:{job_id}:chunk:{N}` keys.
#[tokio::test]
async fn test_shell_job_with_kv_produces_log_chunks() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let kv = Arc::new(mock_kv::MockKvStore::new());

    let config = LocalExecutorWorkerConfig {
        workspace_dir: temp_dir.path().to_path_buf(),
        should_cleanup_workspaces: true,
        kv_store: Some(kv.clone() as Arc<dyn aspen_core::KeyValueStore>),
        ..Default::default()
    };
    let worker = LocalExecutorWorker::new(config);

    let mut payload = create_shell_payload("echo", vec!["hello from CI log bridge"]);
    payload.run_id = Some("test-run-001".to_string());
    let job = create_test_job(payload, "shell_command");

    let result = worker.execute(job).await;
    assert!(result.is_success(), "job should succeed: {:?}", result);

    // Check KV store for log chunks
    let data = kv.data.read().await;
    let log_keys: Vec<&String> = data.keys().filter(|k| k.starts_with("_ci:logs:")).collect();
    assert!(
        !log_keys.is_empty(),
        "Expected CI log chunk keys in KV store, found none. Keys: {:?}",
        data.keys().collect::<Vec<_>>()
    );

    // Verify key format: _ci:logs:{run_id}:{job_id}:{seq_number}
    for key in &log_keys {
        if !key.contains("__complete__") {
            assert!(
                key.starts_with("_ci:logs:test-run-001:"),
                "Log key should start with _ci:logs:test-run-001: but got {key}"
            );
        }
    }

    // Verify a complete marker exists: _ci:logs:{run_id}:{job_id}:__complete__
    let has_complete = data.keys().any(|k| k.contains("__complete__"));
    assert!(has_complete, "Expected __complete__ marker key. Keys: {:?}", data.keys().collect::<Vec<_>>());
}

/// Test 3.6: Shell job without KV store still captures output in JobOutput.
///
/// Verifies that the shell executor works normally when no KV store is configured,
/// and stdout/stderr are still captured in the job result.
#[tokio::test]
async fn test_shell_job_without_kv_captures_output() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let worker = create_test_worker(&temp_dir);

    let payload = create_shell_payload("echo", vec!["captured output"]);
    let job = create_test_job(payload, "shell_command");

    let result = worker.execute(job).await;
    assert!(result.is_success(), "job should succeed: {:?}", result);

    match result {
        JobResult::Success(output) => {
            let data: Value = serde_json::from_value(output.data).expect("output should be JSON");
            let stdout = extract_stdout(&data);
            assert!(stdout.contains("captured output"), "stdout should contain 'captured output', got: {stdout}");
        }
        other => panic!("expected Success, got: {:?}", other),
    }
}
