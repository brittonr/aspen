//! Trust initialization for Shamir cluster secret sharing.
//!
//! When trust is enabled during cluster init, the leader:
//! 1. Generates a 32-byte cluster root secret
//! 2. Splits it into K-of-N Shamir shares
//! 3. Computes SHA3-256 digests for each share
//! 4. Stores each node's share in the trust_shares table
//! 5. Stores all digests in the trust_digests table
//!
//! For single-node init, the leader stores its own share directly.
//! For multi-node init, shares are distributed via Raft-committed entries.

use std::collections::BTreeMap;

use aspen_cluster_types::InitRequest;
use aspen_trust::secret::ClusterSecret;
use aspen_trust::secret::Threshold;
use aspen_trust::shamir;
use rand::rngs::ThreadRng;
use tracing::info;

use super::RaftNode;

impl RaftNode {
    /// Initialize trust (Shamir secret sharing) during cluster bootstrap.
    ///
    /// Called by the leader after successful Raft initialization.
    pub(crate) fn initialize_trust(&self, request: &InitRequest) -> Result<(), String> {
        let n = request.initial_members.len();
        let threshold = match request.trust.threshold {
            Some(t) => Threshold::new(t).ok_or_else(|| "threshold must be >= 1".to_string())?,
            None => Threshold::default_for_cluster_size(n as u32),
        };

        // Validate threshold against cluster size
        if threshold.value() as usize > n {
            return Err(format!("threshold {} exceeds cluster size {n}", threshold.value()));
        }

        info!(threshold = threshold.value(), total = n, "generating cluster secret and splitting into shares");

        // Generate the root secret
        let secret = ClusterSecret::generate();

        // Split into shares
        let mut rng: ThreadRng = rand::rng();
        let shares = shamir::split_secret(secret.as_bytes(), threshold.value(), n as u8, &mut rng)
            .map_err(|e| format!("failed to split secret: {e}"))?;

        // Compute digests for all shares
        let epoch: u64 = 1; // First epoch
        let mut digests: BTreeMap<u64, shamir::ShareDigest> = BTreeMap::new();
        for (i, share) in shares.iter().enumerate() {
            let node_id = request.initial_members[i].id;
            digests.insert(node_id, shamir::share_digest(share));
        }

        // Store this node's share and all digests
        let self_node_id: u64 = self.node_id().into();
        let self_share_idx = request
            .initial_members
            .iter()
            .position(|m| m.id == self_node_id)
            .ok_or_else(|| "this node not found in initial_members".to_string())?;

        match self.state_machine() {
            crate::StateMachineVariant::Redb(storage) => {
                storage
                    .store_share(epoch, &shares[self_share_idx])
                    .map_err(|e| format!("failed to store share: {e}"))?;
                storage.store_digests(epoch, &digests).map_err(|e| format!("failed to store digests: {e}"))?;
            }
            crate::StateMachineVariant::InMemory(_) => {
                // In-memory state machine doesn't support trust storage.
                // Trust is only available with redb storage.
                return Err("trust requires redb storage (not in-memory)".to_string());
            }
        }

        info!(
            epoch = epoch,
            node_id = self_node_id,
            share_x = shares[self_share_idx].x,
            "trust initialized: share stored, {} digests recorded",
            digests.len()
        );

        Ok(())
    }
}
