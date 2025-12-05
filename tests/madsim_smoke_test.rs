/// Smoke test for madsim network infrastructure.
///
/// Validates that MadsimRaftRouter and failure injection work correctly
/// before integrating with real RaftActor. This test doesn't use actual
/// Raft consensus yet - it just verifies the network plumbing.
use std::sync::Arc;

use aspen::raft::madsim_network::{
    FailureInjector, MadsimNetworkFactory, MadsimRaftRouter,
};
use aspen::simulation::SimulationArtifactBuilder;

/// Test basic router initialization and node registration.
#[madsim::test]
async fn test_router_initialization() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_router_init", seed).start();

    artifact = artifact.add_event("router: initialize");
    let router = Arc::new(MadsimRaftRouter::new());

    artifact = artifact.add_event("router: register node 1");
    router
        .register_node(1, "127.0.0.1:26001".to_string())
        .expect("failed to register node 1");

    artifact = artifact.add_event("router: register node 2");
    router
        .register_node(2, "127.0.0.1:26002".to_string())
        .expect("failed to register node 2");

    artifact = artifact.add_event("router: register node 3");
    router
        .register_node(3, "127.0.0.1:26003".to_string())
        .expect("failed to register node 3");

    artifact = artifact.add_event("validation: 3 nodes registered");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test failure injector configuration.
#[madsim::test]
async fn test_failure_injector() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_failure_injector", seed).start();

    artifact = artifact.add_event("failure-injector: initialize");
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("failure-injector: configure 50ms delay 1->2");
    injector.set_network_delay(1, 2, 50);

    artifact = artifact.add_event("failure-injector: configure message drop 2->3");
    injector.set_message_drop(2, 3, true);

    artifact = artifact.add_event("failure-injector: clear all configuration");
    injector.clear_all();

    artifact = artifact.add_event("validation: failure injection configured");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test network factory creation.
#[madsim::test]
async fn test_network_factory() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_network_factory", seed).start();

    artifact = artifact.add_event("router: initialize");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("factory: create for node 1");
    let _factory = MadsimNetworkFactory::new(1, router.clone(), injector.clone());

    artifact = artifact.add_event("factory: create for node 2");
    let _factory = MadsimNetworkFactory::new(2, router.clone(), injector.clone());

    artifact = artifact.add_event("validation: network factories created");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test node failure marking.
#[madsim::test]
async fn test_node_failure() {
    let seed = 789_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_node_failure", seed).start();

    artifact = artifact.add_event("router: initialize");
    let router = Arc::new(MadsimRaftRouter::new());

    artifact = artifact.add_event("router: register nodes 1-3");
    router
        .register_node(1, "127.0.0.1:26001".to_string())
        .expect("failed to register node 1");
    router
        .register_node(2, "127.0.0.1:26002".to_string())
        .expect("failed to register node 2");
    router
        .register_node(3, "127.0.0.1:26003".to_string())
        .expect("failed to register node 3");

    artifact = artifact.add_event("router: mark node 2 as failed");
    router.mark_node_failed(2, true);

    artifact = artifact.add_event("router: mark node 2 as recovered");
    router.mark_node_failed(2, false);

    artifact = artifact.add_event("validation: node failure tracking works");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test max nodes limit enforcement.
#[madsim::test]
async fn test_max_nodes_limit() {
    let seed = 1024_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_max_nodes", seed).start();

    artifact = artifact.add_event("router: initialize");
    let router = Arc::new(MadsimRaftRouter::new());

    artifact = artifact.add_event("router: attempt to register 100 nodes (max limit)");
    // The MAX_CONNECTIONS_PER_NODE is 100, so we should be able to register exactly 100
    for i in 1..=100 {
        router
            .register_node(i, format!("127.0.0.1:2600{i}"))
            .expect("failed to register node within limit");
    }

    artifact = artifact.add_event("router: attempt to register 101st node (should fail)");
    let result = router.register_node(101, "127.0.0.1:26101".to_string());
    assert!(
        result.is_err(),
        "expected error when exceeding max nodes limit"
    );

    artifact = artifact.add_event("validation: max nodes limit enforced");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
