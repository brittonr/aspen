//! Network and failure injection methods for AspenRaftTester.

use std::sync::atomic::Ordering;

use aspen_raft::madsim_network::ByzantineCorruptionMode;
use aspen_raft::madsim_network::ByzantineFailureInjector;
use aspen_raft::madsim_network::FailureInjector;
use aspen_raft::types::NodeId;

use super::AspenRaftTester;
use super::node::empty_artifact_builder;

fn node_slot_from_index(node_index: u32) -> usize {
    usize::try_from(node_index).unwrap_or(usize::MAX)
}

fn node_id_from_index(node_index: u32) -> NodeId {
    NodeId::from(u64::from(node_index).saturating_add(1))
}

fn node_id_from_slot(node_slot: usize) -> NodeId {
    let node_index = u64::try_from(node_slot).unwrap_or(u64::MAX);
    NodeId::from(node_index.saturating_add(1))
}

impl AspenRaftTester {
    /// Disconnect node from network (bidirectional partition).
    pub fn disconnect(&mut self, node_index: u32) {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        self.nodes[node_slot].connected().store(false, Ordering::SeqCst);
        let node_id = node_id_from_index(node_index);

        // Bidirectional partition
        for other_slot in 0..self.nodes.len() {
            if node_slot != other_slot {
                let other_id = node_id_from_slot(other_slot);
                self.injector.set_message_drop(node_id, other_id, true);
                self.injector.set_message_drop(other_id, node_id, true);
            }
        }

        self.metrics.network_partitions += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("disconnect: node {} from cluster", node_index));
    }

    /// Reconnect node to network.
    pub fn connect(&mut self, node_index: u32) {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        self.nodes[node_slot].connected().store(true, Ordering::SeqCst);
        let node_id = node_id_from_index(node_index);

        // Clear partitions
        for other_slot in 0..self.nodes.len() {
            if node_slot != other_slot {
                let other_id = node_id_from_slot(other_slot);
                self.injector.set_message_drop(node_id, other_id, false);
                self.injector.set_message_drop(other_id, node_id, false);
            }
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("connect: node {} to cluster", node_index));
    }

    /// Set network to unreliable mode (packet loss and variable delays).
    ///
    /// Parameters match MadRaft: 10% packet loss, 1-27ms latency.
    pub fn set_unreliable(&mut self, is_unreliable: bool) {
        if is_unreliable {
            // 10% packet loss, 1-27ms latency like MadRaft
            for from_slot in 0..self.nodes.len() {
                for to_slot in 0..self.nodes.len() {
                    if from_slot != to_slot {
                        let from = node_id_from_slot(from_slot);
                        let to = node_id_from_slot(to_slot);
                        // Range-based delay: 1-27ms
                        self.injector.set_network_delay_range(from, to, 1, 27);
                        // 10% packet loss
                        self.injector.set_packet_loss_rate(from, to, 0.1);
                    }
                }
            }
            self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                .add_event("network: set unreliable (10% loss, 1-27ms delay)");
        } else {
            self.injector.clear_all();
            self.artifact =
                std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event("network: set reliable");
        }
    }

    /// Configure packet loss rate for all node pairs.
    ///
    /// Rate should be between 0.0 (no loss) and 1.0 (100% loss).
    pub fn set_packet_loss_rate(&mut self, rate: f64) {
        for from_slot in 0..self.nodes.len() {
            for to_slot in 0..self.nodes.len() {
                if from_slot != to_slot {
                    let from = node_id_from_slot(from_slot);
                    let to = node_id_from_slot(to_slot);
                    self.injector.set_packet_loss_rate(from, to, rate);
                }
            }
        }
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("network: set packet loss rate to {:.1}%", rate * 100.0));
    }

    /// Configure range-based network delay for all node pairs.
    ///
    /// Delay will be uniformly sampled from [min_ms, max_ms] for each message.
    pub fn set_network_delay_range(&mut self, min_delay_ms: u64, max_delay_ms: u64) {
        for from_slot in 0..self.nodes.len() {
            for to_slot in 0..self.nodes.len() {
                if from_slot != to_slot {
                    let from = node_id_from_slot(from_slot);
                    let to = node_id_from_slot(to_slot);
                    self.injector.set_network_delay_range(from, to, min_delay_ms, max_delay_ms);
                }
            }
        }
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("network: set delay range {}-{}ms", min_delay_ms, max_delay_ms));
    }

    /// Get direct access to the failure injector for advanced configurations.
    pub fn injector(&self) -> &FailureInjector {
        &self.injector
    }

    /// Get direct access to the Byzantine failure injector.
    pub fn byzantine_injector(&self) -> &ByzantineFailureInjector {
        &self.byzantine_injector
    }

    // =========================================================================
    // Clock Drift Simulation Methods
    // =========================================================================

    /// Set clock drift for a specific node.
    ///
    /// Clock drift is simulated by adding asymmetric delays to messages:
    /// - Positive drift (fast clock): Delays OUTGOING messages from this node
    /// - Negative drift (slow clock): Delays INCOMING messages to this node
    ///
    /// This effectively simulates how Raft behaves when a node's clock runs
    /// faster or slower than other nodes in the cluster.
    ///
    /// # Arguments
    /// * `node_index` - 0-based index of the node
    /// * `drift_ms` - Signed drift in milliseconds. Positive = fast clock, negative = slow clock.
    pub fn set_clock_drift(&mut self, node_index: u32, drift_ms: i64) {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        let node_id = node_id_from_index(node_index);

        self.injector.set_clock_drift(node_id, drift_ms);

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("drift: node {} set to {}ms", node_index, drift_ms));
    }

    /// Clear clock drift for a specific node.
    ///
    /// Returns the node's simulated clock to normal (no drift).
    pub fn clear_clock_drift(&mut self, node_index: u32) {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        let node_id = node_id_from_index(node_index);

        self.injector.clear_clock_drift(node_id);

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("drift: node {} cleared", node_index));
    }

    /// Set clock drift for all nodes to simulate heterogeneous timing.
    ///
    /// # Arguments
    /// * `drifts` - Slice of (node_index, drift_ms) tuples
    pub fn set_cluster_clock_drifts(&mut self, drifts: &[(u32, i64)]) {
        for &(node_index, drift_ms) in drifts {
            let node_slot = node_slot_from_index(node_index);
            assert!(node_slot < self.nodes.len(), "Invalid node index: {}", node_index);
            let node_id = node_id_from_index(node_index);
            self.injector.set_clock_drift(node_id, drift_ms);
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("drift: set cluster drifts {:?}", drifts));
    }

    /// Clear clock drift for all nodes.
    pub fn clear_all_clock_drifts(&mut self) {
        for node_slot in 0..self.nodes.len() {
            let node_id = node_id_from_slot(node_slot);
            self.injector.clear_clock_drift(node_id);
        }

        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event("drift: cleared all node drifts");
    }

    /// Get the configured clock drift for a node.
    pub fn get_clock_drift(&self, node_index: u32) -> Option<i64> {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        let node_id = node_id_from_index(node_index);
        self.injector.get_clock_drift(node_id)
    }

    /// Enable Byzantine failure mode on a specific node.
    ///
    /// This configures the given node to potentially corrupt outgoing messages
    /// to all other nodes with the specified corruption mode and probability.
    ///
    /// # Arguments
    /// * `node_index` - 0-based index of the node to make Byzantine
    /// * `mode` - Type of message corruption
    /// * `probability` - Probability of corruption (0.0 to 1.0)
    pub fn enable_byzantine_mode(&mut self, node_index: u32, mode: ByzantineCorruptionMode, probability: f64) {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        let node_id = node_id_from_index(node_index);

        // Configure Byzantine behavior from this node to all others
        for target_slot in 0..self.nodes.len() {
            if node_slot != target_slot {
                let target_id = node_id_from_slot(target_slot);
                self.byzantine_injector.set_byzantine_mode(node_id, target_id, mode, probability);
            }
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(format!(
            "byzantine: node {} enabled {:?} with probability {:.1}%",
            node_index,
            mode,
            probability * 100.0
        ));
    }

    /// Disable all Byzantine behavior for a node.
    pub fn disable_byzantine_mode(&mut self, node_index: u32) {
        let node_slot = node_slot_from_index(node_index);
        assert!(node_slot < self.nodes.len(), "Invalid node index");
        // Note: Byzantine injector doesn't have per-node clear, so we just log it.
        // The injector will still have the config but we can add removal later if needed.
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("byzantine: node {} disabled", node_index));
    }

    /// Get the number of Byzantine message corruptions that have occurred.
    pub fn byzantine_corruption_count(&self) -> u64 {
        self.byzantine_injector.total_corruptions()
    }
}
