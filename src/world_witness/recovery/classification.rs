pub(super) struct Facts {
    pub binding: super::binding::Transaction,
    pub current: Option<molten_core::world_head::WorldHeadState>,
    pub retained: molten_core::world_witness::RetainedWitnessState,
    pub currentness: molten_core::world_witness::CurrentnessInput,
    pub receipt: Option<crate::world_witness::records::Receipt>,
}

pub(super) fn classify(
    request: super::Request<'_>,
    facts: Facts,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<molten_core::world_witness::ReconciliationDecision> {
    let Facts {
        binding,
        current,
        retained,
        mut currentness,
        receipt: historical,
    } = facts;
    let admitted = match molten_core::world_witness::validate_currentness(&currentness) {
        Ok(admitted) => admitted,
        Err(issue) => return Ok(molten_core::world_witness::ReconciliationDecision::Denied(issue)),
    };
    let proof = admitted
        .highest
        .as_ref()
        .ok_or_else(|| crate::world_witness::store::failure("recovery requires independent proof"))?;
    let receipt = super::binding::identity(&crate::preserves_rail::content_ref_from_bytes(
        &crate::world_witness::records::canonical_highest(proof)?,
    ))?;
    let publication_observation = publication(&request, &binding, current.as_ref());
    if current.as_ref() == Some(&request.stage.staged.transition.after)
        && !matches!(&retained, molten_core::world_witness::RetainedWitnessState::Present(highest)
            if highest.subject == proof.subject
                && highest.checkpoint.provider_id == proof.checkpoint.provider_id
                && highest.checkpoint.revision <= proof.checkpoint.revision
                && (highest.checkpoint.revision != proof.checkpoint.revision
                    || highest.checkpoint.checkpoint_id == proof.checkpoint.checkpoint_id)
                && highest.quorum.is_subset(&proof.quorum))
    {
        return Ok(molten_core::world_witness::ReconciliationDecision::Denied(
            molten_core::world_witness::WitnessIssue::MissingHighestState,
        ));
    }
    let authority_observation = observed_authority(&request, current.as_ref(), historical.as_ref(), authority)?;
    if current.as_ref() == Some(&request.stage.staged.transition.after)
        && matches!(&currentness.observation, Some(molten_core::world_witness::WitnessObservation::Available(observation))
            if observation.current_head.phase == molten_core::world_witness::CurrentHeadPhase::Reserved)
    {
        return Ok(molten_core::world_witness::ReconciliationDecision::Retryable);
    }
    // Completed operations use actual historical receipt roles, not a new
    // grant checked against an invented predecessor state.
    currentness.local_generation = request.stage.staged.transition.before.as_ref().map_or(0, |head| head.generation);
    Ok(molten_core::world_witness::reconcile(&molten_core::world_witness::ReconciliationInput {
        finalization: molten_core::world_witness::FinalizationInput {
            staged: request.stage.staged.clone(),
            durable_stage: Some(request.stage.staged.clone()),
            current_head: current,
            authority: authority_observation,
            currentness,
        },
        publication_intent: binding.publication,
        publication_observation,
        reservation: binding.reservation,
        operation: binding.operation,
        terminal: Some(transactional_reconciliation_core::TerminalObservation::new(
            binding.reservation.plan_identity(),
            binding.reservation.operation_identity(),
            binding.reservation.attempt_identity(),
            transactional_reconciliation_core::EffectTerminal::Succeeded(receipt),
        )),
    }))
}
fn publication(
    request: &super::Request<'_>,
    binding: &super::binding::Transaction,
    current: Option<&molten_core::world_head::WorldHeadState>,
) -> transactional_reconciliation_core::PublicationObservation {
    let transition = &request.stage.staged.transition;
    if current == Some(&transition.after) {
        transactional_reconciliation_core::PublicationObservation::Applied {
            predecessor: binding.publication.expected_revision(),
            successor: binding.publication.successor_revision(),
            successor_content_identity: binding.publication.successor_content_identity(),
        }
    } else if current == transition.before.as_ref() {
        transactional_reconciliation_core::PublicationObservation::NotApplied {
            current_revision: binding.publication.expected_revision(),
        }
    } else {
        transactional_reconciliation_core::PublicationObservation::NotApplied {
            current_revision: transactional_reconciliation_core::Revision::observed(
                current.map_or(0, |head| head.generation),
            ),
        }
    }
}

fn observed_authority(
    request: &super::Request<'_>,
    current: Option<&molten_core::world_head::WorldHeadState>,
    historical: Option<&crate::world_witness::records::Receipt>,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<molten_core::world_head::WorldHeadAuthorityObservation> {
    if current == Some(&request.stage.staged.transition.after) {
        let receipt = historical
            .ok_or_else(|| crate::world_witness::store::failure("completed operation lacks durable receipt"))?;
        crate::world_witness::records::receipt::validate(receipt)?;
        if receipt.operation_id != request.stage.operation_ref
            || receipt.claim_ref != request.claim.claim_ref
            || receipt.authentication_ref.as_str() != request.stage.authentication_ref
            || receipt.authority_ref.as_str() != request.stage.authority_ref
            || receipt.local_persistence != crate::world_witness::records::PersistenceDecision::Committed
        {
            return Err(crate::world_witness::store::failure(
                "historical completion receipt substitutes operation or decision roles",
            ));
        }
        return Ok(molten_core::world_head::WorldHeadAuthorityObservation {
            authority_ref: receipt.authority_ref.clone(),
            policy_ref: request.stage.staged.transition.after.policy_ref.clone(),
            admitted: receipt.branch_authorization == crate::world_witness::records::RoleDecision::Admitted,
            observed_generation: request.stage.staged.transition.before.as_ref().map_or(0, |head| head.generation),
        });
    }
    let fresh = authority.recheck(request.claim, current)?;
    if !fresh.authentication_passed {
        return Err(crate::world_witness::store::failure("recovery claim authentication denied"));
    }
    Ok(fresh.authority)
}
