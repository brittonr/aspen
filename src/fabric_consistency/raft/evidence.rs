mod health;

use std::collections::BTreeSet;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

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
pub fn validate_replica_quorum_evidence(evidence: &ReplicaQuorumEvidence) -> Result<ValidatedReplicaQuorumEvidence> {
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

fn validated_quorum_member_sets(evidence: &ReplicaQuorumEvidence) -> Result<(BTreeSet<String>, Vec<String>)> {
    if evidence.admitted_voters.len() != STATIC_VOTER_COUNT {
        return Err(MoltenError::invalid_harness("Raft quorum evidence does not bind the exact static voter count"));
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

fn unique_quorum_members(members: &[String], label: &str, duplicate_diagnostic: &str) -> Result<BTreeSet<String>> {
    for member in members {
        if member.is_empty() || member.len() > MAX_QUORUM_MEMBER_IDENTIFIER_BYTES {
            return Err(MoltenError::invalid_harness(format!(
                "Raft quorum evidence {label} is empty or exceeds its byte bound"
            )));
        }
    }
    let unique = members.iter().cloned().collect::<BTreeSet<_>>();
    if unique.len() != members.len() {
        return Err(MoltenError::invalid_harness(duplicate_diagnostic));
    }
    Ok(unique)
}

fn require_admitted_majority(admitted: &BTreeSet<String>, acknowledgements: &BTreeSet<String>) -> Result<()> {
    if acknowledgements.iter().any(|member| !admitted.contains(member)) {
        return Err(MoltenError::invalid_harness(
            "Raft quorum evidence contains an acknowledgement outside admitted membership",
        ));
    }
    if acknowledgements.len() < STATIC_QUORUM_COUNT {
        return Err(MoltenError::invalid_harness(
            "Raft quorum evidence lacks the required distinct admitted acknowledgements",
        ));
    }
    Ok(())
}

fn quorum_evidence_ref(
    evidence: &ReplicaQuorumEvidence,
    admitted: &BTreeSet<String>,
    acknowledgement_members: &[String],
) -> Result<String> {
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

impl ReplicaEvidenceLedger {
    pub fn new(plan: &ReplicaStartPlan) -> Result<Self> {
        Self::with_capacity(plan, MAX_REPLICA_EVIDENCE_RECORDS)
    }

    pub(super) fn with_capacity(plan: &ReplicaStartPlan, capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > MAX_REPLICA_EVIDENCE_RECORDS {
            return Err(MoltenError::invalid_harness("live Raft evidence capacity is outside its static bound"));
        }
        crate::preserves_rail::validate_content_ref(&plan.state.profile.group_binding_ref)?;
        let mut ledger = Self {
            group_binding_ref: plan.state.profile.group_binding_ref.clone(),
            service_generation: plan.state.profile.service_generation,
            node_id: plan.state.node_id.clone(),
            capacity,
            next_sequence: 1,
            records: Vec::with_capacity(capacity),
            suppressed_heartbeat_count: 0,
            saturated: false,
            diagnostic: None,
        };
        ledger.record(
            ReplicaEvidenceKind::GroupAdmission,
            plan.state.current_term,
            INITIAL_COMMIT_INDEX,
            plan.state.profile.group_binding_ref.clone(),
        )?;
        ledger.record(
            ReplicaEvidenceKind::Configuration,
            plan.state.profile.fencing_epoch,
            plan.state.membership.config_epoch,
            plan.state.membership.membership_ref.clone(),
        )?;
        if plan.initial_effects.iter().any(|effect| {
            matches!(effect, ReplicaEffect::RestoreApplicationSnapshot { .. } | ReplicaEffect::ApplyCommitted { .. })
        }) {
            let source_ref = plan
                .state
                .snapshot
                .as_ref()
                .map_or_else(|| plan.state.profile.group_binding_ref.clone(), |snapshot| snapshot.snapshot_ref.clone());
            ledger.record(
                ReplicaEvidenceKind::Recovery,
                plan.state.current_term,
                plan.state.commit_index,
                source_ref,
            )?;
        }
        Ok(ledger)
    }

    pub fn records(&self) -> &[ReplicaEvidenceRecord] {
        &self.records
    }

    pub const fn suppressed_heartbeat_count(&self) -> u64 {
        self.suppressed_heartbeat_count
    }

    pub const fn saturated(&self) -> bool {
        self.saturated
    }

    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    pub fn observe(
        &mut self,
        before: &ReplicaState,
        event: &ReplicaEvent,
        outcome: &ReplicaExecutionOutcome,
    ) -> Result<()> {
        let records_before = self.records.len();
        match outcome {
            ReplicaExecutionOutcome::Applied(executed) => self.observe_applied(before, event, executed)?,
            ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
                self.record_failure(before, "denied", diagnostic)?;
            }
            ReplicaExecutionOutcome::Failed(failed) => {
                self.record_failure(before, failed.failed_kind.as_str(), &failed.diagnostic)?;
            }
        }
        if matches!(event, ReplicaEvent::HeartbeatTimeout) && self.records.len() == records_before {
            self.suppressed_heartbeat_count = self
                .suppressed_heartbeat_count
                .checked_add(1)
                .ok_or_else(|| MoltenError::invalid_harness("live Raft suppressed heartbeat count overflow"))?;
        }
        Ok(())
    }

    pub fn note_internal_error(&mut self, diagnostic: String) {
        self.diagnostic = Some(diagnostic);
    }

    pub fn aggregate_health(
        &self,
        state: &ReplicaState,
        production_admitted: bool,
    ) -> Result<ReplicaAggregateHealthEvidence> {
        health::aggregate(self, state, production_admitted)
    }

    fn observe_applied(
        &mut self,
        before: &ReplicaState,
        event: &ReplicaEvent,
        executed: &ExecutedReplicaTransition,
    ) -> Result<()> {
        if executed.next.commit_index > before.commit_index && executed.next.role == ReplicaRole::Leader {
            let source_ref = required_observation_ref(&executed.observations, ReplicaEffectKind::PersistCommit)?;
            let quorum = commit_quorum_evidence(&executed.next, source_ref.clone())?;
            self.record_with_quorum(
                ReplicaEvidenceKind::Commit,
                executed.next.current_term,
                executed.next.commit_index,
                source_ref,
                Some(quorum),
            )?;
        }
        self.record_read_currentness(before, event, executed)?;
        if executed.next.snapshot != before.snapshot {
            let source_ref = executed
                .next
                .snapshot
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("live Raft snapshot evidence lost its snapshot"))?
                .snapshot_ref
                .clone();
            self.record(
                ReplicaEvidenceKind::Snapshot,
                executed.next.current_term,
                executed.next.last_applied,
                source_ref,
            )?;
        }
        Ok(())
    }

    fn record_read_currentness(
        &mut self,
        before: &ReplicaState,
        event: &ReplicaEvent,
        executed: &ExecutedReplicaTransition,
    ) -> Result<()> {
        let Some(source_ref) = observation_ref(&executed.observations, ReplicaEffectKind::ReadOutcome) else {
            return Ok(());
        };
        let Some((read_index, quorum)) = read_quorum_evidence(before, event, source_ref.clone())? else {
            return Ok(());
        };
        self.record_with_quorum(
            ReplicaEvidenceKind::ReadCurrentness,
            executed.next.current_term,
            read_index,
            source_ref,
            Some(quorum),
        )
    }

    fn record_failure(&mut self, state: &ReplicaState, class: &str, diagnostic: &str) -> Result<()> {
        let source_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-failure-source-v1", vec![
                crate::preserves_rail::string(class),
                crate::preserves_rail::string(diagnostic),
            ]))?;
        self.record(ReplicaEvidenceKind::Failure, state.current_term, state.commit_index, source_ref)
    }

    fn record(&mut self, kind: ReplicaEvidenceKind, term: u64, index: u64, source_ref: String) -> Result<()> {
        self.record_with_quorum(kind, term, index, source_ref, None)
    }

    fn record_with_quorum(
        &mut self,
        kind: ReplicaEvidenceKind,
        term: u64,
        index: u64,
        source_ref: String,
        quorum: Option<ValidatedReplicaQuorumEvidence>,
    ) -> Result<()> {
        crate::preserves_rail::validate_content_ref(&source_ref)?;
        if self.records.len() == self.capacity {
            self.saturated = true;
            return Ok(());
        }
        let sequence = self.next_sequence;
        let quorum_evidence_ref = quorum.as_ref().map(|evidence| evidence.evidence_ref.clone());
        let quorum_members = quorum.map_or_else(Vec::new, |evidence| evidence.acknowledgement_members);
        let quorum_value = quorum_evidence_ref.as_deref().map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |reference| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)]),
        );
        let evidence_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-selected-evidence-v2", vec![
                crate::preserves_rail::string(&self.group_binding_ref),
                crate::preserves_rail::u64_value(self.service_generation),
                crate::preserves_rail::string(&self.node_id),
                crate::preserves_rail::u64_value(sequence),
                crate::preserves_rail::string(kind.as_str()),
                crate::preserves_rail::u64_value(term),
                crate::preserves_rail::u64_value(index),
                crate::preserves_rail::string(&source_ref),
                quorum_value,
                crate::preserves_rail::sequence(quorum_members.iter().map(crate::preserves_rail::string).collect()),
            ]))?;
        self.records.push(ReplicaEvidenceRecord {
            sequence,
            kind,
            term,
            index,
            source_ref,
            quorum_evidence_ref,
            quorum_members,
            evidence_ref,
        });
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("live Raft evidence sequence overflow"))?;
        Ok(())
    }
}

fn observation_ref(observations: &[ReplicaEffectObservation], kind: ReplicaEffectKind) -> Option<String> {
    observations
        .iter()
        .find(|observation| observation.kind == kind)
        .map(|observation| observation.evidence_ref.clone())
}

fn required_observation_ref(observations: &[ReplicaEffectObservation], kind: ReplicaEffectKind) -> Result<String> {
    observation_ref(observations, kind)
        .ok_or_else(|| MoltenError::invalid_harness("live Raft selected evidence lacks its effect observation"))
}

fn commit_quorum_evidence(state: &ReplicaState, source_ref: String) -> Result<ValidatedReplicaQuorumEvidence> {
    let acknowledgement_members = state
        .membership
        .voters
        .iter()
        .filter(|voter| {
            voter.as_str() == state.node_id
                || state.match_index.get(voter.as_str()).copied().unwrap_or(INITIAL_COMMIT_INDEX) >= state.commit_index
        })
        .cloned()
        .collect();
    validate_replica_quorum_evidence(&ReplicaQuorumEvidence {
        boundary: ReplicaQuorumEvidenceBoundary::Commit,
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        config_epoch: state.membership.config_epoch,
        term: state.current_term,
        index: state.commit_index,
        admitted_voters: state.membership.voters.clone(),
        acknowledgement_members,
        source_ref,
    })
}

fn read_quorum_evidence(
    before: &ReplicaState,
    event: &ReplicaEvent,
    source_ref: String,
) -> Result<Option<(u64, ValidatedReplicaQuorumEvidence)>> {
    let ReplicaEvent::Message { envelope } = event else {
        return Ok(None);
    };
    let RaftMessage::ReadAcknowledgement {
        term,
        follower_id,
        request_ref,
        ..
    } = &envelope.message
    else {
        return Ok(None);
    };
    let pending = before
        .pending_reads
        .get(request_ref)
        .ok_or_else(|| MoltenError::invalid_harness("read-currentness evidence lost its pending read"))?;
    let mut acknowledgement_members = pending.acknowledgements.iter().cloned().collect::<Vec<_>>();
    acknowledgement_members.push(follower_id.clone());
    let validated = validate_replica_quorum_evidence(&ReplicaQuorumEvidence {
        boundary: ReplicaQuorumEvidenceBoundary::ReadCurrentness,
        group_binding_ref: before.profile.group_binding_ref.clone(),
        membership_ref: before.membership.membership_ref.clone(),
        config_epoch: before.membership.config_epoch,
        term: *term,
        index: pending.required_index,
        admitted_voters: before.membership.voters.clone(),
        acknowledgement_members,
        source_ref,
    })?;
    Ok(Some((pending.required_index, validated)))
}
