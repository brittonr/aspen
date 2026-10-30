pub(crate) struct Prepared<'a> {
    pub stage: &'a super::durable::Stage,
    pub claim: &'a crate::world_head::CanonicalWorldHeadClaim,
    pub currentness: &'a molten_core::world_witness::CurrentnessInput,
    pub reservation: &'a crate::world_witness::provider::Reservation,
}

pub(crate) fn finalize<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    store: &mut crate::world_witness::store::Database<'_>,
    provider: &mut crate::world_witness::provider::Index<T, P>,
    authority: &mut impl crate::world_witness::FreshAdmission,
    input: Prepared<'_>,
) -> crate::error::Result<crate::world_witness::Outcome> {
    let mut recheck = Recheck {
        provider,
        authority,
        input: Prepared { ..input },
    };
    let outcome = store.finalize(
        crate::world_witness::store::FinalizeInput {
            stage: input.stage,
            claim: input.claim,
        },
        &mut recheck,
    )?;
    match outcome {
        crate::world_head::WorldHeadMutationOutcome::Stale => {
            if provider.orphan(input.reservation).is_err() {
                return Ok(super::shell::uncertain(input.stage, crate::world_witness::WitnessUncertainty::Orphan));
            }
            Ok(crate::world_witness::Outcome::Superseded)
        }
        crate::world_head::WorldHeadMutationOutcome::Uncertain => {
            Ok(super::shell::uncertain(input.stage, crate::world_witness::WitnessUncertainty::LocalCommit))
        }
        crate::world_head::WorldHeadMutationOutcome::Applied
        | crate::world_head::WorldHeadMutationOutcome::AlreadyApplied => {
            complete(store, provider, input.reservation, input.currentness, input.stage)
        }
    }
}

fn observe<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    provider: &mut crate::world_witness::provider::Index<T, P>,
    reservation: &crate::world_witness::provider::Reservation,
    template: &molten_core::world_witness::CurrentnessInput,
    stage: &super::durable::Stage,
) -> crate::error::Result<molten_core::world_witness::CurrentnessInput> {
    let observation = provider.observe_reserved(reservation).map_err(crate::world_witness::store::failure)?;
    normalized(provider, reservation, template, stage, &observation)
}

fn normalized<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    provider: &mut crate::world_witness::provider::Index<T, P>,
    reservation: &crate::world_witness::provider::Reservation,
    template: &molten_core::world_witness::CurrentnessInput,
    stage: &super::durable::Stage,
    observation: &crate::world_witness::provider::Observation,
) -> crate::error::Result<molten_core::world_witness::CurrentnessInput> {
    let mut currentness = template.clone();
    currentness.operation_id = stage.operation_ref.clone();
    currentness.subject = molten_core::world_witness::subject(&stage.staged);
    currentness.local_generation = stage.staged.transition.before.as_ref().map_or(0, |state| state.generation);
    currentness.now = provider.observed_at()?;
    currentness.observation = Some(
        provider
            .normalize(observation, crate::world_witness::provider::normalize::Input {
                contract: &template.policy.provider,
                subject: &currentness.subject,
                retained: &template.retained,
                observed_at: currentness.now,
                operation_id: &stage.operation_ref,
                reservation,
            })
            .map_err(crate::world_witness::store::failure)?,
    );
    Ok(currentness)
}

struct Recheck<'a, T, P, A> {
    provider: &'a mut crate::world_witness::provider::Index<T, P>,
    authority: &'a mut A,
    input: Prepared<'a>,
}

impl<T, P, A> crate::world_witness::WitnessRecheck for Recheck<'_, T, P, A>
where
    T: crate::world_witness::provider::Transport,
    P: crate::world_witness::provider::Ports,
    A: crate::world_witness::FreshAdmission,
{
    fn recheck(
        &mut self,
        claim: &crate::world_head::CanonicalWorldHeadClaim,
        observed: Option<&molten_core::world_head::WorldHeadState>,
        retained: molten_core::world_witness::RetainedWitnessState,
    ) -> crate::error::Result<(crate::world_head::WorldHeadFreshAdmission, molten_core::world_witness::CurrentnessInput)>
    {
        let mut template = self.input.currentness.clone();
        template.retained = retained;
        let currentness = observe(self.provider, self.input.reservation, &template, self.input.stage)?;
        if self.authority.witness_policy_ref()? != super::durable::policy_identity(&self.input.stage.witness_policy)? {
            return Err(crate::world_witness::store::failure("installed witness policy changed before final CAS"));
        }
        let fresh = self.authority.recheck(claim, observed)?;
        Ok((fresh, currentness))
    }
}

pub(crate) fn complete<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    store: &mut crate::world_witness::store::Database<'_>,
    provider: &mut crate::world_witness::provider::Index<T, P>,
    reservation: &crate::world_witness::provider::Reservation,
    template: &molten_core::world_witness::CurrentnessInput,
    stage: &super::durable::Stage,
) -> crate::error::Result<crate::world_witness::Outcome> {
    let observation = match provider.commit(reservation) {
        Ok(observation) => observation,
        Err(_) => return Ok(super::shell::uncertain(stage, crate::world_witness::WitnessUncertainty::ExternalCommit)),
    };
    let mut original = template.clone();
    original.retained = stage.staged.retained.clone();
    original.policy = stage.witness_policy.clone();
    let currentness = normalized(provider, reservation, &original, stage, &observation)?;
    let admitted = molten_core::world_witness::validate_committed_currentness(&currentness)
        .map_err(|issue| crate::world_witness::store::failure(format!("committed currentness denied: {issue:?}")))?;
    let receipt = stage.receipt(admitted.highest, crate::world_witness::records::RoleDecision::Admitted)?;
    if store.complete(stage, &receipt).is_err() {
        return Ok(super::shell::uncertain(stage, crate::world_witness::WitnessUncertainty::LocalCommit));
    }
    Ok(crate::world_witness::Outcome::Committed {
        receipt_ref: crate::world_witness::records::receipt::identity(&receipt)?,
    })
}
