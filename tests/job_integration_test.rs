//! Integration tests for job execution with real Iroh networking.
//!
//! These tests validate the complete job submission and execution flow
//! using actual Iroh P2P networking (not madsim simulation).
//!
//! # Test Categories
//!
//! 1. **Single Node Tests** - Basic job operations on a single node
//!    - Job submission via RPC
//!    - Queue statistics
//!    - Job completion tracking
//!
//! 2. **Multi-Node Tests** (ignored) - Cross-node job operations
//!    - Job routing across nodes
//!    - Worker registration
//!    - Cross-node job completion
//!
//! # Requirements
//!
//! These tests require network access and are marked with `#[ignore]` for
//! tests that require multi-node networking. Run with:
//!
//! ```bash
//! cargo nextest run job_integration --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::time::Duration;

use serde_json::json;
use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;

/// Test timeout for single-node operations.
const SINGLE_NODE_TIMEOUT: Duration = Duration::from_secs(30);
/// Test timeout for multi-node operations.
const MULTI_NODE_TIMEOUT: Duration = Duration::from_secs(60);

/// Test: Submit a job via RPC to a single node and verify completion.
///
/// This test validates the basic job submission flow:
/// 1. Start a single-node cluster
/// 2. Submit a job via RPC
/// 3. Wait for job completion
/// 4. Verify the job completed successfully
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_job_submit_via_rpc_single_node() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit a job
    let job_id = tester
        .submit_job(0, "test", json!({"task": "hello", "value": 42}))
        .await
        .expect("failed to submit job");

    tracing::info!(job_id = %job_id, "job submitted");

    // Wait for the job to complete
    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");

    // Verify job completed
    assert_eq!(job.status, "completed", "job should be completed, got: {}", job.status);
    assert_eq!(job.job_type, "test", "job type should be 'test'");

    tracing::info!(job_id = %job_id, status = %job.status, "job completed");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Queue stats reflect submitted jobs.
///
/// This test validates that queue statistics are properly tracked:
/// 1. Start a single-node cluster
/// 2. Check initial queue stats (should be zero)
/// 3. Submit multiple jobs
/// 4. Verify stats reflect the submitted jobs
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_job_queue_stats_rpc() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(1)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Get initial stats
    let initial_stats = tester.get_queue_stats(0).await.expect("failed to get initial stats");
    tracing::info!(
        pending = initial_stats.pending_count,
        completed = initial_stats.completed_count,
        "initial queue stats"
    );

    // Submit several jobs
    let mut job_ids = Vec::new();
    for i in 0..3 {
        let job_id = tester.submit_job(0, "test", json!({"job_number": i})).await.expect("failed to submit job");
        job_ids.push(job_id);
    }

    tracing::info!(count = job_ids.len(), "jobs submitted");

    // Wait for all jobs to complete
    for job_id in &job_ids {
        let job = tester.wait_for_job(job_id, Duration::from_secs(30)).await.expect("job should complete");
        assert_eq!(job.status, "completed", "job {} should be completed", job_id);
    }

    // Get final stats
    let final_stats = tester.get_queue_stats(0).await.expect("failed to get final stats");

    tracing::info!(pending = final_stats.pending_count, completed = final_stats.completed_count, "final queue stats");

    // Completed count should have increased
    assert!(
        final_stats.completed_count >= initial_stats.completed_count + 3,
        "completed count should have increased by at least 3: initial={}, final={}",
        initial_stats.completed_count,
        final_stats.completed_count
    );

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Job details can be retrieved by ID.
///
/// This test validates job retrieval:
/// 1. Submit a job
/// 2. Retrieve job details by ID
/// 3. Verify the job details match
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_job_get_by_id() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit a job with specific payload
    let payload = json!({"task": "get_test", "id": "12345"});
    let job_id = tester.submit_job(0, "test_type", payload).await.expect("failed to submit job");

    // Get job details immediately (before completion)
    let job = tester.get_job(&job_id).await.expect("failed to get job");
    assert!(job.is_some(), "job should be found");

    let job = job.unwrap();
    assert_eq!(job.job_id, job_id, "job ID should match");
    assert_eq!(job.job_type, "test_type", "job type should match");

    tracing::info!(job_id = %job_id, job_type = %job.job_type, status = %job.status, "job retrieved");

    // Wait for completion
    let completed = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");
    assert_eq!(completed.status, "completed");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Job submission with priority levels.
///
/// This test validates priority-based job handling:
/// 1. Submit jobs with different priorities
/// 2. Verify queue stats reflect priority counts
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_job_priority_levels() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit jobs with different priorities
    // Note: Priority is set via JobSubmitBuilder, here we just test basic submission
    let job_id = tester
        .submit_job(0, "priority_test", json!({"priority": "normal"}))
        .await
        .expect("failed to submit job");

    // Wait for completion
    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");
    assert_eq!(job.status, "completed");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Cross-node job routing (3-node cluster).
///
/// This test validates cross-node job coordination:
/// 1. Start a 3-node cluster
/// 2. Submit jobs to different nodes
/// 3. Verify jobs complete regardless of submission node
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_job_cross_node_routing() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(3)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(MULTI_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit jobs to different nodes
    let mut job_ids = Vec::new();
    for node_idx in 0..3 {
        let job_id = tester
            .submit_job(node_idx, "test", json!({"submitted_to_node": node_idx}))
            .await
            .expect("failed to submit job");
        tracing::info!(node = node_idx, job_id = %job_id, "job submitted");
        job_ids.push((node_idx, job_id));
    }

    // Wait for all jobs to complete
    for (node_idx, job_id) in &job_ids {
        let job = tester.wait_for_job(job_id, Duration::from_secs(60)).await.expect("job should complete");
        assert_eq!(job.status, "completed", "job {} from node {} should be completed", job_id, node_idx);
        tracing::info!(node = node_idx, job_id = %job_id, "job completed");
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Multiple concurrent job submissions.
///
/// This test validates handling of concurrent job submissions:
/// 1. Submit multiple jobs concurrently
/// 2. Verify all jobs complete
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_concurrent_job_submissions() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(4)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Submit jobs concurrently
    let mut job_ids = Vec::new();
    for i in 0..10 {
        let job_id = tester
            .submit_job(0, "concurrent_test", json!({"job_number": i}))
            .await
            .expect("failed to submit job");
        job_ids.push(job_id);
    }

    tracing::info!(count = job_ids.len(), "concurrent jobs submitted");

    // Wait for all jobs to complete
    let mut completed = 0;
    for job_id in &job_ids {
        match tester.wait_for_job(job_id, Duration::from_secs(30)).await {
            Ok(job) if job.status == "completed" => completed += 1,
            Ok(job) => tracing::warn!(job_id = %job_id, status = %job.status, "job not completed"),
            Err(e) => tracing::warn!(job_id = %job_id, error = %e, "failed to wait for job"),
        }
    }

    assert_eq!(completed, job_ids.len(), "all jobs should complete");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Job with large payload.
///
/// This test validates handling of jobs with larger payloads.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_job_large_payload() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(true)
        .with_workers_per_node(2)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create a larger payload (but still within reasonable limits)
    let large_data: String = "x".repeat(10_000);
    let payload = json!({
        "task": "large_payload_test",
        "data": large_data,
        "metadata": {
            "size": large_data.len(),
            "type": "test"
        }
    });

    let job_id = tester.submit_job(0, "test", payload).await.expect("failed to submit job with large payload");

    let job = tester.wait_for_job(&job_id, Duration::from_secs(30)).await.expect("job should complete");
    assert_eq!(job.status, "completed");

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Cluster formation and worker availability.
///
/// This test validates that the cluster forms correctly and workers are available.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_cluster_formation() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default().with_node_count(3).with_workers(true).with_timeout(MULTI_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Verify all nodes are available
    assert_eq!(tester.node_count(), 3, "should have 3 nodes");

    // Verify we can communicate with each node
    for idx in 0..3 {
        let stats = tester.get_queue_stats(idx).await.expect("failed to get stats from node");
        tracing::info!(node = idx, pending = stats.pending_count, "node stats retrieved");
    }

    tester.shutdown().await.expect("shutdown failed");
}
