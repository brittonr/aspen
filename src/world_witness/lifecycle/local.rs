/// The local profile fences stale generations only while local state is intact.
pub fn advance_generation(
    store: &mut crate::world_witness::store::Database<'_>,
    mut input: crate::world_witness::Transition<'_>,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<crate::world_witness::Outcome> {
    if input.currentness.selected_profile != molten_core::world_witness::AssuranceProfile::LocalGenerationV1 {
        return Err(crate::world_witness::store::failure("local publication requires local-generation-v1"));
    }
    input.currentness.retained = store.retained_local(&input.plan.after.branch_id)?;
    let staged = super::durable::Stage::prepare(&input)?;
    input.currentness.operation_id = staged.operation_ref.clone();
    input.currentness.subject = molten_core::world_witness::subject(&staged.staged);
    if authority.witness_policy_ref()? != super::durable::policy_identity(&staged.witness_policy)? {
        return Err(crate::world_witness::store::failure("installed policy does not admit local assurance"));
    }
    let fresh = authority.recheck(input.claim, input.plan.before.as_ref())?;
    molten_core::world_witness::plan_stage(&molten_core::world_witness::StageInput {
        staged: staged.staged.clone(),
        authenticated: fresh.authentication_passed,
        authority: fresh.authority,
    })
    .map_err(|issue| crate::world_witness::store::failure(format!("local stage denied: {issue:?}")))?;
    molten_core::world_witness::validate_currentness(&input.currentness)
        .map_err(|issue| crate::world_witness::store::failure(format!("local currentness denied: {issue:?}")))?;
    match store.stage(&staged)? {
        crate::world_head::WorldHeadMutationOutcome::Stale => return Ok(crate::world_witness::Outcome::Superseded),
        crate::world_head::WorldHeadMutationOutcome::Uncertain => {
            return Ok(super::shell::uncertain(&staged, crate::world_witness::WitnessUncertainty::Staging));
        }
        crate::world_head::WorldHeadMutationOutcome::Applied
        | crate::world_head::WorldHeadMutationOutcome::AlreadyApplied => {}
    }
    publish(store, &staged, &input, authority)
}

fn publish(
    store: &mut crate::world_witness::store::Database<'_>,
    staged: &super::durable::Stage,
    input: &crate::world_witness::Transition<'_>,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<crate::world_witness::Outcome> {
    let receipt = staged.receipt(None, crate::world_witness::records::RoleDecision::Admitted)?;
    let reference = crate::world_witness::records::receipt::identity(&receipt)?;
    let mut recheck = Recheck {
        authority,
        template: &input.currentness,
    };
    let outcome = store.finalize(
        crate::world_witness::store::FinalizeInput {
            stage: staged,
            claim: input.claim,
        },
        &mut recheck,
    )?;
    Ok(match outcome {
        crate::world_head::WorldHeadMutationOutcome::Applied => {
            crate::world_witness::Outcome::Committed { receipt_ref: reference }
        }
        crate::world_head::WorldHeadMutationOutcome::AlreadyApplied => crate::world_witness::Outcome::AlreadyComplete,
        crate::world_head::WorldHeadMutationOutcome::Stale => crate::world_witness::Outcome::Superseded,
        crate::world_head::WorldHeadMutationOutcome::Uncertain => {
            super::shell::uncertain(staged, crate::world_witness::WitnessUncertainty::LocalCommit)
        }
    })
}

struct Recheck<'a, A> {
    authority: &'a mut A,
    template: &'a molten_core::world_witness::CurrentnessInput,
}

impl<A: crate::world_witness::FreshAdmission> crate::world_witness::WitnessRecheck for Recheck<'_, A> {
    fn recheck(
        &mut self,
        claim: &crate::world_head::CanonicalWorldHeadClaim,
        observed: Option<&molten_core::world_head::WorldHeadState>,
        retained: molten_core::world_witness::RetainedWitnessState,
    ) -> crate::error::Result<(crate::world_head::WorldHeadFreshAdmission, molten_core::world_witness::CurrentnessInput)>
    {
        if self.authority.witness_policy_ref()? != super::durable::policy_identity(&self.template.policy)? {
            return Err(crate::world_witness::store::failure("installed witness policy changed before local CAS"));
        }
        let fresh = self.authority.recheck(claim, observed)?;
        let mut currentness = self.template.clone();
        currentness.retained = retained;
        currentness.local_generation = observed.map_or(0, |head| head.generation);
        Ok((fresh, currentness))
    }
}
