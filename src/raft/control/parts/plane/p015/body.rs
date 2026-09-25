
// r[impl molten.consensus.scope]
fn manifest_checks(profile: &ConsensusAlgorithmProfileInput) -> Vec<(&'static str, &'static str)> {
    let profile_check = match profile.algorithm_profile.as_str() {
        CONSENSUS_PROFILE_RAFT | CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => "diagnostic",
        _ => "fail",
    };
    vec![
        ("explicit-consistency-group", "pass"),
        ("explicit-command-schemas", "pass"),
        ("read-index-default", "pass"),
        ("algorithm-profile-declared", "pass"),
        ("linearizable-read-supported", "pass"),
        ("placement-ref-bound", "pass"),
        ("production-profile", profile_check),
    ]
}

fn consensus_production_status(profile: &str) -> &'static str {
    match profile {
        CONSENSUS_PROFILE_RAFT => PRODUCTION_STATUS_MODEL_ONLY,
        CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => PRODUCTION_STATUS_EXPERIMENTAL,
        _ => "unsupported",
    }
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_RAFT_REFS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(actual, maximum, label)
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn session_record_value(session: &ClientSessionRecord) -> IoValue {
    record("session", vec![
        string(&session.client_session),
        u64_value(session.sequence),
        string(&session.result_command_ref),
    ])
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_ref_value(&fields[0])
}

fn record_iovalue(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn record_bool(value: &Value<IoValue>, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}
