pub mod binding;
mod classification;
pub(super) mod resume;

pub struct Request<'a> {
    pub stage: &'a crate::world_witness::lifecycle::durable::Stage,
    pub claim: &'a crate::world_head::CanonicalWorldHeadClaim,
    pub template: &'a molten_core::world_witness::CurrentnessInput,
}

/// Observe both stores; returned Retryable/Finalize is a plan, never an automatic redispatch.
// r[impl molten.world_witness.reconciliation]
pub fn recover<T: super::provider::Transport, P: super::provider::Ports>(
    store: &mut super::store::Database<'_>,
    provider: &mut super::provider::Index<T, P>,
    request: Request<'_>,
    authority: &mut impl super::FreshAdmission,
) -> crate::error::Result<molten_core::world_witness::ReconciliationDecision> {
    let binding = binding::prepare(request.stage)?;
    if authority.witness_policy_ref()?
        != crate::world_witness::lifecycle::durable::policy_identity(&request.stage.witness_policy)?
    {
        return Ok(molten_core::world_witness::ReconciliationDecision::Denied(
            molten_core::world_witness::WitnessIssue::InvalidStage,
        ));
    }
    if store.reconciliation(&request.stage.operation_ref)?.as_deref() != Some(binding::canonical(&binding)?.as_slice())
        || store.stage_bytes(&request.stage.operation_ref)?.as_deref() != Some(request.stage.bytes.as_slice())
        || request.claim.claim_ref != request.stage.staged.transition.claim_ref
        || request.claim.bytes != request.stage.claim_bytes
        || request.template.policy != request.stage.witness_policy
    {
        return Ok(molten_core::world_witness::ReconciliationDecision::ManualReview);
    }
    let current = store.current(&request.stage.staged.transition.after.branch_id)?;
    let retained = store.retained(&request.stage.staged.transition.after.branch_id)?;
    if matches!(retained, molten_core::world_witness::RetainedWitnessState::Missing) {
        return Ok(molten_core::world_witness::ReconciliationDecision::Denied(
            molten_core::world_witness::WitnessIssue::MissingHighestState,
        ));
    }
    if current.as_ref() == request.stage.staged.transition.before.as_ref() && retained != request.stage.staged.retained
    {
        return Ok(molten_core::world_witness::ReconciliationDecision::Denied(
            molten_core::world_witness::WitnessIssue::FailedConsistency,
        ));
    }
    let reservation = match reservation(store, provider, &request) {
        Ok(Some(reservation)) => reservation,
        Ok(None) if current.as_ref() == request.stage.staged.transition.before.as_ref() => {
            return Ok(molten_core::world_witness::ReconciliationDecision::Retryable);
        }
        Ok(None) => return Ok(molten_core::world_witness::ReconciliationDecision::ManualReview),
        Err(_) => return Ok(molten_core::world_witness::ReconciliationDecision::ManualReview),
    };
    let observation = match provider.query(reservation.revision) {
        Ok(observation) => observation,
        Err(_) => return Ok(molten_core::world_witness::ReconciliationDecision::ManualReview),
    };
    let currentness = currentness(provider, &request, &reservation, &observation)?;
    let receipt = store.operation_receipt(&request.stage.operation_ref)?;
    classification::classify(
        request,
        classification::Facts {
            binding,
            current,
            retained,
            currentness,
            receipt,
        },
        authority,
    )
}

pub(crate) fn reservation<T: super::provider::Transport, P: super::provider::Ports>(
    store: &mut super::store::Database<'_>,
    provider: &mut super::provider::Index<T, P>,
    request: &Request<'_>,
) -> crate::error::Result<Option<super::provider::Reservation>> {
    if let Some(bytes) = store.reservation(&request.stage.operation_ref)? {
        return serde_json::from_slice(&bytes).map(Some).map_err(super::store::failure);
    }
    let candidate = super::lifecycle::reserve::candidate(request.stage)?;
    let minimum = match &request.stage.staged.retained {
        molten_core::world_witness::RetainedWitnessState::Present(highest) => highest.checkpoint.revision,
        _ => 0,
    };
    let recovered = provider.recover_reservation(&candidate, minimum).map_err(super::store::failure)?;
    if let Some(reservation) = &recovered {
        store.save_reservation(
            &request.stage.operation_ref,
            &serde_json::to_vec(reservation).map_err(super::store::failure)?,
        )?;
    }
    Ok(recovered)
}

pub(crate) fn currentness<T: super::provider::Transport, P: super::provider::Ports>(
    provider: &mut super::provider::Index<T, P>,
    request: &Request<'_>,
    reservation: &super::provider::Reservation,
    observation: &super::provider::Observation,
) -> crate::error::Result<molten_core::world_witness::CurrentnessInput> {
    let mut input = request.template.clone();
    input.now = provider.observed_at()?;
    input.operation_id = request.stage.operation_ref.clone();
    input.subject = molten_core::world_witness::subject(&request.stage.staged);
    input.retained = request.stage.staged.retained.clone();
    input.observation = Some(
        provider
            .normalize(observation, super::provider::normalize::Input {
                contract: &input.policy.provider,
                subject: &input.subject,
                retained: &input.retained,
                observed_at: input.now,
                operation_id: &input.operation_id,
                reservation,
            })
            .map_err(super::store::failure)?,
    );
    Ok(input)
}
