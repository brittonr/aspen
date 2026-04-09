//! Trust initialization for Shamir cluster secret sharing.
//!
//! When trust is enabled during cluster init, the leader:
//! 1. Generates a 32-byte cluster root secret
//! 2. Splits it into K-of-N Shamir shares
//! 3. Computes SHA3-256 digests for each share
//! 4. Commits a `TrustInitialize` Raft request carrying per-node share bytes
//! 5. Lets each node persist its own share when the request is applied locally

use aspen_cluster_types::InitRequest;
use aspen_trust::secret::ClusterSecret;
use aspen_trust::secret::Threshold;
use aspen_trust::shamir;
use rand::rngs::ThreadRng;
use tracing::info;

use super::RaftNode;
use crate::StateMachineVariant;
use crate::types::AppRequest;
use crate::types::TrustInitializePayload;

pub(crate) fn build_trust_initialize_payload(request: &InitRequest) -> Result<TrustInitializePayload, String> {
    let total_members = request.initial_members.len();
    let threshold = match request.trust.threshold {
        Some(value) => Threshold::new(value).ok_or_else(|| "threshold must be >= 1".to_string())?,
        None => Threshold::default_for_cluster_size(total_members as u32),
    };

    if threshold.value() as usize > total_members {
        return Err(format!("threshold {} exceeds cluster size {total_members}", threshold.value()));
    }

    info!(
        threshold = threshold.value(),
        total = total_members,
        "generating cluster secret and splitting into shares"
    );

    let secret = ClusterSecret::generate();
    let mut rng: ThreadRng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), threshold.value(), total_members as u8, &mut rng)
        .map_err(|e| format!("failed to split secret: {e}"))?;

    let shares: Vec<(u64, Vec<u8>)> = request
        .initial_members
        .iter()
        .zip(shares.iter())
        .map(|(member, share)| (member.id, share.to_bytes().to_vec()))
        .collect();

    let digests: Vec<(u64, shamir::ShareDigest)> = request
        .initial_members
        .iter()
        .zip(shares_from_payload(&shares)?.iter())
        .map(|(member, share)| (member.id, shamir::share_digest(share)))
        .collect();

    Ok(TrustInitializePayload {
        epoch: 1,
        shares,
        digests,
    })
}

fn shares_from_payload(shares: &[(u64, Vec<u8>)]) -> Result<Vec<shamir::Share>, String> {
    let mut decoded = Vec::with_capacity(shares.len());
    for (_node_id, share_bytes) in shares {
        if share_bytes.len() != shamir::SECRET_SIZE + 1 {
            return Err(format!("invalid share length {}, expected {}", share_bytes.len(), shamir::SECRET_SIZE + 1));
        }
        let mut array = [0u8; shamir::SECRET_SIZE + 1];
        array.copy_from_slice(share_bytes);
        let share = shamir::Share::from_bytes(&array).map_err(|e| format!("failed to decode share bytes: {e}"))?;
        decoded.push(share);
    }
    Ok(decoded)
}

impl RaftNode {
    /// Initialize trust (Shamir secret sharing) during cluster bootstrap.
    ///
    /// Called by the leader after successful Raft initialization.
    pub(crate) async fn initialize_trust(&self, request: &InitRequest) -> Result<(), String> {
        match self.state_machine() {
            StateMachineVariant::Redb(_) => {}
            StateMachineVariant::InMemory(_) => {
                return Err("trust requires redb storage (not in-memory)".to_string());
            }
        }

        let payload = build_trust_initialize_payload(request)?;
        let local_node_id: u64 = self.node_id().into();
        let share_x = payload
            .shares
            .iter()
            .find_map(|(node_id, share_bytes)| {
                if *node_id == local_node_id {
                    share_bytes.first().copied()
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("this node {local_node_id} not found in initial_members"))?;

        self.raft()
            .client_write(AppRequest::TrustInitialize(payload.clone()))
            .await
            .map_err(|e| format!("failed to commit trust initialization: {e}"))?;

        info!(
            epoch = payload.epoch,
            node_id = local_node_id,
            share_x,
            digests = payload.digests.len(),
            "trust initialized through committed raft request"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use aspen_cluster_types::ClusterNode;

    use super::*;

    fn request_with_ids(ids: &[u64], threshold: Option<u8>) -> InitRequest {
        InitRequest {
            initial_members: ids.iter().map(|id| ClusterNode::new(*id, format!("node-{id}"), None)).collect(),
            trust: match threshold {
                Some(value) => aspen_cluster_types::TrustConfig::with_threshold(value),
                None => aspen_cluster_types::TrustConfig::enabled(),
            },
        }
    }

    #[test]
    fn test_build_trust_initialize_payload_assigns_every_member() {
        let request = request_with_ids(&[1, 2, 3], None);
        let payload = build_trust_initialize_payload(&request).unwrap();

        assert_eq!(payload.epoch, 1);
        assert_eq!(payload.shares.len(), 3);
        assert_eq!(payload.digests.len(), 3);

        let decoded = shares_from_payload(&payload.shares).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(shamir::reconstruct_secret(&decoded[..2]).unwrap().len(), shamir::SECRET_SIZE);

        for ((node_id, share_bytes), (_, digest)) in payload.shares.iter().zip(payload.digests.iter()) {
            let mut share_array = [0u8; shamir::SECRET_SIZE + 1];
            share_array.copy_from_slice(share_bytes);
            let share = shamir::Share::from_bytes(&share_array).unwrap();
            assert_eq!(shamir::share_digest(&share), *digest);
            assert!([1u64, 2, 3].contains(node_id));
        }
    }

    #[test]
    fn test_build_trust_initialize_payload_rejects_invalid_threshold() {
        let request = request_with_ids(&[1, 2, 3], Some(4));
        let err = build_trust_initialize_payload(&request).unwrap_err();
        assert!(err.contains("exceeds cluster size 3"));
    }
}
