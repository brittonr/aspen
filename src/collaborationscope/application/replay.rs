use super::*;

/// The pinned verifier's replay port, persisted through the same admitted
/// durability mechanism as application commits. Exhaustion and uncertainty deny.
pub struct NonceLedger {
    store: std::sync::Mutex<Database>,
}

impl NonceLedger {
    pub fn new(store: Database) -> Self {
        Self {
            store: std::sync::Mutex::new(store),
        }
    }

    pub(crate) fn reserve(&self, key_ref: &str, digest: &str) -> Result<(), basalt::ucan::HolderInvocationReplayError> {
        self.store
            .lock()
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?
            .reserve_replay(key_ref, digest)
    }
}

impl basalt::ucan::HolderInvocationReplayAdmission for NonceLedger {
    fn admit_holder_invocation(
        &self,
        candidate: &basalt::ucan::HolderInvocationReplayCandidate<'_>,
    ) -> Result<(), basalt::ucan::HolderInvocationReplayError> {
        let audience =
            crate::preserves_rail::record("collaboration-replay-audience-v1", vec![crate::preserves_rail::string(
                candidate.audience().as_str(),
            )]);
        let audience_ref = crate::preserves_rail::canonical_hash(&audience)
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?;
        let nonce_ref = crate::preserves_rail::content_ref_from_bytes(candidate.nonce().as_bytes());
        let key = crate::preserves_rail::record("collaboration-replay-key-v1", vec![
            crate::preserves_rail::string(audience_ref),
            crate::preserves_rail::string(nonce_ref),
        ]);
        let key_ref = crate::preserves_rail::canonical_hash(&key)
            .map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)?;
        let digest = format!("blake3:{}", blake3::Hash::from(*candidate.request_digest().as_bytes()));
        self.reserve(&key_ref, &digest)
    }
}
