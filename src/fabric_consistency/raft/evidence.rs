mod health;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub const MAX_REPLICA_EVIDENCE_RECORDS: usize = 1_024;

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
    pub evidence_ref: String,
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
            ReplicaExecutionOutcome::Applied(executed) => self.observe_applied(before, executed)?,
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

    fn observe_applied(&mut self, before: &ReplicaState, executed: &ExecutedReplicaTransition) -> Result<()> {
        if executed.next.commit_index > before.commit_index {
            self.record_from_observation(
                ReplicaEvidenceKind::Commit,
                &executed.next,
                &executed.observations,
                ReplicaEffectKind::PersistCommit,
            )?;
        }
        if executed.observations.iter().any(|item| item.kind == ReplicaEffectKind::ReadOutcome) {
            self.record_from_observation(
                ReplicaEvidenceKind::ReadCurrentness,
                &executed.next,
                &executed.observations,
                ReplicaEffectKind::ReadOutcome,
            )?;
        }
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

    fn record_from_observation(
        &mut self,
        kind: ReplicaEvidenceKind,
        state: &ReplicaState,
        observations: &[ReplicaEffectObservation],
        observation_kind: ReplicaEffectKind,
    ) -> Result<()> {
        let source_ref = observations
            .iter()
            .find(|observation| observation.kind == observation_kind)
            .map(|observation| observation.evidence_ref.clone())
            .ok_or_else(|| MoltenError::invalid_harness("live Raft selected evidence lacks its effect observation"))?;
        self.record(kind, state.current_term, state.last_applied, source_ref)
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
        crate::preserves_rail::validate_content_ref(&source_ref)?;
        if self.records.len() == self.capacity {
            self.saturated = true;
            return Ok(());
        }
        let sequence = self.next_sequence;
        let evidence_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-selected-evidence-v1", vec![
                crate::preserves_rail::string(&self.group_binding_ref),
                crate::preserves_rail::u64_value(self.service_generation),
                crate::preserves_rail::string(&self.node_id),
                crate::preserves_rail::u64_value(sequence),
                crate::preserves_rail::string(kind.as_str()),
                crate::preserves_rail::u64_value(term),
                crate::preserves_rail::u64_value(index),
                crate::preserves_rail::string(&source_ref),
            ]))?;
        self.records.push(ReplicaEvidenceRecord {
            sequence,
            kind,
            term,
            index,
            source_ref,
            evidence_ref,
        });
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("live Raft evidence sequence overflow"))?;
        Ok(())
    }
}
