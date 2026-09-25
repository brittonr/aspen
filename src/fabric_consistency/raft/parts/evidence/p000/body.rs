
use super::*;

pub const MAX_REPLICA_EVIDENCE_RECORDS: usize = 1_024;
const MAX_QUORUM_MEMBER_IDENTIFIER_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaQuorumEvidenceBoundary {
    Commit,
    ReadCurrentness,
}

impl ReplicaQuorumEvidenceBoundary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::ReadCurrentness => "read-currentness",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaQuorumEvidence {
    pub boundary: ReplicaQuorumEvidenceBoundary,
    pub group_binding_ref: String,
    pub membership_ref: String,
    pub config_epoch: u64,
    pub term: u64,
    pub index: u64,
    pub admitted_voters: Vec<String>,
    pub acknowledgement_members: Vec<String>,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedReplicaQuorumEvidence {
    pub acknowledgement_members: Vec<String>,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplicaEvidenceKind {
    GroupAdmission,
    Configuration,
    Commit,
    ReadCurrentness,
    Snapshot,
    Recovery,
    Failure,
}

impl ReplicaEvidenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GroupAdmission => "group-admission",
            Self::Configuration => "configuration",
            Self::Commit => "commit",
            Self::ReadCurrentness => "read-currentness",
            Self::Snapshot => "snapshot",
            Self::Recovery => "recovery",
            Self::Failure => "failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaEvidenceRecord {
    pub sequence: u64,
    pub kind: ReplicaEvidenceKind,
    pub term: u64,
    pub index: u64,
    pub source_ref: String,
    pub quorum_evidence_ref: Option<String>,
    pub quorum_members: Vec<String>,
    pub evidence_ref: String,
}

// r[impl molten.fabric_consistency.final_validation]
pub fn validate_replica_quorum_evidence(
    evidence: &ReplicaQuorumEvidence,
) -> crate::error::Result<ValidatedReplicaQuorumEvidence> {
    crate::preserves_rail::validate_content_ref(&evidence.group_binding_ref)?;
    crate::preserves_rail::validate_content_ref(&evidence.membership_ref)?;
    crate::preserves_rail::validate_content_ref(&evidence.source_ref)?;
    let (admitted, acknowledgement_members) = validated_quorum_member_sets(evidence)?;
    let evidence_ref = quorum_evidence_ref(evidence, &admitted, &acknowledgement_members)?;
    Ok(ValidatedReplicaQuorumEvidence {
        acknowledgement_members,
        evidence_ref,
    })
}

fn validated_quorum_member_sets(
    evidence: &ReplicaQuorumEvidence,
) -> crate::error::Result<(std::collections::BTreeSet<String>, Vec<String>)> {
    if evidence.admitted_voters.len() != STATIC_VOTER_COUNT {
        return Err(crate::error::MoltenError::invalid_harness(
            "Raft quorum evidence does not bind the exact static voter count",
        ));
    }
    let admitted = unique_quorum_members(
        &evidence.admitted_voters,
        "admitted voter",
        "Raft quorum evidence contains duplicate admitted voters",
    )?;
    let acknowledgements = unique_quorum_members(
        &evidence.acknowledgement_members,
        "acknowledgement member",
        "Raft quorum evidence contains duplicate acknowledgements",
    )?;
    require_admitted_majority(&admitted, &acknowledgements)?;
    Ok((admitted, acknowledgements.into_iter().collect()))
}

fn unique_quorum_members(
    members: &[String],
    label: &str,
    duplicate_diagnostic: &str,
) -> crate::error::Result<std::collections::BTreeSet<String>> {
    for member in members {
        if member.is_empty() || member.len() > MAX_QUORUM_MEMBER_IDENTIFIER_BYTES {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "Raft quorum evidence {label} is empty or exceeds its byte bound"
            )));
        }
    }
    let unique = members.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != members.len() {
        return Err(crate::error::MoltenError::invalid_harness(duplicate_diagnostic));
    }
    Ok(unique)
}

fn require_admitted_majority(
    admitted: &std::collections::BTreeSet<String>,
    acknowledgements: &std::collections::BTreeSet<String>,
) -> crate::error::Result<()> {
    if acknowledgements.iter().any(|member| !admitted.contains(member)) {
        return Err(crate::error::MoltenError::invalid_harness(
            "Raft quorum evidence contains an acknowledgement outside admitted membership",
        ));
    }
    if acknowledgements.len() < STATIC_QUORUM_COUNT {
        return Err(crate::error::MoltenError::invalid_harness(
            "Raft quorum evidence lacks the required distinct admitted acknowledgements",
        ));
    }
    Ok(())
}

fn quorum_evidence_ref(
    evidence: &ReplicaQuorumEvidence,
    admitted: &std::collections::BTreeSet<String>,
    acknowledgement_members: &[String],
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-quorum-evidence-v1", vec![
        crate::preserves_rail::string(evidence.boundary.as_str()),
        crate::preserves_rail::string(&evidence.group_binding_ref),
        crate::preserves_rail::string(&evidence.membership_ref),
        crate::preserves_rail::u64_value(evidence.config_epoch),
        crate::preserves_rail::u64_value(evidence.term),
        crate::preserves_rail::u64_value(evidence.index),
        crate::preserves_rail::sequence(admitted.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::sequence(acknowledgement_members.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::string(&evidence.source_ref),
    ]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaAggregateHealthEvidence {
    pub status: String,
    pub selected_record_count: usize,
    pub suppressed_heartbeat_count: u64,
    pub saturated: bool,
    pub diagnostic: Option<String>,
    pub evidence_ref: String,
    pub production_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaEvidenceLedger {
    group_binding_ref: String,
    service_generation: u64,
    node_id: String,
    capacity: usize,
    next_sequence: u64,
    records: Vec<ReplicaEvidenceRecord>,
    suppressed_heartbeat_count: u64,
    saturated: bool,
    diagnostic: Option<String>,
}
