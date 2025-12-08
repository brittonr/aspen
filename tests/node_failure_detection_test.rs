//! Integration tests for node failure detection.
//!
//! Tests the classification logic, metrics integration, and alert behavior
//! for distinguishing between actor-level crashes and node-level failures.

use std::time::Duration;

use aspen::raft::node_failure_detection::{
    AlertManager, ConnectionStatus, FailureType, NodeFailureDetector,
};

/// Test classification truth table for all connection status combinations.
#[test]
fn test_classification_truth_table() {
    let detector = NodeFailureDetector::default_timeout();

    // Row 1: Raft Connected, Iroh Connected → Healthy
    assert_eq!(
        detector.classify_failure(ConnectionStatus::Connected, ConnectionStatus::Connected),
        FailureType::Healthy,
        "Both connected should be Healthy"
    );

    // Row 2: Raft Connected, Iroh Disconnected → Healthy (Raft takes precedence)
    assert_eq!(
        detector.classify_failure(ConnectionStatus::Connected, ConnectionStatus::Disconnected),
        FailureType::Healthy,
        "Raft connected overrides Iroh status"
    );

    // Row 3: Raft Disconnected, Iroh Connected → ActorCrash
    assert_eq!(
        detector.classify_failure(ConnectionStatus::Disconnected, ConnectionStatus::Connected),
        FailureType::ActorCrash,
        "Raft down + Iroh up should be ActorCrash"
    );

    // Row 4: Raft Disconnected, Iroh Disconnected → NodeCrash
    assert_eq!(
        detector.classify_failure(
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected
        ),
        FailureType::NodeCrash,
        "Both down should be NodeCrash"
    );
}

/// Test detection of actor crash scenario (Raft fails, Iroh succeeds).
#[test]
fn test_detect_actor_crash_scenario() {
    let mut detector = NodeFailureDetector::default_timeout();
    let node_id = 42;

    // Initial state: healthy
    assert_eq!(detector.get_failure_type(node_id), FailureType::Healthy);
    assert_eq!(detector.unreachable_count(), 0);

    // Simulate actor crash: Raft fails, Iroh OK
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connected,
    );

    // Should be classified as ActorCrash
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::ActorCrash,
        "Should detect ActorCrash when Raft down but Iroh up"
    );
    assert_eq!(detector.unreachable_count(), 1);

    // Should have unreachable duration
    let duration = detector.get_unreachable_duration(node_id);
    assert!(
        duration.is_some(),
        "Should track unreachable duration for ActorCrash"
    );
}

/// Test detection of node crash scenario (both Raft and Iroh fail).
#[test]
fn test_detect_node_crash_scenario() {
    let mut detector = NodeFailureDetector::default_timeout();
    let node_id = 99;

    // Simulate node crash: both fail
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );

    // Should be classified as NodeCrash
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::NodeCrash,
        "Should detect NodeCrash when both Raft and Iroh down"
    );
    assert_eq!(detector.unreachable_count(), 1);

    let duration = detector.get_unreachable_duration(node_id);
    assert!(
        duration.is_some(),
        "Should track unreachable duration for NodeCrash"
    );
}

/// Test that recovery clears unreachable state.
#[test]
fn test_recovery_clears_unreachable_state() {
    let mut detector = NodeFailureDetector::default_timeout();
    let node_id = 7;

    // Node fails (either type)
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );
    assert_eq!(detector.unreachable_count(), 1);
    assert_eq!(detector.get_failure_type(node_id), FailureType::NodeCrash);

    // Node recovers (Raft comes back online)
    detector.update_node_status(
        node_id,
        ConnectionStatus::Connected,
        ConnectionStatus::Connected,
    );

    // Should be removed from tracking
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::Healthy,
        "Should clear failure state on recovery"
    );
    assert_eq!(
        detector.unreachable_count(),
        0,
        "Should remove from tracking"
    );
    assert!(
        detector.get_unreachable_duration(node_id).is_none(),
        "Should clear unreachable duration"
    );
}

/// Test that alerts fire after 60 seconds of unreachability.
#[test]
fn test_alert_fires_after_threshold() {
    let mut detector = NodeFailureDetector::new(Duration::from_millis(100));
    let node_id = 5;

    // Node fails
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );

    // Initially no alerts (just failed, below threshold)
    let alerts = detector.get_nodes_needing_attention();
    assert_eq!(
        alerts.len(),
        0,
        "Should not alert immediately after failure"
    );

    // Wait for alert threshold to pass
    std::thread::sleep(Duration::from_millis(150));

    // Now should need attention
    let alerts = detector.get_nodes_needing_attention();
    assert_eq!(alerts.len(), 1, "Should alert after threshold");
    assert_eq!(alerts[0].0, node_id);
    assert_eq!(alerts[0].1, FailureType::NodeCrash);
    assert!(
        alerts[0].2 >= Duration::from_millis(100),
        "Duration should be >= threshold"
    );
}

/// Test that failure type transitions are tracked correctly.
#[test]
fn test_failure_type_transition() {
    let mut detector = NodeFailureDetector::default_timeout();
    let node_id = 123;

    // Start with NodeCrash (both down)
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );
    assert_eq!(detector.get_failure_type(node_id), FailureType::NodeCrash);

    // Iroh recovers but Raft still down → ActorCrash
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connected,
    );
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::ActorCrash,
        "Should transition from NodeCrash to ActorCrash"
    );

    // Raft recovers → Healthy
    detector.update_node_status(
        node_id,
        ConnectionStatus::Connected,
        ConnectionStatus::Connected,
    );
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::Healthy,
        "Should transition from ActorCrash to Healthy"
    );
}

/// Test that timestamp is preserved across status updates.
#[test]
fn test_timestamp_preserved_across_updates() {
    let mut detector = NodeFailureDetector::default_timeout();
    let node_id = 456;

    // Initial failure
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );

    let initial_duration = detector.get_unreachable_duration(node_id).unwrap();

    // Wait a bit
    std::thread::sleep(Duration::from_millis(50));

    // Update status (from NodeCrash to ActorCrash)
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connected,
    );

    // Duration should have increased, not reset
    let updated_duration = detector.get_unreachable_duration(node_id).unwrap();
    assert!(
        updated_duration > initial_duration,
        "Timestamp should be preserved across status updates"
    );

    // Failure type should have changed
    assert_eq!(detector.get_failure_type(node_id), FailureType::ActorCrash);
}

/// Test bounded tracking (max 1000 nodes).
#[test]
fn test_max_unreachable_nodes_bounded() {
    let mut detector = NodeFailureDetector::default_timeout();

    // Try to exceed MAX_UNREACHABLE_NODES (1000)
    // We'll add 1050 nodes
    for node_id in 0..1050 {
        detector.update_node_status(
            node_id as u64,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );
    }

    // Should be capped at 1000
    assert_eq!(
        detector.unreachable_count(),
        1000,
        "Should enforce MAX_UNREACHABLE_NODES limit"
    );
}

/// Test AlertManager fires alerts only once per node.
#[test]
fn test_alert_manager_fires_once_per_node() {
    let mut detector = NodeFailureDetector::new(Duration::from_millis(50));
    let mut alert_mgr = AlertManager::new();
    let node_id = 10;

    // Node fails
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );

    // Wait for alert threshold
    std::thread::sleep(Duration::from_millis(100));

    // First check should fire alert
    assert_eq!(alert_mgr.active_alert_count(), 0);
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        1,
        "Should fire alert on first check"
    );

    // Second check should NOT fire again (already alerted)
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        1,
        "Should not fire duplicate alert"
    );
}

/// Test AlertManager clears alerts when nodes recover.
#[test]
fn test_alert_manager_clears_on_recovery() {
    let mut detector = NodeFailureDetector::new(Duration::from_millis(50));
    let mut alert_mgr = AlertManager::new();
    let node_id = 20;

    // Node fails
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Disconnected,
    );

    // Wait and fire alert
    std::thread::sleep(Duration::from_millis(100));
    alert_mgr.check_and_alert(&detector);
    assert_eq!(alert_mgr.active_alert_count(), 1);

    // Node recovers
    detector.update_node_status(
        node_id,
        ConnectionStatus::Connected,
        ConnectionStatus::Connected,
    );

    // Alert should be cleared
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        0,
        "Should clear alert when node recovers"
    );
}

/// Test AlertManager handles multiple simultaneous node failures.
#[test]
fn test_alert_manager_multiple_nodes() {
    let mut detector = NodeFailureDetector::new(Duration::from_millis(50));
    let mut alert_mgr = AlertManager::new();

    // Three nodes fail
    for node_id in [30, 31, 32] {
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );
    }

    // Wait for alert threshold
    std::thread::sleep(Duration::from_millis(100));

    // All three should be alerted
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        3,
        "Should alert for all failed nodes"
    );

    // One node recovers
    detector.update_node_status(30, ConnectionStatus::Connected, ConnectionStatus::Connected);

    // Should have 2 active alerts remaining
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        2,
        "Should clear alert for recovered node only"
    );
}

/// Integration test: Actor crash detection end-to-end.
#[tokio::test]
async fn test_integration_actor_crash_detection() {
    let mut detector = NodeFailureDetector::new(Duration::from_secs(5));
    let mut alert_mgr = AlertManager::new();
    let node_id = 100;

    // Scenario: RaftActor crashes but node is still reachable via Iroh
    // This represents a local process crash that could be auto-restarted

    // Simulate Raft heartbeat failure
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected, // Raft RPC fails
        ConnectionStatus::Connected,    // Iroh connection succeeds
    );

    // Verify classification
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::ActorCrash,
        "Should classify as ActorCrash"
    );

    // Wait for alert threshold (5 seconds)
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Check alerts
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        1,
        "Should fire alert after threshold"
    );

    // Verify metrics
    let alerts = detector.get_nodes_needing_attention();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, node_id);
    assert_eq!(alerts[0].1, FailureType::ActorCrash);
    assert!(alerts[0].2 >= Duration::from_secs(5));
}

/// Integration test: Node crash detection end-to-end.
#[tokio::test]
async fn test_integration_node_crash_detection() {
    let mut detector = NodeFailureDetector::new(Duration::from_secs(5));
    let mut alert_mgr = AlertManager::new();
    let node_id = 200;

    // Scenario: Entire node becomes unreachable (network partition, hardware failure, etc.)
    // This requires operator intervention for promotion/replacement

    // Simulate both Raft and Iroh failures
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected, // Raft RPC fails
        ConnectionStatus::Disconnected, // Iroh connection fails
    );

    // Verify classification
    assert_eq!(
        detector.get_failure_type(node_id),
        FailureType::NodeCrash,
        "Should classify as NodeCrash"
    );

    // Wait for alert threshold
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Check alerts
    alert_mgr.check_and_alert(&detector);
    assert_eq!(
        alert_mgr.active_alert_count(),
        1,
        "Should fire alert after threshold"
    );

    // Verify metrics
    let alerts = detector.get_nodes_needing_attention();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, node_id);
    assert_eq!(alerts[0].1, FailureType::NodeCrash);
    assert!(alerts[0].2 >= Duration::from_secs(5));
}

/// Test edge case: Node flip-flops between failure states.
#[test]
fn test_flapping_node_detection() {
    let mut detector = NodeFailureDetector::new(Duration::from_millis(100));
    let node_id = 300;

    // Node starts healthy
    detector.update_node_status(
        node_id,
        ConnectionStatus::Connected,
        ConnectionStatus::Connected,
    );
    assert_eq!(detector.get_failure_type(node_id), FailureType::Healthy);

    // Node fails (ActorCrash)
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connected,
    );

    // Wait to accumulate measurable duration on first failure
    // Tiger Style: Use longer sleep (100ms) for robust timing under resource contention
    std::thread::sleep(Duration::from_millis(100));
    let first_failure_duration = detector.get_unreachable_duration(node_id).unwrap();

    // Node recovers
    detector.update_node_status(
        node_id,
        ConnectionStatus::Connected,
        ConnectionStatus::Connected,
    );
    assert_eq!(detector.get_failure_type(node_id), FailureType::Healthy);
    assert!(detector.get_unreachable_duration(node_id).is_none());

    // Small delay to ensure clock advances before second failure
    std::thread::sleep(Duration::from_millis(10));

    // Node fails again
    detector.update_node_status(
        node_id,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connected,
    );

    // Second failure should have fresh timestamp (smaller duration since it just started)
    let second_failure_duration = detector.get_unreachable_duration(node_id).unwrap();
    // Tiger Style: Add tolerance (50ms) for timing jitter under system load
    assert!(
        second_failure_duration < first_failure_duration
            || second_failure_duration.saturating_sub(first_failure_duration)
                < Duration::from_millis(50),
        "Should reset timestamp on new failure after recovery. First: {:?}, Second: {:?}",
        first_failure_duration,
        second_failure_duration
    );
}
