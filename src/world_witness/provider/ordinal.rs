//! External index ordinals are provider namespace positions, not Molten head generations.
//! The head generation stays bound inside the canonical claim digest; the index ordinal
//! advances by one except across an authenticated resolved-orphan high-water.

/// Exact consumer claim facts; the external ordinal is derived only from a fresh signed query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub digest: [u8; 32],
    pub predecessor: Option<[u8; 32]>,
}

impl Candidate {
    pub fn matches(&self, claim: &release_channel_index::model::Claim) -> bool {
        claim.digest == self.digest && claim.predecessor == self.predecessor
    }
}

/// Plans the single admissible external claim for `candidate` from an authenticated snapshot.
///
/// An exact pending reservation is recovered unchanged. Otherwise the ordinal is the
/// retained external high-water plus one. A high-water above the committed claim is
/// accepted only as a resolved orphan that extends the candidate's exact predecessor;
/// unresolved reservations, orphans, and abandoned claim reuse fail closed.
// r[impl molten.world_witness.currentness]
pub fn plan(
    snapshot: &release_channel_index::model::Snapshot,
    candidate: &Candidate,
) -> Result<release_channel_index::model::Claim, super::types::Error> {
    snapshot.validate().map_err(super::types::Error::invalid)?;
    let committed = snapshot.committed.as_ref().map(|claim| claim.digest);
    if committed != candidate.predecessor {
        return Err(super::types::Error::invalid("external committed claim is not the consumer's exact predecessor"));
    }
    if let Some(pending) = &snapshot.pending {
        if pending.status == release_channel_index::model::PendingStatus::Reserved && candidate.matches(&pending.claim)
        {
            return Ok(pending.claim.clone());
        }
        return Err(super::types::Error::invalid("unresolved external reservation or orphan blocks advance"));
    }
    let Some(highest) = &snapshot.highest else {
        return Ok(release_channel_index::model::Claim {
            generation: 1,
            digest: candidate.digest,
            predecessor: None,
        });
    };
    if highest.digest == candidate.digest {
        return Err(super::types::Error::invalid("abandoned or committed claim cannot be reserved again"));
    }
    if snapshot.highest != snapshot.committed
        && (highest.predecessor != committed
            || snapshot.committed.as_ref().is_some_and(|claim| highest.generation <= claim.generation))
    {
        return Err(super::types::Error::invalid("external high-water gap is not a resolved orphan"));
    }
    let generation = highest
        .generation
        .checked_add(1)
        .ok_or_else(|| super::types::Error::invalid("external ordinal overflow"))?;
    Ok(release_channel_index::model::Claim {
        generation,
        digest: candidate.digest,
        predecessor: candidate.predecessor,
    })
}
