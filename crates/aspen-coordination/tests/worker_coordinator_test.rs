//! Tests for distributed worker coordinator.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use aspen_coordination::DistributedWorkerCoordinator;
use aspen_coordination::GroupState;
use aspen_coordination::HealthStatus;
use aspen_coordination::LoadBalancingStrategy;
use aspen_coordination::WorkerCoordinatorConfig;
use aspen_coordination::WorkerFilter;
use aspen_coordination::WorkerGroup;
use aspen_coordination::WorkerInfo;
use aspen_coordination::WorkerStats;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;

/// Create a test store.
async fn create_test_store() -> Arc<aspen_core::DeterministicKeyValueStore> {
    Arc::new(aspen_core::DeterministicKeyValueStore::new())
}

/// Create a test worker info.
fn create_test_worker(id: &str, node: &str, load: f32) -> WorkerInfo {
    WorkerInfo {
        worker_id: id.to_string(),
        node_id: node.to_string(),
        peer_id: None,
        capabilities: vec!["test".to_string()],
        load,
        active_jobs: (load * 10.0) as usize,
        max_concurrent: 10,
        queue_depth: 0,
        health: HealthStatus::Healthy,
        tags: vec!["test".to_string()],
        last_heartbeat_ms: aspen_coordination::now_unix_ms(),
        started_at_ms: aspen_coordination::now_unix_ms(),
        total_processed: 0,
        total_failed: 0,
        avg_processing_time_ms: 50,
        groups: HashSet::new(),
    }
}

#[tokio::test]
async fn test_worker_registration() {
    let store = create_test_store().await;
    let coordinator = DistributedWorkerCoordinator::new(store);

    // Register a worker
    let worker = create_test_worker("w1", "n1", 0.5);
    coordinator.register_worker(worker.clone()).await.unwrap();

    // Get workers
    let workers = coordinator.get_workers(WorkerFilter::default()).await.unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].worker_id, "w1");
}

#[tokio::test]
async fn test_worker_heartbeat() {
    let store = create_test_store().await;
    let coordinator = DistributedWorkerCoordinator::new(store);

    // Register a worker
    let worker = create_test_worker("w1", "n1", 0.3);
    coordinator.register_worker(worker).await.unwrap();

    // Send heartbeat with updated stats
    let stats = WorkerStats {
        load: 0.7,
        active_jobs: 7,
        queue_depth: 3,
        total_processed: 100,
        total_failed: 2,
        avg_processing_time_ms: 45,
        health: HealthStatus::Healthy,
    };

    coordinator.heartbeat("w1", stats).await.unwrap();

    // Verify updated stats
    let workers = coordinator.get_workers(WorkerFilter::default()).await.unwrap();
    assert_eq!(workers[0].load, 0.7);
    assert_eq!(workers[0].active_jobs, 7);
    assert_eq!(workers[0].queue_depth, 3);
}

#[tokio::test]
async fn test_worker_deregistration() {
    let store = create_test_store().await;
    let coordinator = DistributedWorkerCoordinator::new(store);

    // Register workers
    let w1 = create_test_worker("w1", "n1", 0.5);
    let w2 = create_test_worker("w2", "n1", 0.3);
    coordinator.register_worker(w1).await.unwrap();
    coordinator.register_worker(w2).await.unwrap();

    // Deregister one worker
    coordinator.deregister_worker("w1").await.unwrap();

    // Verify only w2 remains
    let workers = coordinator.get_workers(WorkerFilter::default()).await.unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].worker_id, "w2");
}

#[tokio::test]
async fn test_worker_selection_round_robin() {
    let store = create_test_store().await;
    let mut config = WorkerCoordinatorConfig::default();
    config.strategy = LoadBalancingStrategy::RoundRobin;
    let coordinator = DistributedWorkerCoordinator::with_config(store, config);

    // Register workers
    for i in 1..=3 {
        let worker = create_test_worker(&format!("w{}", i), "n1", 0.5);
        coordinator.register_worker(worker).await.unwrap();
    }

    // Select workers - should round-robin
    let selections: Vec<_> = futures::future::join_all((0..6).map(|_| coordinator.select_worker("test", None))).await;

    // Verify round-robin distribution
    let worker_ids: Vec<_> = selections.into_iter().filter_map(|r| r.ok().flatten()).map(|w| w.worker_id).collect();

    assert_eq!(worker_ids.len(), 6);
    // Should cycle through w1, w2, w3, w1, w2, w3
}

#[tokio::test]
async fn test_worker_selection_least_loaded() {
    let store = create_test_store().await;
    let mut config = WorkerCoordinatorConfig::default();
    config.strategy = LoadBalancingStrategy::LeastLoaded;
    let coordinator = DistributedWorkerCoordinator::with_config(store, config);

    // Register workers with different loads
    let w1 = create_test_worker("w1", "n1", 0.8); // High load
    let w2 = create_test_worker("w2", "n1", 0.2); // Low load
    let w3 = create_test_worker("w3", "n1", 0.5); // Medium load

    coordinator.register_worker(w1).await.unwrap();
    coordinator.register_worker(w2).await.unwrap();
    coordinator.register_worker(w3).await.unwrap();

    // Should select w2 (lowest load)
    let selected = coordinator.select_worker("test", None).await.unwrap();
    assert_eq!(selected.unwrap().worker_id, "w2");
}

#[tokio::test]
async fn test_worker_filtering() {
    let store = create_test_store().await;
    let coordinator = DistributedWorkerCoordinator::new(store);

    // Register workers with different attributes
    let mut w1 = create_test_worker("w1", "n1", 0.5);
    w1.tags = vec!["gpu".to_string(), "ml".to_string()];
    w1.capabilities = vec!["training".to_string()];

    let mut w2 = create_test_worker("w2", "n2", 0.3);
    w2.tags = vec!["cpu".to_string()];
    w2.capabilities = vec!["inference".to_string()];

    let mut w3 = create_test_worker("w3", "n1", 0.7);
    w3.health = HealthStatus::Unhealthy;

    coordinator.register_worker(w1).await.unwrap();
    coordinator.register_worker(w2).await.unwrap();
    coordinator.register_worker(w3).await.unwrap();

    // Filter by node
    let filter = WorkerFilter {
        node_id: Some("n1".to_string()),
        ..Default::default()
    };
    let workers = coordinator.get_workers(filter).await.unwrap();
    assert_eq!(workers.len(), 2); // w1 and w3

    // Filter by health
    let filter = WorkerFilter {
        health: Some(HealthStatus::Healthy),
        ..Default::default()
    };
    let workers = coordinator.get_workers(filter).await.unwrap();
    assert_eq!(workers.len(), 2); // w1 and w2

    // Filter by tags
    let filter = WorkerFilter {
        tags: Some(vec!["gpu".to_string()]),
        ..Default::default()
    };
    let workers = coordinator.get_workers(filter).await.unwrap();
    assert_eq!(workers.len(), 1); // w1
    assert_eq!(workers[0].worker_id, "w1");

    // Filter by capability
    let filter = WorkerFilter {
        capability: Some("training".to_string()),
        ..Default::default()
    };
    let workers = coordinator.get_workers(filter).await.unwrap();
    assert_eq!(workers.len(), 1); // w1
}

#[tokio::test]
async fn test_work_stealing_targets() {
    let store = create_test_store().await;
    let mut config = WorkerCoordinatorConfig::default();
    config.steal_load_threshold = 0.3;
    config.steal_queue_threshold = 5;
    let coordinator = DistributedWorkerCoordinator::with_config(store, config);

    // Register workers with different loads
    let mut w1 = create_test_worker("w1", "n1", 0.1); // Low load, can steal
    w1.active_jobs = 1;

    let mut w2 = create_test_worker("w2", "n1", 0.8); // High load
    w2.queue_depth = 10; // Many queued jobs

    let mut w3 = create_test_worker("w3", "n1", 0.2); // Low load, can steal
    w3.active_jobs = 2;

    coordinator.register_worker(w1).await.unwrap();
    coordinator.register_worker(w2).await.unwrap();
    coordinator.register_worker(w3).await.unwrap();

    // Find steal targets (low load workers)
    let targets = coordinator.find_steal_targets().await.unwrap();
    assert_eq!(targets.len(), 2); // w1 and w3

    // Find steal sources (overloaded workers)
    let sources = coordinator.find_steal_sources().await.unwrap();
    assert_eq!(sources.len(), 1); // w2
    assert_eq!(sources[0].worker_id, "w2");
}

#[tokio::test]
async fn test_worker_groups() {
    let store = create_test_store().await;
    let coordinator = DistributedWorkerCoordinator::new(store);

    // Register workers
    for i in 1..=4 {
        let worker = create_test_worker(&format!("w{}", i), "n1", 0.5);
        coordinator.register_worker(worker).await.unwrap();
    }

    // Create a group
    let mut members = HashSet::new();
    members.insert("w1".to_string());
    members.insert("w2".to_string());
    members.insert("w3".to_string());

    let group = WorkerGroup {
        group_id: "g1".to_string(),
        description: "Test group".to_string(),
        members: members.clone(),
        leader: Some("w1".to_string()),
        required_capabilities: vec!["test".to_string()],
        min_members: 2,
        max_members: 5,
        created_at_ms: aspen_coordination::now_unix_ms(),
        state: GroupState::Active,
    };

    coordinator.create_group(group).await.unwrap();

    // Get group
    let retrieved = coordinator.get_group("g1").await.unwrap().unwrap();
    assert_eq!(retrieved.members.len(), 3);
    assert_eq!(retrieved.leader, Some("w1".to_string()));

    // Add member to group
    coordinator.add_to_group("g1", "w4").await.unwrap();
    let retrieved = coordinator.get_group("g1").await.unwrap().unwrap();
    assert_eq!(retrieved.members.len(), 4);

    // Remove member from group
    coordinator.remove_from_group("g1", "w2").await.unwrap();
    let retrieved = coordinator.get_group("g1").await.unwrap().unwrap();
    assert_eq!(retrieved.members.len(), 3);
}

#[tokio::test]
async fn test_affinity_selection() {
    let store = create_test_store().await;
    let mut config = WorkerCoordinatorConfig::default();
    config.strategy = LoadBalancingStrategy::Affinity;
    let coordinator = DistributedWorkerCoordinator::with_config(store, config);

    // Register workers
    for i in 1..=3 {
        let worker = create_test_worker(&format!("w{}", i), "n1", 0.5);
        coordinator.register_worker(worker).await.unwrap();
    }

    // Select with affinity key
    let selected1 = coordinator.select_worker("test", Some("user123")).await.unwrap();
    assert!(selected1.is_some());
    let worker1 = selected1.unwrap();

    // Same affinity key should return same worker
    let selected2 = coordinator.select_worker("test", Some("user123")).await.unwrap();
    assert!(selected2.is_some());
    let worker2 = selected2.unwrap();

    assert_eq!(worker1.worker_id, worker2.worker_id);

    // Different affinity key might return different worker
    let selected3 = coordinator.select_worker("test", Some("user456")).await.unwrap();
    assert!(selected3.is_some());
}

#[tokio::test]
async fn test_max_workers_limit() {
    let store = create_test_store().await;
    let mut config = WorkerCoordinatorConfig::default();
    config.max_workers = 3;
    let coordinator = DistributedWorkerCoordinator::with_config(store, config);

    // Register workers up to limit
    for i in 1..=3 {
        let worker = create_test_worker(&format!("w{}", i), "n1", 0.5);
        coordinator.register_worker(worker).await.unwrap();
    }

    // Try to register beyond limit
    let worker = create_test_worker("w4", "n1", 0.5);
    let result = coordinator.register_worker(worker).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("maximum worker limit"));
}

#[tokio::test]
async fn test_max_groups_limit() {
    let store = create_test_store().await;
    let mut config = WorkerCoordinatorConfig::default();
    config.max_groups = 2;
    let coordinator = DistributedWorkerCoordinator::with_config(store, config);

    // Register workers
    for i in 1..=4 {
        let worker = create_test_worker(&format!("w{}", i), "n1", 0.5);
        coordinator.register_worker(worker).await.unwrap();
    }

    // Create groups up to limit
    for i in 1..=2 {
        let mut members = HashSet::new();
        members.insert(format!("w{}", i));

        let group = WorkerGroup {
            group_id: format!("g{}", i),
            description: format!("Group {}", i),
            members,
            leader: Some(format!("w{}", i)),
            required_capabilities: vec![],
            min_members: 1,
            max_members: 2,
            created_at_ms: aspen_coordination::now_unix_ms(),
            state: GroupState::Active,
        };

        coordinator.create_group(group).await.unwrap();
    }

    // Try to create beyond limit
    let mut members = HashSet::new();
    members.insert("w3".to_string());

    let group = WorkerGroup {
        group_id: "g3".to_string(),
        description: "Group 3".to_string(),
        members,
        leader: Some("w3".to_string()),
        required_capabilities: vec![],
        min_members: 1,
        max_members: 2,
        created_at_ms: aspen_coordination::now_unix_ms(),
        state: GroupState::Active,
    };

    let result = coordinator.create_group(group).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("maximum group limit"));
}
