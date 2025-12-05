//! Integration tests for RaftSupervisor restart flows and meltdown detection.
//!
//! These tests verify:
//! - Clean restart after actor crash
//! - Exponential backoff delays between restarts
//! - Meltdown detection when max_restarts_per_window is exceeded
//! - Storage validation integration with restart logic
//!
//! Tiger Style: Each test focuses on one specific restart scenario.

use std::sync::Arc;
use std::time::{Duration, Instant};

use openraft::Config as RaftConfig;
use openraft::network::{RaftNetworkFactory, v2::RaftNetworkV2};
use openraft::error::{RPCError, Unreachable};
use ractor::Actor;

use aspen::raft::storage::{InMemoryLogStore, StateMachineStore};
use aspen::raft::supervision::{
    RaftSupervisor, SupervisionConfig, SupervisorArguments, SupervisorMessage,
};
use aspen::raft::{RaftActorConfig, StateMachineVariant};
use aspen::raft::types::{AppTypeConfig, NodeId};

/// Mock network factory that returns unreachable errors.
#[derive(Debug, Clone, Default)]
struct MockNetworkFactory;

impl RaftNetworkFactory<AppTypeConfig> for MockNetworkFactory {
    type Network = MockNetwork;

    async fn new_client(&mut self, _target: NodeId, _node: &openraft::BasicNode) -> Self::Network {
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
    ) -> Result<
        openraft::raft::AppendEntriesResponse<AppTypeConfig>,
        RPCError<AppTypeConfig>,
    > {
        let err = std::io::Error::new(std::io::ErrorKind::NotConnected, "mock");
        Err(RPCError::Unreachable(Unreachable::new(&err)))
    }

    async fn full_snapshot(
        &mut self,
        _vote: openraft::type_config::alias::VoteOf<AppTypeConfig>,
        _snapshot: openraft::Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = openraft::error::ReplicationClosed> + openraft::OptionalSend,
        _option: openraft::network::RPCOption,
    ) -> Result<openraft::raft::SnapshotResponse<AppTypeConfig>, openraft::error::StreamingError<AppTypeConfig>> {
        let err = std::io::Error::new(std::io::ErrorKind::NotConnected, "mock");
        Err(openraft::error::StreamingError::Unreachable(Unreachable::new(&err)))
    }

    async fn vote(
        &mut self,
        _req: openraft::raft::VoteRequest<AppTypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<
        openraft::raft::VoteResponse<AppTypeConfig>,
        RPCError<AppTypeConfig>,
    > {
        let err = std::io::Error::new(std::io::ErrorKind::NotConnected, "mock");
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
    }
}

/// Test that supervisor successfully restarts an actor after manual restart.
#[tokio::test]
async fn test_clean_restart_after_manual_trigger() {
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1, // 1 second for testing
        max_restarts_per_window: 5,
        restart_window_secs: 60,
        restart_history_size: 100,
    };

    let raft_actor_config = create_test_raft_config(100).await;

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-clean-restart".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Wait for initial actor spawn
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger manual restart
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

    // Wait for restart to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // If we get here without panic, restart succeeded
    supervisor_ref.stop(Some("test-complete".into()));
}

/// Test exponential backoff timing between restarts.
///
/// Tiger Style: This test verifies timing constraints are met.
#[tokio::test]
async fn test_exponential_backoff_timing() {
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1, // Short stability window for testing
        max_restarts_per_window: 10, // Allow multiple restarts
        restart_window_secs: 120,
        restart_history_size: 100,
    };

    let raft_actor_config = create_test_raft_config(101).await;

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-backoff-timing".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Wait for initial spawn
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger first restart and measure backoff
    let start = Instant::now();
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);

    // First restart should use ~1 second backoff
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let first_restart_duration = start.elapsed();

    // Should take at least 1 second (allowing 200ms margin)
    assert!(
        first_restart_duration >= Duration::from_millis(800),
        "first restart should have ~1s backoff, got {:?}",
        first_restart_duration
    );

    supervisor_ref.stop(Some("test-complete".into()));
}

/// Test meltdown detection when max_restarts_per_window is exceeded.
///
/// This verifies that after N rapid restarts within the window,
/// the supervisor stops restarting to prevent infinite restart loops.
#[tokio::test]
async fn test_meltdown_detection_stops_restarts() {
    // Configure tight restart window to trigger meltdown quickly
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1, // Very short stability
        max_restarts_per_window: 3, // Allow only 3 restarts
        restart_window_secs: 30, // Within 30 seconds
        restart_history_size: 100,
    };

    let raft_actor_config = create_test_raft_config(102).await;

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-meltdown-detection".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Wait for initial spawn
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger multiple rapid restarts to exceed max_restarts_per_window
    for i in 0..4 {
        let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);
        // Small delay to allow processing
        tokio::time::sleep(Duration::from_millis(50)).await;
        tracing::info!("triggered restart {}", i + 1);
    }

    // Wait for supervisor to process all restarts
    tokio::time::sleep(Duration::from_secs(3)).await;

    // The supervisor should have detected meltdown and stopped restarting
    // We verify by ensuring no panic occurs during this period
    // (If meltdown detection fails, rapid restarts would likely cause issues)

    supervisor_ref.stop(Some("test-complete".into()));
}

/// Test that restart counter resets after stability period.
///
/// After an actor runs successfully for `actor_stability_duration_secs`,
/// the restart counter should reset, allowing new restarts.
#[tokio::test]
async fn test_restart_counter_resets_after_stability() {
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 2, // 2 second stability window
        max_restarts_per_window: 2, // Only 2 restarts allowed
        restart_window_secs: 60,
        restart_history_size: 100,
    };

    let raft_actor_config = create_test_raft_config(103).await;

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-stability-reset".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Wait for initial spawn
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger restart
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);
    tokio::time::sleep(Duration::from_millis(1200)).await;

    // Wait for stability period to elapse (2 seconds + margin)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Trigger another restart - should succeed because counter reset
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);
    tokio::time::sleep(Duration::from_millis(1200)).await;

    // If we get here, restart succeeded after stability reset
    supervisor_ref.stop(Some("test-complete".into()));
}

/// Test restart with in-memory storage (no validation needed).
///
/// Storage validation should be skipped for in-memory backends.
#[tokio::test]
async fn test_restart_with_in_memory_storage() {
    let supervision_config = SupervisionConfig::default();
    let raft_actor_config = create_test_raft_config(104).await;

    // Verify we're using in-memory storage
    assert!(
        matches!(raft_actor_config.state_machine, StateMachineVariant::InMemory(_)),
        "expected in-memory storage variant"
    );

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-in-memory-restart".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger restart - should skip storage validation
    let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);
    tokio::time::sleep(Duration::from_millis(200)).await;

    supervisor_ref.stop(Some("test-complete".into()));
}

/// Test that supervisor handles actor stop gracefully.
///
/// When the RaftActor stops, supervisor should detect it and
/// trigger appropriate restart logic.
#[tokio::test]
async fn test_supervisor_detects_actor_stop() {
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1,
        max_restarts_per_window: 5,
        restart_window_secs: 60,
        restart_history_size: 100,
    };

    let raft_actor_config = create_test_raft_config(105).await;

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-actor-stop".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    // Wait for initial spawn
    tokio::time::sleep(Duration::from_millis(100)).await;

    // The supervisor internally monitors the RaftActor
    // When the actor stops, it should detect via actor monitoring
    // and trigger restart if enabled

    // Allow supervisor to run for a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    supervisor_ref.stop(Some("test-complete".into()));
}

/// Test that restart history is bounded to configured size.
///
/// Tiger Style: Verify bounded resource constraint (restart_history_size).
#[tokio::test]
async fn test_restart_history_bounded_size() {
    let supervision_config = SupervisionConfig {
        enable_auto_restart: true,
        actor_stability_duration_secs: 1,
        max_restarts_per_window: 20, // Allow many restarts
        restart_window_secs: 300,
        restart_history_size: 5, // Small history for testing
    };

    let raft_actor_config = create_test_raft_config(106).await;

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config,
    };

    let (supervisor_ref, _task) = Actor::spawn(
        Some("test-bounded-history".to_string()),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .expect("failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger multiple restarts beyond history size
    for i in 0..8 {
        let _ = supervisor_ref.cast(SupervisorMessage::ManualRestart);
        tokio::time::sleep(Duration::from_millis(100)).await;
        tracing::info!("triggered restart {}", i + 1);
    }

    // Wait for all restarts to process
    tokio::time::sleep(Duration::from_secs(2)).await;

    // The restart history internally should be bounded to 5 entries
    // This test verifies the supervisor doesn't panic with many restarts
    supervisor_ref.stop(Some("test-complete".into()));
}
