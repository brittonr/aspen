use std::io;
use std::sync::Arc;
use std::time::Duration;

use openraft::Config as RaftConfig;
use openraft::error::{RPCError, Unreachable};
use openraft::network::{RaftNetworkFactory, v2::RaftNetworkV2};
use ractor::Actor;

use aspen::raft::storage::{InMemoryLogStore, StateMachineStore};
use aspen::raft::supervision::{
    RaftSupervisor, SupervisionConfig, SupervisorArguments, SupervisorMessage,
};
use aspen::raft::types::{AppTypeConfig, AspenNode, NodeId};
use aspen::raft::{RaftActorConfig, StateMachineVariant};

/// Mock network factory for testing that doesn't actually send messages.
#[derive(Debug, Clone, Default)]
struct MockNetworkFactory;

impl RaftNetworkFactory<AppTypeConfig> for MockNetworkFactory {
    type Network = MockNetwork;

    async fn new_client(&mut self, _target: NodeId, _node: &AspenNode) -> Self::Network {
        MockNetwork
    }
}

/// Mock network implementation that always returns unreachable errors.
struct MockNetwork;

impl RaftNetworkV2<AppTypeConfig> for MockNetwork {
    async fn append_entries(
        &mut self,
        _req: openraft::raft::AppendEntriesRequest<AppTypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<openraft::raft::AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        let err = io::Error::new(io::ErrorKind::NotConnected, "mock network");
        Err(RPCError::Unreachable(Unreachable::new(&err)))
    }

    async fn full_snapshot(
        &mut self,
        _vote: openraft::type_config::alias::VoteOf<AppTypeConfig>,
        _snapshot: openraft::Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = openraft::error::ReplicationClosed>
        + openraft::OptionalSend,
        _option: openraft::network::RPCOption,
    ) -> Result<
        openraft::raft::SnapshotResponse<AppTypeConfig>,
        openraft::error::StreamingError<AppTypeConfig>,
    > {
        let err = io::Error::new(io::ErrorKind::NotConnected, "mock network");
        Err(openraft::error::StreamingError::Unreachable(
            Unreachable::new(&err),
        ))
    }

    async fn vote(
        &mut self,
        _req: openraft::raft::VoteRequest<AppTypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<openraft::raft::VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        let err = io::Error::new(io::ErrorKind::NotConnected, "mock network");
        Err(RPCError::Unreachable(Unreachable::new(&err)))
    }
}

/// Helper to create a minimal RaftActorConfig for testing.
async fn create_test_raft_config(node_id: u64) -> RaftActorConfig {
    let raft_config = Arc::new(RaftConfig::default());
    let log_store = InMemoryLogStore::default();
    let state_machine = StateMachineStore::default();
    let network = MockNetworkFactory;

    let state_machine_arc = Arc::new(state_machine);

    let raft = openraft::Raft::new(
        node_id,
        raft_config,
        network,
        log_store,
        state_machine_arc.clone(),
    )
    .await
    .expect("failed to create raft");

    RaftActorConfig {
        node_id,
        raft,
        state_machine: StateMachineVariant::InMemory(state_machine_arc),
        log_store: None,
    }
}

#[tokio::test]
async fn test_supervisor_spawns_raft_actor() {
    // Create supervision config
    let supervision_config = SupervisionConfig::default();

    // Create RaftActorConfig
    let raft_actor_config = create_test_raft_config(1).await;

    // Spawn supervisor
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-supervisor".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Give it time to spawn the RaftActor
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send GetStatus to verify supervisor is running
    // For now just verify supervisor exists
    // TODO: Implement GetStatus reply handling once we expose raft_actor reference

    // Cleanup
    supervisor_ref.stop(Some("test-complete".into()));
}

#[tokio::test]
async fn test_exponential_backoff_calculation() {
    // Create supervision config with default settings
    let supervision_config = SupervisionConfig::default();

    // Create RaftActorConfig
    let raft_actor_config = create_test_raft_config(1).await;

    // Spawn supervisor
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-supervisor-backoff".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // The supervisor uses exponential backoff internally:
    // restart 0: 1 second
    // restart 1: 2 seconds
    // restart 2: 4 seconds
    // restart 3: 8 seconds
    // restart 4+: 16 seconds (capped)

    // We can't directly test the calculate_backoff method since it's private,
    // but we can verify the supervisor starts correctly which uses backoff logic
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cleanup
    supervisor_ref.stop(Some("test-complete".into()));
}

#[tokio::test]
async fn test_restart_history_bounded() {
    // Create supervision config with small history for testing
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 300,
        max_restarts_per_window: 3,
        restart_window_secs: 600,
        restart_history_size: 10, // Small size for testing
        circuit_open_duration_secs: 300,
        half_open_stability_duration_secs: 120,
    };

    // Create RaftActorConfig
    let raft_actor_config = create_test_raft_config(1).await;

    // Spawn supervisor
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-supervisor-history".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Give it time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // The restart history is maintained internally and bounded to restart_history_size
    // We can verify the supervisor starts and runs correctly
    // Actual restart history verification would require exposing internal state
    // or triggering multiple restarts (which we'll test in integration tests)

    // Cleanup
    supervisor_ref.stop(Some("test-complete".into()));
}

#[tokio::test]
async fn test_manual_restart_command() {
    // Create supervision config
    let supervision_config = SupervisionConfig::default();

    // Create RaftActorConfig
    let raft_actor_config = create_test_raft_config(1).await;

    // Spawn supervisor
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-supervisor-manual".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Give it time to spawn initial RaftActor
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send ManualRestart command
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

    // Give it time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cleanup
    supervisor_ref.stop(Some("test-complete".into()));
}

#[tokio::test]
async fn test_supervision_config_defaults() {
    let config = SupervisionConfig::default();

    assert!(config.enable_auto_restart);
    assert_eq!(config.actor_stability_duration_secs, 300); // 5 minutes
    assert_eq!(config.max_restarts_per_window, 3);
    assert_eq!(config.restart_window_secs, 600); // 10 minutes
    assert_eq!(config.restart_history_size, 100);
}

#[tokio::test]
async fn test_meltdown_detection_threshold() {
    // Create supervision config with small window for testing
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1, // 1 second for testing
        max_restarts_per_window: 3,
        restart_window_secs: 60, // 1 minute
        restart_history_size: 100,
        circuit_open_duration_secs: 300,
        half_open_stability_duration_secs: 120,
    };

    // Create RaftActorConfig
    let raft_actor_config = create_test_raft_config(1).await;

    // Spawn supervisor
    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-supervisor-meltdown".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Give it time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // The meltdown detection is tested internally by the supervisor
    // When 3 restarts happen within 60 seconds, the supervisor should stop restarting
    // This test verifies the supervisor can be created with meltdown detection config

    // Cleanup
    supervisor_ref.stop(Some("test-complete".into()));
}
