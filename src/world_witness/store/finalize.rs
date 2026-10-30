impl super::Database<'_> {
    pub fn finalize(
        &mut self,
        input: super::FinalizeInput<'_>,
        admission: &mut impl super::super::WitnessRecheck,
    ) -> crate::error::Result<crate::world_head::WorldHeadMutationOutcome> {
        input.stage.validate()?;
        if input.claim.claim_ref != input.stage.staged.transition.claim_ref
            || input.claim.bytes != input.stage.claim_bytes
        {
            return Err(super::failure("finalization substitutes the staged claim"));
        }
        let write = self.heads.database().begin_write().map_err(super::failure)?;
        let branch = input.stage.staged.transition.after.branch_id.as_str();
        let observed = super::read::head(&write, branch)?;
        if observed.as_ref() == Some(&input.stage.staged.transition.after) {
            return repeated(&write, &input);
        }
        if observed != input.stage.staged.transition.before {
            return Ok(crate::world_head::WorldHeadMutationOutcome::Stale);
        }
        let plan = admit(&write, &input, admission, observed)?;
        persist(&write, &input, plan)?;
        Ok(match write.commit() {
            Ok(()) => crate::world_head::WorldHeadMutationOutcome::Applied,
            Err(_) => crate::world_head::WorldHeadMutationOutcome::Uncertain,
        })
    }
}

fn repeated(
    write: &redb::WriteTransaction,
    input: &super::FinalizeInput<'_>,
) -> crate::error::Result<crate::world_head::WorldHeadMutationOutcome> {
    let receipts = write.open_table(super::WORLD_WITNESS_RECEIPTS).map_err(super::failure)?;
    let Some(bytes) =
        redb::ReadableTable::get(&receipts, input.stage.operation_ref.as_str()).map_err(super::failure)?
    else {
        return Ok(crate::world_head::WorldHeadMutationOutcome::Stale);
    };
    let receipt = super::super::records::receipt::parse(bytes.value())?;
    let transition = &input.stage.staged.transition;
    if receipt.operation_id != input.stage.operation_ref
        || receipt.claim_ref != input.claim.claim_ref
        || receipt.profile != input.stage.staged.selected_profile
        || receipt.authentication_ref.as_str() != input.stage.authentication_ref
        || receipt.authority_ref.as_str() != input.stage.authority_ref
        || receipt.local_persistence != super::super::records::PersistenceDecision::Committed
        || receipt.highest.as_ref().is_some_and(|highest| {
            highest.subject.branch_id != transition.after.branch_id
                || highest.subject.generation != transition.after.generation
        })
    {
        return Ok(crate::world_head::WorldHeadMutationOutcome::Stale);
    }
    let receipt_ref = crate::preserves_rail::content_ref_from_bytes(bytes.value());
    let transitions = write.open_table(super::TRANSITIONS).map_err(super::failure)?;
    Ok(
        if redb::ReadableTable::get(&transitions, receipt_ref.as_str())
            .map_err(super::failure)?
            .as_ref()
            .map(|stored| stored.value())
            == Some(bytes.value())
        {
            crate::world_head::WorldHeadMutationOutcome::AlreadyApplied
        } else {
            crate::world_head::WorldHeadMutationOutcome::Stale
        },
    )
}

fn admit(
    write: &redb::WriteTransaction,
    input: &super::FinalizeInput<'_>,
    admission: &mut impl super::super::WitnessRecheck,
    observed: Option<molten_core::world_head::WorldHeadState>,
) -> crate::error::Result<molten_core::world_witness::FinalizationPlan> {
    let durable = write.open_table(super::STAGES).map_err(super::failure)?;
    let exact = redb::ReadableTable::get(&durable, input.stage.operation_ref.as_str()).map_err(super::failure)?;
    if exact.as_ref().map(|value| value.value()) != Some(input.stage.bytes.as_slice()) {
        return Err(super::failure("durable staged claim changed or disappeared"));
    }
    let retained = retained(write, input, observed.is_some())?;
    let (fresh, currentness) = admission.recheck(input.claim, observed.as_ref(), retained)?;
    if !fresh.authentication_passed {
        return Err(super::failure("actual claim authentication denied"));
    }
    if fresh.authority.authority_ref.as_str() != input.stage.authority_ref {
        return Err(super::failure("branch authority decision changed since staging"));
    }
    if currentness.policy != input.stage.witness_policy
        || currentness.local_generation != observed.as_ref().map_or(0, |state| state.generation)
    {
        return Err(super::failure("witness policy or local generation changed before publication"));
    }
    molten_core::world_witness::plan_finalization(&molten_core::world_witness::FinalizationInput {
        staged: input.stage.staged.clone(),
        durable_stage: Some(input.stage.staged.clone()),
        current_head: observed,
        authority: fresh.authority,
        currentness,
    })
    .map_err(|issue| super::failure(format!("witness finalization denied: {issue:?}")))
}

fn retained(
    write: &redb::WriteTransaction,
    input: &super::FinalizeInput<'_>,
    has_head: bool,
) -> crate::error::Result<molten_core::world_witness::RetainedWitnessState> {
    let branch = input.stage.staged.transition.after.branch_id.as_str();
    let retained = super::read::retained_in_transaction(write, branch, has_head)?;
    if input.stage.staged.selected_profile == molten_core::world_witness::AssuranceProfile::LocalGenerationV1
        && matches!(retained, molten_core::world_witness::RetainedWitnessState::Missing)
    {
        let required = write.open_table(super::REQUIRED).map_err(super::failure)?;
        if redb::ReadableTable::get(&required, branch).map_err(super::failure)?.is_none() {
            return Ok(molten_core::world_witness::RetainedWitnessState::Empty);
        }
    }
    Ok(retained)
}

fn persist(
    write: &redb::WriteTransaction,
    input: &super::FinalizeInput<'_>,
    plan: molten_core::world_witness::FinalizationPlan,
) -> crate::error::Result<()> {
    let highest_bytes = plan.currentness.highest.as_ref().map(super::super::records::canonical_highest).transpose()?;
    let (_, after) = crate::world_head::canonical_world_head_state(&plan.transition.after)?;
    let role = if plan.currentness.profile == molten_core::world_witness::AssuranceProfile::IndependentWitnessV1 {
        super::super::records::RoleDecision::Reserved
    } else {
        super::super::records::RoleDecision::Admitted
    };
    let receipt = input.stage.receipt(plan.currentness.highest, role)?;
    let receipt_bytes = super::super::records::receipt::canonical(&receipt)?;
    let receipt_ref = crate::preserves_rail::content_ref_from_bytes(&receipt_bytes);
    let branch = input.stage.staged.transition.after.branch_id.as_str();
    write
        .open_table(super::HEADS)
        .map_err(super::failure)?
        .insert(branch, after.as_slice())
        .map_err(super::failure)?;
    if let Some(highest_bytes) = highest_bytes {
        write
            .open_table(super::HIGHEST)
            .map_err(super::failure)?
            .insert(branch, highest_bytes.as_slice())
            .map_err(super::failure)?;
    }
    write
        .open_table(super::TRANSITIONS)
        .map_err(super::failure)?
        .insert(receipt_ref.as_str(), receipt_bytes.as_slice())
        .map_err(super::failure)?;
    write
        .open_table(super::WORLD_WITNESS_RECEIPTS)
        .map_err(super::failure)?
        .insert(input.stage.operation_ref.as_str(), receipt_bytes.as_slice())
        .map_err(super::failure)?;
    Ok(())
}
