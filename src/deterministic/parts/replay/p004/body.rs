
fn first_divergence_value(
    kind: ReplayDivergenceKind,
    expected: &ReplayRunParts,
    actual: &ReplayRunParts,
) -> Result<IoValue> {
    let (expected_ref, actual_ref) = divergence_refs(kind, expected, actual);
    Ok(record("deterministic-first-divergence-v1", vec![
        string(DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA),
        record("kind", vec![string(kind.as_str())]),
        record("turn-id", vec![string("turn:0001")]),
        record("actor-id", vec![string("actor:helper")]),
        record("log-position", vec![string("0")]),
        record("handler-profile-ref", vec![string(DEFAULT_HANDLER_PROFILE_REF)]),
        record("expected-ref", vec![string(expected_ref)]),
        record("actual-ref", vec![string(actual_ref)]),
        sequence(vec![
            string("safe-canonical-refs-only"),
            string("redact-secret-capability-material"),
        ]),
    ]))
}

fn divergence_refs<'a>(
    kind: ReplayDivergenceKind,
    expected: &'a ReplayRunParts,
    actual: &'a ReplayRunParts,
) -> (&'a str, &'a str) {
    match kind {
        ReplayDivergenceKind::Identity => (&expected.identity_ref, &actual.identity_ref),
        ReplayDivergenceKind::Scheduler => (&expected.scheduler_ref, &actual.scheduler_ref),
        ReplayDivergenceKind::Input => (&expected.input_ref, &actual.input_ref),
        ReplayDivergenceKind::EffectRequest | ReplayDivergenceKind::LiveEffect => {
            (&expected.effect_request_ref, &actual.effect_request_ref)
        }
        ReplayDivergenceKind::EffectResponse => (&expected.effect_response_ref, &actual.effect_response_ref),
        ReplayDivergenceKind::PolicyDecision => (&expected.policy_decision_ref, &actual.policy_decision_ref),
        ReplayDivergenceKind::Action => (&expected.action_ref, &actual.action_ref),
        ReplayDivergenceKind::Receipt => (&expected.receipt_ref, &actual.receipt_ref),
        ReplayDivergenceKind::Output => (&expected.output_ref, &actual.output_ref),
        ReplayDivergenceKind::StateHash => (&expected.after_state_ref, &actual.after_state_ref),
        ReplayDivergenceKind::None => (&expected.turn_journal_ref, &actual.turn_journal_ref),
    }
}

fn verify_checks(decision: &str, divergence: ReplayDivergenceKind) -> Vec<IoValue> {
    let replay_status = if decision == "pass" { "pass" } else { "deny" };
    vec![
        record("check", vec![string("identity-bound"), string("pass")]),
        record("check", vec![string("ordered-boundary-comparison"), string(replay_status)]),
        record("check", vec![string("recorded-effects-only"), string(replay_status)]),
        record("check", vec![string("first-divergence"), string(divergence.as_str())]),
    ]
}

fn rollup_checks(decision: &str, all_inputs_readable: bool) -> Vec<IoValue> {
    vec![
        record("check", vec![string("evidence-only"), string("pass")]),
        record("check", vec![string("no-authority-grant"), string("pass")]),
        record("check", vec![string("individual-receipts-required"), string("pass")]),
        record("check", vec![
            string("all-inputs-readable"),
            string(if all_inputs_readable { "pass" } else { "fail" }),
        ]),
        record("check", vec![string("rollup-decision"), string(decision)]),
    ]
}

fn index_checks(decision: &str, all_inputs_readable: bool) -> Vec<IoValue> {
    vec![
        record("check", vec![string("evidence-only"), string("pass")]),
        record("check", vec![string("no-authority-grant"), string("pass")]),
        record("check", vec![string("rollup-and-receipt-refs-verified"), string("pass")]),
        record("check", vec![
            string("all-inputs-readable"),
            string(if all_inputs_readable { "pass" } else { "fail" }),
        ]),
        record("check", vec![string("index-decision"), string(decision)]),
    ]
}

fn refs_value(refs: &OrderedSet<String>) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn divergence_counts_value(counts: &OrderedMap<String, u64>) -> IoValue {
    sequence(
        counts
            .iter()
            .map(|(kind, count)| record("divergence-count", vec![string(kind), u64_value(*count)]))
            .collect(),
    )
}

fn require_schema_value(value: &PreservesValue<IoValue>, schema: &str, label: &str) -> Result<()> {
    let actual = required_string_value(value, label)?;
    if actual == schema {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} schema mismatch: expected {schema}, got {actual}"
        )))
    }
}

fn record_string_value(value: &PreservesValue<IoValue>, label: &'static str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string_value(&fields[0], label)
}

fn record_u64_value(value: &PreservesValue<IoValue>, label: &'static str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a u64")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("{label} out of range: {error}")))
}

fn record_ref_list_value(value: &PreservesValue<IoValue>, label: &'static str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a sequence")))?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        let reference = required_string_value(item, label)?;
        validate_content_ref(&reference)?;
        refs.push(reference);
    }
    Ok(refs)
}

fn record_divergence_counts_value(value: &PreservesValue<IoValue>) -> Result<OrderedMap<String, u64>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("divergence-counts", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <divergence-counts ...>"))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("divergence-counts must be a sequence"))?;
    if items.len() > MAX_REPLAY_INDEX_INPUTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "divergence count entries exceed {MAX_REPLAY_INDEX_INPUTS}"
        )));
    }
    let mut count_entries = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let count_fields = item
            .collect_simple_record("divergence-count", Some(2))
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <divergence-count ...>"))?;
        let kind = required_string_value(&count_fields[0], "divergence kind")?;
        let count = count_fields[1]
            .as_u64()
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("divergence count must be a u64"))?
            .map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!("divergence count out of range: {error}"))
            })?;
        count_entries.push((kind, count));
    }
    Ok(count_entries.into_iter().collect())
}

fn merge_divergence_counts(target: &mut OrderedMap<String, u64>, source: &OrderedMap<String, u64>) {
    for (kind, count) in source {
        *target.entry(kind.clone()).or_insert(0) += count;
    }
}

fn required_string_value(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a string")))
}

fn validate_replay_decision(decision: &str) -> Result<()> {
    if decision == "pass" || decision == "deny" {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "replay decision must be pass or deny, got {decision}"
        )))
    }
}

fn validate_divergence_ref(reference: &str) -> Result<()> {
    if reference == "none" {
        Ok(())
    } else {
        validate_content_ref(reference)
    }
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/deterministic/parts/replay/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/deterministic/parts/replay/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/deterministic/parts/replay/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/deterministic/parts/replay/tests/m000/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/deterministic/parts/replay/tests/m000/p004/body.rs"));
}
