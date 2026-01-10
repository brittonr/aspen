//! Bridge for system hook events (LeaderElected, HealthChanged, SnapshotCreated/Installed).
//!
//! This module monitors Raft metrics changes and emits hook events when:
//! - A new leader is elected (LeaderElected)
//! - Node health status changes (HealthChanged)
//!
//! # Architecture
//!
//! The system events bridge runs as a background task that polls Raft metrics
//! at a configurable interval. When state transitions are detected, it creates
//! and dispatches the appropriate hook events.
//!
//! # Tiger Style
//!
//! - Fixed polling interval (default: 1 second) prevents CPU spinning
//! - Non-blocking dispatch via tokio::spawn
//! - Graceful shutdown via CancellationToken

use std::sync::Arc;

use aspen_core::ClusterController;
use aspen_core::ControlPlaneError;
use aspen_core::NodeState;
use aspen_hooks::HealthChangedPayload;
use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use aspen_hooks::LeaderElectedPayload;
use aspen_raft::node::RaftNode;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Configuration for the system events bridge.
#[derive(Debug, Clone)]
pub struct SystemEventsBridgeConfig {
    /// Polling interval for metrics in milliseconds.
    pub poll_interval_ms: u64,
}

impl Default for SystemEventsBridgeConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000, // 1 second default
        }
    }
}

/// State tracked between polling intervals.
struct TrackedState {
    /// Last known leader ID.
    leader_id: Option<u64>,
    /// Last known term.
    term: u64,
    /// Last known health state.
    health_state: NodeState,
    /// Last known healthy flag.
    healthy: bool,
}

impl Default for TrackedState {
    fn default() -> Self {
        Self {
            leader_id: None,
            term: 0,
            health_state: NodeState::Follower,
            healthy: true,
        }
    }
}

/// Run the system events bridge that monitors Raft metrics for state changes.
///
/// This task polls Raft metrics and emits hook events when:
/// - Leader changes (LeaderElected)
/// - Health status changes (HealthChanged)
///
/// # Non-blocking Guarantee
///
/// Each event dispatch is spawned as a separate task to ensure the bridge
/// never blocks on slow handlers.
///
/// # Task Tracking (Tiger Style)
///
/// Uses `JoinSet` to track spawned dispatch tasks. On shutdown, waits for
/// in-flight dispatches to complete.
pub async fn run_system_events_bridge(
    raft_node: Arc<RaftNode>,
    service: Arc<HookService>,
    node_id: u64,
    config: SystemEventsBridgeConfig,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(config.poll_interval_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut state = TrackedState::default();

    // Tiger Style: Track spawned tasks with JoinSet for graceful shutdown
    let mut dispatch_tasks: JoinSet<()> = JoinSet::new();

    // Initialize state from current metrics
    if let Ok(metrics) = raft_node.get_metrics().await {
        state.leader_id = metrics.current_leader;
        state.term = metrics.current_term;
        state.health_state = metrics.state;
        state.healthy = metrics.state.is_healthy();
    }

    info!(node_id, poll_interval_ms = config.poll_interval_ms, "system events bridge started");

    loop {
        // Drain completed tasks
        while dispatch_tasks.try_join_next().is_some() {}

        tokio::select! {
            _ = cancel.cancelled() => {
                debug!(node_id, "system events bridge shutting down");
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = check_and_emit_events(&raft_node, &service, node_id, &mut state, &mut dispatch_tasks).await {
                    warn!(node_id, error = ?e, "failed to check metrics for system events");
                }
            }
        }
    }

    // Tiger Style: Wait for in-flight dispatches to complete on shutdown
    let in_flight = dispatch_tasks.len();
    if in_flight > 0 {
        info!(node_id, in_flight, "waiting for in-flight system event dispatches to complete");
        while dispatch_tasks.join_next().await.is_some() {}
        debug!(node_id, "all in-flight system event dispatches completed");
    }

    debug!(node_id, "system events bridge stopped");
}

/// Check Raft metrics for state changes and emit hook events.
///
/// Tiger Style: Uses provided JoinSet to track spawned dispatch tasks.
async fn check_and_emit_events(
    raft_node: &Arc<RaftNode>,
    service: &Arc<HookService>,
    node_id: u64,
    state: &mut TrackedState,
    dispatch_tasks: &mut JoinSet<()>,
) -> Result<(), ControlPlaneError> {
    let metrics = raft_node.get_metrics().await?;

    // Check for leader change
    if metrics.current_leader != state.leader_id || metrics.current_term != state.term {
        // Leader has changed
        if let Some(new_leader) = metrics.current_leader {
            let event = create_leader_elected_event(
                node_id,
                new_leader,
                state.leader_id,
                metrics.current_term,
                &metrics.voters,
                &metrics.learners,
            );

            let service_clone = Arc::clone(service);
            // Tiger Style: Track task in JoinSet
            dispatch_tasks.spawn(async move {
                if let Err(e) = service_clone.dispatch(&event).await {
                    warn!(error = ?e, "failed to dispatch LeaderElected event");
                }
            });

            info!(
                node_id,
                new_leader,
                previous_leader = ?state.leader_id,
                term = metrics.current_term,
                "leader elected event emitted"
            );
        }

        state.leader_id = metrics.current_leader;
        state.term = metrics.current_term;
    }

    // Check for health state change
    let current_healthy = metrics.state.is_healthy();
    if metrics.state != state.health_state || current_healthy != state.healthy {
        let event =
            create_health_changed_event(node_id, &state.health_state, &metrics.state, state.healthy, current_healthy);

        let service_clone = Arc::clone(service);
        // Tiger Style: Track task in JoinSet
        dispatch_tasks.spawn(async move {
            if let Err(e) = service_clone.dispatch(&event).await {
                warn!(error = ?e, "failed to dispatch HealthChanged event");
            }
        });

        info!(
            node_id,
            previous_state = ?state.health_state,
            current_state = ?metrics.state,
            was_healthy = state.healthy,
            now_healthy = current_healthy,
            "health changed event emitted"
        );

        state.health_state = metrics.state;
        state.healthy = current_healthy;
    }

    Ok(())
}

/// Create a LeaderElected hook event.
fn create_leader_elected_event(
    node_id: u64,
    new_leader_id: u64,
    previous_leader_id: Option<u64>,
    term: u64,
    voters: &[u64],
    learners: &[u64],
) -> HookEvent {
    let payload = LeaderElectedPayload {
        new_leader_id,
        previous_leader_id,
        term,
        voters: voters.to_vec(),
        learners: learners.to_vec(),
    };

    HookEvent::new(HookEventType::LeaderElected, node_id, serde_json::to_value(payload).unwrap_or_default())
}

/// Create a HealthChanged hook event.
fn create_health_changed_event(
    node_id: u64,
    previous_state: &NodeState,
    current_state: &NodeState,
    was_healthy: bool,
    now_healthy: bool,
) -> HookEvent {
    let reason = if was_healthy && !now_healthy {
        Some("node became unhealthy".to_string())
    } else if !was_healthy && now_healthy {
        Some("node recovered".to_string())
    } else {
        Some(format!("state transition: {:?} -> {:?}", previous_state, current_state))
    };

    let payload = HealthChangedPayload {
        previous_state: format!("{:?}", previous_state),
        current_state: format!("{:?}", current_state),
        reason,
    };

    HookEvent::new(HookEventType::HealthChanged, node_id, serde_json::to_value(payload).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SystemEventsBridgeConfig::default();
        assert_eq!(config.poll_interval_ms, 1000);
    }

    #[test]
    fn test_leader_elected_event_creation() {
        let event = create_leader_elected_event(1, 2, Some(1), 5, &[1, 2, 3], &[4]);

        assert_eq!(event.event_type, HookEventType::LeaderElected);
        assert_eq!(event.node_id, 1);

        let payload: LeaderElectedPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.new_leader_id, 2);
        assert_eq!(payload.previous_leader_id, Some(1));
        assert_eq!(payload.term, 5);
        assert_eq!(payload.voters, vec![1, 2, 3]);
        assert_eq!(payload.learners, vec![4]);
    }

    #[test]
    fn test_health_changed_event_creation() {
        let event = create_health_changed_event(1, &NodeState::Leader, &NodeState::Follower, true, true);

        assert_eq!(event.event_type, HookEventType::HealthChanged);
        assert_eq!(event.node_id, 1);

        let payload: HealthChangedPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.previous_state, "Leader");
        assert_eq!(payload.current_state, "Follower");
        assert!(payload.reason.is_some());
    }

    #[test]
    fn test_health_changed_unhealthy_transition() {
        let event = create_health_changed_event(1, &NodeState::Leader, &NodeState::Shutdown, true, false);

        let payload: HealthChangedPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.reason, Some("node became unhealthy".to_string()));
    }

    #[test]
    fn test_health_changed_recovery() {
        let event = create_health_changed_event(1, &NodeState::Shutdown, &NodeState::Follower, false, true);

        let payload: HealthChangedPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.reason, Some("node recovered".to_string()));
    }
}
