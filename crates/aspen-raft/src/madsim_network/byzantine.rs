//! ByzantineFailureInjector - Byzantine failure injection for consensus testing.

use std::collections::HashMap;

use openraft::raft::AppendEntriesRequest;
use openraft::raft::VoteResponse;
use parking_lot::Mutex as SyncMutex;

use crate::types::AppTypeConfig;
use crate::types::NodeId;

/// Byzantine failure injection type for testing consensus under Byzantine conditions.
///
/// These corruption modes simulate various Byzantine behaviors without creating
/// actual malicious nodes. This enables testing Raft's behavior when messages
/// are corrupted in transit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByzantineCorruptionMode {
    /// Flip the vote_granted field in VoteResponse messages.
    /// This tests whether Raft correctly handles conflicting vote responses.
    FlipVote,

    /// Increment the term in messages to simulate stale/future term attacks.
    /// This tests term validation logic.
    IncrementTerm,

    /// Duplicate messages (send twice) to test idempotency.
    /// This tests whether the system handles duplicate delivery.
    DuplicateMessage,

    /// Corrupt log entries by clearing the entries field.
    /// This tests whether followers validate received entries.
    ClearEntries,
}

impl std::hash::Hash for ByzantineCorruptionMode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

/// Byzantine network wrapper that corrupts messages according to configured modes.
///
/// This wrapper sits between the Raft node and the underlying MadsimRaftNetwork,
/// intercepting and potentially corrupting messages to test Byzantine fault tolerance.
///
/// **Note**: Raft does not provide Byzantine fault tolerance. This testing is useful for:
/// 1. Verifying Raft's behavior under message corruption (e.g., from network bit flips)
/// 2. Testing term validation and log consistency checks
/// 3. Understanding failure modes
///
/// **Usage in AspenRaftTester**:
/// ```ignore
/// tester.enable_byzantine_mode(node_idx, ByzantineCorruptionMode::FlipVote, 0.3);
/// ```
pub struct ByzantineFailureInjector {
    /// Per-link Byzantine configuration: (source, target) -> (mode, probability)
    #[allow(clippy::type_complexity)]
    pub(super) links: SyncMutex<HashMap<(NodeId, NodeId), Vec<(ByzantineCorruptionMode, f64)>>>,
    /// Statistics: count of corruptions per mode
    pub(super) corruption_counts: SyncMutex<HashMap<ByzantineCorruptionMode, u64>>,
}

impl ByzantineFailureInjector {
    /// Create a new Byzantine failure injector.
    pub fn new() -> Self {
        Self {
            links: SyncMutex::new(HashMap::new()),
            corruption_counts: SyncMutex::new(HashMap::new()),
        }
    }

    /// Configure Byzantine behavior on a specific link.
    ///
    /// # Arguments
    /// * `source` - Source node ID
    /// * `target` - Target node ID
    /// * `mode` - Type of Byzantine corruption
    /// * `probability` - Probability of corruption (0.0 to 1.0)
    pub fn set_byzantine_mode(&self, source: NodeId, target: NodeId, mode: ByzantineCorruptionMode, probability: f64) {
        assert!((0.0..=1.0).contains(&probability), "probability must be between 0.0 and 1.0");
        let mut links = self.links.lock();
        let configs = links.entry((source, target)).or_default();
        // Update existing or add new
        if let Some(existing) = configs.iter_mut().find(|(m, _)| *m == mode) {
            existing.1 = probability;
        } else {
            configs.push((mode, probability));
        }
    }

    /// Clear all Byzantine configurations.
    pub fn clear_all(&self) {
        let mut links = self.links.lock();
        links.clear();
    }

    /// Check if a particular corruption should be applied.
    fn should_corrupt(&self, source: NodeId, target: NodeId, mode: ByzantineCorruptionMode) -> bool {
        let links = self.links.lock();
        if let Some(configs) = links.get(&(source, target)) {
            for (m, prob) in configs {
                if *m == mode {
                    let random_value: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
                    if random_value < *prob {
                        // Record corruption
                        drop(links); // Release lock before acquiring another
                        let mut counts = self.corruption_counts.lock();
                        *counts.entry(mode).or_insert(0) += 1;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Potentially corrupt an AppendEntries request.
    pub fn maybe_corrupt_append_entries(
        &self,
        source: NodeId,
        target: NodeId,
        mut rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> AppendEntriesRequest<AppTypeConfig> {
        // IncrementTerm: Add 1 to term to simulate stale term attack
        if self.should_corrupt(source, target, ByzantineCorruptionMode::IncrementTerm) {
            rpc.vote.leader_id.term = rpc.vote.leader_id.term.saturating_add(1);
            tracing::warn!(
                %source, %target,
                "BYZANTINE: Corrupted AppendEntries term to {}",
                rpc.vote.leader_id.term
            );
        }

        // ClearEntries: Remove all entries to test log validation
        if self.should_corrupt(source, target, ByzantineCorruptionMode::ClearEntries) {
            let original_len = rpc.entries.len();
            rpc.entries.clear();
            tracing::warn!(
                %source, %target,
                "BYZANTINE: Cleared {} entries from AppendEntries",
                original_len
            );
        }

        rpc
    }

    /// Potentially corrupt a Vote response.
    pub fn maybe_corrupt_vote_response(
        &self,
        source: NodeId,
        target: NodeId,
        mut resp: VoteResponse<AppTypeConfig>,
    ) -> VoteResponse<AppTypeConfig> {
        // FlipVote: Invert the vote_granted field
        if self.should_corrupt(source, target, ByzantineCorruptionMode::FlipVote) {
            resp.vote_granted = !resp.vote_granted;
            tracing::warn!(
                %source, %target,
                "BYZANTINE: Flipped vote_granted to {}",
                resp.vote_granted
            );
        }

        resp
    }

    /// Check if a message should be duplicated.
    pub fn should_duplicate(&self, source: NodeId, target: NodeId) -> bool {
        self.should_corrupt(source, target, ByzantineCorruptionMode::DuplicateMessage)
    }

    /// Get corruption statistics.
    pub fn get_corruption_stats(&self) -> HashMap<ByzantineCorruptionMode, u64> {
        let counts = self.corruption_counts.lock();
        counts.clone()
    }

    /// Get total corruption count.
    pub fn total_corruptions(&self) -> u64 {
        let counts = self.corruption_counts.lock();
        counts.values().sum()
    }
}

impl Default for ByzantineFailureInjector {
    fn default() -> Self {
        Self::new()
    }
}
