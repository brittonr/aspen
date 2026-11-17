impl super::Authority {
    /// TRUST PREMISE: the shell authenticated the configured observation source,
    /// associated the external result with this exact attempt and complete plan,
    /// and supplies its current checkpoint. Deserializing bytes, a signer label,
    /// or an imported trust flag is not enough to call this constructor.
    /// Each attempt reserves one unknown and one terminal observation slot.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; authenticated observations are validated before filling a reserved per-attempt unknown or terminal slot; mutation is confined to exclusively owned records and counters."
    )]
    pub fn observe_from_trusted_host(
        &mut self,
        description: super::ObservationDescription,
    ) -> Result<super::Observation, super::BoundaryError> {
        match self.validate_observation(&description) {
            Ok(()) => (),
            Err(error) => return Err(error),
        }
        let record = &mut self.attempts[description.attempt.slot];
        let existing = match description.outcome {
            super::ObservedOutcome::Unknown => {
                if record.terminal.is_some() {
                    return Err(super::BoundaryError::ConflictingObservation);
                }
                &mut record.unknown
            }
            super::ObservedOutcome::OperationFailure | super::ObservedOutcome::OperationSuccess => {
                &mut record.terminal
            }
        };
        let observation = super::Observation {
            attempt: description.attempt,
            outcome: description.outcome,
        };
        match existing {
            Some(previous) if *previous == description => Ok(observation),
            Some(_) => Err(super::BoundaryError::ConflictingObservation),
            None => {
                *existing = Some(description);
                self.counters.approved_observations += 1;
                Ok(observation)
            }
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; provenance validation uses runtime retained Vec access and derived equality of the complete bounded Plan and checkpoint; those checks are non-const."
    )]
    fn validate_observation(
        &self,
        description: &super::ObservationDescription,
    ) -> Result<(), super::BoundaryError> {
        if description.source != self.profile.observation_source {
            return Err(super::BoundaryError::WrongSource);
        }
        let record = attempt!(self.attempt_record(description.attempt));
        if !record.started {
            return Err(super::BoundaryError::NotStarted);
        }
        if description.plan != record.plan {
            return Err(super::BoundaryError::ChangedPlan);
        }
        if description.checkpoint != self.profile.checkpoint {
            return Err(super::BoundaryError::StaleObservation);
        }
        if description.checkpoint.sequence < record.admitted.sequence {
            return Err(super::BoundaryError::StaleObservation);
        }
        if description.checkpoint.now < record.admitted.now {
            return Err(super::BoundaryError::StaleObservation);
        }
        Ok(())
    }

    pub(super) fn approved_observation(
        &self,
        observation: &super::Observation,
    ) -> Result<&super::ObservationDescription, super::BoundaryError> {
        let record = attempt!(self.attempt_record(observation.attempt));
        let retained = match observation.outcome {
            super::ObservedOutcome::Unknown => &record.unknown,
            super::ObservedOutcome::OperationFailure | super::ObservedOutcome::OperationSuccess => {
                &record.terminal
            }
        };
        match retained {
            Some(retained) if retained.outcome == observation.outcome => Ok(retained),
            Some(_) => Err(super::BoundaryError::InapplicableObservation),
            None => Err(super::BoundaryError::MissingObservation),
        }
    }

    /// Cancellation revokes invocation availability, not an external operation.
    /// Late approved observations remain admissible but can never deliver a
    /// successful invocation result. Resource retirement is the adapter's
    /// separate obligation; this API returns no owner or resource.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; cancellation is the explicit irreversible invocation transition and retires only this owned table's live witnesses; outstanding external observations remain separate."
    )]
    pub fn cancel_invocation(&mut self) -> Result<super::InvocationOutcome, super::BoundaryError> {
        match self.invocation {
            super::InvocationOutcome::Pending => {
                self.invocation = super::InvocationOutcome::Cancelled;
                self.retire_live_witnesses();
                Ok(self.invocation)
            }
            super::InvocationOutcome::Cancelled => Ok(self.invocation),
            super::InvocationOutcome::Succeeded | super::InvocationOutcome::Failed => {
                Err(super::BoundaryError::InvocationFinished)
            }
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; failure is the explicit irreversible invocation transition and retires only live witnesses in the exclusively owned table."
    )]
    pub fn fail_invocation(&mut self) -> Result<super::InvocationOutcome, super::BoundaryError> {
        match self.invocation {
            super::InvocationOutcome::Pending => {
                self.invocation = super::InvocationOutcome::Failed;
                self.retire_live_witnesses();
                Ok(self.invocation)
            }
            super::InvocationOutcome::Failed => Ok(self.invocation),
            super::InvocationOutcome::Succeeded | super::InvocationOutcome::Cancelled => {
                Err(super::BoundaryError::InvocationFinished)
            }
        }
    }

    /// The trusted shell commits successful delivery of this invocation's
    /// result, which can describe operation success, failure, or unknown.
    /// External success alone does not deliver a result. Before this local
    /// commit the shell accounts for all result/input resources; this records
    /// the invocation boundary's observation, not external operation success.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; successful delivery records the local invocation boundary before returning its receipt and retires owned live witnesses; this never changes the approved external operation outcome."
    )]
    pub fn deliver_success(
        &mut self,
        attempt: super::AttemptKey,
        observation: &super::Observation,
    ) -> Result<super::Receipt, super::BoundaryError> {
        let approved = attempt!(self.approved_observation(observation));
        if approved.attempt != attempt {
            return Err(super::BoundaryError::InapplicableObservation);
        }
        if self.invocation != super::InvocationOutcome::Pending {
            return Err(super::BoundaryError::InvocationFinished);
        }
        self.invocation = super::InvocationOutcome::Succeeded;
        self.delivered = Some(super::DeliveryRecord {
            attempt,
            checkpoint: self.profile.checkpoint,
        });
        self.counters.successful_deliveries += 1;
        self.retire_live_witnesses();
        self.receipt(
            attempt,
            super::ReceiptClaim::InvocationSuccess,
            Some(observation),
        )
    }

    /// Explicit host retirement, not guest discard or authority restoration.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; retirement validates the retained identity and live state before irrevocably retiring the exclusively owned witness record; it does not restore authority."
    )]
    pub fn retire_witness(&mut self, witness: super::Witness) -> Result<(), super::Denial> {
        match self.witness_record(witness.claim) {
            Ok(record) => match record.state {
                super::WitnessState::Live => (),
                super::WitnessState::Consumed => return Err(super::Denial::ConsumedWitness),
                super::WitnessState::Retired => return Err(super::Denial::RetiredWitness),
            },
            Err(denial) => return Err(denial),
        }
        self.witnesses[witness.claim.slot].state = super::WitnessState::Retired;
        Ok(())
    }

    pub fn witness_snapshot(
        &self,
        claim: super::WitnessClaim,
    ) -> Result<super::WitnessSnapshot, super::Denial> {
        match self.witness_record(claim) {
            Ok(record) => Ok(super::WitnessSnapshot {
                claim: record.claim,
                state: record.state,
            }),
            Err(denial) => Err(denial),
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; this length-bounded owned-state transition retires live retained witnesses after invocation termination and leaves consumed or already retired records unchanged."
    )]
    fn retire_live_witnesses(&mut self) {
        let mut index = 0;
        while index < self.witnesses.len() {
            match self.witnesses[index].state {
                super::WitnessState::Live => {
                    self.witnesses[index].state = super::WitnessState::Retired
                }
                super::WitnessState::Consumed | super::WitnessState::Retired => (),
            }
            index += 1;
        }
    }
}
