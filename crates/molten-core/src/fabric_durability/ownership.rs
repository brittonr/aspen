//! Pure object-store compare-and-swap lease decisions.
//!
//! The shell owns durable-store reads and writes. This module only compares
//! supplied lease snapshots and returns a deterministic decision.

const CAS_LEASE_CONTRACT_IDENTITY: &str = "aspen.cas.contract.v1";
const CAS_LEASE_IMPLEMENTATION_IDENTITY: &str = "molten-core.cas-coordinator.v1";
const CAS_LEASE_DECISION_CONTEXT: &str = "molten.cas-lease-decision.v1";
const MAX_OWNER_BYTES: usize = 256;
const NON_CLAIM_RUNTIME_CORRECTNESS: &str = "does-not-prove-runtime-correctness";
const NON_CLAIM_DATA_INTEGRITY: &str = "does-not-prove-data-integrity";
const NON_CLAIM_RELEASE_READINESS: &str = "does-not-prove-release-readiness";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasLease {
    pub owner: String,
    pub epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipPosture {
    ReplaceableNodes,
    FixedMembership,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasLeaseDecisionInput {
    pub current: CasLease,
    pub expected: CasLease,
    pub proposed: CasLease,
    pub membership: MembershipPosture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasLeaseDisposition {
    Acquire,
    Reject,
}

impl CasLeaseDisposition {
    fn identity(self) -> &'static str {
        match self {
            Self::Acquire => "acquire",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasLeaseRejection {
    InvalidOwner,
    FixedMembership,
    OwnerMismatch,
    EpochMismatch,
    EpochNotAdvanced,
}

impl CasLeaseRejection {
    fn identity(self) -> &'static str {
        match self {
            Self::InvalidOwner => "invalid-owner",
            Self::FixedMembership => "fixed-membership",
            Self::OwnerMismatch => "owner-mismatch",
            Self::EpochMismatch => "epoch-mismatch",
            Self::EpochNotAdvanced => "epoch-not-advanced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasLeaseDecision {
    pub before: CasLease,
    pub after: CasLease,
    pub disposition: CasLeaseDisposition,
    pub rejection: Option<CasLeaseRejection>,
    pub contract_identity: &'static str,
    pub implementation_identity: &'static str,
    pub decision_ref: String,
    pub non_claims: Vec<&'static str>,
}

// r[impl aspen.cas.contract]
// r[impl aspen.cas.decision]
// r[impl aspen.cas.boundary]
pub fn decide_cas_lease(input: &CasLeaseDecisionInput) -> CasLeaseDecision {
    let rejection = rejection_reason(input);
    let disposition = if rejection.is_none() {
        CasLeaseDisposition::Acquire
    } else {
        CasLeaseDisposition::Reject
    };
    let after = if disposition == CasLeaseDisposition::Acquire {
        input.proposed.clone()
    } else {
        input.current.clone()
    };
    let decision_ref = decision_identity(input, disposition, rejection);

    CasLeaseDecision {
        before: input.current.clone(),
        after,
        disposition,
        rejection,
        contract_identity: CAS_LEASE_CONTRACT_IDENTITY,
        implementation_identity: CAS_LEASE_IMPLEMENTATION_IDENTITY,
        decision_ref,
        non_claims: vec![
            NON_CLAIM_RUNTIME_CORRECTNESS,
            NON_CLAIM_DATA_INTEGRITY,
            NON_CLAIM_RELEASE_READINESS,
        ],
    }
}

fn rejection_reason(input: &CasLeaseDecisionInput) -> Option<CasLeaseRejection> {
    if !valid_owner(&input.current.owner) || !valid_owner(&input.expected.owner) || !valid_owner(&input.proposed.owner)
    {
        return Some(CasLeaseRejection::InvalidOwner);
    }
    if input.membership == MembershipPosture::FixedMembership {
        return Some(CasLeaseRejection::FixedMembership);
    }
    if input.expected.owner != input.current.owner {
        return Some(CasLeaseRejection::OwnerMismatch);
    }
    if input.expected.epoch != input.current.epoch {
        return Some(CasLeaseRejection::EpochMismatch);
    }
    if input.proposed.epoch <= input.current.epoch {
        return Some(CasLeaseRejection::EpochNotAdvanced);
    }
    None
}

fn valid_owner(owner: &str) -> bool {
    !owner.trim().is_empty() && owner.len() <= MAX_OWNER_BYTES && !owner.contains('\0')
}

fn decision_identity(
    input: &CasLeaseDecisionInput,
    disposition: CasLeaseDisposition,
    rejection: Option<CasLeaseRejection>,
) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(CAS_LEASE_DECISION_CONTEXT);
    hash_text(&mut hasher, "current-owner", &input.current.owner);
    hash_number(&mut hasher, "current-epoch", input.current.epoch);
    hash_text(&mut hasher, "expected-owner", &input.expected.owner);
    hash_number(&mut hasher, "expected-epoch", input.expected.epoch);
    hash_text(&mut hasher, "proposed-owner", &input.proposed.owner);
    hash_number(&mut hasher, "proposed-epoch", input.proposed.epoch);
    hash_text(&mut hasher, "membership", membership_identity(input.membership));
    hash_text(&mut hasher, "disposition", disposition.identity());
    hash_text(&mut hasher, "rejection", rejection.map(CasLeaseRejection::identity).unwrap_or("none"));
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn membership_identity(posture: MembershipPosture) -> &'static str {
    match posture {
        MembershipPosture::ReplaceableNodes => "replaceable-nodes",
        MembershipPosture::FixedMembership => "fixed-membership",
    }
}

fn hash_number(hasher: &mut blake3::Hasher, label: &str, value: u64) {
    hash_text(hasher, label, &value.to_string());
}

fn hash_text(hasher: &mut blake3::Hasher, label: &str, value: &str) {
    hasher.update(label.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(label.as_bytes());
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_EPOCH: u64 = 7;
    const NEXT_EPOCH: u64 = 8;
    const STALE_EPOCH: u64 = CURRENT_EPOCH;
    const CURRENT_OWNER: &str = "node-a";
    const NEXT_OWNER: &str = "node-b";
    const LOST_OWNER: &str = "node-lost";

    fn lease(owner: &str, epoch: u64) -> CasLease {
        CasLease {
            owner: owner.to_string(),
            epoch,
        }
    }

    fn valid_transfer() -> CasLeaseDecisionInput {
        CasLeaseDecisionInput {
            current: lease(CURRENT_OWNER, CURRENT_EPOCH),
            expected: lease(CURRENT_OWNER, CURRENT_EPOCH),
            proposed: lease(NEXT_OWNER, NEXT_EPOCH),
            membership: MembershipPosture::ReplaceableNodes,
        }
    }

    // r[verify aspen.cas.contract]
    // r[verify aspen.cas.decision]
    // r[verify aspen.cas.verification]
    #[test]
    fn matching_expected_lease_with_advanced_epoch_acquires() {
        let input = valid_transfer();
        let decision = decide_cas_lease(&input);

        assert_eq!(decision.disposition, CasLeaseDisposition::Acquire);
        assert_eq!(decision.rejection, None);
        assert_eq!(decision.before, input.current);
        assert_eq!(decision.after, input.proposed);
        assert!(decision.decision_ref.starts_with("blake3:"));
        assert_eq!(decision.decision_ref, decide_cas_lease(&input).decision_ref);
    }

    // r[verify aspen.cas.decision]
    // r[verify aspen.cas.verification]
    #[test]
    fn mismatched_expected_owner_rejects_without_state_change() {
        let mut input = valid_transfer();
        input.expected.owner = LOST_OWNER.to_string();
        let decision = decide_cas_lease(&input);

        assert_eq!(decision.disposition, CasLeaseDisposition::Reject);
        assert_eq!(decision.rejection, Some(CasLeaseRejection::OwnerMismatch));
        assert_eq!(decision.after, input.current);
    }

    #[test]
    fn stale_expected_epoch_rejects_without_state_change() {
        let mut input = valid_transfer();
        input.expected.epoch = CURRENT_EPOCH - 1;
        let decision = decide_cas_lease(&input);

        assert_eq!(decision.disposition, CasLeaseDisposition::Reject);
        assert_eq!(decision.rejection, Some(CasLeaseRejection::EpochMismatch));
        assert_eq!(decision.after, input.current);
    }

    #[test]
    fn non_advanced_proposed_epoch_rejects_without_state_change() {
        let mut input = valid_transfer();
        input.proposed.epoch = STALE_EPOCH;
        let decision = decide_cas_lease(&input);

        assert_eq!(decision.disposition, CasLeaseDisposition::Reject);
        assert_eq!(decision.rejection, Some(CasLeaseRejection::EpochNotAdvanced));
        assert_eq!(decision.after, input.current);
    }

    // r[verify aspen.cas.boundary]
    // r[verify aspen.cas.verification]
    #[test]
    fn lost_lease_and_fixed_membership_assumptions_fail_closed() {
        let mut lost = valid_transfer();
        lost.current.owner = NEXT_OWNER.to_string();
        let lost_decision = decide_cas_lease(&lost);
        assert_eq!(lost_decision.rejection, Some(CasLeaseRejection::OwnerMismatch));
        assert_eq!(lost_decision.after, lost.current);

        let mut fixed = valid_transfer();
        fixed.membership = MembershipPosture::FixedMembership;
        let fixed_decision = decide_cas_lease(&fixed);
        assert_eq!(fixed_decision.rejection, Some(CasLeaseRejection::FixedMembership));
        assert_eq!(fixed_decision.after, fixed.current);
        assert!(fixed_decision.non_claims.contains(&NON_CLAIM_RUNTIME_CORRECTNESS));
        assert!(fixed_decision.non_claims.contains(&NON_CLAIM_DATA_INTEGRITY));
        assert!(fixed_decision.non_claims.contains(&NON_CLAIM_RELEASE_READINESS));
    }

    #[test]
    fn malformed_owner_rejects_before_acquisition() {
        let mut input = valid_transfer();
        input.proposed.owner = String::new();
        let decision = decide_cas_lease(&input);

        assert_eq!(decision.disposition, CasLeaseDisposition::Reject);
        assert_eq!(decision.rejection, Some(CasLeaseRejection::InvalidOwner));
        assert_eq!(decision.after, input.current);
    }
}
