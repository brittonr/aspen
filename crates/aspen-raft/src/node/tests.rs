//! Unit tests for RaftNode.

use std::sync::Arc;

use aspen_cluster_types::AddLearnerRequest;
use aspen_cluster_types::ChangeMembershipRequest;
use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::ControlPlaneError;
use aspen_cluster_types::InitRequest;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadConsistency;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::ClusterController;
use aspen_traits::CoordinationBackend;
use aspen_traits::KeyValueStore;
use openraft::Config;

use super::RaftNode;
use super::arc_wrapper::ArcRaftNode;
use super::conversions::node_state_from_openraft;
use super::health::RaftNodeHealth;
use crate::madsim_network::FailureInjector;
use crate::madsim_network::MadsimNetworkFactory;
use crate::madsim_network::MadsimRaftRouter;

/// Maximum concurrent operations (mirrors the constant in mod.rs for testing)
const MAX_CONCURRENT_OPS: usize = 1000;
use crate::StateMachineVariant;
use crate::storage::InMemoryLogStore;
use crate::storage::InMemoryStateMachine;
use crate::types::AppTypeConfig;
use crate::types::NodeId;

/// Create a test Raft node with in-memory storage using the madsim network factory.
async fn create_test_node(node_id: u64) -> RaftNode {
    let config = Config {
        cluster_name: "test-cluster".to_string(),
        ..Default::default()
    };
    let config = Arc::new(config);

    let log_storage = InMemoryLogStore::default();
    let state_machine = Arc::new(InMemoryStateMachine::default());

    // Create madsim network infrastructure
    let router = Arc::new(MadsimRaftRouter::new());
    let failure_injector = Arc::new(FailureInjector::new());
    let network_factory = MadsimNetworkFactory::new(NodeId(node_id), router.clone(), failure_injector);

    let raft: openraft::Raft<AppTypeConfig> =
        openraft::Raft::new(NodeId(node_id), config, network_factory, log_storage, state_machine.clone())
            .await
            .expect("Failed to create Raft instance");

    // Register node with router for RPC dispatch
    router
        .register_node(NodeId(node_id), format!("127.0.0.1:{}", 26000 + node_id), raft.clone())
        .expect("Failed to register node with router");

    RaftNode::new(NodeId(node_id), Arc::new(raft), StateMachineVariant::InMemory(state_machine))
}

/// Helper to set the initialized flag for testing.
fn set_initialized(node: &RaftNode, value: bool) {
    node.set_initialized_for_test(value);
}

/// Test RaftNode creation.
#[tokio::test]
async fn test_raft_node_creation() {
    let node = create_test_node(1).await;
    assert_eq!(node.node_id().0, 1);
    assert!(!node.is_initialized());
}

/// Test node ID accessor.
#[tokio::test]
async fn test_node_id() {
    let node = create_test_node(42).await;
    assert_eq!(node.node_id().0, 42);
}

/// Test is_initialized starts as false.
#[tokio::test]
async fn test_is_initialized_default() {
    let node = create_test_node(1).await;
    assert!(!node.is_initialized());
}

/// Test batching disabled by default.
#[tokio::test]
async fn test_batching_disabled_by_default() {
    let node = create_test_node(1).await;
    assert!(!node.is_batching_enabled());
}

/// Test state machine accessor.
#[tokio::test]
async fn test_state_machine_accessor() {
    let node = create_test_node(1).await;
    match node.state_machine() {
        StateMachineVariant::InMemory(_) => {} // Expected
        StateMachineVariant::Redb(_) => panic!("Expected InMemory state machine"),
    }
}

/// Test raft accessor.
#[tokio::test]
async fn test_raft_accessor() {
    let node = create_test_node(1).await;
    let _raft = node.raft();
    // Just verify it doesn't panic
}

/// Test ClusterController::is_initialized trait method.
#[tokio::test]
async fn test_cluster_controller_is_initialized() {
    let node = create_test_node(1).await;
    // Before init, should return false
    assert!(!ClusterController::is_initialized(&node));
}

/// Test ClusterController::init with empty members fails.
#[tokio::test]
async fn test_init_empty_members_fails() {
    let node = create_test_node(1).await;
    let request = InitRequest {
        initial_members: vec![],
    };

    let result = ClusterController::init(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("must not be empty"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test ClusterController::init with members missing iroh_addr fails.
#[tokio::test]
async fn test_init_missing_iroh_addr_fails() {
    let node = create_test_node(1).await;
    let request = InitRequest {
        initial_members: vec![ClusterNode::new(1, "test-node-1", None)], // No node_addr
    };

    let result = ClusterController::init(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("node_addr must be set"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test ensure_initialized returns error before init.
#[tokio::test]
async fn test_ensure_initialized_before_init() {
    let node = create_test_node(1).await;
    let result = node.ensure_initialized();
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        _ => panic!("Expected NotInitialized error"),
    }
}

/// Test KeyValueStore operations require initialization.
#[tokio::test]
async fn test_kv_operations_require_init() {
    let node = create_test_node(1).await;

    // Read should fail when not initialized
    let read_request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Linearizable,
    };
    let result = KeyValueStore::read(&node, read_request).await;
    assert!(result.is_err());

    // Write should fail when not initialized
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        },
    };
    let result = KeyValueStore::write(&node, write_request).await;
    assert!(result.is_err());
}

/// Test change_membership with empty members fails.
#[tokio::test]
async fn test_change_membership_empty_fails() {
    let node = create_test_node(1).await;
    // Manually set initialized flag for this test
    set_initialized(&node, true);

    let request = ChangeMembershipRequest { members: vec![] };
    let result = ClusterController::change_membership(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("at least one voter"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test add_learner with missing iroh_addr fails.
#[tokio::test]
async fn test_add_learner_missing_addr_fails() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = AddLearnerRequest {
        learner: ClusterNode::new(2, "test-node-2", None), // No node_addr
    };
    let result = ClusterController::add_learner(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("node_addr must be set"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test node_state_from_openraft conversion.
#[test]
fn test_node_state_conversion() {
    use aspen_cluster_types::NodeState;

    assert_eq!(node_state_from_openraft(openraft::ServerState::Learner), NodeState::Learner);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Follower), NodeState::Follower);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Candidate), NodeState::Candidate);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Leader), NodeState::Leader);
    assert_eq!(node_state_from_openraft(openraft::ServerState::Shutdown), NodeState::Shutdown);
}

/// Test RaftNodeHealth creation.
#[tokio::test]
async fn test_raft_node_health_creation() {
    let node = Arc::new(create_test_node(1).await);
    let health = RaftNodeHealth::new(node.clone());
    // Health check on uninitialized node
    let status = health.status().await;
    assert!(!status.is_healthy); // Not healthy because no membership
    assert!(!status.has_membership);
}

/// Test RaftNodeHealth with custom threshold.
#[tokio::test]
async fn test_raft_node_health_threshold() {
    let node = Arc::new(create_test_node(1).await);
    let health = RaftNodeHealth::with_threshold(node.clone(), 5);
    // Just verify creation works
    let status = health.status().await;
    assert_eq!(status.consecutive_failures, 0);
}

/// Test RaftNodeHealth failure reset.
#[tokio::test]
async fn test_raft_node_health_reset() {
    let node = Arc::new(create_test_node(1).await);
    let health = RaftNodeHealth::new(node.clone());
    health.reset_failures();
    let status = health.status().await;
    assert_eq!(status.consecutive_failures, 0);
}

/// Test ArcRaftNode wrapper.
#[tokio::test]
async fn test_arc_raft_node_wrapper() {
    let node = Arc::new(create_test_node(1).await);
    let arc_node = ArcRaftNode::from(node.clone());

    // Test Deref - use explicit deref to access RaftNode::node_id() vs CoordinationBackend::node_id()
    assert_eq!((*arc_node).node_id().0, 1);

    // Test inner() and into_inner()
    assert_eq!(Arc::as_ptr(arc_node.inner()), Arc::as_ptr(&node));

    let recovered = arc_node.into_inner();
    assert_eq!(Arc::as_ptr(&recovered), Arc::as_ptr(&node));
}

/// Test ArcRaftNode CoordinationBackend trait.
#[tokio::test]
async fn test_arc_raft_node_coordination_backend() {
    let node = Arc::new(create_test_node(1).await);
    let arc_node = ArcRaftNode::from(node);

    // Test trait methods
    let node_id = CoordinationBackend::node_id(&arc_node).await;
    assert_eq!(node_id, 1);

    let is_leader = CoordinationBackend::is_leader(&arc_node).await;
    assert!(!is_leader); // Uninitialized node is not leader

    let _now = CoordinationBackend::now_unix_ms(&arc_node).await;
    // Just verify it doesn't panic

    let _kv_store = CoordinationBackend::kv_store(&arc_node);
    let _cluster_controller = CoordinationBackend::cluster_controller(&arc_node);
}

/// Test MAX_CONCURRENT_OPS constant.
#[test]
fn test_max_concurrent_ops() {
    assert_eq!(MAX_CONCURRENT_OPS, 1000);
}

// ========================================================================
// ClusterController Validation Tests - Pre-initialization Checks
// ========================================================================

/// Test add_learner requires initialization.
///
/// Note: ensure_initialized() is called before iroh_addr validation,
/// so we expect NotInitialized even without providing an iroh address.
#[tokio::test]
async fn test_add_learner_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = AddLearnerRequest {
        // No iroh_addr needed - ensure_initialized() is checked first
        learner: ClusterNode::new(2, "test-node-2", None),
    };
    let result = ClusterController::add_learner(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test change_membership requires initialization.
#[tokio::test]
async fn test_change_membership_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = ChangeMembershipRequest { members: vec![1] };
    let result = ClusterController::change_membership(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test current_state requires initialization.
#[tokio::test]
async fn test_current_state_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::current_state(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test get_leader requires initialization.
#[tokio::test]
async fn test_get_leader_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::get_leader(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test get_metrics requires initialization.
#[tokio::test]
async fn test_get_metrics_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::get_metrics(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

/// Test trigger_snapshot requires initialization.
#[tokio::test]
async fn test_trigger_snapshot_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let result = ClusterController::trigger_snapshot(&node).await;
    assert!(result.is_err());
    match result {
        Err(ControlPlaneError::NotInitialized) => {} // Expected
        other => panic!("Expected NotInitialized error, got {:?}", other),
    }
}

// ========================================================================
// KeyValueStore Validation Tests - Pre-initialization Checks
// ========================================================================

/// Test scan requires initialization.
#[tokio::test]
async fn test_scan_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = ScanRequest {
        prefix: "test".to_string(),
        limit_results: None,
        continuation_token: None,
    };
    let result = KeyValueStore::scan(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::Failed { reason }) => {
            assert!(reason.contains("not initialized"));
        }
        other => panic!("Expected Failed error with 'not initialized', got {:?}", other),
    }
}

/// Test delete requires initialization.
#[tokio::test]
async fn test_delete_requires_initialization() {
    let node = create_test_node(1).await;
    // Node is not initialized

    let request = DeleteRequest {
        key: "test".to_string(),
    };
    let result = KeyValueStore::delete(&node, request).await;
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::Failed { reason }) => {
            assert!(reason.contains("not initialized"));
        }
        other => panic!("Expected Failed error with 'not initialized', got {:?}", other),
    }
}

// ========================================================================
// Validation Tests - Input Bounds and Edge Cases
// ========================================================================

/// Test change_membership with single voter is valid.
///
/// While single-node clusters may not be fault-tolerant, they are a valid
/// configuration for development and testing scenarios.
#[tokio::test]
async fn test_change_membership_single_voter_valid() {
    let node = create_test_node(1).await;
    // Manually set initialized flag for this test
    set_initialized(&node, true);

    // Single-node membership change should pass validation
    // (it will fail at Raft level due to no leader, but that's not a validation error)
    let request = ChangeMembershipRequest { members: vec![1] };
    let result = ClusterController::change_membership(&node, request).await;

    // Should NOT be an InvalidRequest error - single voter is valid
    // Timeout or Failed is acceptable (no leader elected)
    if let Err(ControlPlaneError::InvalidRequest { .. }) = result {
        panic!("Single voter should be valid, got InvalidRequest error")
    }
}

/// Test that validate_write_command is called for writes.
///
/// This test verifies that the node properly validates write commands
/// before attempting to apply them through Raft.
#[tokio::test]
async fn test_write_validates_key_size() {
    let node = create_test_node(1).await;
    // Set initialized so we get past that check
    set_initialized(&node, true);

    // Create a write command with invalid key (> MAX_KEY_SIZE = 1KB)
    let oversized_key = "x".repeat(2000);
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: oversized_key,
            value: "value".to_string(),
        },
    };
    let result = KeyValueStore::write(&node, write_request).await;

    // Should fail validation before reaching Raft
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::KeyTooLarge { size, max }) => {
            assert_eq!(size, 2000);
            assert!(max <= 1024); // MAX_KEY_SIZE
        }
        other => panic!("Expected KeyTooLarge error for oversized key, got {:?}", other),
    }
}

/// Test that validate_write_command catches oversized values.
#[tokio::test]
async fn test_write_validates_value_size() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    // Create a write command with invalid value (> MAX_VALUE_SIZE = 1MB)
    let oversized_value = "x".repeat(2_000_000);
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: oversized_value,
        },
    };
    let result = KeyValueStore::write(&node, write_request).await;

    // Should fail validation before reaching Raft
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::ValueTooLarge { size, max }) => {
            assert_eq!(size, 2_000_000);
            assert!(max <= 1_048_576); // MAX_VALUE_SIZE
        }
        other => panic!("Expected ValueTooLarge error for oversized value, got {:?}", other),
    }
}

// ========================================================================
// ReadConsistency Routing Tests
// ========================================================================

/// Test that ReadConsistency::Linearizable routes through ReadIndex.
///
/// The actual ReadIndex call requires a working Raft leader, but we can
/// verify the routing logic and that the operation fails appropriately
/// when there's no leader (which is expected in a single uninitialized node).
#[tokio::test]
async fn test_read_linearizable_uses_read_index() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Linearizable,
    };
    let result = KeyValueStore::read(&node, request).await;

    // Should fail due to not being leader (ReadIndex requires leader)
    // The error indicates the read went through the correct path
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotLeader { .. }) => {
            // Expected: ReadIndex requires leader, which we don't have
        }
        Err(KeyValueStoreError::Timeout { .. }) => {
            // Also acceptable: timeout waiting for ReadIndex
        }
        other => panic!("Expected NotLeader or Timeout for linearizable read without leader, got {:?}", other),
    }
}

/// Test that ReadConsistency::Lease routes through LeaseRead.
#[tokio::test]
async fn test_read_lease_uses_lease_read() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Lease,
    };
    let result = KeyValueStore::read(&node, request).await;

    // Should fail due to not being leader (LeaseRead requires leader)
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotLeader { .. }) => {
            // Expected: LeaseRead requires leader
        }
        Err(KeyValueStoreError::Timeout { .. }) => {
            // Also acceptable: timeout waiting for lease
        }
        other => panic!("Expected NotLeader or Timeout for lease read without leader, got {:?}", other),
    }
}

/// Test that ReadConsistency::Stale skips linearizer and reads directly.
///
/// Stale reads don't require a leader and return immediately from local
/// state machine, which means they should fail with NotFound (no data)
/// rather than NotLeader.
#[tokio::test]
async fn test_read_stale_skips_linearizer() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ReadRequest {
        key: "test".to_string(),
        consistency: ReadConsistency::Stale,
    };
    let result = KeyValueStore::read(&node, request).await;

    // Stale reads go directly to state machine, no leader needed
    // Should fail with NotFound (key doesn't exist) rather than NotLeader
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotFound { key }) => {
            assert_eq!(key, "test");
        }
        other => panic!("Expected NotFound for stale read of missing key, got {:?}", other),
    }
}

/// Test ReadConsistency routing exhaustiveness.
#[test]
fn test_read_consistency_variants_exhaustive() {
    // Verify all ReadConsistency variants are accounted for
    let variants = [
        ReadConsistency::Linearizable,
        ReadConsistency::Lease,
        ReadConsistency::Stale,
    ];

    for consistency in variants {
        // Pattern match to ensure exhaustive handling
        match consistency {
            ReadConsistency::Linearizable => {} // Uses ReadIndex
            ReadConsistency::Lease => {}        // Uses LeaseRead
            ReadConsistency::Stale => {}        // Direct state machine read
        }
    }
}

/// Test that scan uses ReadIndex for linearizability.
#[tokio::test]
async fn test_scan_uses_read_index() {
    let node = create_test_node(1).await;
    set_initialized(&node, true);

    let request = ScanRequest {
        prefix: "test".to_string(),
        limit_results: None,
        continuation_token: None,
    };
    let result = KeyValueStore::scan(&node, request).await;

    // Scan always uses ReadIndex for linearizability
    // Should fail due to not being leader
    assert!(result.is_err());
    match result {
        Err(KeyValueStoreError::NotLeader { .. }) => {
            // Expected: ReadIndex requires leader
        }
        Err(KeyValueStoreError::Timeout { .. }) => {
            // Also acceptable: timeout waiting for ReadIndex
        }
        other => panic!("Expected NotLeader or Timeout for scan without leader, got {:?}", other),
    }
}
