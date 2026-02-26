//! BUGGIFY fault injection methods for AspenRaftTester.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use aspen_raft::madsim_network::ByzantineCorruptionMode;
use aspen_raft::types::NodeId;

use super::AspenRaftTester;
use super::buggify::BuggifyFault;
use super::node::empty_artifact_builder;

impl AspenRaftTester {
    /// Enable BUGGIFY fault injection for this test.
    pub fn enable_buggify(&mut self, custom_probs: Option<HashMap<BuggifyFault, f64>>) {
        self.buggify.enable(custom_probs);
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event("buggify: enabled with fault injection");
    }

    /// Disable BUGGIFY fault injection.
    pub fn disable_buggify(&mut self) {
        self.buggify.disable();
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event("buggify: disabled");
    }

    /// Apply BUGGIFY faults if they should trigger.
    ///
    /// This method checks each fault type and applies them if triggered.
    /// Called periodically during test execution.
    pub async fn apply_buggify_faults(&mut self) {
        self.apply_buggify_network_delay();
        self.apply_buggify_network_drop().await;
        self.apply_buggify_node_crash().await;
        self.apply_buggify_message_corruption();
        self.apply_buggify_election_timeout().await;
        self.apply_buggify_network_partition().await;
        self.apply_buggify_snapshot_trigger().await;
    }

    fn apply_buggify_network_delay(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NetworkDelay) {
            return;
        }
        let delay_ms = 50 + (self.seed % 200); // 50-250ms delay

        // Apply delay to all node pairs
        for i in 0..self.nodes.len() {
            for j in 0..self.nodes.len() {
                if i != j {
                    self.injector.set_network_delay(NodeId::from(i as u64 + 1), NodeId::from(j as u64 + 1), delay_ms);
                }
            }
        }

        self.add_event(format!("buggify: injected {}ms network delay", delay_ms));
        self.metrics.buggify_triggers += 1;
    }

    async fn apply_buggify_network_drop(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NetworkDrop) {
            return;
        }
        // Apply packet loss to all links
        for i in 0..self.nodes.len() {
            for j in 0..self.nodes.len() {
                if i != j {
                    self.injector.set_packet_loss_rate(
                        NodeId::from(i as u64 + 1),
                        NodeId::from(j as u64 + 1),
                        0.1, // 10% loss rate
                    );
                }
            }
        }

        self.add_event("buggify: enabled 10% packet drop");
        self.metrics.buggify_triggers += 1;

        // Restore after some time
        madsim::time::sleep(Duration::from_secs(2)).await;

        // Clear packet loss
        for i in 0..self.nodes.len() {
            for j in 0..self.nodes.len() {
                if i != j {
                    self.injector.set_packet_loss_rate(NodeId::from(i as u64 + 1), NodeId::from(j as u64 + 1), 0.0);
                }
            }
        }

        self.add_event("buggify: restored packet delivery");
    }

    async fn apply_buggify_node_crash(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NodeCrash) {
            return;
        }
        let connected_nodes: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.connected().load(Ordering::Relaxed))
            .map(|(i, _)| i)
            .collect();

        if connected_nodes.len() > 2 {
            // Keep at least 2 nodes alive
            let victim = connected_nodes[self.seed as usize % connected_nodes.len()];
            self.crash_node(victim).await;
            self.add_event(format!("buggify: crashed node {}", victim));
            self.metrics.buggify_triggers += 1;
        }
    }

    fn apply_buggify_message_corruption(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::MessageCorruption) {
            return;
        }
        // Pick a random Byzantine corruption mode
        let modes = [
            ByzantineCorruptionMode::FlipVote,
            ByzantineCorruptionMode::IncrementTerm,
            ByzantineCorruptionMode::DuplicateMessage,
        ];
        let mode = modes[self.seed as usize % modes.len()];

        // Apply to a random node pair
        let src = self.seed as usize % self.nodes.len();
        let dst = (src + 1) % self.nodes.len();

        self.byzantine_injector.set_byzantine_mode(
            NodeId::from(src as u64 + 1),
            NodeId::from(dst as u64 + 1),
            mode,
            0.5,
        );

        self.add_event(format!("buggify: enabled {:?} corruption on link {}->{}", mode, src, dst));
        self.metrics.buggify_triggers += 1;
    }

    async fn apply_buggify_election_timeout(&mut self) {
        // Use has_leader_now() instead of check_one_leader() to avoid blocking delays
        if !self.buggify.should_trigger(BuggifyFault::ElectionTimeout) || !self.has_leader_now() {
            return;
        }
        // Find current leader without blocking retries
        let leader_idx = self
            .nodes
            .iter()
            .enumerate()
            .find_map(|(i, node)| {
                if node.connected().load(Ordering::Relaxed) {
                    let metrics = node.raft().metrics().borrow().clone();
                    metrics.current_leader.map(|_| i)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        self.disconnect(leader_idx);
        self.add_event(format!("buggify: partitioned leader {} to force re-election", leader_idx));
        self.metrics.buggify_triggers += 1;

        // Restore after election timeout - reduced from 5s to 2s for faster tests
        // This is still longer than election_timeout_max (3s) so elections can complete
        madsim::time::sleep(Duration::from_secs(2)).await;
        self.connect(leader_idx);
        self.add_event(format!("buggify: restored node {} connectivity", leader_idx));
    }

    async fn apply_buggify_network_partition(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NetworkPartition) {
            return;
        }
        let mid = self.nodes.len() / 2;
        for i in 0..mid {
            self.disconnect(i);
        }
        self.add_event(format!("buggify: created network partition (nodes 0-{} isolated)", mid - 1));
        self.metrics.buggify_triggers += 1;

        // Heal after some time - reduced from 10s to 3s for faster tests
        // while still being long enough to force leader re-election
        madsim::time::sleep(Duration::from_secs(3)).await;
        for i in 0..mid {
            self.connect(i);
        }
        self.add_event("buggify: healed network partition");
    }

    async fn apply_buggify_snapshot_trigger(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::SnapshotTrigger) {
            return;
        }
        if let Some(leader_idx) = self.check_one_leader().await {
            // Use Raft::trigger().snapshot() to manually trigger snapshot on leader
            let raft = self.nodes[leader_idx].raft();
            if let Err(e) = raft.trigger().snapshot().await {
                self.add_event(format!("buggify: snapshot trigger failed: {:?}", e));
            } else {
                self.add_event("buggify: triggered snapshot on leader");
            }
            self.metrics.buggify_triggers += 1;
        }
    }

    /// Run a test loop with BUGGIFY enabled, periodically applying faults.
    ///
    /// This runs for the specified duration, applying BUGGIFY faults every second.
    pub async fn run_with_buggify_loop(&mut self, duration: Duration) {
        let start = Instant::now();

        while start.elapsed() < duration {
            // Apply faults
            self.apply_buggify_faults().await;

            // Wait before next fault injection
            madsim::time::sleep(Duration::from_secs(1)).await;
        }

        self.add_event(format!("buggify: completed {} seconds of fault injection", duration.as_secs()));
    }

    /// Apply a specific BUGGIFY fault with 100% probability.
    ///
    /// This is useful for property-based testing where you want to inject
    /// specific faults deterministically.
    pub async fn apply_single_fault(&mut self, fault: BuggifyFault) {
        if !self.buggify.enabled.load(Ordering::Relaxed) {
            // Enable BUGGIFY if not already enabled
            self.enable_buggify(None);
        }

        // Save current probabilities
        let saved = self.buggify.probabilities.lock().unwrap().clone();

        // Set 100% probability for this specific fault
        let mut probs = HashMap::new();
        probs.insert(fault, 1.0);
        *self.buggify.probabilities.lock().unwrap() = probs;

        // Apply the fault
        self.apply_buggify_faults().await;

        // Restore original probabilities
        *self.buggify.probabilities.lock().unwrap() = saved;
    }
}
