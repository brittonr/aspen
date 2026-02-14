//! Tests for the madsim_network module.

use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::types::NodeId;

// =========================================================================
// MadsimRaftRouter Tests
// =========================================================================

#[test]
fn test_router_new_creates_empty() {
    let router = MadsimRaftRouter::new();
    // The router should start with no nodes
    let nodes = router.nodes.lock();
    assert!(nodes.is_empty());
}

#[test]
fn test_router_default_creates_empty() {
    let router = MadsimRaftRouter::default();
    let nodes = router.nodes.lock();
    assert!(nodes.is_empty());
}

#[test]
fn test_router_is_node_failed_default_false() {
    let router = MadsimRaftRouter::new();
    // Unregistered nodes should not be marked as failed
    assert!(!router.is_node_failed(NodeId::new(1)));
    assert!(!router.is_node_failed(NodeId::new(999)));
}

#[test]
fn test_router_mark_node_failed_and_check() {
    let router = MadsimRaftRouter::new();

    // Initially not failed
    assert!(!router.is_node_failed(NodeId::new(1)));

    // Mark as failed
    router.mark_node_failed(NodeId::new(1), true);
    assert!(router.is_node_failed(NodeId::new(1)));

    // Other nodes still not failed
    assert!(!router.is_node_failed(NodeId::new(2)));

    // Unmark as failed
    router.mark_node_failed(NodeId::new(1), false);
    assert!(!router.is_node_failed(NodeId::new(1)));
}

// =========================================================================
// FailureInjector Tests
// =========================================================================

#[test]
fn test_failure_injector_new_creates_empty() {
    let injector = FailureInjector::new();
    // All maps should be empty
    assert!(injector.delays.lock().is_empty());
    assert!(injector.delay_ranges.lock().is_empty());
    assert!(injector.drops.lock().is_empty());
    assert!(injector.loss_rates.lock().is_empty());
    assert!(injector.clock_drifts.lock().is_empty());
}

#[test]
fn test_failure_injector_default_creates_empty() {
    let injector = FailureInjector::default();
    assert!(injector.delays.lock().is_empty());
}

#[test]
fn test_set_network_delay() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_network_delay(node1, node2, 100);

    let delays = injector.delays.lock();
    assert_eq!(delays.get(&(node1, node2)), Some(&100));
}

#[test]
fn test_set_network_delay_range() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_network_delay_range(node1, node2, 10, 50);

    let delay_ranges = injector.delay_ranges.lock();
    assert_eq!(delay_ranges.get(&(node1, node2)), Some(&(10, 50)));
}

#[test]
#[should_panic(expected = "min_ms must be <= max_ms")]
fn test_set_network_delay_range_invalid() {
    let injector = FailureInjector::new();
    // min > max should panic
    injector.set_network_delay_range(NodeId::new(1), NodeId::new(2), 100, 10);
}

#[test]
fn test_set_packet_loss_rate() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_packet_loss_rate(node1, node2, 0.5);

    let loss_rates = injector.loss_rates.lock();
    assert_eq!(loss_rates.get(&(node1, node2)), Some(&0.5));
}

#[test]
#[should_panic(expected = "loss rate must be between 0.0 and 1.0")]
fn test_set_packet_loss_rate_negative_invalid() {
    let injector = FailureInjector::new();
    injector.set_packet_loss_rate(NodeId::new(1), NodeId::new(2), -0.1);
}

#[test]
#[should_panic(expected = "loss rate must be between 0.0 and 1.0")]
fn test_set_packet_loss_rate_over_one_invalid() {
    let injector = FailureInjector::new();
    injector.set_packet_loss_rate(NodeId::new(1), NodeId::new(2), 1.1);
}

#[test]
fn test_set_message_drop() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    // Initially no drops
    assert!(!injector.should_drop_message(node1, node2));

    injector.set_message_drop(node1, node2, true);

    // Now should drop
    assert!(injector.should_drop_message(node1, node2));

    // Reverse direction not affected
    assert!(!injector.should_drop_message(node2, node1));

    // Clear the drop
    injector.set_message_drop(node1, node2, false);
    assert!(!injector.should_drop_message(node1, node2));
}

#[test]
fn test_set_clock_drift() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);

    // No initial drift
    assert_eq!(injector.get_clock_drift(node1), None);

    // Set positive drift (fast clock)
    injector.set_clock_drift(node1, 100);
    assert_eq!(injector.get_clock_drift(node1), Some(100));

    // Set negative drift (slow clock)
    injector.set_clock_drift(node1, -50);
    assert_eq!(injector.get_clock_drift(node1), Some(-50));

    // Set to zero removes the entry
    injector.set_clock_drift(node1, 0);
    assert_eq!(injector.get_clock_drift(node1), None);
}

#[test]
fn test_clear_clock_drift() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);

    injector.set_clock_drift(node1, 100);
    assert!(injector.get_clock_drift(node1).is_some());

    injector.clear_clock_drift(node1);
    assert!(injector.get_clock_drift(node1).is_none());
}

#[test]
fn test_get_network_delay_with_fixed_delay() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    // No delay configured
    assert!(injector.get_network_delay(node1, node2).is_none());

    // Set fixed delay
    injector.set_network_delay(node1, node2, 100);
    let delay = injector.get_network_delay(node1, node2);
    assert_eq!(delay, Some(Duration::from_millis(100)));
}

#[test]
fn test_get_network_delay_with_clock_drift_positive_source() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    // Positive drift on source adds delay to outgoing
    injector.set_clock_drift(node1, 50);
    let delay = injector.get_network_delay(node1, node2);
    assert_eq!(delay, Some(Duration::from_millis(50)));
}

#[test]
fn test_get_network_delay_with_clock_drift_negative_target() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    // Negative drift on target adds delay to incoming
    injector.set_clock_drift(node2, -30);
    let delay = injector.get_network_delay(node1, node2);
    assert_eq!(delay, Some(Duration::from_millis(30)));
}

#[test]
fn test_get_network_delay_combines_fixed_and_drift() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_network_delay(node1, node2, 100);
    injector.set_clock_drift(node1, 50); // Positive drift on source

    let delay = injector.get_network_delay(node1, node2);
    assert_eq!(delay, Some(Duration::from_millis(150))); // 100 + 50
}

#[test]
fn test_clear_all() {
    let injector = FailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    // Set up various configurations
    injector.set_network_delay(node1, node2, 100);
    injector.set_network_delay_range(node1, node2, 10, 50);
    injector.set_message_drop(node1, node2, true);
    injector.set_packet_loss_rate(node1, node2, 0.5);
    injector.set_clock_drift(node1, 100);

    // Clear all
    injector.clear_all();

    // Verify everything is empty
    assert!(injector.delays.lock().is_empty());
    assert!(injector.delay_ranges.lock().is_empty());
    assert!(injector.drops.lock().is_empty());
    assert!(injector.loss_rates.lock().is_empty());
    assert!(injector.clock_drifts.lock().is_empty());
}

// =========================================================================
// ByzantineFailureInjector Tests
// =========================================================================

#[test]
fn test_byzantine_injector_new() {
    let injector = ByzantineFailureInjector::new();
    assert!(injector.links.lock().is_empty());
    assert!(injector.corruption_counts.lock().is_empty());
}

#[test]
fn test_byzantine_injector_default() {
    let injector = ByzantineFailureInjector::default();
    assert!(injector.links.lock().is_empty());
}

#[test]
fn test_set_byzantine_mode() {
    let injector = ByzantineFailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);

    let links = injector.links.lock();
    let configs = links.get(&(node1, node2)).expect("should have configs");
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0], (ByzantineCorruptionMode::FlipVote, 0.5));
}

#[test]
fn test_set_byzantine_mode_multiple_modes() {
    let injector = ByzantineFailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);
    injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::IncrementTerm, 0.3);

    let links = injector.links.lock();
    let configs = links.get(&(node1, node2)).expect("should have configs");
    assert_eq!(configs.len(), 2);
}

#[test]
fn test_set_byzantine_mode_updates_existing() {
    let injector = ByzantineFailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);
    injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.8);

    let links = injector.links.lock();
    let configs = links.get(&(node1, node2)).expect("should have configs");
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0], (ByzantineCorruptionMode::FlipVote, 0.8));
}

#[test]
#[should_panic(expected = "probability must be between 0.0 and 1.0")]
fn test_set_byzantine_mode_invalid_probability() {
    let injector = ByzantineFailureInjector::new();
    injector.set_byzantine_mode(NodeId::new(1), NodeId::new(2), ByzantineCorruptionMode::FlipVote, 1.5);
}

#[test]
fn test_byzantine_clear_all() {
    let injector = ByzantineFailureInjector::new();
    let node1 = NodeId::new(1);
    let node2 = NodeId::new(2);

    injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);
    assert!(!injector.links.lock().is_empty());

    injector.clear_all();
    assert!(injector.links.lock().is_empty());
}

#[test]
fn test_get_corruption_stats_empty() {
    let injector = ByzantineFailureInjector::new();
    let stats = injector.get_corruption_stats();
    assert!(stats.is_empty());
}

#[test]
fn test_total_corruptions_empty() {
    let injector = ByzantineFailureInjector::new();
    assert_eq!(injector.total_corruptions(), 0);
}

// =========================================================================
// ByzantineCorruptionMode Tests
// =========================================================================

#[test]
fn test_byzantine_corruption_mode_debug() {
    assert_eq!(format!("{:?}", ByzantineCorruptionMode::FlipVote), "FlipVote");
    assert_eq!(format!("{:?}", ByzantineCorruptionMode::IncrementTerm), "IncrementTerm");
    assert_eq!(format!("{:?}", ByzantineCorruptionMode::DuplicateMessage), "DuplicateMessage");
    assert_eq!(format!("{:?}", ByzantineCorruptionMode::ClearEntries), "ClearEntries");
}

#[test]
fn test_byzantine_corruption_mode_clone() {
    let mode = ByzantineCorruptionMode::FlipVote;
    let cloned = mode;
    assert_eq!(mode, cloned);
}

#[test]
fn test_byzantine_corruption_mode_eq() {
    assert_eq!(ByzantineCorruptionMode::FlipVote, ByzantineCorruptionMode::FlipVote);
    assert_ne!(ByzantineCorruptionMode::FlipVote, ByzantineCorruptionMode::IncrementTerm);
}

#[test]
fn test_byzantine_corruption_mode_hash() {
    use std::hash::Hash;
    use std::hash::Hasher;

    fn hash_mode(mode: ByzantineCorruptionMode) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        mode.hash(&mut hasher);
        hasher.finish()
    }

    // Same mode should hash the same
    assert_eq!(hash_mode(ByzantineCorruptionMode::FlipVote), hash_mode(ByzantineCorruptionMode::FlipVote));

    // Different modes should (likely) hash differently
    assert_ne!(hash_mode(ByzantineCorruptionMode::FlipVote), hash_mode(ByzantineCorruptionMode::IncrementTerm));
}

// =========================================================================
// MadsimNetworkFactory Tests
// =========================================================================

#[test]
fn test_network_factory_creation() {
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());
    let node_id = NodeId::new(1);

    let factory = MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());
    assert_eq!(factory.source_node_id, node_id);
}

#[test]
fn test_network_factory_clone() {
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());
    let node_id = NodeId::new(1);

    let factory = MadsimNetworkFactory::new(node_id, router, injector);
    let cloned = factory.clone();

    assert_eq!(factory.source_node_id, cloned.source_node_id);
}
