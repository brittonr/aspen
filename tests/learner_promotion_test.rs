//! Integration tests for manual learner promotion API.
//!
//! Tests the operator-controlled promotion of learners to voters with
//! safety validation.

use std::time::Duration;

use aspen::api::{ClusterController, ClusterNode, InitRequest};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::raft::RaftControlClient;
use aspen::raft::learner_promotion::{LearnerPromotionCoordinator, PromotionRequest};

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
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir.join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir.join("state-machine.db")),
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

    // Use unique node IDs to avoid actor name conflicts when tests run in parallel
    const NODE_ID_BASE: u64 = 1000;

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for i in 1..=3 {
        let node_id = NODE_ID_BASE + i;
        let config = create_test_config(node_id, temp_dirs[(i - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles
        .iter()
        .map(|h| h.iroh_manager.node_addr().clone())
        .collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                let peer_id = NODE_ID_BASE + (j as u64) + 1;
                handle.network_factory.add_peer(peer_id, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 1001
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(
                NODE_ID_BASE + 1,
                "127.0.0.1:20001",
                Some("127.0.0.1:20001".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 2,
                "127.0.0.1:20002",
                Some("127.0.0.1:20002".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 3,
                "127.0.0.1:20003",
                Some("127.0.0.1:20003".to_string()),
            ),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader AND for initial membership to be committed
    // The init() call writes a membership log at index 0, which must be committed
    // before any subsequent membership changes (like add_learner) can be made
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        let is_leader = metrics.current_leader == Some(NODE_ID_BASE + 1);
        let membership_log_id = metrics.membership_config.log_id();
        let last_applied = metrics.last_applied.as_ref().map(|log_id| log_id.index);

        // The initial membership is at index 0. Wait until it's applied.
        let membership_committed = membership_log_id
            .as_ref()
            .and_then(|m_log_id| last_applied.map(|applied_idx| applied_idx >= m_log_id.index))
            .unwrap_or(false);

        if is_leader && membership_committed {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!(
                "Timeout waiting for leader election and membership commit. Last metrics: {:?}",
                metrics
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Add node 1004 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(NODE_ID_BASE + 4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle
            .network_factory
            .add_peer(NODE_ID_BASE + 4, learner_addr.clone())
            .await;
        let peer_id = NODE_ID_BASE + (i as u64) + 1;
        learner_handle
            .network_factory
            .add_peer(peer_id, addrs[i].clone())
            .await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(
            NODE_ID_BASE + 4,
            "127.0.0.1:20004",
            Some("127.0.0.1:20004".to_string()),
        ),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // Promote learner to voter
    let promotion_request = PromotionRequest {
        learner_id: NODE_ID_BASE + 4,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(result.is_ok(), "Promotion should succeed: {:?}", result);

    let promotion_result = result.unwrap();
    assert_eq!(promotion_result.learner_id, NODE_ID_BASE + 4);
    assert_eq!(
        promotion_result.previous_voters,
        vec![NODE_ID_BASE + 1, NODE_ID_BASE + 2, NODE_ID_BASE + 3]
    );
    assert_eq!(
        promotion_result.new_voters,
        vec![
            NODE_ID_BASE + 1,
            NODE_ID_BASE + 2,
            NODE_ID_BASE + 3,
            NODE_ID_BASE + 4
        ]
    );

    // Verify cluster state
    let state = controller_1.current_state().await.unwrap();
    assert_eq!(
        state.members,
        vec![
            NODE_ID_BASE + 1,
            NODE_ID_BASE + 2,
            NODE_ID_BASE + 3,
            NODE_ID_BASE + 4
        ]
    );

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

    // Use unique node IDs to avoid actor name conflicts when tests run in parallel
    const NODE_ID_BASE: u64 = 2000;

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for i in 1..=3 {
        let node_id = NODE_ID_BASE + i;
        let config = create_test_config(node_id, temp_dirs[(i - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles
        .iter()
        .map(|h| h.iroh_manager.node_addr().clone())
        .collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                let peer_id = NODE_ID_BASE + (j as u64) + 1;
                handle.network_factory.add_peer(peer_id, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 2001
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(
                NODE_ID_BASE + 1,
                "127.0.0.1:21001",
                Some("127.0.0.1:21001".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 2,
                "127.0.0.1:21002",
                Some("127.0.0.1:21002".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 3,
                "127.0.0.1:21003",
                Some("127.0.0.1:21003".to_string()),
            ),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader AND for initial membership to be committed
    // The init() call writes a membership log at index 0, which must be committed
    // before any subsequent membership changes (like add_learner) can be made
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        let is_leader = metrics.current_leader == Some(NODE_ID_BASE + 1);
        let membership_log_id = metrics.membership_config.log_id();
        let last_applied = metrics.last_applied.as_ref().map(|log_id| log_id.index);

        // The initial membership is at index 0. Wait until it's applied.
        let membership_committed = membership_log_id
            .as_ref()
            .and_then(|m_log_id| last_applied.map(|applied_idx| applied_idx >= m_log_id.index))
            .unwrap_or(false);

        if is_leader && membership_committed {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!(
                "Timeout waiting for leader election and membership commit. Last metrics: {:?}",
                metrics
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Add node 2004 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(NODE_ID_BASE + 4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle
            .network_factory
            .add_peer(NODE_ID_BASE + 4, learner_addr.clone())
            .await;
        let peer_id = NODE_ID_BASE + (i as u64) + 1;
        learner_handle
            .network_factory
            .add_peer(peer_id, addrs[i].clone())
            .await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(
            NODE_ID_BASE + 4,
            "127.0.0.1:21004",
            Some("127.0.0.1:21004".to_string()),
        ),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // Promote learner to replace node 2003
    let promotion_request = PromotionRequest {
        learner_id: NODE_ID_BASE + 4,
        replace_node: Some(NODE_ID_BASE + 3),
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(result.is_ok(), "Promotion should succeed: {:?}", result);

    let promotion_result = result.unwrap();
    assert_eq!(promotion_result.learner_id, NODE_ID_BASE + 4);
    assert_eq!(
        promotion_result.previous_voters,
        vec![NODE_ID_BASE + 1, NODE_ID_BASE + 2, NODE_ID_BASE + 3]
    );
    assert_eq!(
        promotion_result.new_voters,
        vec![NODE_ID_BASE + 1, NODE_ID_BASE + 2, NODE_ID_BASE + 4]
    );

    // Verify cluster state
    let state = controller_1.current_state().await.unwrap();
    assert_eq!(
        state.members,
        vec![NODE_ID_BASE + 1, NODE_ID_BASE + 2, NODE_ID_BASE + 4]
    );

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

    // Use unique node IDs to avoid actor name conflicts when tests run in parallel
    const NODE_ID_BASE: u64 = 3000;

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for i in 1..=3 {
        let node_id = NODE_ID_BASE + i;
        let config = create_test_config(node_id, temp_dirs[(i - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles
        .iter()
        .map(|h| h.iroh_manager.node_addr().clone())
        .collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                let peer_id = NODE_ID_BASE + (j as u64) + 1;
                handle.network_factory.add_peer(peer_id, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 3001
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(
                NODE_ID_BASE + 1,
                "127.0.0.1:22001",
                Some("127.0.0.1:22001".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 2,
                "127.0.0.1:22002",
                Some("127.0.0.1:22002".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 3,
                "127.0.0.1:22003",
                Some("127.0.0.1:22003".to_string()),
            ),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader AND for initial membership to be committed
    // The init() call writes a membership log at index 0, which must be committed
    // before any subsequent membership changes (like add_learner) can be made
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        let is_leader = metrics.current_leader == Some(NODE_ID_BASE + 1);
        let membership_log_id = metrics.membership_config.log_id();
        let last_applied = metrics.last_applied.as_ref().map(|log_id| log_id.index);

        // The initial membership is at index 0. Wait until it's applied.
        let membership_committed = membership_log_id
            .as_ref()
            .and_then(|m_log_id| last_applied.map(|applied_idx| applied_idx >= m_log_id.index))
            .unwrap_or(false);

        if is_leader && membership_committed {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!(
                "Timeout waiting for leader election and membership commit. Last metrics: {:?}",
                metrics
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Add node 3004 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(NODE_ID_BASE + 4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle
            .network_factory
            .add_peer(NODE_ID_BASE + 4, learner_addr.clone())
            .await;
        let peer_id = NODE_ID_BASE + (i as u64) + 1;
        learner_handle
            .network_factory
            .add_peer(peer_id, addrs[i].clone())
            .await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(
            NODE_ID_BASE + 4,
            "127.0.0.1:22004",
            Some("127.0.0.1:22004".to_string()),
        ),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // First promotion should succeed
    let promotion_request = PromotionRequest {
        learner_id: NODE_ID_BASE + 4,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(
        result.is_ok(),
        "First promotion should succeed: {:?}",
        result
    );

    // Add node 3005 as learner
    let temp_dir_5 = tempfile::tempdir().unwrap();
    let learner_config_5 = create_test_config(NODE_ID_BASE + 5, temp_dir_5.path());
    let learner_handle_5 = bootstrap_node(learner_config_5).await.unwrap();

    // Exchange peer addresses with node 5
    let learner_addr_5 = learner_handle_5.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle
            .network_factory
            .add_peer(NODE_ID_BASE + 5, learner_addr_5.clone())
            .await;
        let peer_id = NODE_ID_BASE + (i as u64) + 1;
        learner_handle_5
            .network_factory
            .add_peer(peer_id, addrs[i].clone())
            .await;
    }
    // Also connect node 3004 and node 3005 to each other
    learner_handle_5
        .network_factory
        .add_peer(NODE_ID_BASE + 4, learner_addr.clone())
        .await;
    learner_handle
        .network_factory
        .add_peer(NODE_ID_BASE + 5, learner_addr_5.clone())
        .await;

    let add_learner_req_5 = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(
            NODE_ID_BASE + 5,
            "127.0.0.1:22005",
            Some("127.0.0.1:22005".to_string()),
        ),
    };

    controller_1.add_learner(add_learner_req_5).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Immediate second promotion should fail (cooldown not elapsed)
    let promotion_request_2 = PromotionRequest {
        learner_id: NODE_ID_BASE + 5,
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

    // Use unique node IDs to avoid actor name conflicts when tests run in parallel
    const NODE_ID_BASE: u64 = 4000;

    // Create 3-node cluster
    let temp_dirs: Vec<_> = (1..=3).map(|_| tempfile::tempdir().unwrap()).collect();

    let mut handles = vec![];
    for i in 1..=3 {
        let node_id = NODE_ID_BASE + i;
        let config = create_test_config(node_id, temp_dirs[(i - 1) as usize].path());
        let handle = bootstrap_node(config).await.unwrap();
        handles.push(handle);
    }

    // Exchange peer addresses between all nodes
    let addrs: Vec<_> = handles
        .iter()
        .map(|h| h.iroh_manager.node_addr().clone())
        .collect();
    for (i, handle) in handles.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                let peer_id = NODE_ID_BASE + (j as u64) + 1;
                handle.network_factory.add_peer(peer_id, addr.clone()).await;
            }
        }
    }

    // Initialize cluster on node 4001
    let controller_1 = RaftControlClient::new(handles[0].raft_actor.clone());

    let init_request = InitRequest {
        initial_members: vec![
            ClusterNode::new(
                NODE_ID_BASE + 1,
                "127.0.0.1:23001",
                Some("127.0.0.1:23001".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 2,
                "127.0.0.1:23002",
                Some("127.0.0.1:23002".to_string()),
            ),
            ClusterNode::new(
                NODE_ID_BASE + 3,
                "127.0.0.1:23003",
                Some("127.0.0.1:23003".to_string()),
            ),
        ],
    };

    controller_1.init(init_request).await.unwrap();

    // Wait for node to become leader AND for initial membership to be committed
    // The init() call writes a membership log at index 0, which must be committed
    // before any subsequent membership changes (like add_learner) can be made
    let timeout = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let metrics = controller_1.get_metrics().await.unwrap();

        let is_leader = metrics.current_leader == Some(NODE_ID_BASE + 1);
        let membership_log_id = metrics.membership_config.log_id();
        let last_applied = metrics.last_applied.as_ref().map(|log_id| log_id.index);

        // The initial membership is at index 0. Wait until it's applied.
        let membership_committed = membership_log_id
            .as_ref()
            .and_then(|m_log_id| last_applied.map(|applied_idx| applied_idx >= m_log_id.index))
            .unwrap_or(false);

        if is_leader && membership_committed {
            break;
        }

        if std::time::Instant::now() > timeout {
            panic!(
                "Timeout waiting for leader election and membership commit. Last metrics: {:?}",
                metrics
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Add node 4004 as learner
    let temp_dir_4 = tempfile::tempdir().unwrap();
    let learner_config = create_test_config(NODE_ID_BASE + 4, temp_dir_4.path());
    let learner_handle = bootstrap_node(learner_config).await.unwrap();

    // Exchange peer addresses with the learner
    let learner_addr = learner_handle.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle
            .network_factory
            .add_peer(NODE_ID_BASE + 4, learner_addr.clone())
            .await;
        let peer_id = NODE_ID_BASE + (i as u64) + 1;
        learner_handle
            .network_factory
            .add_peer(peer_id, addrs[i].clone())
            .await;
    }

    let add_learner_req = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(
            NODE_ID_BASE + 4,
            "127.0.0.1:23004",
            Some("127.0.0.1:23004".to_string()),
        ),
    };

    controller_1.add_learner(add_learner_req).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create promotion coordinator
    let coordinator = LearnerPromotionCoordinator::new(std::sync::Arc::new(controller_1.clone()));

    // First promotion
    let promotion_request = PromotionRequest {
        learner_id: NODE_ID_BASE + 4,
        replace_node: None,
        force: false,
    };

    let result = coordinator.promote_learner(promotion_request).await;
    assert!(
        result.is_ok(),
        "First promotion should succeed: {:?}",
        result
    );

    // Add node 4005 as learner
    let temp_dir_5 = tempfile::tempdir().unwrap();
    let learner_config_5 = create_test_config(NODE_ID_BASE + 5, temp_dir_5.path());
    let learner_handle_5 = bootstrap_node(learner_config_5).await.unwrap();

    // Exchange peer addresses with node 5
    let learner_addr_5 = learner_handle_5.iroh_manager.node_addr().clone();
    for (i, handle) in handles.iter().enumerate() {
        handle
            .network_factory
            .add_peer(NODE_ID_BASE + 5, learner_addr_5.clone())
            .await;
        let peer_id = NODE_ID_BASE + (i as u64) + 1;
        learner_handle_5
            .network_factory
            .add_peer(peer_id, addrs[i].clone())
            .await;
    }
    // Also connect node 4004 and node 4005 to each other
    learner_handle_5
        .network_factory
        .add_peer(NODE_ID_BASE + 4, learner_addr.clone())
        .await;
    learner_handle
        .network_factory
        .add_peer(NODE_ID_BASE + 5, learner_addr_5.clone())
        .await;

    let add_learner_req_5 = aspen::api::AddLearnerRequest {
        learner: ClusterNode::new(
            NODE_ID_BASE + 5,
            "127.0.0.1:23005",
            Some("127.0.0.1:23005".to_string()),
        ),
    };

    controller_1.add_learner(add_learner_req_5).await.unwrap();

    // Wait for learner to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Immediate second promotion with force=true should succeed
    let promotion_request_2 = PromotionRequest {
        learner_id: NODE_ID_BASE + 5,
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
