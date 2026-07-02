
struct RuntimePredicateReceiptInput {
    predicate: &'static str,
    input_value: IoValue,
    decision: PredicateDecision,
    state_refs: Vec<String>,
    checks: Vec<String>,
    diagnostics: Vec<String>,
}

fn build_runtime_predicate_receipt(input: RuntimePredicateReceiptInput) -> Result<RuntimePredicateReceipt> {
    let input_ref = crate::preserves_rail::canonical_hash(&input.input_value)?;
    let value = crate::preserves_rail::record("runtime-predicate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA),
        crate::preserves_rail::string(input.predicate),
        crate::preserves_rail::string(PREDICATE_ENGINE),
        crate::preserves_rail::record("input-ref", vec![crate::preserves_rail::string(&input_ref)]),
        crate::preserves_rail::string(input.decision.as_str()),
        crate::preserves_rail::sequence(input.state_refs.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::sequence(input.checks.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::sequence(input.diagnostics.iter().map(crate::preserves_rail::string).collect()),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(RuntimePredicateReceipt {
        receipt_ref,
        predicate: input.predicate.to_string(),
        input_ref,
        decision: input.decision,
        state_refs: input.state_refs,
        checks: input.checks,
        diagnostics: input.diagnostics,
        value,
    })
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn action_summary_value(action: &TurnAction) -> IoValue {
    match action {
        TurnAction::Send(message) => crate::preserves_rail::record("turn-action-send-v1", vec![message.to_value()]),
        TurnAction::Observe(observer) => {
            crate::preserves_rail::record("turn-action-observe-v1", vec![observer.to_value()])
        }
        TurnAction::Assert(assertion) => {
            crate::preserves_rail::record("turn-action-assert-v1", vec![assertion.to_value()])
        }
        TurnAction::Retract(assertion) => {
            crate::preserves_rail::record("turn-action-retract-v1", vec![assertion.to_value()])
        }
    }
}

fn apply_actions_to_snapshot(before: &RuntimeSnapshot, turn: &PendingTurn) -> RuntimeSnapshot {
    let mut after = before.clone();
    for action in &turn.actions {
        match action {
            TurnAction::Send(message) => {
                after.messages.insert(message.clone());
            }
            TurnAction::Observe(observer) => {
                after.observers.insert(observer.clone());
            }
            TurnAction::Assert(assertion) => {
                after.assertions.insert(assertion.clone());
            }
            TurnAction::Retract(assertion) => {
                after.assertions.remove(assertion);
            }
        }
    }
    after
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/predicates/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/predicates/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/predicates/parts/mod/tests/m000/p002/body.rs"));
}
