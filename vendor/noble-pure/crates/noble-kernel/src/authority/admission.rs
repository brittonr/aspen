impl super::Authority {
    /// A raw/serialized guest claim without the retained opaque host value is
    /// rejected (`witness == None`). Failed preflight preserves that value.
    /// The trusted shell supplies fresh independent facts at its current local
    /// checkpoint and performs no protected action until this returns Committed.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; admission preflights retained witness, current facts, quota and capacity before the single owned-state commit; all denial/preflight paths return the original opaque obligation."
    )]
    pub fn admit(
        &mut self,
        witness: Option<super::Witness>,
        request: &super::AdmissionRequest,
        facts: &super::CurrentFacts,
    ) -> super::Admission {
        if !self.request_available() {
            let rejection = self.preflight(
                request.plan.clone(),
                super::PreflightFailure::RequestCapacity,
                super::ReceiptScope::AdmissionBoundary,
            );
            return super::Admission::Preflight { witness, rejection };
        }
        self.counters.admission_requests += 1;
        self.effects.push(request.plan.description.operation.effect);
        let available = match self.validate_entry(witness.as_ref(), request, facts) {
            Ok(available) => available,
            Err(denial) => {
                let rejection = self.deny(
                    request.plan.clone(),
                    denial,
                    super::ReceiptScope::AdmissionBoundary,
                );
                return super::Admission::Denied { witness, rejection };
            }
        };
        if self.attempts.len() >= self.profile.limits.attempts {
            let rejection = self.preflight(
                request.plan.clone(),
                super::PreflightFailure::AttemptCapacity,
                super::ReceiptScope::AdmissionBoundary,
            );
            return super::Admission::Preflight { witness, rejection };
        }
        let admitted = match self.counters.attempts_admitted.checked_add(1) {
            Some(admitted) => admitted,
            None => {
                let rejection = self.preflight(
                    request.plan.clone(),
                    super::PreflightFailure::AttemptCapacity,
                    super::ReceiptScope::AdmissionBoundary,
                );
                return super::Admission::Preflight { witness, rejection };
            }
        };
        // Preflight proved table indices, quota arithmetic and reserved capacity.
        // No shell callback can interleave consumption and attempt registration.
        match self.commit(request, available, admitted) {
            Ok(execution) => super::Admission::Committed(execution),
            Err(denial) => {
                let rejection = self.deny(
                    request.plan.clone(),
                    denial,
                    super::ReceiptScope::AdmissionBoundary,
                );
                super::Admission::Denied { witness, rejection }
            }
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; binding an opaque witness uses derived claim and bounded Plan equality plus runtime retained Vec lookup; those operations are non-const."
    )]
    fn validate_entry(
        &self,
        witness: Option<&super::Witness>,
        request: &super::AdmissionRequest,
        facts: &super::CurrentFacts,
    ) -> Result<u64, super::Denial> {
        let witness = match witness {
            Some(witness) => witness,
            None => return Err(super::Denial::UntrustedWitnessClaim),
        };
        let record = attempt!(self.witness_record(request.claim));
        if witness.claim != request.claim {
            return Err(super::Denial::UntrustedWitnessClaim);
        }
        match record.state {
            super::WitnessState::Live => (),
            super::WitnessState::Consumed => return Err(super::Denial::ConsumedWitness),
            super::WitnessState::Retired => return Err(super::Denial::RetiredWitness),
        }
        if record.plan != request.plan {
            return Err(super::Denial::ChangedPlan);
        }
        match self.validate_facts(&request.plan, facts) {
            Ok(()) => (),
            Err(denial) => return Err(denial),
        }
        match facts.quota_remaining {
            Some(available) => Ok(available),
            None => Err(super::Denial::MissingFacts),
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; commit reserves quota, consumes one retained witness and publishes one attempt as a serialized owned-state transition; cloning and pushing bounded Plan/Vec data are non-const."
    )]
    fn commit(
        &mut self,
        request: &super::AdmissionRequest,
        available: u64,
        admitted: u32,
    ) -> Result<super::Execution, super::Denial> {
        let key = super::AttemptKey {
            owner: self.profile.owner,
            slot: self.attempts.len(),
            generation: u64::from(admitted),
        };
        let record = super::AttemptRecord {
            key,
            plan: request.plan.clone(),
            started: false,
            admitted: self.profile.checkpoint,
            unknown: None,
            terminal: None,
        };
        match self.charge_quota(&request.plan, available) {
            Ok(()) => (),
            Err(denial) => return Err(denial),
        };
        self.witnesses[request.claim.slot].state = super::WitnessState::Consumed;
        self.attempts.push(record);
        self.counters.witness_consumptions = admitted;
        self.counters.attempts_admitted = admitted;
        Ok(super::Execution { attempt: key })
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; the admitted quota reservation updates only the exclusively owned checkpoint ledger after checked addition; runtime Vec access and insertion are non-const."
    )]
    fn charge_quota(&mut self, plan: &super::Plan, available: u64) -> Result<(), super::Denial> {
        let units = plan.description.constraints.units;
        match self.quota_position(plan) {
            Some(index) => {
                let spent = match self.quotas[index].spent.checked_add(units) {
                    Some(spent) => spent,
                    None => return Err(super::Denial::QuotaExhausted),
                };
                self.quotas[index].spent = spent;
            }
            None => {
                self.quotas.push(super::QuotaRecord {
                    quota: plan.description.constraints.quota,
                    checkpoint: self.profile.checkpoint,
                    available,
                    spent: units,
                });
            }
        }
        Ok(())
    }

    pub(super) fn witness_record(
        &self,
        claim: super::WitnessClaim,
    ) -> Result<&super::WitnessRecord, super::Denial> {
        if claim.owner != self.profile.owner {
            return Err(super::Denial::WrongOwnerContext);
        }
        let record = match self.witnesses.get(claim.slot) {
            Some(record) => record,
            None => return Err(super::Denial::UnknownWitness),
        };
        if claim.generation != record.claim.generation {
            return Err(super::Denial::StaleGeneration);
        }
        if claim.kind != record.claim.kind {
            return Err(super::Denial::WrongResourceKind);
        }
        if claim.rights != record.claim.rights {
            return Err(super::Denial::MissingRights);
        }
        if claim.rights & super::EXECUTE_RIGHT == 0 {
            return Err(super::Denial::MissingRights);
        }
        if claim.scope != record.claim.scope {
            return Err(super::Denial::WrongScope);
        }
        Ok(record)
    }

    /// This is the last local step before shell work. It consumes the execution
    /// permit even on failure; cancellation never restores witness availability.
    /// The shell must perform at most one action with the returned Started value.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; the one-shot execution boundary records the started attempt and counter before returning the consumed permit; only the exclusively owned authority state changes."
    )]
    pub fn start_execution(
        &mut self,
        execution: super::Execution,
    ) -> Result<super::Started, super::BoundaryError> {
        let record = attempt!(self.attempt_record(execution.attempt));
        if record.started {
            return Err(super::BoundaryError::AlreadyStarted);
        }
        if self.invocation != super::InvocationOutcome::Pending {
            return Err(super::BoundaryError::InvocationFinished);
        }
        let started = super::Started {
            attempt: record.key,
            plan: record.plan.clone(),
        };
        self.attempts[execution.attempt.slot].started = true;
        self.counters.protected_operations += 1;
        Ok(started)
    }

    pub(super) fn attempt_record(
        &self,
        key: super::AttemptKey,
    ) -> Result<&super::AttemptRecord, super::BoundaryError> {
        if key.owner != self.profile.owner {
            return Err(super::BoundaryError::WrongContext);
        }
        let record = match self.attempts.get(key.slot) {
            Some(record) => record,
            None => return Err(super::BoundaryError::UnknownAttempt),
        };
        if key.generation != record.key.generation {
            return Err(super::BoundaryError::StaleGeneration);
        }
        Ok(record)
    }

    pub fn counters(&self) -> super::Counters {
        self.counters
    }

    /// Each bounded protected-entry request retains its exact requested effect,
    /// including denial/preflight. Authorization-boundary calls are counted
    /// separately; their WIT request effects belong to the component adapter.
    pub fn requested_effects(&self) -> &[crate::types::EffId] {
        &self.effects
    }

    pub fn invocation_outcome(&self) -> super::InvocationOutcome {
        self.invocation
    }

    pub fn attempt_snapshot(
        &self,
        key: super::AttemptKey,
    ) -> Result<super::AttemptSnapshot, super::BoundaryError> {
        let record = attempt!(self.attempt_record(key));
        let outcome = match &record.terminal {
            Some(observation) => Some(observation.outcome),
            None => record
                .unknown
                .as_ref()
                .map(|observation| observation.outcome),
        };
        Ok(super::AttemptSnapshot {
            key: record.key,
            effect: record.plan.description.operation.effect,
            started: record.started,
            outcome,
        })
    }
}
