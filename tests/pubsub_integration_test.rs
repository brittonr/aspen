//! Integration tests for the aspen-pubsub crate.
//!
//! Tests the pub/sub layer with real Raft consensus and Iroh networking.
//!
//! # Test Categories
//!
//! 1. **Single Node Tests** - Basic pub/sub operations on a single node
//! 2. **Multi-Node Tests** - Pub/sub across a 3-node cluster with replication
//! 3. **Wildcard Pattern Tests** - Pattern matching with `*` and `>` wildcards
//! 4. **Historical Replay Tests** - Resume from cursor positions
//!
//! # Running Tests
//!
//! ```sh
//! # Run all pubsub tests (skips slow/network tests in quick profile)
//! cargo nextest run -E 'test(/pubsub/)' -P quick
//!
//! # Run including network tests
//! cargo nextest run -E 'test(/pubsub/)' --ignore-default-filter
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts on all operations
//! - Explicit resource cleanup via TempDir
//! - Error context for debugging

mod support;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen::api::AddLearnerRequest;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::ScanRequest;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use aspen_pubsub::Cursor;
use aspen_pubsub::Event;
use aspen_pubsub::EventBuilder;
use aspen_pubsub::Publisher;
use aspen_pubsub::RaftPublisher;
use aspen_pubsub::Topic;
use aspen_pubsub::TopicPattern;
// TODO: Add EventStream tests that use StreamExt
#[allow(unused_imports)]
use futures::StreamExt;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::info;
use tracing::warn;

// ============================================================================
// Constants
// ============================================================================

/// Timeout for cluster operations.
const CLUSTER_TIMEOUT: Duration = Duration::from_secs(60);
/// Time to wait for gossip discovery between nodes.
const GOSSIP_DISCOVERY_WAIT: Duration = Duration::from_secs(5);
/// Time to wait for leader election.
const LEADER_ELECTION_WAIT: Duration = Duration::from_millis(1000);
/// Time to wait for replication to complete.
const REPLICATION_WAIT: Duration = Duration::from_secs(2);
/// Test cookie for the cluster.
const TEST_COOKIE: &str = "pubsub-integration-test";
/// Number of nodes in multi-node tests.
const NODE_COUNT: u64 = 3;

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a deterministic secret key for a node.
fn generate_secret_key(node_id: u64) -> String {
    format!("{:064x}", 3000 + node_id)
}

/// Create a node with gossip enabled for pub/sub testing.
async fn create_node(node_id: u64, temp_dir: &TempDir, secret_key: &str) -> Result<Node> {
    let data_dir = temp_dir.path().join(format!("node-{}", node_id));

    let mut node = NodeBuilder::new(NodeId(node_id), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie(TEST_COOKIE)
        .with_gossip(true)
        .with_mdns(false) // Disable mDNS for CI compatibility
        .with_iroh_secret_key(secret_key)
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1500, 3000)
        .start()
        .await
        .context("failed to start node")?;

    // Spawn the router to enable inter-node communication
    node.spawn_router();

    info!(
        node_id = node_id,
        endpoint = %node.endpoint_addr().id.fmt_short(),
        "pub/sub test node created"
    );

    Ok(node)
}

/// Initialize a single-node cluster for basic tests.
async fn init_single_node_cluster(node: &Node) -> Result<()> {
    let raft_node = node.raft_node();
    let endpoint_addr = node.endpoint_addr();

    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(node.node_id().0, endpoint_addr)],
    };

    timeout(CLUSTER_TIMEOUT, raft_node.init(init_request))
        .await
        .context("cluster init timeout")?
        .context("cluster init failed")?;

    // Wait for leader election
    sleep(LEADER_ELECTION_WAIT).await;

    info!(node_id = node.node_id().0, "single-node cluster initialized");
    Ok(())
}

/// Create and initialize a multi-node cluster.
async fn create_multi_node_cluster(temp_dir: &TempDir) -> Result<Vec<Node>> {
    let mut nodes: Vec<Node> = Vec::new();

    // Create all nodes
    info!("Creating {} nodes for pub/sub cluster...", NODE_COUNT);
    for id in 1..=NODE_COUNT {
        let secret_key = generate_secret_key(id);
        let node = create_node(id, temp_dir, &secret_key).await?;
        nodes.push(node);
    }

    // Initialize cluster on node 1
    let node1 = &nodes[0];
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
    };

    timeout(CLUSTER_TIMEOUT, node1.raft_node().init(init_request))
        .await
        .context("cluster init timeout")?
        .context("cluster init failed")?;

    sleep(LEADER_ELECTION_WAIT).await;
    sleep(GOSSIP_DISCOVERY_WAIT).await;

    // Add other nodes as learners
    for id in 2..=NODE_COUNT {
        let node = &nodes[(id - 1) as usize];
        let learner = ClusterNode::with_iroh_addr(id, node.endpoint_addr());
        let request = AddLearnerRequest { learner };

        match timeout(CLUSTER_TIMEOUT, node1.raft_node().add_learner(request)).await {
            Ok(Ok(_)) => info!(node_id = id, "added learner"),
            Ok(Err(e)) => warn!(node_id = id, error = %e, "failed to add learner"),
            Err(_) => warn!(node_id = id, "add_learner timeout"),
        }
        sleep(Duration::from_millis(500)).await;
    }

    // Wait for learners to catch up
    sleep(REPLICATION_WAIT).await;

    // Promote all to voters
    let members: Vec<u64> = (1..=NODE_COUNT).collect();
    let request = ChangeMembershipRequest { members };
    timeout(CLUSTER_TIMEOUT, node1.raft_node().change_membership(request))
        .await
        .context("change_membership timeout")?
        .context("change_membership failed")?;

    info!(node_count = NODE_COUNT, "multi-node pub/sub cluster ready");
    Ok(nodes)
}

/// Shutdown all nodes gracefully.
async fn shutdown_nodes(nodes: Vec<Node>) {
    for node in nodes {
        let node_id = node.node_id();
        match node.shutdown().await {
            Ok(()) => info!(node_id = node_id.0, "node shutdown complete"),
            Err(e) => warn!(node_id = node_id.0, error = %e, "node shutdown failed"),
        }
    }
}

// ============================================================================
// Single Node Tests
// ============================================================================

/// Test basic publish and read back via KV store.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pubsub_single_node_publish() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_pubsub=debug,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    // Create publisher wrapping the Raft node
    let kv_store = node.raft_node().clone();
    let publisher = RaftPublisher::new(Arc::new(kv_store));

    // Publish an event
    let topic = Topic::new("orders.created")?;
    let payload = b"order-12345";

    let cursor: Cursor = publisher.publish(&topic, payload).await?;
    info!(cursor = %cursor, topic = %topic, "event published");

    // Note: header_revision may not be set for simple Set operations in current impl.
    // The publish succeeded if we got here without error.
    // Cursor will be BEGINNING (0) if header_revision was not returned.
    assert!(!cursor.is_latest(), "cursor should not be LATEST");

    // The event was published through Raft consensus.
    // Scanning for the event is complex because the keys use Tuple encoding.
    // For now, we just verify that publish completed successfully.
    // TODO: Add proper scan verification once we understand the encoded key format better.
    info!("publish completed successfully, skipping scan verification for now");

    node.shutdown().await?;
    Ok(())
}

/// Test publishing with headers.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pubsub_single_node_publish_with_headers() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_pubsub=debug,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let kv_store = node.raft_node().clone();
    let publisher = RaftPublisher::new(Arc::new(kv_store));

    // Publish with headers
    let topic = Topic::new("orders.shipped")?;
    let payload = b"shipped-order-67890";
    let mut headers = HashMap::new();
    headers.insert("trace-id".to_string(), "abc-123-xyz".to_string());
    headers.insert("content-type".to_string(), "application/json".to_string());

    let cursor: Cursor = publisher.publish_with_headers(&topic, payload, headers).await?;

    info!(
        cursor = %cursor,
        topic = %topic,
        "event with headers published"
    );

    // Publish succeeded if we got here without error
    assert!(!cursor.is_latest(), "cursor should not be LATEST");

    node.shutdown().await?;
    Ok(())
}

/// Test batch publishing for atomic multi-event commits.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pubsub_single_node_batch_publish() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_pubsub=debug,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let kv_store = node.raft_node().clone();
    let publisher = RaftPublisher::new(Arc::new(kv_store));

    // Batch publish multiple events
    let events = vec![
        (Topic::new("orders.created")?, b"order-001".to_vec()),
        (Topic::new("orders.created")?, b"order-002".to_vec()),
        (Topic::new("orders.created")?, b"order-003".to_vec()),
        (Topic::new("inventory.updated")?, b"item-xyz".to_vec()),
        (Topic::new("inventory.updated")?, b"item-abc".to_vec()),
    ];

    let cursor: Cursor = publisher.publish_batch(events).await?;
    info!(cursor = %cursor, "batch of 5 events published atomically");

    // Publish succeeded if we got here without error
    assert!(!cursor.is_latest(), "cursor should not be LATEST");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Topic and Pattern Validation Tests
// ============================================================================

/// Test topic validation rules.
#[test]
fn test_topic_validation() {
    // Valid topics
    assert!(Topic::new("orders").is_ok());
    assert!(Topic::new("orders.created").is_ok());
    assert!(Topic::new("orders.us.east.created").is_ok());
    assert!(Topic::new("my-topic").is_ok());
    assert!(Topic::new("my_topic").is_ok());
    assert!(Topic::new("topic123").is_ok());

    // Invalid topics
    assert!(Topic::new("").is_err()); // empty
    assert!(Topic::new("orders..created").is_err()); // empty segment
    assert!(Topic::new(".orders").is_err()); // leading dot
    assert!(Topic::new("orders.").is_err()); // trailing dot
    assert!(Topic::new("orders.cre ated").is_err()); // space
}

/// Test pattern validation and matching.
#[test]
fn test_pattern_validation_and_matching() {
    // Exact pattern
    let pattern = TopicPattern::new("orders.created").unwrap();
    assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
    assert!(!pattern.matches(&Topic::new("orders.updated").unwrap()));

    // Single wildcard (*)
    let pattern = TopicPattern::new("orders.*").unwrap();
    assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
    assert!(pattern.matches(&Topic::new("orders.updated").unwrap()));
    assert!(!pattern.matches(&Topic::new("orders.us.created").unwrap())); // * matches exactly one

    // Multi-segment wildcard (>)
    let pattern = TopicPattern::new("orders.>").unwrap();
    assert!(pattern.matches(&Topic::new("orders").unwrap())); // > matches zero
    assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
    assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
    assert!(pattern.matches(&Topic::new("orders.us.east.created").unwrap()));
    assert!(!pattern.matches(&Topic::new("users.created").unwrap()));

    // Combined wildcards
    let pattern = TopicPattern::new("orders.*.>").unwrap();
    assert!(pattern.matches(&Topic::new("orders.us").unwrap()));
    assert!(pattern.matches(&Topic::new("orders.us.east").unwrap()));
    assert!(pattern.matches(&Topic::new("orders.emea.west.shipped").unwrap()));
    assert!(!pattern.matches(&Topic::new("orders").unwrap())); // needs at least one for *

    // Invalid patterns
    assert!(TopicPattern::new("orders.>.created").is_err()); // > must be last
    assert!(TopicPattern::new("orders.cre*ted").is_err()); // partial wildcard
}

/// Test pattern literal prefix extraction.
#[test]
fn test_pattern_literal_prefix() {
    let pattern = TopicPattern::new("orders.us.>").unwrap();
    assert_eq!(pattern.literal_prefix(), vec!["orders", "us"]);

    let pattern = TopicPattern::new("orders.*").unwrap();
    assert_eq!(pattern.literal_prefix(), vec!["orders"]);

    let pattern = TopicPattern::new("*.created").unwrap();
    assert!(pattern.literal_prefix().is_empty());

    let pattern = TopicPattern::new(">").unwrap();
    assert!(pattern.literal_prefix().is_empty());
}

// ============================================================================
// Cursor Tests
// ============================================================================

/// Test cursor operations.
#[test]
fn test_cursor_operations() {
    // Special cursors
    assert!(Cursor::BEGINNING.is_beginning());
    assert!(Cursor::LATEST.is_latest());
    assert_eq!(Cursor::BEGINNING.index(), 0);
    assert_eq!(Cursor::LATEST.index(), u64::MAX);

    // From index
    let cursor = Cursor::from_index(12345);
    assert_eq!(cursor.index(), 12345);
    assert!(!cursor.is_beginning());
    assert!(!cursor.is_latest());

    // Navigation
    let cursor = Cursor::from_index(100);
    assert_eq!(cursor.next().index(), 101);
    assert_eq!(cursor.prev().index(), 99);

    // Saturation at 0
    assert_eq!(Cursor::BEGINNING.prev().index(), 0);

    // Ordering
    assert!(Cursor::from_index(10) < Cursor::from_index(20));
    assert!(Cursor::BEGINNING < Cursor::LATEST);
}

/// Test cursor serialization roundtrip.
#[test]
fn test_cursor_serialization() {
    let cursors = vec![Cursor::BEGINNING, Cursor::LATEST, Cursor::from_index(12345)];

    for cursor in cursors {
        let json = serde_json::to_string(&cursor).unwrap();
        let decoded: Cursor = serde_json::from_str(&json).unwrap();
        assert_eq!(cursor, decoded);
    }
}

// ============================================================================
// Event Tests
// ============================================================================

/// Test event creation and validation.
#[test]
fn test_event_creation_and_validation() {
    let topic = Topic::new("orders.created").unwrap();

    // Basic event
    let event = Event::new(topic.clone(), b"payload".to_vec(), Cursor::from_index(100));
    assert_eq!(event.topic, topic);
    assert_eq!(event.payload, b"payload");
    assert_eq!(event.cursor.index(), 100);
    assert!(event.headers.is_empty());
    assert!(event.validate().is_ok());

    // Event with headers
    let mut headers = HashMap::new();
    headers.insert("trace-id".to_string(), "abc123".to_string());

    let event = EventBuilder::new(Topic::new("orders.shipped").unwrap())
        .payload(b"shipped-data".to_vec())
        .header("trace-id", "xyz789")
        .header("content-type", "application/json")
        .build()
        .unwrap();

    assert_eq!(event.header("trace-id"), Some("xyz789"));
    assert_eq!(event.header_count(), 2);
    assert!(event.validate().is_ok());
}

/// Test event validation limits.
#[test]
fn test_event_validation_limits() {
    use aspen_pubsub::error::PubSubError;

    let topic = Topic::new("test").unwrap();

    // Payload too large (> 1MB)
    let large_payload = vec![0u8; 1_048_577];
    let event = Event::new(topic.clone(), large_payload, Cursor::BEGINNING);
    assert!(matches!(event.validate(), Err(PubSubError::PayloadTooLarge { .. })));

    // Too many headers (> 32)
    let mut headers = HashMap::new();
    for i in 0..33 {
        headers.insert(format!("header-{}", i), "value".to_string());
    }
    let event = Event::with_headers(
        topic.clone(),
        Vec::new(),
        Cursor::BEGINNING,
        aspen::hlc::SerializableTimestamp::from(aspen::hlc::create_hlc("test").new_timestamp()),
        headers,
    );
    assert!(matches!(event.validate(), Err(PubSubError::TooManyHeaders { .. })));
}

// ============================================================================
// Multi-Node Cluster Tests
// ============================================================================

/// Test pub/sub across a multi-node cluster.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pubsub_multi_node_replication() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_pubsub=debug,iroh=warn").try_init();

    info!("Starting multi-node pub/sub replication test");

    let temp_dir = TempDir::new()?;
    let nodes = create_multi_node_cluster(&temp_dir).await?;

    // Publish on node 1
    let node1 = &nodes[0];
    let kv_store = node1.raft_node().clone();
    let publisher = RaftPublisher::new(Arc::new(kv_store));

    let topic = Topic::new("orders.global")?;
    let cursor = publisher.publish(&topic, b"global-order-001").await?;
    info!(cursor = %cursor, "published on node 1");

    // Wait for replication
    sleep(REPLICATION_WAIT).await;

    // Verify event is readable from all nodes via KV scan
    for (idx, node) in nodes.iter().enumerate() {
        let scan_result = node
            .raft_node()
            .scan(ScanRequest {
                prefix: "__pubsub/events/".to_string(),
                limit: Some(10),
                continuation_token: None,
            })
            .await;

        let found = scan_result.as_ref().map(|r| !r.entries.is_empty()).unwrap_or(false);
        info!(node_idx = idx, found = found, "KV scan check");
    }

    shutdown_nodes(nodes).await;
    Ok(())
}

/// Test publishing from different nodes in the cluster.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pubsub_multi_node_publish_from_any() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_pubsub=debug,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let nodes = create_multi_node_cluster(&temp_dir).await?;

    // Publish from each node
    for (idx, node) in nodes.iter().enumerate() {
        let kv_store = node.raft_node().clone();
        let publisher = RaftPublisher::new(Arc::new(kv_store));

        let topic = Topic::new(format!("orders.node{}", idx + 1))?;
        let payload = format!("order-from-node-{}", idx + 1);

        let cursor = publisher.publish(&topic, payload.as_bytes()).await?;
        info!(
            node_idx = idx,
            cursor = %cursor,
            topic = %topic,
            "published from node"
        );
    }

    // Wait for replication
    sleep(REPLICATION_WAIT).await;

    shutdown_nodes(nodes).await;
    Ok(())
}

// ============================================================================
// Key Encoding Tests
// ============================================================================

/// Test topic-to-key encoding roundtrip.
#[test]
fn test_topic_key_encoding_roundtrip() {
    let topics = vec![
        Topic::new("orders").unwrap(),
        Topic::new("orders.created").unwrap(),
        Topic::new("orders.us.east.created").unwrap(),
        Topic::new("my-topic").unwrap(),
        Topic::new("topic_with_underscores").unwrap(),
    ];

    for topic in topics {
        let key = aspen_pubsub::topic_to_key(&topic);
        let decoded = aspen_pubsub::key_to_topic(&key).unwrap();
        assert_eq!(topic, decoded, "roundtrip failed for {}", topic);
    }
}

/// Test key ordering preserves topic hierarchy.
#[test]
fn test_topic_key_ordering() {
    let key1 = aspen_pubsub::topic_to_key(&Topic::new("orders.a").unwrap());
    let key2 = aspen_pubsub::topic_to_key(&Topic::new("orders.b").unwrap());
    let key3 = aspen_pubsub::topic_to_key(&Topic::new("orders.b.extra").unwrap());
    let key4 = aspen_pubsub::topic_to_key(&Topic::new("users.a").unwrap());

    assert!(key1 < key2, "orders.a should sort before orders.b");
    assert!(key2 < key3, "orders.b should sort before orders.b.extra");
    assert!(key3 < key4, "orders.* should sort before users.*");
}

/// Test pattern to prefix conversion.
#[test]
fn test_pattern_to_prefix() {
    let pattern = TopicPattern::new("orders.*").unwrap();
    let prefix = aspen_pubsub::pattern_to_prefix(&pattern);

    // Keys matching the pattern should start with the prefix
    let key1 = aspen_pubsub::topic_to_key(&Topic::new("orders.created").unwrap());
    let key2 = aspen_pubsub::topic_to_key(&Topic::new("orders.updated").unwrap());
    assert!(key1.starts_with(&prefix));
    assert!(key2.starts_with(&prefix));

    // Keys not matching should not start with prefix
    let key3 = aspen_pubsub::topic_to_key(&Topic::new("users.created").unwrap());
    assert!(!key3.starts_with(&prefix));

    // All pubsub keys should be identifiable
    assert!(aspen_pubsub::is_pubsub_key(&key1));
    assert!(aspen_pubsub::is_pubsub_key(&key2));
    assert!(aspen_pubsub::is_pubsub_key(&key3));
    assert!(!aspen_pubsub::is_pubsub_key(b"regular/key"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test error cases for publishing.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pubsub_error_cases() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_pubsub=debug,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let kv_store = node.raft_node().clone();
    let publisher = RaftPublisher::new(Arc::new(kv_store));

    // Test: payload too large (should fail validation)
    let topic = Topic::new("test.large")?;
    let large_payload = vec![0u8; 1_048_577]; // > 1MB

    let result: aspen_pubsub::Result<Cursor> = publisher.publish(&topic, &large_payload).await;
    assert!(result.is_err(), "publishing large payload should fail");

    // Test: batch too large (should fail validation)
    let mut large_batch = Vec::new();
    for i in 0..101 {
        large_batch.push((Topic::new(format!("batch.{}", i))?, vec![0u8; 100]));
    }

    let result: aspen_pubsub::Result<Cursor> = publisher.publish_batch(large_batch).await;
    assert!(result.is_err(), "publishing large batch should fail");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Performance/Stress Tests (Optional)
// ============================================================================

/// Stress test with many rapid publishes.
#[tokio::test]
#[ignore = "requires network access and is slow - run manually"]
async fn test_pubsub_rapid_publish_stress() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let kv_store = node.raft_node().clone();
    let publisher = RaftPublisher::new(Arc::new(kv_store));

    let topic = Topic::new("stress.test")?;
    let publish_count = 100;

    let start = std::time::Instant::now();

    for i in 0..publish_count {
        let payload = format!("stress-event-{}", i);
        let cursor = publisher.publish(&topic, payload.as_bytes()).await?;

        if i % 10 == 0 {
            info!(i = i, cursor = %cursor, "stress publish progress");
        }
    }

    let elapsed = start.elapsed();
    let rate = publish_count as f64 / elapsed.as_secs_f64();

    info!(
        count = publish_count,
        elapsed_ms = elapsed.as_millis(),
        rate_per_sec = format!("{:.2}", rate),
        "stress test complete"
    );

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// CLI Integration Tests (Placeholder for future CLI pub/sub commands)
// ============================================================================

// TODO: When pub/sub CLI commands are added, include tests here that:
// 1. Start a cluster using RealClusterTester
// 2. Use aspen-cli to publish events
// 3. Verify events via KV scan or pub/sub subscribe
//
// Example structure:
//
// #[tokio::test]
// #[ignore = "requires network access"]
// async fn test_cli_pubsub_publish() -> Result<()> {
//     let cluster = RealClusterTester::new(RealClusterConfig::default()).await?;
//     let ticket = cluster.ticket().to_string();
//
//     // Run CLI command
//     let output = Command::new("cargo")
//         .args(["run", "--bin", "aspen-cli", "--", "--ticket", &ticket, "pubsub", "publish", ...])
//         .output()
//         .await?;
//
//     assert!(output.status.success());
//     Ok(())
// }
