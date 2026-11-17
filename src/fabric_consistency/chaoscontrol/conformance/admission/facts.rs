// Bounded observation projection and independent application/continuity checks.

pub(super) struct Facts {
    // The producer APIs accept contiguous slices; these are the only necessary projection clones.
    pub records: Vec<chaoscontrol_smr::observation::ProtocolObservation>,
    pub chains: Vec<chaoscontrol_smr::ChainObservation>,
    pub operation_ids: Vec<String>,
    pub is_incomplete: bool,
    pub failures: std::collections::BTreeSet<&'static str>,
    pub committed: u64,
}

pub(super) fn facts(
    expected: &super::ExpectedCohort,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    observations: &[super::super::wire::BoundObservation],
    proposals: &[chaoscontrol_smr::ProposalAttempt],
) -> Result<Facts, super::ImportIssue> {
    let p = &expected.protocol;
    let observation_count = u64::try_from(observations.len()).map_err(|_| super::ImportIssue::Bounds)?;
    if observation_count > super::MAX_OBSERVATIONS
        || observation_count > p.bounds.records
        || observation_count > p.bounds.oracle_work
        || observation_count > plan.profile.bounds.maximum_trace_events
        || u64::try_from(proposals.len()).map_err(|_| super::ImportIssue::Bounds)? > plan.profile.bounds.maximum_trace_events
    {
        return Err(super::ImportIssue::Bounds);
    }
    chaoscontrol_smr::validate_proposal_attempts(proposals, &plan.profile.bounds).map_err(|_| super::ImportIssue::Observation)?;
    let mut retained = Accumulator::new(expected, plan, proposals, observations.len())?;
    for observation in observations {
        retained.observe(observation)?;
    }
    retained.complete(proposals, observations)?;
    Ok(retained.facts)
}

struct Accumulator<'a> {
    expected: &'a super::ExpectedCohort,
    plan: &'a chaoscontrol_smr::SmrWorkloadPlan,
    policy: std::collections::BTreeMap<&'a str, &'a super::ExpectedOperation>,
    proposal_ids: std::collections::BTreeSet<&'a str>,
    previous: std::collections::BTreeMap<&'a str, &'a super::super::wire::ApplyBinding>,
    seen_operations: std::collections::BTreeMap<(&'a str, &'a str), u64>,
    seen_receipts: std::collections::BTreeSet<&'a String>,
    operation_indices: std::collections::BTreeMap<u64, String>,
    facts: Facts,
}

impl<'a> Accumulator<'a> {
    fn new(
        expected: &'a super::ExpectedCohort,
        plan: &'a chaoscontrol_smr::SmrWorkloadPlan,
        proposals: &'a [chaoscontrol_smr::ProposalAttempt],
        capacity: usize,
    ) -> Result<Self, super::ImportIssue> {
        let policy: std::collections::BTreeMap<_, _> = expected.operations.iter().map(|o| (o.operation_ref.as_str(), o)).collect();
        let mut proposal_ids = std::collections::BTreeSet::new();
        for proposal in proposals {
            let operation = policy.get(proposal.operation_id.as_str()).ok_or(super::ImportIssue::Identity)?;
            if proposal.logical_operation_id != operation.operation_ref || proposal.command_ref != operation.command_ref {
                return Err(super::ImportIssue::Identity);
            }
            proposal_ids.insert(proposal.operation_id.as_str());
        }
        Ok(Self {
            expected, plan, policy, proposal_ids,
            previous: std::collections::BTreeMap::new(),
            seen_operations: std::collections::BTreeMap::new(),
            seen_receipts: std::collections::BTreeSet::new(),
            operation_indices: std::collections::BTreeMap::new(),
            facts: Facts {
                records: Vec::with_capacity(capacity), chains: Vec::with_capacity(capacity),
                operation_ids: Vec::new(), is_incomplete: false,
                failures: std::collections::BTreeSet::new(), committed: u64::MAX,
            },
        })
    }

    fn observe(&mut self, observation: &'a super::super::wire::BoundObservation) -> Result<(), super::ImportIssue> {
        super::super::wire::validate(&self.expected.protocol, self.plan, &self.expected.group_ref, observation)
            .map_err(|_| super::ImportIssue::Observation)?;
        let envelope = &observation.envelope;
        let apply = &observation.apply;
        let chain = &apply.chain;
        if envelope.source_sequence > super::MAX_OBSERVATIONS {
            return Err(super::ImportIssue::Bounds);
        }
        self.operation(observation)?;
        if !self.seen_receipts.insert(&apply.application_receipt_ref) {
            self.facts.failures.insert("reused-application-receipt");
        }
        if self.seen_operations
            .insert((apply.participant_ref.as_str(), apply.operation_ref.as_str()), chain.command_index)
            .is_some()
        {
            self.facts.failures.insert("operation-applied-more-than-once");
        }
        self.continuity(observation)?;
        self.operation_indices.entry(chain.command_index).or_insert_with(|| apply.operation_ref.clone());
        self.facts.records.push(envelope.clone());
        self.facts.chains.push(chain.clone());
        Ok(())
    }

    fn operation(&mut self, observation: &super::super::wire::BoundObservation) -> Result<(), super::ImportIssue> {
        let envelope = &observation.envelope;
        let apply = &observation.apply;
        let chain = &apply.chain;
        let operation = self.policy.get(apply.operation_ref.as_str()).ok_or(super::ImportIssue::Identity)?;
        if envelope.boundary_ref != operation.boundary_ref
            || envelope.scheduler_ref != self.expected.scheduler_ref
            || envelope.transition_class_ref != operation.transition_class_ref
        {
            return Err(super::ImportIssue::Identity);
        }
        if !self.proposal_ids.contains(apply.operation_ref.as_str()) {
            return Err(super::ImportIssue::Observation);
        }
        if apply.command_ref != operation.command_ref || apply.command_bytes != operation.command_bytes {
            super::bounds::command(&apply.command_bytes, &apply.command_ref)?;
        }
        if apply.command_ref != operation.command_ref
            || apply.command_bytes != operation.command_bytes
            || chain.command_index != operation.command_index
        {
            self.facts.failures.insert("operation-command-index-drift");
        }
        if chain.application_state_ref != operation.application_state_ref {
            self.facts.failures.insert("application-state-drift");
        }
        Ok(())
    }

    fn continuity(&mut self, observation: &'a super::super::wire::BoundObservation) -> Result<(), super::ImportIssue> {
        let envelope = &observation.envelope;
        let apply = &observation.apply;
        let chain = &apply.chain;
        if let Some(prior) = self.previous.insert(&apply.participant_ref, apply) {
            if envelope.source_sequence <= prior.chain.event_sequence {
                return Err(super::ImportIssue::Observation);
            }
            if envelope.source_sequence != prior.chain.event_sequence + 1 {
                self.facts.is_incomplete = true;
            }
            if chain.command_index > prior.chain.command_index.saturating_add(1) {
                self.facts.is_incomplete = true;
            }
            if chain.command_index <= prior.chain.command_index {
                self.facts.failures.insert("committed-index-rollback-or-duplicate");
            }
            if chain.command_index == prior.chain.command_index + 1 && chain.prior_digest != prior.chain.digest {
                self.facts.failures.insert("discontinuous-chain");
            }
            if apply.lifecycle_generation < prior.lifecycle_generation {
                return Err(super::ImportIssue::Observation);
            }
        } else if chain.command_index != 1 || envelope.source_sequence != 1 {
            self.facts.is_incomplete = true;
        }
        if envelope.dropped != 0 {
            self.facts.is_incomplete = true;
        }
        Ok(())
    }

    fn complete(&mut self, proposals: &[chaoscontrol_smr::ProposalAttempt], observations: &[super::super::wire::BoundObservation]) -> Result<(), super::ImportIssue> {
        let p = &self.expected.protocol;
        let operation_count = u64::try_from(self.expected.operations.len()).map_err(|_| super::ImportIssue::Bounds)?;
        for participant in &p.participants {
            match self.previous.get(participant.participant_ref.as_str()) {
                Some(last) => {
                    self.facts.committed = self.facts.committed.min(last.chain.command_index);
                    if last.chain.command_index < operation_count {
                        self.facts.is_incomplete = true;
                    }
                }
                None => {
                    self.facts.is_incomplete = true;
                    self.facts.committed = 0;
                }
            }
        }
        for proposal in proposals {
            if proposal.outcome == chaoscontrol_smr::ProposalOutcome::DefinitelyRejected
                && observations.iter().any(|o| o.apply.operation_ref == proposal.operation_id)
            {
                self.facts.failures.insert("definitely-rejected-operation-applied");
            }
        }
        self.facts.operation_ids = std::mem::take(&mut self.operation_indices).into_values().collect();
        Ok(())
    }
}
