impl super::Authority {
    /// Only an applicable approved observation supports an operation claim.
    /// Plans, witnesses, attempts, unknowns and failures cannot imply success.
    pub fn receipt(
        &self,
        attempt: super::AttemptKey,
        claim: super::ReceiptClaim,
        observation: Option<&super::Observation>,
    ) -> Result<super::Receipt, super::BoundaryError> {
        let approved = attempt!(self.applicable(attempt, claim, observation));
        let (scope, source, checkpoint) = match claim {
            super::ReceiptClaim::InvocationSuccess => match &self.delivered {
                Some(delivery) => (
                    super::ReceiptScope::InvocationDelivery,
                    self.profile.invocation_source,
                    delivery.checkpoint,
                ),
                None => return Err(super::BoundaryError::InapplicableReceipt),
            },
            super::ReceiptClaim::Unknown
            | super::ReceiptClaim::OperationFailure
            | super::ReceiptClaim::OperationSuccess => (
                super::ReceiptScope::OperationBoundary,
                approved.source,
                approved.checkpoint,
            ),
            super::ReceiptClaim::Denial | super::ReceiptClaim::PreflightFailure => {
                return Err(super::BoundaryError::InapplicableReceipt)
            }
        };
        Ok(super::Receipt {
            description: super::ReceiptDescription {
                plan: approved.plan.clone(),
                boundary_owner: self.profile.owner,
                attempt: Some(attempt),
                source,
                checkpoint,
                claim,
                scope,
                invocation: self.invocation,
                denial: None,
                preflight: None,
            },
        })
    }

    pub(super) fn applicable(
        &self,
        attempt: super::AttemptKey,
        claim: super::ReceiptClaim,
        observation: Option<&super::Observation>,
    ) -> Result<&super::ObservationDescription, super::BoundaryError> {
        let observation = match observation {
            Some(observation) => observation,
            None => return Err(super::BoundaryError::MissingObservation),
        };
        if observation.attempt != attempt {
            return Err(super::BoundaryError::InapplicableObservation);
        }
        let approved = attempt!(self.approved_observation(observation));
        match (claim, approved.outcome) {
            (super::ReceiptClaim::Unknown, super::ObservedOutcome::Unknown)
            | (super::ReceiptClaim::OperationFailure, super::ObservedOutcome::OperationFailure)
            | (super::ReceiptClaim::OperationSuccess, super::ObservedOutcome::OperationSuccess) => {
                Ok(approved)
            }
            (
                super::ReceiptClaim::InvocationSuccess,
                super::ObservedOutcome::Unknown
                | super::ObservedOutcome::OperationFailure
                | super::ObservedOutcome::OperationSuccess,
            ) => match &self.delivered {
                Some(delivery)
                    if self.invocation == super::InvocationOutcome::Succeeded
                        && delivery.attempt == attempt =>
                {
                    Ok(approved)
                }
                Some(_) | None => Err(super::BoundaryError::InapplicableReceipt),
            },
            (
                super::ReceiptClaim::Denial | super::ReceiptClaim::PreflightFailure,
                super::ObservedOutcome::Unknown
                | super::ObservedOutcome::OperationFailure
                | super::ObservedOutcome::OperationSuccess,
            )
            | (
                super::ReceiptClaim::Unknown,
                super::ObservedOutcome::OperationFailure | super::ObservedOutcome::OperationSuccess,
            )
            | (
                super::ReceiptClaim::OperationFailure,
                super::ObservedOutcome::Unknown | super::ObservedOutcome::OperationSuccess,
            )
            | (
                super::ReceiptClaim::OperationSuccess,
                super::ObservedOutcome::Unknown | super::ObservedOutcome::OperationFailure,
            ) => Err(super::BoundaryError::InapplicableObservation),
        }
    }

    /// Import establishes no trust. This explicit admission requires an already
    /// approved local observation and exact provenance/context/claim equality.
    /// Integrity of the imported artifact alone never satisfies this contract.
    pub fn admit_receipt(
        &self,
        imported: &super::UntrustedReceipt,
        observation: &super::Observation,
    ) -> Result<super::Receipt, super::BoundaryError> {
        let attempt = match imported.description.attempt {
            Some(attempt) => attempt,
            None => return Err(super::BoundaryError::InapplicableReceipt),
        };
        let receipt =
            attempt!(self.receipt(attempt, imported.description.claim, Some(observation)));
        if receipt.description != imported.description {
            return Err(super::BoundaryError::InapplicableReceipt);
        }
        Ok(receipt)
    }

    pub(super) fn deny(
        &self,
        plan: super::Plan,
        denial: super::Denial,
        scope: super::ReceiptScope,
    ) -> super::Rejection {
        super::Rejection {
            receipt: super::Receipt {
                description: super::ReceiptDescription {
                    plan,
                    boundary_owner: self.profile.owner,
                    attempt: None,
                    source: self.profile.authority_source,
                    checkpoint: self.profile.checkpoint,
                    claim: super::ReceiptClaim::Denial,
                    scope,
                    invocation: self.invocation,
                    denial: Some(denial),
                    preflight: None,
                },
            },
        }
    }

    pub(super) fn preflight(
        &self,
        plan: super::Plan,
        failure: super::PreflightFailure,
        scope: super::ReceiptScope,
    ) -> super::Rejection {
        super::Rejection {
            receipt: super::Receipt {
                description: super::ReceiptDescription {
                    plan,
                    boundary_owner: self.profile.owner,
                    attempt: None,
                    source: self.profile.authority_source,
                    checkpoint: self.profile.checkpoint,
                    claim: super::ReceiptClaim::PreflightFailure,
                    scope,
                    invocation: self.invocation,
                    denial: None,
                    preflight: Some(failure),
                },
            },
        }
    }
}
