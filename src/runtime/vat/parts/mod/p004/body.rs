
fn effects(run_input: &RunInput, input_ref: &str) -> Result<Effects> {
    let effect_request = record("vat-replay-effect-request-v1", vec![
        string("clock"),
        string("logical-time"),
        record("input-ref", vec![string(input_ref)]),
        record("profile", vec![string("replay")]),
    ]);
    let effect_request_ref = canonical_hash(&effect_request)?;
    let effect_response = record("vat-replay-effect-response-v1", vec![
        string(run_input.effect_response),
        record("request-ref", vec![string(&effect_request_ref)]),
        record("source", vec![string("recorded-effect-log")]),
    ]);
    let effect_response_ref = canonical_hash(&effect_response)?;
    let random_request = record("vat-replay-effect-request-v1", vec![
        string("random"),
        string(run_input.random_sequence),
        record("input-ref", vec![string(input_ref)]),
        record("profile", vec![string("replay")]),
    ]);
    let random_request_ref = canonical_hash(&random_request)?;
    let random_response = record("vat-replay-effect-response-v1", vec![
        string(run_input.random_response),
        record("request-ref", vec![string(&random_request_ref)]),
        record("source", vec![string("seeded-prng")]),
    ]);
    let random_response_ref = canonical_hash(&random_response)?;
    Ok(Effects {
        effect_request_ref,
        effect_response_ref,
        random_request_ref,
        random_response_ref,
    })
}

fn tail(run_input: &RunInput, objects: &Objects, inputs: &Inputs, effects: &Effects) -> Result<Tail> {
    let policy_decision = record("vat-replay-policy-decision-v1", vec![
        string(run_input.policy_decision),
        record("input-ref", vec![string(&inputs.input_ref)]),
        record("effect-response-ref", vec![string(&effects.effect_response_ref)]),
        record("random-response-ref", vec![string(&effects.random_response_ref)]),
    ]);
    let policy_decision_ref = canonical_hash(&policy_decision)?;
    let final_state = record("vat-replay-final-state-v1", vec![
        record("initial-state-ref", vec![string(&inputs.initial_state_ref)]),
        record("input-ref", vec![string(&inputs.input_ref)]),
        record("effect-response-ref", vec![string(&effects.effect_response_ref)]),
        record("random-response-ref", vec![string(&effects.random_response_ref)]),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
        record("state-marker", vec![string(run_input.state_marker)]),
        sequence([objects.root_ref.clone(), objects.helper_ref.clone()].iter().map(string).collect()),
    ]);
    let final_state_hash = canonical_hash(&final_state)?;
    let trace = record("vat-replay-turn-trace-v1", vec![
        string("turn:replay:0001"),
        record("scheduler-key", vec![string("logical:0:priority:0:queue:0:vat:fixture:local")]),
        record("input-ref", vec![string(&inputs.input_ref)]),
        record("effect-request-ref", vec![string(&effects.effect_request_ref)]),
        record("effect-response-ref", vec![string(&effects.effect_response_ref)]),
        record("random-request-ref", vec![string(&effects.random_request_ref)]),
        record("random-response-ref", vec![string(&effects.random_response_ref)]),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
        record("after-state-ref", vec![string(&final_state_hash)]),
    ]);
    let trace_ref = canonical_hash(&trace)?;
    Ok(Tail {
        policy_decision_ref,
        final_state_hash,
        trace_ref,
    })
}

fn run_value(run_input: &RunInput, inputs: &Inputs, effects: &Effects, tail: &Tail) -> IoValue {
    let Effects {
        effect_request_ref,
        effect_response_ref,
        random_request_ref,
        random_response_ref,
    } = effects;
    let Tail {
        policy_decision_ref,
        final_state_hash,
        trace_ref,
    } = tail;
    record("vat-deterministic-replay-run-v1", vec![
        string(RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA),
        record("profile", vec![string("replay")]),
        record("seed", vec![string(run_input.seed)]),
        record("initial-state-ref", vec![string(&inputs.initial_state_ref)]),
        record("input-ref", vec![string(&inputs.input_ref)]),
        record("effect-request-ref", vec![string(effect_request_ref)]),
        record("effect-response-ref", vec![string(effect_response_ref)]),
        record("random-request-ref", vec![string(random_request_ref)]),
        record("random-response-ref", vec![string(random_response_ref)]),
        record("policy-decision-ref", vec![string(policy_decision_ref)]),
        record("trace-ref", vec![string(trace_ref)]),
        record("final-state-ref", vec![string(final_state_hash)]),
        record("external-effects", vec![string("denied")]),
    ])
}

fn vat_replay_run(run_input: RunInput) -> Result<VatReplayRun> {
    let objects = objects()?;
    let inputs = inputs(&run_input, &objects)?;
    let effects = effects(&run_input, &inputs.input_ref)?;
    let tail = tail(&run_input, &objects, &inputs, &effects)?;
    let value = run_value(&run_input, &inputs, &effects, &tail);
    let run_ref = canonical_hash(&value)?;
    let Effects {
        effect_request_ref,
        effect_response_ref,
        random_request_ref,
        random_response_ref,
    } = effects;
    let Tail {
        policy_decision_ref,
        final_state_hash,
        trace_ref,
    } = tail;
    Ok(VatReplayRun {
        value,
        run_ref,
        trace_ref,
        effect_request_ref,
        effect_response_ref,
        random_request_ref,
        random_response_ref,
        policy_decision_ref,
        final_state_hash,
    })
}

fn vat_replay_divergence(expected: &VatReplayRun, actual: &VatReplayRun) -> VatReplayDivergenceKind {
    if expected.run_ref == actual.run_ref {
        return VatReplayDivergenceKind::None;
    }
    if expected.effect_request_ref != actual.effect_request_ref {
        return VatReplayDivergenceKind::Input;
    }
    if expected.effect_response_ref != actual.effect_response_ref {
        return VatReplayDivergenceKind::EffectResponse;
    }
    if expected.random_request_ref != actual.random_request_ref {
        return VatReplayDivergenceKind::EffectRequest;
    }
    if expected.random_response_ref != actual.random_response_ref {
        return VatReplayDivergenceKind::EffectResponse;
    }
    if expected.policy_decision_ref != actual.policy_decision_ref {
        return VatReplayDivergenceKind::PolicyDecision;
    }
    VatReplayDivergenceKind::StateHash
}

fn vat_replay_receipt_value(expected: &VatReplayRun, actual: &VatReplayRun) -> Result<IoValue> {
    let divergence = vat_replay_divergence(expected, actual);
    let decision = if divergence == VatReplayDivergenceKind::None {
        "pass"
    } else {
        "deny"
    };
    let diagnostics = vat_replay_diagnostics(divergence);
    Ok(record("vat-replay-receipt-v1", vec![
        string(RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA),
        string(decision),
        record("profile", vec![string("replay")]),
        record("expected-run-ref", vec![string(&expected.run_ref)]),
        record("actual-run-ref", vec![string(&actual.run_ref)]),
        record("divergence", vec![string(divergence.as_str())]),
        record("expected-trace-ref", vec![string(&expected.trace_ref)]),
        record("actual-trace-ref", vec![string(&actual.trace_ref)]),
        record("expected-random-request-ref", vec![string(&expected.random_request_ref)]),
        record("actual-random-request-ref", vec![string(&actual.random_request_ref)]),
        record("expected-random-response-ref", vec![string(&expected.random_response_ref)]),
        record("actual-random-response-ref", vec![string(&actual.random_response_ref)]),
        record("expected-policy-decision-ref", vec![string(&expected.policy_decision_ref)]),
        record("actual-policy-decision-ref", vec![string(&actual.policy_decision_ref)]),
        record("expected-final-state-ref", vec![string(&expected.final_state_hash)]),
        record("actual-final-state-ref", vec![string(&actual.final_state_hash)]),
        sequence(diagnostics.iter().map(string).collect()),
    ]))
}

fn vat_replay_diagnostics(divergence: VatReplayDivergenceKind) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    diagnostics.push("replay-profile-denies-real-external-effects".to_string());
    diagnostics.push("logical-clock-response-stable".to_string());
    diagnostics.push("seeded-random-response-stable".to_string());
    match divergence {
        VatReplayDivergenceKind::None => diagnostics.push("deterministic-replay-identical-trace-and-state".to_string()),
        VatReplayDivergenceKind::Input => diagnostics.push("first-divergence-input".to_string()),
        VatReplayDivergenceKind::EffectRequest => diagnostics.push("first-divergence-effect-request".to_string()),
        VatReplayDivergenceKind::EffectResponse => diagnostics.push("first-divergence-effect-response".to_string()),
        VatReplayDivergenceKind::PolicyDecision => diagnostics.push("first-divergence-policy-decision".to_string()),
        VatReplayDivergenceKind::StateHash => diagnostics.push("first-divergence-state-hash".to_string()),
    }
    diagnostics
}

fn authority_edge_value(from_ref: &str, to_ref: &str, edge_kind: &str) -> IoValue {
    record("authority-edge-v1", vec![string(from_ref), string(to_ref), string(edge_kind)])
}

fn authority_descriptor_ref(authority_kind: crate::runtime::RuntimeObjectAuthorityKind) -> Result<String> {
    canonical_hash(&record("vat-authority-descriptor-v1", vec![string(authority_kind.as_str())]))
}

fn versioned_object_value(object_id: &'static str, schema_version: &'static str, state: &'static str) -> IoValue {
    record("vat-versioned-object-v1", vec![string(object_id), string(schema_version), string(state)])
}

fn vat_upgrade_recipe_value(
    source_schema: &'static str,
    target_schema: &'static str,
    transformer: &'static str,
    evidence_ref: &str,
) -> Result<IoValue> {
    Ok(record("vat-object-upgrade-recipe-v1", vec![
        string(RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA),
        string(source_schema),
        string(target_schema),
        string(transformer),
        record("evidence-ref", vec![string(evidence_ref)]),
    ]))
}

fn vat_restore_receipt_value(input: VatRestoreReceiptInput<'_>) -> IoValue {
    record("vat-restore-receipt-v1", vec![
        string(RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA),
        string(input.decision),
        record("snapshot-ref", vec![string(input.snapshot_ref)]),
        optional_ref_value("recipe-ref", input.recipe_ref),
        optional_ref_value("restored-object-ref", input.restored_object_ref),
        sequence(input.diagnostics.iter().map(string).collect()),
    ])
}

fn optional_ref_value(label: &'static str, value: Option<&str>) -> IoValue {
    match value {
        Some(reference) => record(label, vec![string(reference)]),
        None => record(label, Vec::new()),
    }
}

fn restore_diagnostics(receipts: &[IoValue]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let receipt_ref = canonical_hash(receipt)?;
        diagnostics.push(format!("restore-receipt:{receipt_ref}"));
    }
    Ok(diagnostics)
}

fn debug_diagnostics(receipts: &[IoValue]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(receipts.len() + 1);
    for receipt in receipts {
        let receipt_ref = canonical_hash(receipt)?;
        diagnostics.push(format!("debug-receipt:{receipt_ref}"));
    }
    diagnostics.push("evidence-only-debugging-surface".to_string());
    Ok(diagnostics)
}

fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
    refs.sort();
    refs.dedup();
    refs
}

fn fixture_diagnostics(receipts: &[crate::runtime::RuntimePredicateReceipt]) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny) {
        diagnostics.push("expected-denials-present".to_string());
    }
    if receipts.iter().all(|receipt| receipt.decision == crate::runtime::PredicateDecision::Pass) {
        diagnostics.push("missing-negative-coverage".to_string());
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/vat/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/vat/parts/mod/tests/m000/p001/body.rs"));
}
