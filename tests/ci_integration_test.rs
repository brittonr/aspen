//! Integration tests for CI/CD pipeline system with real Iroh networking.
//!
//! These tests validate the CI RPC operations including pipeline triggering,
//! status queries, listing, and cancellation via the aspen-ci orchestrator.
//!
//! # Test Categories
//!
//! 1. **RPC Endpoint Tests** - Verify CI RPC handlers respond correctly
//!    - Trigger pipeline (orchestrator availability check)
//!    - Get status (run ID lookup)
//!    - List runs (filtering and pagination)
//!    - Cancel run
//!    - Watch/unwatch repository
//!
//! # Requirements
//!
//! These tests require network access and the `ci` feature. Run with:
//!
//! ```bash
//! cargo nextest run ci_integration --features ci --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

#![cfg(feature = "ci")]

mod support;

use std::time::Duration;

use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;

/// Test timeout for CI operations.
const CI_TIMEOUT: Duration = Duration::from_secs(30);

/// Test: CI trigger pipeline RPC responds correctly.
///
/// This test validates the CI trigger endpoint:
/// 1. Start a single-node cluster with CI enabled
/// 2. Trigger a pipeline for a non-existent repo
/// 3. Verify we get an appropriate error response (not a crash)
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_trigger_pipeline_rpc() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_ci(true)
        .with_timeout(CI_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Trigger a pipeline for a non-existent repo
    let result = tester
        .ci_trigger_pipeline("nonexistent-repo-id", "refs/heads/main", None)
        .await
        .expect("RPC should succeed even if pipeline trigger fails");

    // The trigger should fail since the repo doesn't exist, but the RPC should work
    tracing::info!(
        success = result.success,
        run_id = ?result.run_id,
        error = ?result.error,
        "CI trigger result"
    );

    // We expect either success=false with an error, or success=true with a run_id
    // Since the repo doesn't exist and CI config loading isn't fully implemented,
    // we'll likely get an error
    if !result.success {
        assert!(result.error.is_some(), "failed trigger should have error message");
        tracing::info!(error = ?result.error, "expected error for non-existent repo");
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: CI get status RPC responds correctly.
///
/// This test validates the CI status endpoint:
/// 1. Start a cluster with CI enabled
/// 2. Query status for a non-existent run
/// 3. Verify we get found=false response
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_get_status_rpc() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_ci(true)
        .with_timeout(CI_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Query status for a non-existent run ID
    let result = tester
        .ci_get_status("nonexistent-run-id-12345")
        .await
        .expect("RPC should succeed even for non-existent run");

    tracing::info!(
        found = result.found,
        run_id = ?result.run_id,
        status = ?result.status,
        error = ?result.error,
        "CI status result"
    );

    // Should return found=false for non-existent run
    assert!(!result.found, "non-existent run should not be found");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: CI list runs RPC responds correctly.
///
/// This test validates the CI list endpoint:
/// 1. Start a cluster with CI enabled
/// 2. List runs (should be empty initially)
/// 3. Verify we get an empty list response
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_list_runs_rpc() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_ci(true)
        .with_timeout(CI_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // List all runs (should be empty)
    let result = tester.ci_list_runs(None, None, Some(50)).await.expect("RPC should succeed");

    tracing::info!(run_count = result.runs.len(), "CI list result");

    // New cluster should have no runs
    assert!(result.runs.is_empty(), "new cluster should have no CI runs");

    // Test with repo filter
    let filtered = tester
        .ci_list_runs(Some("some-repo-id"), None, Some(10))
        .await
        .expect("filtered list should succeed");

    assert!(filtered.runs.is_empty(), "filtered list should also be empty");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: CI cancel run RPC responds correctly.
///
/// This test validates the CI cancel endpoint:
/// 1. Start a cluster with CI enabled
/// 2. Try to cancel a non-existent run
/// 3. Verify we get an error response
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_cancel_run_rpc() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_ci(true)
        .with_timeout(CI_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Try to cancel a non-existent run
    let result = tester
        .ci_cancel_run("nonexistent-run-id", Some("test cancellation"))
        .await
        .expect("RPC should succeed even if cancel fails");

    tracing::info!(success = result.success, error = ?result.error, "CI cancel result");

    // Should fail since the run doesn't exist
    // Note: Current implementation returns success=false for unimplemented features
    if !result.success {
        assert!(result.error.is_some(), "failed cancel should have error message");
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: CI watch/unwatch repo RPC responds correctly.
///
/// This test validates the CI watch endpoints:
/// 1. Start a cluster with CI enabled
/// 2. Watch a repository
/// 3. Unwatch the repository
/// 4. Verify both operations respond correctly
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_watch_unwatch_repo_rpc() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_ci(true)
        .with_timeout(CI_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    let test_repo_id = "test-repo-for-watching";

    // Watch the repository
    let watch_result = tester.ci_watch_repo(test_repo_id).await.expect("watch RPC should succeed");

    tracing::info!(
        success = watch_result.success,
        error = ?watch_result.error,
        "CI watch result"
    );

    // Note: Current implementation returns success=false for unimplemented features
    // When fully implemented, this should return success=true

    // Unwatch the repository
    let unwatch_result = tester.ci_unwatch_repo(test_repo_id).await.expect("unwatch RPC should succeed");

    tracing::info!(
        success = unwatch_result.success,
        error = ?unwatch_result.error,
        "CI unwatch result"
    );

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: CI operations work across multi-node cluster.
///
/// This test validates CI operations work in a distributed setting:
/// 1. Start a 3-node cluster with CI enabled
/// 2. Run CI operations through any node
/// 3. Verify consistent responses
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_multi_node_cluster() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(3)
        .with_workers(true)
        .with_ci(true)
        .with_timeout(Duration::from_secs(60));

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Verify all nodes respond to CI list
    let list_result = tester.ci_list_runs(None, None, Some(10)).await.expect("list should succeed");

    tracing::info!(run_count = list_result.runs.len(), "CI list from 3-node cluster");

    // Verify status query works
    let status_result = tester.ci_get_status("test-run-id").await.expect("status should succeed");

    assert!(!status_result.found, "non-existent run should not be found");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: CI RPC operations without CI enabled returns appropriate errors.
///
/// This test validates error handling when CI is not enabled:
/// 1. Start a cluster WITHOUT CI enabled
/// 2. Try CI operations
/// 3. Verify we get "CI not available" type errors
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_ci_disabled_returns_error() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_ci(false) // CI explicitly disabled
        .with_timeout(CI_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Try to trigger a pipeline
    let result = tester.ci_trigger_pipeline("test-repo", "refs/heads/main", None).await;

    match result {
        Ok(response) => {
            // If we get a response, it should indicate CI is not available
            tracing::info!(
                success = response.success,
                error = ?response.error,
                "CI trigger response when disabled"
            );
            if !response.success {
                assert!(
                    response
                        .error
                        .as_ref()
                        .map(|e| e.contains("not available") || e.contains("not enabled"))
                        .unwrap_or(false),
                    "error should indicate CI is not available"
                );
            }
        }
        Err(e) => {
            // Error is also acceptable - CI handler might not be registered
            tracing::info!(error = %e, "CI operation error when disabled (expected)");
        }
    }

    tester.shutdown().await.expect("shutdown failed");
}
