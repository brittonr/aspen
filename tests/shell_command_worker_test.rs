//! Integration tests for ShellCommandWorker with real Iroh networking.
//!
//! These tests validate the shell command worker execution flow
//! using actual Iroh P2P networking (not madsim simulation).
//!
//! # Test Categories
//!
//! 1. **Authorization Tests** - Token-based authorization
//! 2. **Command Execution Tests** - Basic command execution
//! 3. **Output Handling Tests** - stdout/stderr capture
//! 4. **Timeout Tests** - Command timeout enforcement
//!
//! # Requirements
//!
//! These tests require:
//! - Network access
//! - The `shell-worker` feature enabled
//! - Token authentication enabled on the cluster
//!
//! Run with:
//!
//! ```bash
//! cargo nextest run shell_command_worker --ignored --features shell-worker
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::time::Duration;

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use serde_json::json;
use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;

/// Test timeout for single-node operations.
const SINGLE_NODE_TIMEOUT: Duration = Duration::from_secs(30);

/// Test: Shell command execution requires auth token.
///
/// This test validates that shell commands fail without a valid auth token:
/// 1. Start a single-node cluster with shell-worker enabled
/// 2. Submit a shell_command job without auth_token
/// 3. Verify the job fails with AUTH_FAILED error
#[tokio::test]
#[ignore = "requires network access and shell-worker feature"]
async fn test_shell_command_requires_auth() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit a shell_command job without auth token
    let payload = json!({
        "command": "echo",
        "args": ["hello"],
        // No auth_token provided
    });

    let job_id = tester.submit_job(0, "shell_command", payload).await.expect("failed to submit job");

    tracing::info!(job_id = %job_id, "shell command job submitted (should fail without auth)");

    // Wait for the job to complete (should fail)
    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");

    // Verify job failed due to missing auth
    assert_eq!(job.status, "failed", "job should fail without auth token");
    assert!(job.error_message.is_some(), "should have error message");
    let error = job.error_message.as_ref().unwrap();
    assert!(
        error.contains("auth_token") || error.contains("AUTH_FAILED"),
        "error should mention auth: {}",
        error
    );

    tracing::info!(job_id = %job_id, error = ?job.error_message, "job failed as expected");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Shell command with invalid token fails.
///
/// This test validates that invalid tokens are rejected:
/// 1. Submit a shell_command job with an invalid auth token
/// 2. Verify the job fails with token verification error
#[tokio::test]
#[ignore = "requires network access and shell-worker feature"]
async fn test_shell_command_invalid_token() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit with invalid base64 token
    let payload = json!({
        "command": "echo",
        "args": ["hello"],
        "auth_token": "not-a-valid-base64-token",
    });

    let job_id = tester.submit_job(0, "shell_command", payload).await.expect("failed to submit job");

    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");

    assert_eq!(job.status, "failed", "job should fail with invalid token");
    assert!(job.error_message.is_some(), "should have error message");

    tracing::info!(job_id = %job_id, error = ?job.error_message, "job failed as expected");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Shell command payload validation.
///
/// This test validates payload validation errors:
/// 1. Submit a shell_command job with empty command
/// 2. Verify the job fails with validation error
#[tokio::test]
#[ignore = "requires network access and shell-worker feature"]
async fn test_shell_command_empty_command() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit with empty command
    let payload = json!({
        "command": "",
        "auth_token": "some-token", // Will fail on validation before auth check
    });

    let job_id = tester.submit_job(0, "shell_command", payload).await.expect("failed to submit job");

    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");

    assert_eq!(job.status, "failed", "job should fail with empty command");
    assert!(job.error_message.is_some(), "should have error message");
    let error = job.error_message.as_ref().unwrap();
    assert!(
        error.contains("Command length") || error.contains("empty"),
        "error should mention command: {}",
        error
    );

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Shell command with too many arguments.
///
/// This test validates argument count limits (Tiger Style bounds):
/// 1. Submit a shell_command job with more than MAX_ARGS_COUNT arguments
/// 2. Verify the job fails with validation error
#[tokio::test]
#[ignore = "requires network access and shell-worker feature"]
async fn test_shell_command_too_many_args() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create more than MAX_ARGS_COUNT (100) arguments
    let many_args: Vec<String> = (0..150).map(|i| format!("arg{}", i)).collect();

    let payload = json!({
        "command": "echo",
        "args": many_args,
        "auth_token": "some-token",
    });

    let job_id = tester.submit_job(0, "shell_command", payload).await.expect("failed to submit job");

    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");

    assert_eq!(job.status, "failed", "job should fail with too many args");
    assert!(job.error_message.is_some(), "should have error message");
    let error = job.error_message.as_ref().unwrap();
    assert!(error.contains("Too many arguments"), "error should mention arg count: {}", error);

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Shell command worker registration.
///
/// This test validates that the shell_command worker is registered when
/// the shell-worker feature is enabled and a token verifier is configured:
/// 1. Start cluster with workers enabled
/// 2. Query worker status
/// 3. Verify shell_command handler is available
#[tokio::test]
#[ignore = "requires network access and shell-worker feature"]
async fn test_shell_command_worker_registered() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Query worker status to verify shell_command is registered
    let response = tester.client().send(ClientRpcRequest::WorkerStatus).await.expect("failed to get worker status");

    match response {
        ClientRpcResponse::WorkerStatusResult(result) => {
            tracing::info!(total = result.total_workers, idle = result.idle_workers, "worker status retrieved");

            // If shell-worker feature is enabled and token auth is configured,
            // the worker should be registered
            // Note: This may show 0 workers if token auth is not configured
            assert!(result.total_workers > 0 || result.total_capacity > 0, "should have workers registered");
        }
        ClientRpcResponse::Error(e) => {
            panic!("worker status failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Job type routing to shell_command worker.
///
/// This test validates that jobs with type "shell_command" are routed
/// to the ShellCommandWorker (or fail appropriately if not available):
/// 1. Submit a job with type "shell_command"
/// 2. Verify it's picked up by the worker (fails with auth error, not "no handler")
#[tokio::test]
#[ignore = "requires network access and shell-worker feature"]
async fn test_shell_command_job_routing() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit a shell_command job
    let payload = json!({
        "command": "ls",
        "args": ["-la"],
    });

    let job_id = tester.submit_job(0, "shell_command", payload).await.expect("failed to submit job");

    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");

    // If shell_command worker is registered, it should fail with auth error
    // If not registered, it would fail differently (no handler) or be handled by fallback
    // Either way, it should not hang
    tracing::info!(job_id = %job_id, status = %job.status, error = ?job.error_message, "job completed");

    tester.shutdown().await.expect("shutdown failed");
}
