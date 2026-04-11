//! Sans-IO reconfiguration state machine.
//!
//! Coordinates secret rotation on membership changes. The state machine
//! collects shares from old members, reconstructs the old secret,
//! generates a new secret and shares, builds the encrypted chain,
//! and produces a Raft proposal.
//!
//! No I/O — all interactions go through the `ReconfigAction` outputs
//! that the caller dispatches.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
use snafu::Snafu;

use crate::chain;
use crate::chain::EncryptedSecretChain;
use crate::secret::ClusterSecret;
use crate::shamir;
use crate::shamir::Share;
use crate::shamir::ShareDigest;

/// Context trait for reconfiguration I/O.
///
/// The coordinator calls these methods to interact with the outside world.
/// Production implementations use Iroh+Raft; test implementations record
/// actions for inspection.
pub trait ReconfigCtx {
    /// Send a GetShare request to a node.
    fn send_get_share(&mut self, target_node: u64, epoch: u64);

    /// Propose a new trust configuration via Raft.
    fn propose_new_config(
        &mut self,
        new_shares: BTreeMap<u64, Share>,
        new_digests: BTreeMap<u64, ShareDigest>,
        encrypted_chain: EncryptedSecretChain,
        new_epoch: u64,
    );

    /// Query which old-config members are currently connected.
    fn connected_members(&self) -> BTreeSet<u64>;
}

/// Reconfiguration coordinator states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconfigState {
    /// Collecting shares from old configuration members.
    CollectingOldShares,
    /// Preparing new configuration (reconstructing, splitting, building chain).
    Preparing,
    /// New configuration committed via Raft.
    Committed,
    /// Reconfiguration failed.
    Failed { reason: String },
}

/// Actions the caller must dispatch on behalf of the state machine.
#[derive(Debug, Clone)]
pub enum ReconfigAction {
    /// Send a GetShare request to a node.
    SendGetShare { target_node: u64, epoch: u64 },
    /// Propose the new trust configuration via Raft.
    ProposeNewConfig {
        new_shares: BTreeMap<u64, Share>,
        new_digests: BTreeMap<u64, ShareDigest>,
        encrypted_chain: EncryptedSecretChain,
        new_epoch: u64,
    },
}

/// Errors during reconfiguration.
#[derive(Debug, Snafu)]
pub enum ReconfigError {
    /// Share collection timed out.
    #[snafu(display("share collection timed out: got {collected} of {needed} shares"))]
    Timeout { collected: u32, needed: u32 },

    /// Share reconstruction failed.
    #[snafu(display("secret reconstruction failed: {reason}"))]
    ReconstructFailed { reason: String },

    /// Chain encryption failed.
    #[snafu(display("chain encryption failed: {source}"))]
    ChainFailed { source: chain::ChainError },

    /// Share splitting failed.
    #[snafu(display("share splitting failed: {reason}"))]
    SplitFailed { reason: String },

    /// Invalid share received (digest mismatch).
    #[snafu(display("invalid share from node {node_id}: digest mismatch"))]
    InvalidShare { node_id: u64 },
}

/// Reconfiguration coordinator (sans-IO state machine).
pub struct ReconfigCoordinator {
    /// Current state.
    state: ReconfigState,
    /// Old epoch we're rotating from.
    old_epoch: u64,
    /// New epoch we're rotating to.
    new_epoch: u64,
    /// Threshold for reconstruction from the old configuration.
    old_threshold: u8,
    /// Threshold for new shares in the next configuration.
    new_threshold: u8,
    /// Nodes in the old configuration.
    old_members: BTreeSet<u64>,
    /// Nodes in the new configuration.
    new_members: BTreeSet<u64>,
    /// Collected shares from old members.
    collected_shares: Vec<Share>,
    /// Which nodes have provided shares.
    share_sources: BTreeSet<u64>,
    /// Expected digests (from stored state).
    expected_digests: BTreeMap<u64, ShareDigest>,
    /// Cluster identifier.
    cluster_id: Vec<u8>,
    /// Prior secrets (for chain construction). Empty on first reconfig.
    prior_secrets: BTreeMap<u64, [u8; 32]>,
}

impl ReconfigCoordinator {
    /// Create a new coordinator for a membership change.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        old_epoch: u64,
        new_epoch: u64,
        old_threshold: u8,
        new_threshold: u8,
        old_members: BTreeSet<u64>,
        new_members: BTreeSet<u64>,
        expected_digests: BTreeMap<u64, ShareDigest>,
        cluster_id: Vec<u8>,
        prior_secrets: BTreeMap<u64, [u8; 32]>,
    ) -> Self {
        Self {
            state: ReconfigState::CollectingOldShares,
            old_epoch,
            new_epoch,
            old_threshold,
            new_threshold,
            old_members,
            new_members,
            collected_shares: Vec::new(),
            share_sources: BTreeSet::new(),
            expected_digests,
            cluster_id,
            prior_secrets,
        }
    }

    /// Current state.
    pub fn state(&self) -> &ReconfigState {
        &self.state
    }

    /// Start share collection — returns actions to send GetShare requests.
    pub fn start(&self) -> Vec<ReconfigAction> {
        self.old_members
            .iter()
            .map(|&node| ReconfigAction::SendGetShare {
                target_node: node,
                epoch: self.old_epoch,
            })
            .collect()
    }

    /// Process a received share from a node.
    ///
    /// Returns actions if the state machine transitions (e.g., propose new config).
    pub fn on_share_received(&mut self, from_node: u64, share: Share) -> Result<Vec<ReconfigAction>, ReconfigError> {
        if self.state != ReconfigState::CollectingOldShares {
            return Ok(vec![]);
        }

        // Validate: node is in old config and hasn't already sent a share
        if !self.old_members.contains(&from_node) || self.share_sources.contains(&from_node) {
            return Ok(vec![]);
        }

        // Validate digest if we have one
        if let Some(expected) = self.expected_digests.get(&from_node) {
            let actual = shamir::share_digest(&share);
            if &actual != expected {
                return Err(ReconfigError::InvalidShare { node_id: from_node });
            }
        }

        self.collected_shares.push(share);
        self.share_sources.insert(from_node);

        // Check if we have enough shares
        if self.collected_shares.len() as u8 >= self.old_threshold {
            self.prepare()
        } else {
            Ok(vec![])
        }
    }

    /// Handle a timeout during share collection.
    pub fn on_timeout(&mut self) -> Result<(), ReconfigError> {
        if self.state == ReconfigState::CollectingOldShares {
            let err = ReconfigError::Timeout {
                collected: self.collected_shares.len() as u32,
                needed: self.old_threshold as u32,
            };
            self.state = ReconfigState::Failed {
                reason: err.to_string(),
            };
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Mark the reconfiguration as committed (Raft log applied).
    pub fn on_committed(&mut self) {
        self.state = ReconfigState::Committed;
    }

    /// Prepare phase: reconstruct old secret, generate new, split, build chain.
    fn prepare(&mut self) -> Result<Vec<ReconfigAction>, ReconfigError> {
        self.state = ReconfigState::Preparing;

        // 1. Reconstruct old secret
        let old_secret_bytes = shamir::reconstruct_secret(&self.collected_shares)
            .map_err(|e| ReconfigError::ReconstructFailed { reason: e.to_string() })?;

        // 2. Generate new secret
        let new_secret = ClusterSecret::generate();

        // 3. Build the prior secrets map (include the just-reconstructed one)
        let mut all_prior = self.prior_secrets.clone();
        all_prior.insert(self.old_epoch, old_secret_bytes);

        // 4. Encrypt chain
        let encrypted_chain = chain::encrypt_chain(&all_prior, new_secret.as_bytes(), &self.cluster_id, self.new_epoch)
            .map_err(|e| ReconfigError::ChainFailed { source: e })?;

        // 5. Split new secret into shares for new members
        let n = self.new_members.len() as u8;
        let mut rng = rand::rng();
        let shares = shamir::split_secret(new_secret.as_bytes(), self.new_threshold, n, &mut rng)
            .map_err(|e| ReconfigError::SplitFailed { reason: e.to_string() })?;

        // Map shares to node IDs
        let new_shares: BTreeMap<u64, Share> =
            self.new_members.iter().zip(shares.iter()).map(|(&id, s)| (id, s.clone())).collect();

        let new_digests: BTreeMap<u64, ShareDigest> =
            new_shares.iter().map(|(&id, s)| (id, shamir::share_digest(s))).collect();

        Ok(vec![ReconfigAction::ProposeNewConfig {
            new_shares,
            new_digests,
            encrypted_chain,
            new_epoch: self.new_epoch,
        }])
    }
}

/// Test implementation of `ReconfigCtx` that records all actions for inspection.
#[cfg(test)]
pub struct TestReconfigCtx {
    /// GetShare requests sent.
    pub get_share_requests: Vec<(u64, u64)>,
    /// Proposals submitted.
    pub proposals: Vec<(BTreeMap<u64, Share>, BTreeMap<u64, ShareDigest>, EncryptedSecretChain, u64)>,
    /// Simulated connected members.
    pub connected: BTreeSet<u64>,
}

#[cfg(test)]
impl TestReconfigCtx {
    pub fn new(connected: BTreeSet<u64>) -> Self {
        Self {
            get_share_requests: Vec::new(),
            proposals: Vec::new(),
            connected,
        }
    }
}

#[cfg(test)]
impl ReconfigCtx for TestReconfigCtx {
    fn send_get_share(&mut self, target_node: u64, epoch: u64) {
        self.get_share_requests.push((target_node, epoch));
    }

    fn propose_new_config(
        &mut self,
        new_shares: BTreeMap<u64, Share>,
        new_digests: BTreeMap<u64, ShareDigest>,
        encrypted_chain: EncryptedSecretChain,
        new_epoch: u64,
    ) {
        self.proposals.push((new_shares, new_digests, encrypted_chain, new_epoch));
    }

    fn connected_members(&self) -> BTreeSet<u64> {
        self.connected.clone()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::secret::ClusterSecret;

    fn setup_3node() -> (Vec<Share>, BTreeMap<u64, ShareDigest>, [u8; 32]) {
        let secret = ClusterSecret::generate();
        let mut rng = rand::rng();
        let shares = shamir::split_secret(secret.as_bytes(), 2, 3, &mut rng).unwrap();

        let digests: BTreeMap<u64, ShareDigest> =
            [1, 2, 3].iter().zip(shares.iter()).map(|(&id, s)| (id, shamir::share_digest(s))).collect();

        (shares, digests, *secret.as_bytes())
    }

    fn dispatch_action(ctx: &mut TestReconfigCtx, action: ReconfigAction) {
        match action {
            ReconfigAction::SendGetShare { target_node, epoch } => ctx.send_get_share(target_node, epoch),
            ReconfigAction::ProposeNewConfig {
                new_shares,
                new_digests,
                encrypted_chain,
                new_epoch,
            } => ctx.propose_new_config(new_shares, new_digests, encrypted_chain, new_epoch),
        }
    }

    fn dispatch_actions(ctx: &mut TestReconfigCtx, actions: Vec<ReconfigAction>) {
        for action in actions {
            dispatch_action(ctx, action);
        }
    }

    fn majority_threshold(count: u8) -> u8 {
        ((u32::from(count) / 2) + 1) as u8
    }

    #[test]
    fn test_full_reconfig_flow() {
        let (shares, digests, _secret_bytes) = setup_3node();

        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let new_members: BTreeSet<u64> = [1, 2, 4].into();

        let mut coordinator = ReconfigCoordinator::new(
            1,
            2,
            2,
            2,
            old_members,
            new_members,
            digests,
            b"test-cluster".to_vec(),
            BTreeMap::new(),
        );

        let actions = coordinator.start();
        assert_eq!(actions.len(), 3);

        let actions = coordinator.on_share_received(1, shares[0].clone()).unwrap();
        assert!(actions.is_empty());

        let actions = coordinator.on_share_received(2, shares[1].clone()).unwrap();
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            ReconfigAction::ProposeNewConfig {
                new_shares,
                new_digests,
                encrypted_chain,
                new_epoch,
            } => {
                assert_eq!(*new_epoch, 2);
                assert_eq!(new_shares.len(), 3);
                assert_eq!(new_digests.len(), 3);
                assert_eq!(encrypted_chain.prior_count, 1);
            }
            ReconfigAction::SendGetShare { .. } => panic!("expected ProposeNewConfig"),
        }

        assert_eq!(coordinator.state(), &ReconfigState::Preparing);
    }

    #[test]
    fn test_reconfig_state_machine_driven_by_test_ctx_commits() {
        let (shares, digests, old_secret) = setup_3node();
        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let new_members: BTreeSet<u64> = [1, 2, 4].into();
        let mut coordinator = ReconfigCoordinator::new(
            10,
            22,
            2,
            2,
            old_members.clone(),
            new_members,
            digests,
            b"ctx-cluster".to_vec(),
            BTreeMap::from([(5, [0x55; 32])]),
        );
        let mut ctx = TestReconfigCtx::new(old_members.clone());

        dispatch_actions(&mut ctx, coordinator.start());
        assert_eq!(ctx.get_share_requests.len(), old_members.len());
        assert_eq!(ctx.connected_members(), old_members);

        dispatch_actions(&mut ctx, coordinator.on_share_received(1, shares[0].clone()).unwrap());
        assert!(ctx.proposals.is_empty());

        dispatch_actions(&mut ctx, coordinator.on_share_received(2, shares[1].clone()).unwrap());
        assert_eq!(ctx.proposals.len(), 1);
        assert_eq!(coordinator.state(), &ReconfigState::Preparing);

        let (new_shares, new_digests, encrypted_chain, new_epoch) = &ctx.proposals[0];
        assert_eq!(*new_epoch, 22);
        assert_eq!(new_shares.len(), 3);
        assert_eq!(new_digests.len(), 3);
        assert_eq!(encrypted_chain.prior_count, 2);

        let reconstructed_new_secret =
            shamir::reconstruct_secret(&new_shares.values().take(2).cloned().collect::<Vec<_>>()).unwrap();
        let prior = chain::decrypt_chain(encrypted_chain, &reconstructed_new_secret, b"ctx-cluster").unwrap();
        assert_eq!(prior.get(&5), Some(&[0x55; 32]));
        assert_eq!(prior.get(&10), Some(&old_secret));

        coordinator.on_committed();
        assert_eq!(coordinator.state(), &ReconfigState::Committed);
    }

    #[test]
    fn test_timeout_during_collection() {
        let (_, digests, _) = setup_3node();

        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let new_members: BTreeSet<u64> = [1, 2, 4].into();

        let mut coordinator = ReconfigCoordinator::new(
            1,
            2,
            2,
            2,
            old_members,
            new_members,
            digests,
            b"cluster".to_vec(),
            BTreeMap::new(),
        );

        let err = coordinator.on_timeout().unwrap_err();
        assert!(matches!(err, ReconfigError::Timeout {
            collected: 0,
            needed: 2
        }));
        assert!(matches!(coordinator.state(), ReconfigState::Failed { .. }));
    }

    #[test]
    fn test_invalid_share_rejected() {
        let (shares, digests, _) = setup_3node();

        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let new_members: BTreeSet<u64> = [1, 2, 3].into();

        let mut coordinator = ReconfigCoordinator::new(
            1,
            2,
            2,
            2,
            old_members,
            new_members,
            digests,
            b"cluster".to_vec(),
            BTreeMap::new(),
        );

        let result = coordinator.on_share_received(2, shares[0].clone());
        assert!(matches!(result, Err(ReconfigError::InvalidShare { node_id: 2 })));
    }

    #[test]
    fn test_duplicate_share_ignored() {
        let (shares, digests, _) = setup_3node();

        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let new_members: BTreeSet<u64> = [1, 2, 3].into();

        let mut coordinator = ReconfigCoordinator::new(
            1,
            2,
            2,
            2,
            old_members,
            new_members,
            digests,
            b"cluster".to_vec(),
            BTreeMap::new(),
        );

        coordinator.on_share_received(1, shares[0].clone()).unwrap();
        let actions = coordinator.on_share_received(1, shares[0].clone()).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_reconfig_ctx_records_actions() {
        let connected: BTreeSet<u64> = [1, 2, 3].into();
        let mut ctx = TestReconfigCtx::new(connected.clone());

        ctx.send_get_share(1, 5);
        ctx.send_get_share(2, 5);
        assert_eq!(ctx.get_share_requests.len(), 2);
        assert_eq!(ctx.get_share_requests[0], (1, 5));
        assert_eq!(ctx.connected_members(), connected);
        assert!(ctx.proposals.is_empty());
    }

    proptest! {
        #[test]
        fn prop_reconfiguration_produces_reconstructible_new_shares(
            old_count in 1u8..=5,
            keep_count in 0u8..=5,
            add_count in 0u8..=3,
        ) {
            prop_assume!(keep_count <= old_count);
            prop_assume!(keep_count > 0 || add_count > 0);
            let new_count = keep_count + add_count;
            prop_assume!(new_count >= 1);
            prop_assume!(new_count <= 6);

            let old_members: BTreeSet<u64> = (1u64..=u64::from(old_count)).collect();
            let mut new_members: BTreeSet<u64> = (1u64..=u64::from(keep_count)).collect();
            for extra in 0..u64::from(add_count) {
                new_members.insert(100 + extra);
            }

            let old_threshold = majority_threshold(old_count);
            let new_threshold = majority_threshold(new_count);
            let old_secret = ClusterSecret::generate();
            let mut rng = rand::rng();
            let shares = shamir::split_secret(old_secret.as_bytes(), old_threshold, old_count, &mut rng).unwrap();
            let expected_digests: BTreeMap<u64, ShareDigest> =
                old_members.iter().zip(shares.iter()).map(|(&node_id, share)| (node_id, shamir::share_digest(share))).collect();

            let old_epoch = 7u64;
            let mut coordinator = ReconfigCoordinator::new(
                old_epoch,
                19,
                old_threshold,
                new_threshold,
                old_members.clone(),
                new_members.clone(),
                expected_digests,
                b"prop-cluster".to_vec(),
                BTreeMap::new(),
            );

            let mut proposal = None;
            for (&node_id, share) in old_members.iter().zip(shares.iter()).take(usize::from(old_threshold)) {
                for action in coordinator.on_share_received(node_id, share.clone()).unwrap() {
                    if let ReconfigAction::ProposeNewConfig {
                        new_shares,
                        new_digests,
                        encrypted_chain,
                        new_epoch,
                    } = action {
                        proposal = Some((new_shares, new_digests, encrypted_chain, new_epoch));
                    }
                }
            }

            prop_assert!(proposal.is_some());
            let (new_shares, new_digests, encrypted_chain, new_epoch) = proposal.unwrap();
            prop_assert_eq!(new_epoch, 19);
            prop_assert_eq!(new_shares.len(), usize::from(new_count));
            prop_assert_eq!(new_digests.len(), usize::from(new_count));

            let share_values: Vec<Share> = new_shares.values().cloned().collect();
            let threshold_usize = usize::from(new_threshold);
            let first_secret = shamir::reconstruct_secret(&share_values[..threshold_usize]).unwrap();
            let last_secret = shamir::reconstruct_secret(&share_values[share_values.len() - threshold_usize..]).unwrap();
            prop_assert_eq!(first_secret, last_secret);

            let prior = chain::decrypt_chain(&encrypted_chain, &first_secret, b"prop-cluster").unwrap();
            prop_assert_eq!(prior.len(), 1);
            prop_assert_eq!(prior.get(&old_epoch), Some(old_secret.as_bytes()));
            prop_assert_eq!(encrypted_chain.prior_count, 1);
            prop_assert_eq!(coordinator.state(), &ReconfigState::Preparing);
        }
    }
}
