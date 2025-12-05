//! Integration tests for manual learner promotion API.
//!
//! Tests the operator-controlled promotion of learners to voters with
//! safety validation.

use std::time::Duration;

use aspen::api::{ClusterController, ClusterNode, InitRequest};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::raft::learner_promotion::{LearnerPromotionCoordinator, PromotionRequest};
use aspen::raft::RaftControlClient;

/// Helper to create a test config with minimal required fields.
fn create_test_config(node_id: u64, temp_dir: &std::path::Path) -> ClusterBootstrapConfig {
    ClusterBootstrapConfig {
        node_id,
        data_dir: Some(temp_dir.to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "learner-promotion-test".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    }
}

/// Test promoting a learner to voter in a healthy cluster.
#[tokio::test]
async fn test_promote_learner_basic() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for node_id in 1..=3 {
        let config = create_test_config(node_id, temp_dirs[(node_id - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles.iter().map(|h| h.iroh_manager.node_addr().clone()).collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                handle.network_factory.add_peer((j + 1) as u64, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 1
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(1, "127.0.0.1:20001", Some("127.0.0.1:20001".to_string())),
            ClusterNode::new(2, "127.0.0.1:20002", Some("127.0.0.1:20002".to_string())),
            ClusterNode::new(3, "127.0.0.1:20003", Some("127.0.0.1:20003".to_string())),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader and cluster to be ready
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        // Check if node is leader with established membership
        if metrics.current_leader == Some(1) {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!("Timeout waiting for leader election. Last metrics: {:?}", metrics);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Additional stabilization time for replication
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Add node 4 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle.network_factory.add_peer(4, learner_addr.clone()).await;
        learner_handle.network_factory.add_peer((i + 1) as u64, addrs[i].clone()).await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(4, "127.0.0.1:20004", Some("127.0.0.1:20004".to_string())),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // Promote learner to voter
    let promotion_request = PromotionRequest {
        learner_id: 4,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(result.is_ok(), "Promotion should succeed: {:?}", result);

    let promotion_result = result.unwrap();
    assert_eq!(promotion_result.learner_id, 4);
    assert_eq!(promotion_result.previous_voters, vec![1, 2, 3]);
    assert_eq!(promotion_result.new_voters, vec![1, 2, 3, 4]);

    // Verify cluster state
    let state = controller_1.current_state().await.unwrap();
    assert_eq!(state.members, vec![1, 2, 3, 4]);

    // Cleanup
    for handle in handles {
        handle.shutdown().await.unwrap();
    }
    learner_handle.shutdown().await.unwrap();
}

/// Test promoting a learner to replace a failed voter.
#[tokio::test]
async fn test_promote_learner_replace_voter() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for node_id in 1..=3 {
        let config = create_test_config(node_id, temp_dirs[(node_id - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles.iter().map(|h| h.iroh_manager.node_addr().clone()).collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                handle.network_factory.add_peer((j + 1) as u64, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 1
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(1, "127.0.0.1:21001", Some("127.0.0.1:21001".to_string())),
            ClusterNode::new(2, "127.0.0.1:21002", Some("127.0.0.1:21002".to_string())),
            ClusterNode::new(3, "127.0.0.1:21003", Some("127.0.0.1:21003".to_string())),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader and cluster to be ready
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        // Check if node is leader with established membership
        if metrics.current_leader == Some(1) {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!("Timeout waiting for leader election. Last metrics: {:?}", metrics);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Additional stabilization time for replication
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Add node 4 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle.network_factory.add_peer(4, learner_addr.clone()).await;
        learner_handle.network_factory.add_peer((i + 1) as u64, addrs[i].clone()).await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(4, "127.0.0.1:21004", Some("127.0.0.1:21004".to_string())),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // Promote learner to replace node 3
    let promotion_request = PromotionRequest {
        learner_id: 4,
        replace_node: Some(3),
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(result.is_ok(), "Promotion should succeed: {:?}", result);

    let promotion_result = result.unwrap();
    assert_eq!(promotion_result.learner_id, 4);
    assert_eq!(promotion_result.previous_voters, vec![1, 2, 3]);
    assert_eq!(promotion_result.new_voters, vec![1, 2, 4]);

    // Verify cluster state
    let state = controller_1.current_state().await.unwrap();
    assert_eq!(state.members, vec![1, 2, 4]);

    // Cleanup
    for handle in handles {
        handle.shutdown().await.unwrap();
    }
    learner_handle.shutdown().await.unwrap();
}

/// Test that membership cooldown is enforced.
#[tokio::test]
async fn test_membership_cooldown_enforced() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for node_id in 1..=3 {
        let config = create_test_config(node_id, temp_dirs[(node_id - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles.iter().map(|h| h.iroh_manager.node_addr().clone()).collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                handle.network_factory.add_peer((j + 1) as u64, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 1
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(1, "127.0.0.1:22001", Some("127.0.0.1:22001".to_string())),
            ClusterNode::new(2, "127.0.0.1:22002", Some("127.0.0.1:22002".to_string())),
            ClusterNode::new(3, "127.0.0.1:22003", Some("127.0.0.1:22003".to_string())),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader and cluster to be ready
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        // Check if node is leader with established membership
        if metrics.current_leader == Some(1) {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!("Timeout waiting for leader election. Last metrics: {:?}", metrics);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Additional stabilization time for replication
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Add node 4 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle.network_factory.add_peer(4, learner_addr.clone()).await;
        learner_handle.network_factory.add_peer((i + 1) as u64, addrs[i].clone()).await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(4, "127.0.0.1:22004", Some("127.0.0.1:22004".to_string())),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // First promotion should succeed
    let promotion_request = PromotionRequest {
        learner_id: 4,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(result.is_ok(), "First promotion should succeed: {:?}", result);

    // Add node 5 as learner
    let temp_dir_5 = tempfile::tempdir().unwrap();
    let learner_config_5 = create_test_config(5, temp_dir_5.path());
    let learner_handle_5 = bootstrap_node(learner_config_5).await.unwrap();

    // Exchange peer addresses with node 5
    let learner_addr_5 = learner_handle_5.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle.network_factory.add_peer(5, learner_addr_5.clone()).await;
        learner_handle_5.network_factory.add_peer((i + 1) as u64, addrs[i].clone()).await;
    }
    // Also connect node 4 and node 5 to each other
    learner_handle_5.network_factory.add_peer(4, learner_addr.clone()).await;
    learner_handle.network_factory.add_peer(5, learner_addr_5.clone()).await;

    let add_learner_req_5 = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(5, "127.0.0.1:22005", Some("127.0.0.1:22005".to_string())),
    };

    controller_1.add_learner(add_learner_req_5).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Immediate second promotion should fail (cooldown not elapsed)
    let promotion_request_2 = PromotionRequest {
        learner_id: 5,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request_2).await;
    assert!(
        result.is_err(),
        "Second promotion should fail due to cooldown"
    );

    // Verify error is membership change too recent
    match result {
        Err(e) => {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Membership was changed recently"),
                "Expected cooldown error, got: {}",
                error_msg
            );
        }
        Ok(_) => panic!("Expected error, got success"),
    }

    // Cleanup
    for handle in handles {
        handle.shutdown().await.unwrap();
    }
    learner_handle.shutdown().await.unwrap();
    learner_handle_5.shutdown().await.unwrap();
}

/// Test that force flag bypasses cooldown.
#[tokio::test]
async fn test_force_bypasses_cooldown() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for node_id in 1..=3 {
        let config = create_test_config(node_id, temp_dirs[(node_id - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles.iter().map(|h| h.iroh_manager.node_addr().clone()).collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                handle.network_factory.add_peer((j + 1) as u64, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 1
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(1, "127.0.0.1:23001", Some("127.0.0.1:23001".to_string())),
            ClusterNode::new(2, "127.0.0.1:23002", Some("127.0.0.1:23002".to_string())),
            ClusterNode::new(3, "127.0.0.1:23003", Some("127.0.0.1:23003".to_string())),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader and cluster to be ready
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        // Check if node is leader with established membership
        if metrics.current_leader == Some(1) {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!("Timeout waiting for leader election. Last metrics: {:?}", metrics);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Additional stabilization time for replication
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Add node 4 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle.network_factory.add_peer(4, learner_addr.clone()).await;
        learner_handle.network_factory.add_peer((i + 1) as u64, addrs[i].clone()).await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(4, "127.0.0.1:23004", Some("127.0.0.1:23004".to_string())),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // First promotion
    let promotion_request = PromotionRequest {
        learner_id: 4,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(result.is_ok(), "First promotion should succeed: {:?}", result);

    // Add node 5 as learner
    let temp_dir_5 = tempfile::tempdir().unwrap();
    let learner_config_5 = create_test_config(5, temp_dir_5.path());
    let learner_handle_5 = bootstrap_node(learner_config_5).await.unwrap();

    // Exchange peer addresses with node 5
    let learner_addr_5 = learner_handle_5.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle.network_factory.add_peer(5, learner_addr_5.clone()).await;
        learner_handle_5.network_factory.add_peer((i + 1) as u64, addrs[i].clone()).await;
    }
    // Also connect node 4 and node 5 to each other
    learner_handle_5.network_factory.add_peer(4, learner_addr.clone()).await;
    learner_handle.network_factory.add_peer(5, learner_addr_5.clone()).await;

    let add_learner_req_5 = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(5, "127.0.0.1:23005", Some("127.0.0.1:23005".to_string())),
    };

    controller_1.add_learner(add_learner_req_5).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Immediate second promotion with force=true should succeed
    let promotion_request_2 = PromotionRequest {
        learner_id: 5,
        replace_node: None,
        force: true, // Force flag set
    };

    let result = coordinator.promote_learner(promotion_request_2).await;
    assert!(
        result.is_ok(),
        "Second promotion with force should succeed: {:?}",
        result
    );

    // Cleanup
    for handle in handles {
        handle.shutdown().await.unwrap();
    }
    learner_handle.shutdown().await.unwrap();
    learner_handle_5.shutdown().await.unwrap();
}
