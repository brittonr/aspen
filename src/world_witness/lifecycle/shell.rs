/// External reservation and commit are separate durable effects. Publication
/// never claims a cross-system atomic transaction.
// r[impl molten.world_witness.finalization]
pub fn advance<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    store: &mut crate::world_witness::store::Database<'_>,
    provider: &mut crate::world_witness::provider::Index<T, P>,
    mut input: crate::world_witness::Transition<'_>,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<crate::world_witness::Outcome> {
    if input.currentness.selected_profile != molten_core::world_witness::AssuranceProfile::IndependentWitnessV1 {
        return Err(crate::world_witness::store::failure("independent publication requires independent-witness-v1"));
    }
    input.currentness.retained = store.retained(&input.plan.after.branch_id)?;
    let staged = super::durable::Stage::prepare(&input)?;
    input.currentness.operation_id = staged.operation_ref.clone();
    let fresh = authority.recheck(input.claim, input.plan.before.as_ref())?;
    if fresh.authority.authority_ref.as_str() != staged.authority_ref
        || authority.witness_policy_ref()? != super::durable::policy_identity(&staged.witness_policy)?
    {
        return Err(crate::world_witness::store::failure(
            "installed authority or witness policy differs from staged request",
        ));
    }
    molten_core::world_witness::plan_stage(&molten_core::world_witness::StageInput {
        staged: staged.staged.clone(),
        authenticated: fresh.authentication_passed,
        authority: fresh.authority,
    })
    .map_err(|issue| crate::world_witness::store::failure(format!("witness stage denied: {issue:?}")))?;
    match store.stage(&staged)? {
        crate::world_head::WorldHeadMutationOutcome::Stale => return Ok(crate::world_witness::Outcome::Superseded),
        crate::world_head::WorldHeadMutationOutcome::Uncertain => {
            return Ok(uncertain(&staged, crate::world_witness::WitnessUncertainty::Staging));
        }
        crate::world_head::WorldHeadMutationOutcome::Applied
        | crate::world_head::WorldHeadMutationOutcome::AlreadyApplied => {}
    }
    let reservation = match super::reserve::acquire(store, provider, &staged)? {
        Some(reservation) => reservation,
        None => return Ok(uncertain(&staged, crate::world_witness::WitnessUncertainty::Reserve)),
    };
    super::effects::finalize(store, provider, authority, super::effects::Prepared {
        stage: &staged,
        claim: input.claim,
        currentness: &input.currentness,
        reservation: &reservation,
    })
}

pub(crate) fn uncertain(
    stage: &super::durable::Stage,
    phase: crate::world_witness::WitnessUncertainty,
) -> crate::world_witness::Outcome {
    crate::world_witness::Outcome::Uncertain {
        operation_ref: stage.operation_ref.clone(),
        phase,
    }
}
