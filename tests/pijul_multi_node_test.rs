//! Multi-node Pijul integration tests.
//!
//! Tests P2P synchronization, conflict detection, and network partition handling
//! across multiple Aspen nodes with real Iroh networking.
//!
//! NOTE: These tests are currently disabled due to circular dependency issues.
//! The PijulMultiNodeTester type needs access to aspen::node::Node which creates
//! a circular dependency (aspen-testing -> aspen -> aspen-testing).
//!
//! TODO: Either:
//! 1. Move PijulMultiNodeTester into this file (inline the test harness)
//! 2. Move the Node/NodeBuilder types to aspen-cluster
//! 3. Create a separate pijul-testing crate that can depend on the main aspen crate
//!
//! Previous run command (when tests were enabled):
//! `cargo nextest run -E 'test(/pijul_multi_node/)' --run-ignored all`

// Compile-time disable: these tests need PijulMultiNodeTester from aspen-testing
// which requires the 'pijul' feature, but that creates a circular dependency.
// Re-enable once the Node/NodeBuilder types are moved to aspen-cluster.
#![allow(unexpected_cfgs)]
#![cfg(all(feature = "pijul", feature = "__pijul_multi_node_tests_disabled__"))]

use std::time::Duration;

use anyhow::Result;
// NOTE: This import will fail until the circular dependency is resolved
// use aspen_testing::PijulMultiNodeTester;
use tempfile::TempDir;
use tracing::info;

/// Time to wait for gossip discovery between nodes.
const _GOSSIP_SETTLE_TIME: Duration = Duration::from_secs(5);
/// Default sync timeout.
const _SYNC_TIMEOUT: Duration = Duration::from_secs(30);

// All tests below are disabled via the cfg attribute above.
// They will be re-enabled once PijulMultiNodeTester is available.

#[tokio::test]
#[ignore = "disabled: circular dependency - see module docs"]
async fn test_pijul_cluster_setup() -> Result<()> {
    unimplemented!("PijulMultiNodeTester not available due to circular dependency")
}

#[tokio::test]
#[ignore = "disabled: circular dependency - see module docs"]
async fn test_pijul_create_repo() -> Result<()> {
    unimplemented!("PijulMultiNodeTester not available due to circular dependency")
}

#[tokio::test]
#[ignore = "disabled: circular dependency - see module docs"]
async fn test_basic_two_node_sync() -> Result<()> {
    unimplemented!("PijulMultiNodeTester not available due to circular dependency")
}

#[tokio::test]
#[ignore = "disabled: circular dependency - see module docs"]
async fn test_change_chain_dependencies() -> Result<()> {
    unimplemented!("PijulMultiNodeTester not available due to circular dependency")
}

#[tokio::test]
#[ignore = "disabled: circular dependency - see module docs"]
async fn test_three_node_gossip_propagation() -> Result<()> {
    unimplemented!("PijulMultiNodeTester not available due to circular dependency")
}

#[tokio::test]
#[ignore = "disabled: circular dependency - see module docs"]
async fn test_request_deduplication() -> Result<()> {
    unimplemented!("PijulMultiNodeTester not available due to circular dependency")
}
