fn commit_prepared_mutation(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    before_snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
) -> Result<CoordinationApplyResult> {
    let intent = commit_intent_for(&request, &before_snapshot, &transition)?;
    let proposal = propose_change(ChangeInput {
        runtime,
        request: &request,
        intent: &intent,
    })?;
    if proposal.decision != "pass" {
        let diagnostics = vec!["control-plane commit denied for coordination mutation".to_string()];
        return deny_result(runtime, request, before_snapshot, diagnostics, &["control-plane-commit", "fail"]);
    }
    record_success(SuccessInput {
        runtime,
        request,
        before_snapshot,
        transition,
        intent,
        proposal,
    })
}

fn deny_result(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    diagnostics: Vec<String>,
    extra_check: &[&'static str; 2],
) -> Result<CoordinationApplyResult> {
    let transition = primitive_denial_transition(runtime, &request, diagnostics)?;
    deny_transition_result(runtime, request, snapshot, transition, extra_check)
}

fn deny_transition_result(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
    extra_check: &[&'static str; 2],
) -> Result<CoordinationApplyResult> {
    finish_denial_transition(runtime, DenialTransitionInput { request, snapshot, transition, extra_check, record_operation: true })
}

struct DenialTransitionInput<'a> {
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
    extra_check: &'a [&'static str; 2],
    record_operation: bool,
}
