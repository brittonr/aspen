/// Inputs of the admission recheck that the store runs inside its mutation boundary.
struct FreshAdmissionInput<'a> {
    plan: &'a WorldHeadTransitionPlan,
    plan_request: &'a WorldHeadPlanRequest,
    canonical: &'a CanonicalWorldHeadClaim,
    authentication_policy: &'a AuthenticationPolicy,
    signatures: &'a [WorldHeadSignatureCarrier],
}

fn fresh_admission<A: WorldHeadAuthorityPort>(
    input: &FreshAdmissionInput<'_>,
    authority: &mut A,
    observed: Option<&molten_core::world_head::WorldHeadState>,
) -> std::result::Result<WorldHeadFreshAdmission, WorldHeadPortError> {
    if observed != input.plan.before.as_ref() {
        return Err(WorldHeadPortError::new("fresh-head-mismatch", "head changed inside mutation boundary"));
    }
    let fresh_authentication = evaluate_world_head_authentication(
        input.canonical,
        input.authentication_policy,
        input.signatures,
        &mut *authority,
    )
    .map_err(|error| WorldHeadPortError::new("fresh-authentication", error.to_string()))?;
    // Re-run pure admission over the fresh signer observations so a signer role, authority,
    // currentness, or threshold withdrawn since planning denies inside the mutation boundary.
    // Branch authority and its generation are rechecked by the store from `fresh_authority`.
    // r[impl molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let fresh_request = WorldHeadPlanRequest {
        current: observed.cloned(),
        authentication: fresh_authentication.observation,
        ..input.plan_request.clone()
    };
    let is_fresh_authentication_admitted = matches!(
        plan_world_head_transition(&fresh_request),
        WorldHeadDecision::Admitted(fresh_plan) if fresh_plan == *input.plan
    );
    let fresh_authority = authority.observe_authority(
        &input.plan.after.branch_id,
        &input.plan.after.policy_ref,
        input.plan.before.as_ref().map_or(0, |state| state.generation),
    )?;
    Ok(WorldHeadFreshAdmission {
        authentication_passed: is_fresh_authentication_admitted,
        authority: fresh_authority,
    })
}

/// Maps the observed CAS outcome to the execution status and issues its receipt records.
fn mutation_status(mutation: WorldHeadMutationOutcome) -> (WorldHeadExecutionStatus, Vec<WorldHeadIssue>) {
    match mutation {
        WorldHeadMutationOutcome::Applied => (WorldHeadExecutionStatus::Applied, Vec::new()),
        WorldHeadMutationOutcome::AlreadyApplied => (WorldHeadExecutionStatus::AlreadyApplied, Vec::new()),
        WorldHeadMutationOutcome::Stale => (WorldHeadExecutionStatus::Stale, vec![WorldHeadIssue::StaleExpectedHead]),
        WorldHeadMutationOutcome::Uncertain => (WorldHeadExecutionStatus::Uncertain, Vec::new()),
    }
}

struct OutcomeReceiptContext<'a> {
    plan: &'a WorldHeadTransitionPlan,
    claim: &'a CanonicalWorldHeadClaim,
    authentication: &'a WorldHeadAuthenticationResult,
    authority_ref: &'a str,
}

impl OutcomeReceiptContext<'_> {
    fn receipt(
        &self,
        status: WorldHeadExecutionStatus,
        issues: &[WorldHeadIssue],
    ) -> Result<CanonicalWorldHeadTransitionReceipt> {
        transition_receipt(TransitionReceiptInput {
            decision: status.as_str(),
            plan: Some(self.plan),
            claim: self.claim,
            authentication: self.authentication,
            authority_ref: self.authority_ref,
            issues,
        })
    }
}

fn denied_execution(
    decision: WorldHeadDecision,
    canonical: &CanonicalWorldHeadClaim,
    authentication: &WorldHeadAuthenticationResult,
    authority_ref: &str,
) -> Result<WorldHeadExecutionResult> {
    let issues = decision_issues(decision);
    let receipt = transition_receipt(TransitionReceiptInput {
        decision: DECISION_DENIED,
        plan: None,
        claim: canonical,
        authentication,
        authority_ref,
        issues: &issues,
    })?;
    Ok(WorldHeadExecutionResult {
        status: WorldHeadExecutionStatus::Denied,
        plan: None,
        issues,
        receipt,
    })
}

pub fn record_world_head_conflict<S: WorldHeadConflictPort>(
    store: &mut S,
    plans: &[WorldHeadTransitionPlan],
    maximum: u32,
) -> Result<Option<(WorldHeadConflictSet, CanonicalWorldHeadConflict)>> {
    let conflict = classify_world_head_conflict(plans, maximum)
        .map_err(|issues| Failure::invalid_harness(format!("world-head conflict denied: {issues:?}")))?;
    let Some(conflict) = conflict else {
        return Ok(None);
    };
    let canonical = canonical_world_head_conflict(&conflict)?;
    store.record_conflict(&conflict, &canonical).map_err(port_error)?;
    Ok(Some((conflict, canonical)))
}
