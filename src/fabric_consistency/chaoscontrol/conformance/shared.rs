//! Shared consensus semantics across whole-system simulation and ChaosControl.
//!
//! Both profiles may use one canonical operation corpus and one invariant
//! vocabulary. Their evidence-profile labels remain distinct and neither
//! profile can substitute for the other. This module performs no I/O and
//! establishes no authority or release claim.

use std::collections::BTreeSet;

pub const CORPUS_RECORD_LABEL: &str = "molten-consensus-operation-corpus-v1";
pub const INVARIANT_IDS: [&str; 6] = [
    "cohort-agreement",
    "recovered-state-matches-durable-state",
    "operation-applied-more-than-once",
    "operation-command-index-drift",
    "reused-application-receipt",
    "bounded-liveness-failure",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceProfile {
    WholeSystemSimulation,
    ChaosControlKvm,
}

impl EvidenceProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WholeSystemSimulation => "whole-system-simulation",
            Self::ChaosControlKvm => "chaoscontrol-kvm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryIssue {
    Corpus,
    Invariants,
    ProfileSubstitution,
}

impl std::fmt::Display for BoundaryIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Corpus => "corpus",
            Self::Invariants => "invariants",
            Self::ProfileSubstitution => "profile-substitution",
        })
    }
}

impl std::error::Error for BoundaryIssue {}

/// Canonical corpus identity shared by both evidence profiles.
pub fn corpus_ref() -> Result<String, BoundaryIssue> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(CORPUS_RECORD_LABEL, vec![
        crate::preserves_rail::string(crate::fabric_simulation::bounded_client::CLIENT_SCHEMA),
        crate::preserves_rail::string(crate::fabric_simulation::bounded_client::NATIVE_PROFILE),
        crate::preserves_rail::string(crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY),
    ]))
    .map_err(|_| BoundaryIssue::Corpus)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub profile: EvidenceProfile,
    pub operation_corpus_ref: String,
    pub invariant_ids: Vec<String>,
}

/// Construct the one admitted shared claim for a profile.
pub fn claim(profile: EvidenceProfile) -> Result<Claim, BoundaryIssue> {
    Ok(Claim {
        profile,
        operation_corpus_ref: corpus_ref()?,
        invariant_ids: INVARIANT_IDS.iter().map(|value| (*value).to_string()).collect(),
    })
}

/// Require equal canonical semantics and distinct evidence-profile labels.
// r[impl molten.testing.chaoscontrol_smr_claim_boundary]
pub fn compare(left: &Claim, right: &Claim) -> Result<(), BoundaryIssue> {
    if left.profile == right.profile {
        return Err(BoundaryIssue::ProfileSubstitution);
    }
    if left.operation_corpus_ref != right.operation_corpus_ref || left.operation_corpus_ref != corpus_ref()? {
        return Err(BoundaryIssue::Corpus);
    }
    let expected = INVARIANT_IDS.into_iter().collect::<BTreeSet<_>>();
    let left_set = left.invariant_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let right_set = right.invariant_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if left_set != expected || right_set != expected {
        return Err(BoundaryIssue::Invariants);
    }
    Ok(())
}
