//! Integration tests for ShellCommandWorker.
//!
//! These tests exercise real shell command execution with:
//! - Capability-based authorization
//! - Process group management
//! - Output capture and streaming
//! - Timeout enforcement
//!
//! Requires the `shell-worker` feature to be enabled.

#![cfg(feature = "shell-worker")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aspen_auth::Capability;
use aspen_auth::TokenBuilder;
use aspen_auth::TokenVerifier;
use aspen_jobs::Job;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::Worker;
use aspen_jobs::workers::ShellCommandWorker;
use aspen_jobs::workers::shell_command::ShellCommandPayload;
use aspen_jobs::workers::shell_command::ShellCommandWorkerConfig;

/// Create a test worker with default configuration.
fn create_test_worker() -> (ShellCommandWorker, iroh::SecretKey) {
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let token_verifier = Arc::new(TokenVerifier::new());

    let config = ShellCommandWorkerConfig {
        node_id: 1,
        token_verifier,
        blob_store: None,
        default_working_dir: std::env::temp_dir(),
    };

    (ShellCommandWorker::new(config), secret_key)
}

/// Get an environment map with proper PATH for Nix environments.
fn get_test_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    // Preserve PATH from the test environment (important for NixOS)
    if let Ok(path) = std::env::var("PATH") {
        env.insert("PATH".to_string(), path);
    }
    env
}

/// Create a valid capability token for shell execution.
fn create_shell_token(secret_key: &iroh::SecretKey, command_pattern: &str, working_dir: Option<&str>) -> String {
    let cap = Capability::ShellExecute {
        command_pattern: command_pattern.to_string(),
        working_dir: working_dir.map(|s| s.to_string()),
    };

    let token = TokenBuilder::new(secret_key.clone())
        .with_capability(cap)
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    token.to_base64().expect("should encode token")
}

/// Create a job from a payload.
fn create_job(payload: ShellCommandPayload) -> Job {
    let spec = JobSpec::new("shell_command")
        .payload(serde_json::to_value(&payload).expect("should serialize payload"))
        .expect("should create spec");

    Job::from_spec(spec)
}

// ============================================================================
// Basic Execution Tests
// ============================================================================

#[tokio::test]
async fn test_execute_echo_command() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "echo".to_string(),
        args: vec!["Hello, World!".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "echo", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            assert_eq!(output.data.get("exit_code").and_then(|v| v.as_i64()), Some(0));
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            assert!(stdout.contains("Hello, World!"), "stdout should contain message: {}", stdout);
        }
        JobResult::Failure(f) => panic!("Expected success, got failure: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_execute_sh_command() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo test && echo stderr >&2".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            assert_eq!(output.data.get("exit_code").and_then(|v| v.as_i64()), Some(0));
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            let stderr = output.data.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
            assert!(stdout.contains("test"), "stdout should contain 'test': {}", stdout);
            assert!(stderr.contains("stderr"), "stderr should contain 'stderr': {}", stderr);
        }
        JobResult::Failure(f) => panic!("Expected success, got failure: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_execute_with_environment_variables() {
    let (worker, secret_key) = create_test_worker();

    let mut env = get_test_env();
    env.insert("TEST_VAR".to_string(), "test_value".to_string());

    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
        env,
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            assert!(stdout.contains("test_value"), "stdout should contain env value: {}", stdout);
        }
        JobResult::Failure(f) => panic!("Expected success, got failure: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_execute_with_working_directory() {
    let (worker, secret_key) = create_test_worker();

    let temp_dir = std::env::temp_dir();
    let temp_dir_str = temp_dir.to_string_lossy().to_string();

    let payload = ShellCommandPayload {
        command: "pwd".to_string(),
        args: vec![],
        env: get_test_env(),
        working_dir: Some(temp_dir.clone()),
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "pwd", Some(&temp_dir_str))),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            // Canonicalize both paths for comparison
            let expected = temp_dir.canonicalize().unwrap_or(temp_dir);
            let actual_path = PathBuf::from(stdout.trim());
            let actual = actual_path.canonicalize().unwrap_or(actual_path);
            assert_eq!(actual, expected, "pwd should return working directory");
        }
        JobResult::Failure(f) => panic!("Expected success, got failure: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_reject_missing_token() {
    let (worker, _secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "echo".to_string(),
        args: vec!["test".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: None, // No token!
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(f.reason.contains("auth_token"), "should mention missing token: {}", f.reason);
            assert!(!f.is_retryable, "auth failure should not be retryable");
        }
        JobResult::Success(_) => panic!("Expected failure for missing token"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_reject_unauthorized_command() {
    let (worker, secret_key) = create_test_worker();

    // Token only allows "echo" but we try to run "ls"
    let payload = ShellCommandPayload {
        command: "ls".to_string(),
        args: vec![],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "echo", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(
                f.reason.contains("ShellExecute") || f.reason.contains("capability"),
                "should mention capability: {}",
                f.reason
            );
        }
        JobResult::Success(_) => panic!("Expected failure for unauthorized command"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_glob_pattern_authorization() {
    let (worker, secret_key) = create_test_worker();

    // Token allows "echo*" pattern - should match "echo"
    let cap = Capability::ShellExecute {
        command_pattern: "echo*".to_string(),
        working_dir: None,
    };

    let token = TokenBuilder::new(secret_key.clone())
        .with_capability(cap)
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    let payload = ShellCommandPayload {
        command: "echo".to_string(),
        args: vec!["glob test".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(token.to_base64().expect("encode")),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            assert!(stdout.contains("glob test"), "command should execute: {}", stdout);
        }
        JobResult::Failure(f) => panic!("Expected success with glob pattern: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_wildcard_authorization() {
    let (worker, secret_key) = create_test_worker();

    // Token allows "*" - any command
    let cap = Capability::ShellExecute {
        command_pattern: "*".to_string(),
        working_dir: None,
    };

    let token = TokenBuilder::new(secret_key.clone())
        .with_capability(cap)
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    let payload = ShellCommandPayload {
        command: "echo".to_string(),
        args: vec!["wildcard".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(token.to_base64().expect("encode")),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    assert!(matches!(result, JobResult::Success(_)), "wildcard should allow any command");
}

// ============================================================================
// Exit Code Tests
// ============================================================================

#[tokio::test]
async fn test_nonzero_exit_code_is_failure() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "exit 42".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(f.reason.contains("42"), "should include exit code: {}", f.reason);
            assert!(f.is_retryable, "non-fatal exit codes should be retryable");
            assert_eq!(f.error_code.as_deref(), Some("EXIT_42"));
        }
        JobResult::Success(_) => panic!("Expected failure for non-zero exit"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_command_not_found() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "nonexistent_command_12345".to_string(),
        args: vec![],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "*", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    assert!(matches!(result, JobResult::Failure(_)), "should fail for nonexistent command");
}

// ============================================================================
// Timeout Tests
// ============================================================================

#[tokio::test]
async fn test_timeout_kills_process() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "sleep 60".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 1,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    // Create job with short timeout
    let mut spec = JobSpec::new("shell_command")
        .payload(serde_json::to_value(&payload).expect("serialize"))
        .expect("create spec");
    spec.config.timeout = Some(Duration::from_millis(500));

    let job = Job::from_spec(spec);
    let start = std::time::Instant::now();
    let result = worker.execute(job).await;
    let elapsed = start.elapsed();

    // Should complete quickly (timeout + grace period), not wait 60 seconds
    assert!(elapsed < Duration::from_secs(5), "should timeout quickly, took {:?}", elapsed);

    match result {
        JobResult::Failure(f) => {
            assert!(
                f.reason.contains("timeout") || f.reason.contains("timed out"),
                "should mention timeout: {}",
                f.reason
            );
            assert!(f.is_retryable, "timeout should be retryable");
        }
        JobResult::Success(_) => panic!("Expected timeout failure"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

// ============================================================================
// Output Handling Tests
// ============================================================================

#[tokio::test]
async fn test_large_stdout_truncation() {
    let (worker, secret_key) = create_test_worker();

    // Generate output larger than max
    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "yes | head -n 10000".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024, // Small limit
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            // Should indicate truncation happened
            let truncated = output.data.get("stdout_truncated").and_then(|v| v.as_bool()).unwrap_or(false);
            let total = output.data.get("stdout_total_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
            assert!(
                truncated || total > 1024,
                "should truncate large output: truncated={}, total={}",
                truncated,
                total
            );
        }
        JobResult::Failure(f) => panic!("Expected success: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_binary_output_handling() {
    let (worker, secret_key) = create_test_worker();

    // Output some binary data (null bytes)
    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "printf '\\x00\\x01\\x02'".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    // Should handle binary output without crashing
    assert!(matches!(result, JobResult::Success(_)), "should handle binary output");
}

// ============================================================================
// Input Validation Tests
// ============================================================================

#[tokio::test]
async fn test_reject_empty_command() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "".to_string(), // Empty!
        args: vec![],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "*", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(f.reason.contains("Command length"), "should reject empty command: {}", f.reason);
        }
        JobResult::Success(_) => panic!("Expected failure for empty command"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_reject_too_many_args() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "echo".to_string(),
        args: (0..150).map(|i| format!("arg{}", i)).collect(), // Over limit
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "echo", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(f.reason.contains("Too many arguments"), "should reject too many args: {}", f.reason);
        }
        JobResult::Success(_) => panic!("Expected failure for too many args"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_reject_nonexistent_working_dir() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "pwd".to_string(),
        args: vec![],
        env: get_test_env(),
        working_dir: Some(PathBuf::from("/nonexistent/directory/12345")),
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "pwd", Some("/nonexistent"))),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Failure(f) => {
            assert!(
                f.reason.contains("Working directory") || f.reason.contains("does not exist"),
                "should reject nonexistent dir: {}",
                f.reason
            );
        }
        JobResult::Success(_) => panic!("Expected failure for nonexistent directory"),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

// ============================================================================
// Worker Trait Tests
// ============================================================================

#[tokio::test]
async fn test_worker_job_types() {
    let (worker, _secret_key) = create_test_worker();

    let job_types = worker.job_types();
    assert_eq!(job_types, vec!["shell_command"]);
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[tokio::test]
async fn test_git_version_command() {
    let (worker, secret_key) = create_test_worker();

    // Skip if git not installed
    if std::process::Command::new("git").arg("--version").output().is_err() {
        eprintln!("Skipping test: git not installed");
        return;
    }

    let payload = ShellCommandPayload {
        command: "git".to_string(),
        args: vec!["--version".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "git", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            assert!(stdout.contains("git version"), "should show git version: {}", stdout);
        }
        JobResult::Failure(f) => panic!("Expected success: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}

#[tokio::test]
async fn test_pipeline_command() {
    let (worker, secret_key) = create_test_worker();

    let payload = ShellCommandPayload {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo 'line1\nline2\nline3' | wc -l".to_string()],
        env: get_test_env(),
        working_dir: None,
        capture_stderr: true,
        max_output_bytes: 1024,
        graceful_shutdown_secs: 5,
        auth_token: Some(create_shell_token(&secret_key, "sh", None)),
    };

    let job = create_job(payload);
    let result = worker.execute(job).await;

    match result {
        JobResult::Success(output) => {
            let stdout = output.data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            assert!(stdout.trim() == "3", "should count 3 lines: '{}'", stdout.trim());
        }
        JobResult::Failure(f) => panic!("Expected success: {}", f.reason),
        JobResult::Cancelled => panic!("Unexpected cancellation"),
    }
}
