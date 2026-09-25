
impl ReplicaEvidenceLedger {
    pub fn new(plan: &ReplicaStartPlan) -> crate::error::Result<Self> {
        Self::with_capacity(plan, MAX_REPLICA_EVIDENCE_RECORDS)
    }

    pub(super) fn with_capacity(plan: &ReplicaStartPlan, capacity: usize) -> crate::error::Result<Self> {
        if capacity == 0 || capacity > MAX_REPLICA_EVIDENCE_RECORDS {
            return Err(crate::error::MoltenError::invalid_harness(
                "live Raft evidence capacity is outside its static bound",
            ));
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
    ) -> crate::error::Result<()> {
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
            self.suppressed_heartbeat_count = self.suppressed_heartbeat_count.checked_add(1).ok_or_else(|| {
                crate::error::MoltenError::invalid_harness("live Raft suppressed heartbeat count overflow")
            })?;
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
    ) -> crate::error::Result<ReplicaAggregateHealthEvidence> {
        health::aggregate(self, state, production_admitted)
    }

    fn observe_applied(
        &mut self,
        before: &ReplicaState,
        event: &ReplicaEvent,
        executed: &ExecutedReplicaTransition,
    ) -> crate::error::Result<()> {
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
                .ok_or_else(|| {
                    crate::error::MoltenError::invalid_harness("live Raft snapshot evidence lost its snapshot")
                })?
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
    ) -> crate::error::Result<()> {
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

    fn record_failure(&mut self, state: &ReplicaState, class: &str, diagnostic: &str) -> crate::error::Result<()> {
        let source_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-failure-source-v1", vec![
                crate::preserves_rail::string(class),
                crate::preserves_rail::string(diagnostic),
            ]))?;
        self.record(ReplicaEvidenceKind::Failure, state.current_term, state.commit_index, source_ref)
    }

    fn record(
        &mut self,
        kind: ReplicaEvidenceKind,
        term: u64,
        index: u64,
        source_ref: String,
    ) -> crate::error::Result<()> {
        self.record_with_quorum(kind, term, index, source_ref, None)
    }

    fn record_with_quorum(
        &mut self,
        kind: ReplicaEvidenceKind,
        term: u64,
        index: u64,
        source_ref: String,
        quorum: Option<ValidatedReplicaQuorumEvidence>,
    ) -> crate::error::Result<()> {
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
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Raft evidence sequence overflow"))?;
        Ok(())
    }
}

fn observation_ref(observations: &[ReplicaEffectObservation], kind: ReplicaEffectKind) -> Option<String> {
    observations
        .iter()
        .find(|observation| observation.kind == kind)
        .map(|observation| observation.evidence_ref.clone())
}

fn required_observation_ref(
    observations: &[ReplicaEffectObservation],
    kind: ReplicaEffectKind,
) -> crate::error::Result<String> {
    observation_ref(observations, kind).ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("live Raft selected evidence lacks its effect observation")
    })
}

fn commit_quorum_evidence(
    state: &ReplicaState,
    source_ref: String,
) -> crate::error::Result<ValidatedReplicaQuorumEvidence> {
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
