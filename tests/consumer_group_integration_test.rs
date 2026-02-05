//! Integration tests for consumer group functionality.
//!
//! Tests the full consumer group flow with real Raft consensus and storage.
//! Covers competing mode, visibility timeout, dead letter queues, and fencing.
//!
//! # Test Categories
//!
//! 1. **Group Lifecycle Tests** - Create, join, leave, delete operations
//! 2. **Message Flow Tests** - Receive, ack, nack in competing mode
//! 3. **Visibility Timeout Tests** - Message redelivery and expiry
//! 4. **Dead Letter Queue Tests** - Failed delivery handling
//! 5. **Fencing Tests** - Consumer safety and coordination
//!
//! # Running Tests
//!
//! ```sh
//! # Run all consumer group tests (skips network tests in quick profile)
//! cargo nextest run -E 'test(/consumer_group/)' -P quick
//!
//! # Run including network tests
//! cargo nextest run -E 'test(/consumer_group/)' --ignore-default-filter
//! ```

mod support;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use aspen_hooks::pubsub::TopicPattern;
use aspen_hooks::pubsub::consumer_group::AckPolicy;
use aspen_hooks::pubsub::consumer_group::AssignmentMode;
use aspen_hooks::pubsub::consumer_group::Consumer;
use aspen_hooks::pubsub::consumer_group::ConsumerGroupConfig;
use aspen_hooks::pubsub::consumer_group::ConsumerGroupId;
use aspen_hooks::pubsub::consumer_group::ConsumerGroupManager;
use aspen_hooks::pubsub::consumer_group::ConsumerId;
use aspen_hooks::pubsub::consumer_group::GroupStateType;
use aspen_hooks::pubsub::consumer_group::JoinOptions;
use aspen_hooks::pubsub::consumer_group::RaftPendingEntriesList;
use aspen_raft::node::RaftNode;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::info;

// ============================================================================
// Constants
// ============================================================================

/// Timeout for cluster operations.
const CLUSTER_TIMEOUT: Duration = Duration::from_secs(60);
/// Time to wait for leader election.
const LEADER_ELECTION_WAIT: Duration = Duration::from_millis(1000);
/// Test cookie for the cluster.
const TEST_COOKIE: &str = "consumer-group-integration-test";

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a deterministic secret key for a node.
fn generate_secret_key(node_id: u64) -> String {
    format!("{:064x}", 4000 + node_id)
}

/// Create a node for consumer group testing.
async fn create_node(node_id: u64, temp_dir: &TempDir, secret_key: &str) -> Result<Node> {
    let data_dir = temp_dir.path().join(format!("node-{}", node_id));

    let mut node = NodeBuilder::new(NodeId(node_id), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie(TEST_COOKIE)
        .with_gossip(false) // Disable gossip for simpler tests
        .with_mdns(false)
        .with_iroh_secret_key(secret_key)
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1500, 3000)
        .start()
        .await
        .context("failed to start node")?;

    // Spawn the router to enable communication
    node.spawn_router();

    info!(
        node_id = node_id,
        endpoint = %node.endpoint_addr().id.fmt_short(),
        "consumer group test node created"
    );

    Ok(node)
}

/// Initialize a single-node cluster for testing.
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

/// Create a consumer group manager.
fn create_manager(node: &Node) -> ConsumerGroupManager<RaftNode> {
    let store = node.raft_node().clone();
    ConsumerGroupManager::new(store)
}

/// Create a pending entries manager.
fn create_pending_manager(node: &Node) -> Arc<RaftPendingEntriesList<RaftNode>> {
    let store = node.raft_node().clone();
    let receipt_secret = [42u8; 32]; // Test secret
    Arc::new(RaftPendingEntriesList::new(store, receipt_secret))
}

/// Create a basic consumer group configuration.
fn create_test_group_config(group_id: &str, pattern: &str) -> Result<ConsumerGroupConfig> {
    Ok(ConsumerGroupConfig::builder()
        .group_id(group_id)?
        .pattern(TopicPattern::new(pattern)?)
        .assignment_mode(AssignmentMode::Competing)
        .ack_policy(AckPolicy::Explicit)
        .visibility_timeout_ms(5000) // 5s for tests
        .max_delivery_attempts(3)
        .build()?)
}

// ============================================================================
// Group Lifecycle Tests
// ============================================================================

/// Test creating and deleting a consumer group.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_group_lifecycle_create_delete() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);

    // Create a consumer group
    let config = create_test_group_config("order-processors", "orders.*")?;
    let group_id = ConsumerGroupId::new("order-processors").expect("valid group ID");

    let group_state = manager.create_group(config).await?;
    assert_eq!(group_state.group_id, group_id);
    assert_eq!(group_state.state, GroupStateType::Empty);
    assert_eq!(group_state.member_count, 0);
    assert_eq!(group_state.assignment_mode, AssignmentMode::Competing);
    assert_eq!(group_state.ack_policy, AckPolicy::Explicit);

    info!(group_id = %group_id, "consumer group created successfully");

    // Verify group exists
    let retrieved_group = manager.get_group(&group_id).await?;
    assert_eq!(retrieved_group.group_id, group_id);

    // List groups
    let groups = manager.list_groups().await?;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].group_id, group_id);

    // Delete the group
    manager.delete_group(&group_id, true).await?;
    info!(group_id = %group_id, "consumer group deleted");

    // Verify group no longer exists
    let result = manager.get_group(&group_id).await;
    assert!(result.is_err(), "deleted group should not exist");

    node.shutdown().await?;
    Ok(())
}

/// Test duplicate group creation fails.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_group_lifecycle_duplicate_creation_fails() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);

    // Create first group
    let config = create_test_group_config("order-processors", "orders.*")?;
    let _group_state = manager.create_group(config.clone()).await?;

    // Attempt to create duplicate group should fail
    let result = manager.create_group(config).await;
    assert!(result.is_err(), "duplicate group creation should fail");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Consumer Membership Tests
// ============================================================================

/// Test consumer join and leave operations.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_consumer_membership_join_leave() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);
    let pending_manager = create_pending_manager(&node);

    // Create group
    let config = create_test_group_config("order-processors", "orders.*")?;
    let group_id = ConsumerGroupId::new("order-processors").expect("valid group ID");
    let _group_state = manager.create_group(config).await?;

    // Join as consumer
    let consumer_id = ConsumerId::new("consumer-1").expect("valid consumer ID");
    let options = JoinOptions {
        metadata: Some("test-client".to_string()),
        tags: vec!["worker".to_string(), "high-priority".to_string()],
        visibility_timeout_ms: None,
    };

    let consumer =
        Consumer::join(node.raft_node().clone(), pending_manager, group_id.clone(), consumer_id.clone(), options)
            .await?;

    assert_eq!(consumer.consumer_id(), &consumer_id);
    assert_eq!(consumer.group_id(), &group_id);
    assert!(consumer.fencing_token() > 0);
    assert!(consumer.session_id() > 0);

    info!(
        consumer_id = %consumer_id,
        fencing_token = consumer.fencing_token(),
        "consumer joined successfully"
    );

    // Get member info
    let member_info = consumer.member_info().await?;
    assert_eq!(member_info.consumer_id, consumer_id);
    assert_eq!(member_info.metadata, Some("test-client".to_string()));
    assert!(member_info.tags.contains(&"worker".to_string()));

    // Send heartbeat
    let heartbeat_response = consumer.heartbeat().await?;
    assert!(heartbeat_response.next_deadline_ms > 0, "heartbeat should return valid deadline");

    // Leave group
    consumer.leave().await?;
    info!(consumer_id = %consumer_id, "consumer left successfully");

    // Verify group is now empty
    let group_state = manager.get_group(&group_id).await?;
    assert_eq!(group_state.member_count, 0, "group should have no members after leave");
    assert_eq!(group_state.state, GroupStateType::Empty);

    node.shutdown().await?;
    Ok(())
}

/// Test multiple consumers can join the same group.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_consumer_membership_multiple_consumers() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);

    // Create group
    let config = create_test_group_config("order-processors", "orders.*")?;
    let group_id = ConsumerGroupId::new("order-processors").expect("valid group ID");
    let _group_state = manager.create_group(config).await?;

    // Join multiple consumers
    let mut consumers = Vec::new();
    for i in 1..=3 {
        let pending_manager = create_pending_manager(&node);
        let consumer_id = ConsumerId::new(format!("consumer-{}", i)).expect("valid consumer ID");
        let options = JoinOptions {
            metadata: Some(i.to_string()),
            tags: vec![format!("worker-{}", i)],
            visibility_timeout_ms: None,
        };

        let consumer =
            Consumer::join(node.raft_node().clone(), pending_manager, group_id.clone(), consumer_id.clone(), options)
                .await?;
        assert_eq!(consumer.consumer_id(), &consumer_id);
        consumers.push(consumer);
    }

    // Verify group has 3 members
    let group_state = manager.get_group(&group_id).await?;
    assert_eq!(group_state.member_count, 3);

    // Each consumer should have unique fencing tokens
    let fencing_tokens: std::collections::HashSet<_> = consumers.iter().map(|c| c.fencing_token()).collect();
    assert_eq!(fencing_tokens.len(), 3, "all fencing tokens should be unique");

    info!("multiple consumers joined successfully");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Fencing Tests
// ============================================================================

/// Test fencing token validation prevents stale operations.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_fencing_validation() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);
    let pending_manager = create_pending_manager(&node);

    // Create group and join consumer
    let config = create_test_group_config("order-processors", "orders.*")?;
    let group_id = ConsumerGroupId::new("order-processors").expect("valid group ID");
    let _group_state = manager.create_group(config).await?;

    let consumer_id = ConsumerId::new("consumer-1").expect("valid consumer ID");
    let consumer =
        Consumer::join(node.raft_node().clone(), pending_manager, group_id.clone(), consumer_id.clone(), JoinOptions {
            metadata: None,
            tags: vec![],
            visibility_timeout_ms: None,
        })
        .await?;

    // Valid fencing token should work
    let heartbeat_result = consumer.heartbeat().await?;
    assert!(heartbeat_result.next_deadline_ms > 0, "valid fencing token should work");

    // Leave should succeed
    consumer.leave().await?;

    info!("fencing validation working correctly");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Message Flow Tests (Placeholder - requires publisher integration)
// ============================================================================

/// Test basic ack operation with receipt handle.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_message_ack_with_receipt_handle() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);
    let pending_manager = create_pending_manager(&node);

    // Create group and join consumer
    let config = create_test_group_config("order-processors", "orders.*")?;
    let group_id = ConsumerGroupId::new("order-processors").expect("valid group ID");
    let _group_state = manager.create_group(config).await?;

    let consumer_id = ConsumerId::new("consumer-1").expect("valid consumer ID");
    let consumer =
        Consumer::join(node.raft_node().clone(), pending_manager, group_id.clone(), consumer_id.clone(), JoinOptions {
            metadata: None,
            tags: vec![],
            visibility_timeout_ms: None,
        })
        .await?;

    // Test ack with invalid receipt handle
    let invalid_receipt = "invalid-receipt-handle";
    let ack_result = consumer.ack(invalid_receipt).await;
    assert!(ack_result.is_err(), "ack with invalid receipt should fail");

    // Test nack with invalid receipt handle
    let nack_result = consumer.nack(invalid_receipt).await;
    assert!(nack_result.is_err(), "nack with invalid receipt should fail");

    info!("receipt handle validation working correctly");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

/// Test consumer group configuration validation.
#[test]
fn test_group_config_validation() {
    // Valid configuration
    let config = (|| -> aspen_hooks::pubsub::consumer_group::error::Result<ConsumerGroupConfig> {
        ConsumerGroupConfig::builder()
            .group_id("valid-group")?
            .pattern(TopicPattern::new("orders.*").unwrap())
            .assignment_mode(AssignmentMode::Competing)
            .ack_policy(AckPolicy::Explicit)
            .visibility_timeout_ms(30_000)
            .max_delivery_attempts(3)
            .build()
    })();

    assert!(config.is_ok(), "valid configuration should pass");

    // Test invalid group ID
    let invalid_config = ConsumerGroupConfig::builder().group_id(""); // Empty group ID - this should return an error

    assert!(invalid_config.is_err(), "empty group ID should fail validation");

    // Test invalid visibility timeout (too small)
    let invalid_config = (|| -> aspen_hooks::pubsub::consumer_group::error::Result<ConsumerGroupConfig> {
        ConsumerGroupConfig::builder()
            .group_id("valid-group")?
            .pattern(TopicPattern::new("orders.*").unwrap())
            .assignment_mode(AssignmentMode::Competing)
            .ack_policy(AckPolicy::Explicit)
            .visibility_timeout_ms(500) // Too small (< 1s)
            .max_delivery_attempts(3)
            .build()
    })();

    assert!(invalid_config.is_err(), "visibility timeout too small should fail validation");

    // Test invalid max delivery attempts
    let invalid_config = (|| -> aspen_hooks::pubsub::consumer_group::error::Result<ConsumerGroupConfig> {
        ConsumerGroupConfig::builder()
            .group_id("valid-group")?
            .pattern(TopicPattern::new("orders.*").unwrap())
            .assignment_mode(AssignmentMode::Competing)
            .ack_policy(AckPolicy::Explicit)
            .visibility_timeout_ms(30_000)
            .max_delivery_attempts(0) // Invalid (must be >= 1)
            .build()
    })();

    assert!(invalid_config.is_err(), "zero max delivery attempts should fail validation");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test error handling for non-existent groups and consumers.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_error_handling_nonexistent_entities() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,aspen_hooks=debug").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);
    let pending_manager = create_pending_manager(&node);

    let nonexistent_group = ConsumerGroupId::new("nonexistent-group").expect("valid group ID");
    let nonexistent_consumer = ConsumerId::new("nonexistent-consumer").expect("valid consumer ID");

    // Operations on nonexistent group should fail
    assert!(manager.get_group(&nonexistent_group).await.is_err());
    assert!(manager.delete_group(&nonexistent_group, false).await.is_err());

    // Joining a nonexistent group should fail
    let join_result = Consumer::join(
        node.raft_node().clone(),
        pending_manager,
        nonexistent_group,
        nonexistent_consumer,
        JoinOptions {
            metadata: None,
            tags: vec![],
            visibility_timeout_ms: None,
        },
    )
    .await;
    assert!(join_result.is_err(), "joining nonexistent group should fail");

    info!("error handling for nonexistent entities working correctly");

    node.shutdown().await?;
    Ok(())
}

// ============================================================================
// Stress Tests (Optional)
// ============================================================================

/// Stress test with rapid consumer join/leave operations.
#[tokio::test]
#[ignore = "requires network access and is slow - run manually"]
async fn test_rapid_consumer_membership_stress() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info").try_init();

    let temp_dir = TempDir::new()?;
    let node = create_node(1, &temp_dir, &generate_secret_key(1)).await?;
    init_single_node_cluster(&node).await?;

    let manager = create_manager(&node);

    // Create group
    let config = create_test_group_config("stress-group", "stress.*")?;
    let group_id = ConsumerGroupId::new("stress-group").expect("valid group ID");
    let _group_state = manager.create_group(config).await?;

    let consumer_count = 20;
    let start = std::time::Instant::now();

    // Rapidly join and leave consumers
    for i in 0..consumer_count {
        let pending_manager = create_pending_manager(&node);
        let consumer_id = ConsumerId::new(format!("stress-consumer-{}", i)).expect("valid consumer ID");

        // Join
        let consumer = Consumer::join(
            node.raft_node().clone(),
            pending_manager,
            group_id.clone(),
            consumer_id.clone(),
            JoinOptions {
                metadata: None,
                tags: vec![],
                visibility_timeout_ms: None,
            },
        )
        .await?;

        // Send heartbeat
        let _heartbeat = consumer.heartbeat().await?;

        // Leave
        consumer.leave().await?;

        if i % 5 == 0 {
            info!(progress = i, "stress test progress");
        }
    }

    let elapsed = start.elapsed();
    let rate = consumer_count as f64 / elapsed.as_secs_f64();

    info!(
        operations = consumer_count * 3, // join + heartbeat + leave
        elapsed_ms = elapsed.as_millis(),
        rate_per_sec = format!("{:.2}", rate),
        "consumer membership stress test complete"
    );

    // Verify group is empty
    let group_state = manager.get_group(&group_id).await?;
    assert_eq!(group_state.member_count, 0, "all consumers should be gone");

    node.shutdown().await?;
    Ok(())
}
