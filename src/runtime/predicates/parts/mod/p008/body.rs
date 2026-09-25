
pub fn evaluate_promise_state_transition(
    before: &RuntimePromiseState,
    after: &RuntimePromiseState,
) -> Result<PromiseStateResult> {
    let mut diagnostics = validate_promise_shape(before, "before");
    diagnostics.extend(validate_promise_shape(after, "after"));
    if before.promise_id != after.promise_id {
        diagnostics.push("promise-id-mismatch".to_string());
    }
    if before.status.is_terminal() && before != after {
        diagnostics.push("terminal-promise-state-changed".to_string());
    }
    if before.status == RuntimePromiseStatus::Pending
        && after.status == RuntimePromiseStatus::Pending
        && before != after
    {
        diagnostics.push("pending-promise-mutated-without-resolution".to_string());
    }
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let before_ref = before.promise_ref()?;
    let after_ref = after.promise_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-promise-state-input-v1", vec![
        crate::preserves_rail::record("before-ref", vec![crate::preserves_rail::string(&before_ref)]),
        crate::preserves_rail::record("after-ref", vec![crate::preserves_rail::string(&after_ref)]),
        before.to_value(),
        after.to_value(),
    ]);
    let checks = vec![
        "bounded-promise-state-machine".to_string(),
        "terminal-state-immutability".to_string(),
        "resolved-value-ref-canonical".to_string(),
        "causal-failure-refs-canonical".to_string(),
        "cancel-timeout-reason-required".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: PROMISE_STATE_PREDICATE,
        input_value,
        decision,
        state_refs: vec![before_ref, after_ref],
        checks,
        diagnostics,
    })?;

    Ok(PromiseStateResult { is_allowed, receipt })
}
