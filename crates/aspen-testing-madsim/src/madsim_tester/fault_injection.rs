//! BUGGIFY fault injection methods for AspenRaftTester.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use aspen_raft::madsim_network::ByzantineCorruptionMode;
use aspen_raft::types::NodeId;

use super::AspenRaftTester;
use super::buggify::BuggifyFault;
use super::node::TestNode;
use super::node::empty_artifact_builder;

fn node_id_from_slot(node_slot: usize) -> NodeId {
    let node_index = u64::try_from(node_slot).unwrap_or(u64::MAX);
    NodeId::from(node_index.saturating_add(1))
}

fn pick_slot_by_seed(seed: u64, node_count: usize) -> Option<usize> {
    let node_count_u64 = u64::try_from(node_count).ok()?;
    let slot = seed.checked_rem(node_count_u64)?;
    usize::try_from(slot).ok()
}

fn connected_node_slots(nodes: &[TestNode]) -> Vec<usize> {
    let mut connected_slots = Vec::with_capacity(nodes.len());
    for (node_slot, node) in nodes.iter().enumerate() {
        if node.connected().load(Ordering::Relaxed) {
            connected_slots.push(node_slot);
        }
    }
    connected_slots
}

fn for_each_node_pair(node_count: usize, mut visit: impl FnMut(NodeId, NodeId)) {
    for from_slot in 0..node_count {
        let from = node_id_from_slot(from_slot);
        for to_slot in 0..node_count {
            if from_slot == to_slot {
                continue;
            }
            let to = node_id_from_slot(to_slot);
            visit(from, to);
        }
    }
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "BUGGIFY loop timing uses monotonic instants to bound simulated runtime"
)]
fn current_instant() -> Instant {
    Instant::now()
}

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
        self.apply_buggify_slow_storage().await;
        self.apply_buggify_snapshot_transfer_reset().await;
    }

    fn apply_buggify_network_delay(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NetworkDelay) {
            return;
        }
        let delay_ms = 50_u64.saturating_add(self.seed % 200); // 50-250ms delay

        // Apply delay to all node pairs
        for_each_node_pair(self.nodes.len(), |from, to| {
            self.injector.set_network_delay(from, to, delay_ms);
        });

        self.add_event(format!("buggify: injected {}ms network delay", delay_ms));
        self.metrics.buggify_triggers += 1;
    }

    async fn apply_buggify_network_drop(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NetworkDrop) {
            return;
        }
        // Apply packet loss to all links
        for_each_node_pair(self.nodes.len(), |from, to| {
            self.injector.set_packet_loss_rate(from, to, 0.1); // 10% loss rate
        });

        self.add_event("buggify: enabled 10% packet drop");
        self.metrics.buggify_triggers += 1;

        // Restore after some time
        madsim::time::sleep(Duration::from_secs(2)).await;

        // Clear packet loss
        for_each_node_pair(self.nodes.len(), |from, to| {
            self.injector.set_packet_loss_rate(from, to, 0.0);
        });

        self.add_event("buggify: restored packet delivery");
    }

    async fn apply_buggify_node_crash(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NodeCrash) {
            return;
        }
        let connected_nodes = connected_node_slots(&self.nodes);

        if connected_nodes.len() > 2 {
            // Keep at least 2 nodes alive
            let Some(victim_offset) = pick_slot_by_seed(self.seed, connected_nodes.len()) else {
                return;
            };
            let victim = connected_nodes[victim_offset];
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
        let Some(mode_slot) = pick_slot_by_seed(self.seed, modes.len()) else {
            return;
        };
        let mode = modes[mode_slot];

        // Apply to a random node pair
        let Some(src) = pick_slot_by_seed(self.seed, self.nodes.len()) else {
            return;
        };
        let dst = match src.checked_add(1) {
            Some(next) if next < self.nodes.len() => next,
            Some(_) | None => 0,
        };

        self.byzantine_injector
            .set_byzantine_mode(node_id_from_slot(src), node_id_from_slot(dst), mode, 0.5);

        self.add_event(format!("buggify: enabled {:?} corruption on link {}->{}", mode, src, dst));
        self.metrics.buggify_triggers += 1;
    }

    async fn apply_buggify_election_timeout(&mut self) {
        // Use has_leader_now() instead of check_one_leader() to avoid blocking delays
        if !self.buggify.should_trigger(BuggifyFault::ElectionTimeout) || !self.has_leader_now() {
            return;
        }
        // Find current leader without blocking retries
        let leader_slot = self
            .nodes
            .iter()
            .enumerate()
            .find_map(|(node_slot, node)| {
                if node.connected().load(Ordering::Relaxed) {
                    let metrics = node.raft().metrics().borrow().clone();
                    metrics.current_leader.map(|_| node_slot)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let leader_index = u32::try_from(leader_slot).unwrap_or(u32::MAX);

        self.disconnect(leader_index);
        self.add_event(format!("buggify: partitioned leader {} to force re-election", leader_index));
        self.metrics.buggify_triggers += 1;

        // Restore after election timeout - reduced from 5s to 2s for faster tests
        // This is still longer than election_timeout_max (3s) so elections can complete
        madsim::time::sleep(Duration::from_secs(2)).await;
        self.connect(leader_index);
        self.add_event(format!("buggify: restored node {} connectivity", leader_index));
    }

    async fn apply_buggify_network_partition(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::NetworkPartition) {
            return;
        }
        let midpoint = u32::try_from(self.nodes.len() / 2).unwrap_or(u32::MAX);
        for node_index in 0..midpoint {
            self.disconnect(node_index);
        }
        self.add_event(format!("buggify: created network partition (nodes 0-{} isolated)", midpoint.saturating_sub(1)));
        self.metrics.buggify_triggers += 1;

        // Heal after some time - reduced from 10s to 3s for faster tests
        // while still being long enough to force leader re-election
        madsim::time::sleep(Duration::from_secs(3)).await;
        for node_index in 0..midpoint {
            self.connect(node_index);
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
            if let Err(error) = raft.trigger().snapshot().await {
                self.add_event(format!("buggify: snapshot trigger failed: {:?}", error));
            } else {
                self.add_event("buggify: triggered snapshot on leader");
            }
            self.metrics.buggify_triggers += 1;
        }
    }

    /// Simulate slow storage by injecting high latency on all network links.
    ///
    /// This models disk I/O delays by adding 500-2000ms delay to all inter-node
    /// communication, since in madsim the storage is simulated through network.
    async fn apply_buggify_slow_storage(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::SlowStorage) {
            return;
        }
        let delay_ms = 500_u64.saturating_add(self.seed % 1500); // 500-2000ms

        for_each_node_pair(self.nodes.len(), |from, to| {
            self.injector.set_network_delay(from, to, delay_ms);
        });

        self.add_event(format!("buggify: injected {}ms slow storage delay", delay_ms));
        self.metrics.buggify_triggers += 1;

        // Hold delay for a period then clear
        madsim::time::sleep(Duration::from_secs(3)).await;

        for_each_node_pair(self.nodes.len(), |from, to| {
            self.injector.set_network_delay(from, to, 0);
        });

        self.add_event("buggify: cleared slow storage delay");
    }

    /// Simulate snapshot transfer interruption by crashing and restarting a
    /// follower node while triggering a snapshot on the leader.
    async fn apply_buggify_snapshot_transfer_reset(&mut self) {
        if !self.buggify.should_trigger(BuggifyFault::SnapshotTransferReset) {
            return;
        }

        let connected_nodes = connected_node_slots(&self.nodes);

        // Need at least 3 connected nodes (leader + majority after crash)
        if connected_nodes.len() < 3 {
            return;
        }

        // Find leader and pick a follower victim
        if let Some(leader_idx) = self.check_one_leader().await {
            let victim = connected_nodes.iter().find(|&&node_slot| node_slot != leader_idx).copied().unwrap_or(0);

            // Trigger snapshot on leader
            let raft = self.nodes[leader_idx].raft();
            if let Err(error) = raft.trigger().snapshot().await {
                self.add_event(format!("buggify: snapshot trigger before reset failed: {:?}", error));
                return;
            }

            // Crash the follower mid-transfer
            self.crash_node(victim).await;
            self.add_event(format!("buggify: crashed node {} during snapshot transfer", victim));
            self.metrics.buggify_triggers += 1;

            // Restart after brief delay
            madsim::time::sleep(Duration::from_secs(2)).await;
            self.restart_node(victim).await;
            self.add_event(format!("buggify: restarted node {} after snapshot interrupt", victim));
        }
    }

    /// Run a test loop with BUGGIFY enabled, periodically applying faults.
    ///
    /// This runs for the specified duration, applying BUGGIFY faults every second.
    pub async fn run_with_buggify_loop(&mut self, duration: Duration) {
        let start = current_instant();

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
        let saved = {
            let probabilities = match self.buggify.probabilities.lock() {
                Ok(probabilities) => probabilities,
                Err(poisoned) => poisoned.into_inner(),
            };
            probabilities.clone()
        };

        // Set 100% probability for this specific fault
        let mut probs = HashMap::new();
        probs.insert(fault, 1.0);
        {
            let mut probabilities = match self.buggify.probabilities.lock() {
                Ok(probabilities) => probabilities,
                Err(poisoned) => poisoned.into_inner(),
            };
            *probabilities = probs;
        }

        // Apply the fault
        self.apply_buggify_faults().await;

        // Restore original probabilities
        let mut probabilities = match self.buggify.probabilities.lock() {
            Ok(probabilities) => probabilities,
            Err(poisoned) => poisoned.into_inner(),
        };
        *probabilities = saved;
    }
}
