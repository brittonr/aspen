//! FailureInjector - chaos testing for madsim simulations.

use std::collections::HashMap;
use std::time::Duration;

use parking_lot::Mutex as SyncMutex;

use crate::types::NodeId;

/// Failure injector for chaos testing in madsim simulations.
///
/// FailureInjector controls deterministic failure injection including:
/// - Message drops (network failures)
/// - Network delays (latency simulation)
/// - Range-based delays with jitter
/// - Packet loss rates (probabilistic drops)
/// - Node failures (crash simulation)
/// - Clock drift simulation (asymmetric delays)
///
/// Tiger Style: All delays/timeouts are explicitly u64 milliseconds.
pub struct FailureInjector {
    /// Network delay configuration (source, target) -> delay_ms
    pub(super) delays: SyncMutex<HashMap<(NodeId, NodeId), u64>>,
    /// Range-based delay configuration (source, target) -> (min_ms, max_ms)
    pub(super) delay_ranges: SyncMutex<HashMap<(NodeId, NodeId), (u64, u64)>>,
    /// Message drop configuration (source, target) -> should_drop
    pub(super) drops: SyncMutex<HashMap<(NodeId, NodeId), bool>>,
    /// Packet loss rate configuration (source, target) -> loss_rate (0.0-1.0)
    pub(super) loss_rates: SyncMutex<HashMap<(NodeId, NodeId), f64>>,
    /// Clock drift simulation: node_id -> drift_ms (signed)
    ///
    /// Simulates clock drift by adding asymmetric delays:
    /// - Positive drift (fast clock): Adds delay to OUTGOING messages from this node (simulates the
    ///   node's perception that time has passed faster)
    /// - Negative drift (slow clock): Adds delay to INCOMING messages to this node (simulates the
    ///   node responding late relative to others)
    ///
    /// Note: Madsim uses global virtual time, so we simulate drift effects through
    /// delays rather than actual clock manipulation. This approach effectively tests
    /// how Raft handles nodes that appear to be on different timelines.
    pub(super) clock_drifts: SyncMutex<HashMap<NodeId, i64>>,
}

impl FailureInjector {
    /// Create a new failure injector with no failures configured.
    pub fn new() -> Self {
        Self {
            delays: SyncMutex::new(HashMap::new()),
            delay_ranges: SyncMutex::new(HashMap::new()),
            drops: SyncMutex::new(HashMap::new()),
            loss_rates: SyncMutex::new(HashMap::new()),
            clock_drifts: SyncMutex::new(HashMap::new()),
        }
    }

    /// Configure network delay between two nodes (in milliseconds).
    ///
    /// Tiger Style: Explicit u64 milliseconds, not Duration directly.
    pub fn set_network_delay(&self, source: NodeId, target: NodeId, delay_ms: u64) {
        let mut delays = self.delays.lock();
        delays.insert((source, target), delay_ms);
    }

    /// Configure range-based network delay between two nodes.
    ///
    /// Delay will be uniformly sampled from [min_ms, max_ms] for each message.
    /// This simulates realistic network jitter patterns.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 1-27ms latency like MadRaft
    /// injector.set_network_delay_range(node1, node2, 1, 27);
    /// ```
    pub fn set_network_delay_range(&self, source: NodeId, target: NodeId, min_ms: u64, max_ms: u64) {
        assert!(min_ms <= max_ms, "min_ms must be <= max_ms");
        let mut delay_ranges = self.delay_ranges.lock();
        delay_ranges.insert((source, target), (min_ms, max_ms));
    }

    /// Configure packet loss rate between two nodes.
    ///
    /// Rate should be between 0.0 (no loss) and 1.0 (100% loss).
    /// Messages are dropped probabilistically based on this rate.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 10% packet loss
    /// injector.set_packet_loss_rate(node1, node2, 0.1);
    /// ```
    pub fn set_packet_loss_rate(&self, source: NodeId, target: NodeId, rate: f64) {
        assert!((0.0..=1.0).contains(&rate), "loss rate must be between 0.0 and 1.0");
        let mut loss_rates = self.loss_rates.lock();
        loss_rates.insert((source, target), rate);
    }

    /// Configure message drops between two nodes.
    ///
    /// When enabled, all messages from source to target will be dropped.
    pub fn set_message_drop(&self, source: NodeId, target: NodeId, should_drop: bool) {
        let mut drops = self.drops.lock();
        drops.insert((source, target), should_drop);
    }

    /// Configure clock drift for a node (in milliseconds, signed).
    ///
    /// Clock drift is simulated by adding asymmetric delays to messages:
    /// - Positive drift (fast clock): Delays OUTGOING messages from this node
    /// - Negative drift (slow clock): Delays INCOMING messages to this node
    ///
    /// This effectively simulates how Raft behaves when a node's clock runs
    /// faster or slower than other nodes in the cluster.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Node 1 has a clock that's 100ms "fast" - its heartbeats arrive late
    /// // from the perspective of other nodes
    /// injector.set_clock_drift(1, 100);
    ///
    /// // Node 2 has a clock that's 50ms "slow" - messages to it appear delayed
    /// injector.set_clock_drift(2, -50);
    /// ```
    ///
    /// # Arguments
    ///
    /// * `node_id` - The node to configure drift for
    /// * `drift_ms` - Signed drift in milliseconds. Positive = fast clock, negative = slow clock.
    pub fn set_clock_drift(&self, node_id: NodeId, drift_ms: i64) {
        let mut drifts = self.clock_drifts.lock();
        if drift_ms == 0 {
            drifts.remove(&node_id);
        } else {
            drifts.insert(node_id, drift_ms);
        }
    }

    /// Clear clock drift for a specific node.
    pub fn clear_clock_drift(&self, node_id: NodeId) {
        let mut drifts = self.clock_drifts.lock();
        drifts.remove(&node_id);
    }

    /// Get the configured clock drift for a node.
    pub fn get_clock_drift(&self, node_id: NodeId) -> Option<i64> {
        let drifts = self.clock_drifts.lock();
        drifts.get(&node_id).copied()
    }

    /// Check if a message should be dropped.
    ///
    /// Considers both explicit drops and probabilistic loss rates.
    pub(crate) fn should_drop_message(&self, source: NodeId, target: NodeId) -> bool {
        // Check explicit drops first
        {
            let drops = self.drops.lock();
            if drops.get(&(source, target)).copied().unwrap_or(false) {
                return true;
            }
        }

        // Check packet loss rate
        {
            let loss_rates = self.loss_rates.lock();
            if let Some(&rate) = loss_rates.get(&(source, target))
                && rate > 0.0
            {
                // Use madsim's deterministic random
                let random_value: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
                if random_value < rate {
                    return true;
                }
            }
        }

        false
    }

    /// Get the configured network delay for a message, if any.
    ///
    /// Checks range-based delays first, then fixed delays, then clock drift effects.
    /// For range-based delays, samples uniformly from the range.
    ///
    /// Clock drift is applied as additional delay:
    /// - Source with positive drift (fast clock): Add delay to simulate late arrival
    /// - Target with negative drift (slow clock): Add delay to simulate slow response
    pub(crate) fn get_network_delay(&self, source: NodeId, target: NodeId) -> Option<Duration> {
        let mut total_delay_ms: u64 = 0;
        let mut has_delay = false;

        // Check range-based delays first
        {
            let delay_ranges = self.delay_ranges.lock();
            if let Some(&(min_ms, max_ms)) = delay_ranges.get(&(source, target)) {
                let delay_ms = if min_ms == max_ms {
                    min_ms
                } else {
                    // Sample uniformly using madsim's deterministic random
                    min_ms + (madsim::rand::random::<u64>() % (max_ms - min_ms + 1))
                };
                total_delay_ms = delay_ms;
                has_delay = true;
            }
        }

        // Fall back to fixed delays if no range delay
        if !has_delay {
            let delays = self.delays.lock();
            if let Some(&delay_ms) = delays.get(&(source, target)) {
                total_delay_ms = delay_ms;
                has_delay = true;
            }
        }

        // Add clock drift effects
        // Positive drift on source: messages from this node appear delayed (fast clock)
        // Negative drift on target: messages to this node appear delayed (slow clock)
        {
            let drifts = self.clock_drifts.lock();

            // Source with positive drift: add delay to outgoing messages
            if let Some(&drift_ms) = drifts.get(&source)
                && drift_ms > 0
            {
                total_delay_ms = total_delay_ms.saturating_add(drift_ms as u64);
                has_delay = true;
            }

            // Target with negative drift: add delay to incoming messages
            if let Some(&drift_ms) = drifts.get(&target)
                && drift_ms < 0
            {
                total_delay_ms = total_delay_ms.saturating_add(drift_ms.unsigned_abs());
                has_delay = true;
            }
        }

        if has_delay && total_delay_ms > 0 {
            Some(Duration::from_millis(total_delay_ms))
        } else {
            None
        }
    }

    /// Clear all failure injection configuration.
    pub fn clear_all(&self) {
        let mut delays = self.delays.lock();
        let mut delay_ranges = self.delay_ranges.lock();
        let mut drops = self.drops.lock();
        let mut loss_rates = self.loss_rates.lock();
        let mut clock_drifts = self.clock_drifts.lock();
        delays.clear();
        delay_ranges.clear();
        drops.clear();
        loss_rates.clear();
        clock_drifts.clear();
    }
}

impl Default for FailureInjector {
    fn default() -> Self {
        Self::new()
    }
}
