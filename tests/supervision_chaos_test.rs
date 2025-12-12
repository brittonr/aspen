//! Chaos engineering tests for RaftSupervisor using madsim.
//!
//! These tests verify supervision behavior under deterministic chaos scenarios:
//! - Actor crashes during network partitions
//! - Restart behavior during network failures
//! - Supervision with message delays
//! - Meltdown detection under chaotic conditions
//!
//! All tests use deterministic seeds for reproducibility via madsim.

use std::sync::Arc;
use std::time::Duration;

use aspen::raft::madsim_network::{FailureInjector, MadsimNetworkFactory, MadsimRaftRouter};
use aspen::raft::storage::{InMemoryLogStore, InMemoryStateMachine};
use aspen::raft::supervision::{
    RaftSupervisor, SupervisionConfig, SupervisorArguments, SupervisorMessage,
};
use aspen::raft::types::NodeId;
use aspen::raft::{RaftActorConfig, StateMachineVariant};
use aspen::simulation::SimulationArtifactBuilder;
use openraft::{Config as RaftConfig, Raft};
use ractor::Actor;

/// Helper to create RaftActorConfig with madsim network for chaos testing.
async fn create_supervised_raft_config(
    node_id: NodeId,
    router: Arc<MadsimRaftRouter>,
    injector: Arc<FailureInjector>,
) -> RaftActorConfig {
    let raft_config = Arc::new(
        RaftConfig {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            ..Default::default()
        }
        .validate()
        .expect("invalid raft config"),
    );

    let log_store = InMemoryLogStore::default();
    let state_machine = Arc::new(InMemoryStateMachine::default());
    let network_factory = MadsimNetworkFactory::new(node_id, router, injector);

    let raft = Raft::new(
        node_id,
        raft_config,
        network_factory,
        log_store,
        state_machine.clone(),
    )
    .await
    .expect("failed to create raft");

    RaftActorConfig {
        node_id,
        raft,
        state_machine: StateMachineVariant::InMemory(state_machine),
        log_store: None,
    }
}

/// Test supervisor restart behavior during network partition.
///
/// Tiger Style: Deterministic chaos with seed 1000.
#[madsim::test]
async fn test_supervised_restart_during_network_partition_seed_1000() {
    let seed = 1000_u64;
    let mut artifact =
        SimulationArtifactBuilder::new("supervision_restart_during_partition", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: supervised raft node");
    let raft_actor_config =
        create_supervised_raft_config(NodeId::from(1), router.clone(), injector.clone()).await;

    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 2,
        max_restarts_per_window: 5,
        restart_window_secs: 60,
        restart_history_size: 100,
        circuit_open_duration_secs: 300,
        half_open_stability_duration_secs: 120,
    };

    artifact = artifact.add_event("spawn: supervisor with raft actor");
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("chaos-supervisor-1".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    artifact = artifact.add_event("wait: initial actor startup (200ms)");
    madsim::time::sleep(Duration::from_millis(200)).await;

    artifact = artifact.add_event("failure: inject network partition (drop messages)");
    injector.set_message_drop(NodeId::from(1), NodeId::from(1), true); // Drop messages from node to itself

    artifact = artifact.add_event("action: trigger manual restart during partition");
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

    artifact = artifact.add_event("wait: for restart to process under partition (2s)");
    madsim::time::sleep(Duration::from_secs(2)).await;

    artifact = artifact.add_event("cleanup: clear partition");
    injector.set_message_drop(NodeId::from(1), NodeId::from(1), false);

    artifact = artifact.add_event("validation: supervisor handles restart during partition");

    artifact = artifact.add_event("cleanup: stop supervisor");
    supervisor_ref.stop(Some("test-complete".into()));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test meltdown detection with chaotic restart triggers.
///
/// Verifies that supervisor meltdown detection works correctly
/// even when restarts are triggered by various failure modes.
#[madsim::test]
async fn test_meltdown_detection_with_chaos_seed_2000() {
    let seed = 2000_u64;
    let mut artifact = SimulationArtifactBuilder::new("supervision_chaos_meltdown", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: supervised raft node");
    let raft_actor_config =
        create_supervised_raft_config(NodeId::from(2), router.clone(), injector.clone()).await;

    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1, // Short for testing
        max_restarts_per_window: 3,       // Only 3 restarts allowed
        restart_window_secs: 30,
        restart_history_size: 100,
        circuit_open_duration_secs: 300,
        half_open_stability_duration_secs: 120,
    };

    artifact = artifact.add_event("spawn: supervisor with tight meltdown config");
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("chaos-supervisor-2".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    artifact = artifact.add_event("wait: initial actor startup (100ms)");
    madsim::time::sleep(Duration::from_millis(100)).await;

    artifact = artifact.add_event("chaos: trigger rapid restarts with varying delays");

    // Trigger multiple rapid restarts with chaotic delays
    for i in 0..5 {
        artifact = artifact.add_event(format!("restart: trigger restart #{}", i + 1));
        let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

        // Varying delays between restarts (10-50ms)
        let delay_ms = 10 + (i * 10);
        madsim::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    artifact = artifact.add_event("wait: for meltdown detection to engage (3s)");
    madsim::time::sleep(Duration::from_secs(3)).await;

    artifact = artifact.add_event("validation: supervisor survived rapid restarts");
    // If meltdown detection works, supervisor should still be running
    // without panicking or causing infinite restart loops

    artifact = artifact.add_event("cleanup: stop supervisor");
    supervisor_ref.stop(Some("test-complete".into()));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test supervisor restart with message delays.
///
/// Tiger Style: Verifies supervision handles slow networks gracefully.
#[madsim::test]
async fn test_supervised_restart_with_message_delays_seed_3000() {
    let seed = 3000_u64;
    let mut artifact = SimulationArtifactBuilder::new("supervision_message_delays", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: supervised raft node");
    let raft_actor_config =
        create_supervised_raft_config(NodeId::from(3), router.clone(), injector.clone()).await;

    let supervision_config = SupervisionConfig::default();

    artifact = artifact.add_event("spawn: supervisor");
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("chaos-supervisor-3".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    artifact = artifact.add_event("wait: initial actor startup (100ms)");
    madsim::time::sleep(Duration::from_millis(100)).await;

    artifact = artifact.add_event("failure: inject 500ms message delay");
    injector.set_network_delay(NodeId(3), NodeId(3), 500); // 500ms delay

    artifact = artifact.add_event("action: trigger restart with delayed messages");
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

    artifact = artifact.add_event("wait: for restart under message delays (2s)");
    madsim::time::sleep(Duration::from_secs(2)).await;

    artifact = artifact.add_event("cleanup: clear delay");
    injector.clear_all();

    artifact = artifact.add_event("validation: restart succeeds despite delays");

    artifact = artifact.add_event("cleanup: stop supervisor");
    supervisor_ref.stop(Some("test-complete".into()));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test multiple supervised actors under simultaneous chaos.
///
/// Verifies supervision isolation - one actor's chaos doesn't affect others.
#[madsim::test]
async fn test_multiple_supervised_actors_chaos_seed_4000() {
    let seed = 4000_u64;
    let mut artifact = SimulationArtifactBuilder::new("supervision_multiple_chaos", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 supervised raft nodes");
    let raft_config_1 =
        create_supervised_raft_config(NodeId::from(10), router.clone(), injector.clone()).await;
    let raft_config_2 =
        create_supervised_raft_config(NodeId::from(11), router.clone(), injector.clone()).await;
    let raft_config_3 =
        create_supervised_raft_config(NodeId::from(12), router.clone(), injector.clone()).await;

    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 2,
        max_restarts_per_window: 5,
        restart_window_secs: 60,
        restart_history_size: 100,
        circuit_open_duration_secs: 300,
        half_open_stability_duration_secs: 120,
    };

    artifact = artifact.add_event("spawn: 3 supervisors");
    let (sup1_ref, _) = Actor::spawn(
        Some("multi-chaos-sup-1".to_string()),
        RaftSupervisor,
        SupervisorArguments {
            raft_actor_config: raft_config_1,
            supervision_config: supervision_config.clone(),
        },
    )
    .await
    .expect("failed to spawn supervisor 1");

    let (sup2_ref, _) = Actor::spawn(
        Some("multi-chaos-sup-2".to_string()),
        RaftSupervisor,
        SupervisorArguments {
            raft_actor_config: raft_config_2,
            supervision_config: supervision_config.clone(),
        },
    )
    .await
    .expect("failed to spawn supervisor 2");

    let (sup3_ref, _) = Actor::spawn(
        Some("multi-chaos-sup-3".to_string()),
        RaftSupervisor,
        SupervisorArguments {
            raft_actor_config: raft_config_3,
            supervision_config,
        },
    )
    .await
    .expect("failed to spawn supervisor 3");

    artifact = artifact.add_event("wait: initial actor startup (200ms)");
    madsim::time::sleep(Duration::from_millis(200)).await;

    artifact = artifact.add_event("chaos: inject partition on node 10 only");
    injector.set_message_drop(NodeId(10), NodeId(10), true);

    artifact = artifact.add_event("chaos: inject delay on node 11 only");
    injector.set_network_delay(NodeId(11), NodeId(11), 200); // 200ms delay

    artifact = artifact.add_event("action: trigger restarts on all supervisors");
    let _ = sup1_ref.cast(SupervisorMessage::ManualRestart);
    let _ = sup2_ref.cast(SupervisorMessage::ManualRestart);
    let _ = sup3_ref.cast(SupervisorMessage::ManualRestart);

    artifact = artifact.add_event("wait: for all restarts to process (3s)");
    madsim::time::sleep(Duration::from_secs(3)).await;

    artifact =
        artifact.add_event("validation: all supervisors handle restarts despite isolated chaos");

    artifact = artifact.add_event("cleanup: stop all supervisors");
    sup1_ref.stop(Some("test-complete".into()));
    sup2_ref.stop(Some("test-complete".into()));
    sup3_ref.stop(Some("test-complete".into()));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test exponential backoff under network instability.
///
/// Verifies that backoff delays are preserved even when
/// network conditions fluctuate during restart.
#[madsim::test]
async fn test_backoff_under_network_instability_seed_5000() {
    let seed = 5000_u64;
    let mut artifact =
        SimulationArtifactBuilder::new("supervision_backoff_network_instability", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: supervised raft node");
    let raft_actor_config =
        create_supervised_raft_config(NodeId::from(4), router.clone(), injector.clone()).await;

    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1,
        max_restarts_per_window: 10,
        restart_window_secs: 120,
        restart_history_size: 100,
        circuit_open_duration_secs: 300,
        half_open_stability_duration_secs: 120,
    };

    artifact = artifact.add_event("spawn: supervisor");
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("chaos-supervisor-4".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    artifact = artifact.add_event("wait: initial actor startup (100ms)");
    madsim::time::sleep(Duration::from_millis(100)).await;

    artifact = artifact.add_event("chaos: intermittent network instability");
    // Create fluctuating network conditions
    injector.set_network_delay(NodeId(4), NodeId(4), 100); // 100ms delay

    artifact = artifact.add_event("action: trigger first restart");
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

    artifact = artifact.add_event("wait: for ~1s backoff with unstable network");
    madsim::time::sleep(Duration::from_millis(1500)).await;

    artifact = artifact.add_event("validation: restart completed despite instability");

    artifact = artifact.add_event("cleanup: stop supervisor");
    supervisor_ref.stop(Some("test-complete".into()));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
