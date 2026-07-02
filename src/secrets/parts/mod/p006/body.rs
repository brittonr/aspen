
fn redaction_transform_checks(decision: &str, is_gate_preserving: bool) -> [(&'static str, &'static str); 4] {
    if decision == "pass" && is_gate_preserving {
        [
            ("source-ref-bound", "pass"),
            ("output-ref-bound", "pass"),
            ("marker-ref-bound", "pass"),
            ("semantic-evidence-preserved", "pass"),
        ]
    } else {
        [
            ("source-ref-bound", "pass"),
            ("output-ref-bound", "pass"),
            ("marker-ref-bound", "pass"),
            ("diagnostic-only", "pass"),
        ]
    }
}

fn commitment_replay_checks(decision: &str, is_plaintext_required: bool) -> [(&'static str, &'static str); 3] {
    if decision == "pass" {
        [
            ("commitment-match", "pass"),
            ("plaintext-not-required", "pass"),
            ("replay-without-plaintext", "pass"),
        ]
    } else if is_plaintext_required {
        [
            ("commitment-comparison", "pass"),
            ("plaintext-required", "pass"),
            ("diagnostic-only", "pass"),
        ]
    } else {
        [
            ("commitment-mismatch", "pass"),
            ("fail-closed", "pass"),
            ("audit-receipt", "pass"),
        ]
    }
}

fn secret_cleanup_checks(decision: &str) -> [(&'static str, &'static str); 4] {
    if decision == "pass" {
        [
            ("revocation-bound", "pass"),
            ("tombstone-bound", "pass"),
            ("retention-gc-bound", "pass"),
            ("idempotent-cleanup", "pass"),
        ]
    } else {
        [
            ("cleanup-denied", "pass"),
            ("no-plaintext-default", "pass"),
            ("audit-receipt", "pass"),
            ("retention-preserved", "pass"),
        ]
    }
}

fn first_redaction_reason(value: &IoValue) -> Result<String> {
    let text = to_text(value)?;
    if text.contains("<credential") {
        Ok("credential".to_string())
    } else if text.contains("<private") {
        Ok("private".to_string())
    } else if text.contains("<encrypted-ref") {
        Ok("encrypted-ref".to_string())
    } else {
        Ok("secret".to_string())
    }
}

fn redaction_seed_ref(source_ref: &str, profile_ref: &str, policy_refs: &[String]) -> Result<String> {
    canonical_hash(&record("redaction-receipt-seed-v1", vec![
        record("source", vec![string(source_ref)]),
        record("profile", vec![string(profile_ref)]),
        record("policy", vec![refs_sequence(policy_refs)]),
    ]))
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn diagnostics_value(diagnostics: &[String]) -> IoValue {
    record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())])
}

fn parse_diagnostics(value: &Value<IoValue>) -> Result<Vec<String>> {
    record_strings(value, "diagnostics", "diagnostics")
}

fn record_decision(value: &Value<IoValue>) -> Result<String> {
    let decision = record_string(value, "decision", "decision")?;
    if decision == "pass" || decision == "deny" {
        Ok(decision)
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported secrets decision {decision}")))
    }
}

fn record_string(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<String> {
    let value = record_string(value, record_name, label)?;
    validate_ref(&value, label)?;
    Ok(value)
}

fn record_optional_ref(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    parse_optional_ref(&record[0], label)
}

fn record_bool(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    record[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_strings(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    let values = required_sequence(&record[0], label)?;
    ensure_count_at_most(values.len(), MAX_SECRET_REFS, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn record_refs(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<Vec<String>> {
    let refs = record_strings(value, record_name, label)?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn parse_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        let item = required_string(&some[0], label)?;
        validate_ref(&item, label)?;
        return Ok(Some(item));
    }
    let item = required_string(value, label)?;
    validate_ref(&item, label)?;
    Ok(Some(item))
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {label} schema {actual}; expected {expected}")))
    }
}

fn require_checks(value: &Value<IoValue>, expected: &[&str]) -> Result<()> {
    let value = value_to_iovalue(value);
    let check_record = simple_record(&value, "checks", 1)?;
    let values = required_sequence(&check_record[0], "checks")?;
    ensure_count_at_most(values.len(), MAX_SECRET_REFS, "checks")?;
    let mut seen = BtreeSet::new();
    for value in values.iter() {
        let item = value_to_iovalue(value);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("unsupported check status {status}")));
        }
        seen.insert(name);
    }
    for expected in expected {
        if !seen.contains(*expected) {
            return Err(MoltenError::invalid_harness(format!("missing secrets check {expected}")));
        }
    }
    Ok(())
}

fn simple_record<'a>(value: &'a IoValue, label: &str, arity: usize) -> Result<Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn required_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, label: &str) -> Result<Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))
}

fn validate_classification(value: &str) -> Result<()> {
    match value {
        "secret" | "credential" | "private" | "policy" | "encrypted-ref" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported confidential classification {value}"))),
    }
}

fn validate_redaction_reason(value: &str) -> Result<()> {
    match value {
        "secret" | "credential" | "private" | "policy" | "encrypted-ref" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported redaction reason {value}"))),
    }
}

fn validate_purpose(value: &str) -> Result<()> {
    match value {
        "debug" | "replay" | "export" | "adapter-use" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported secret purpose {value}"))),
    }
}

fn validate_allowed_uses(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_SECRET_USES, "secret allowed uses")?;
    for value in values {
        validate_purpose(value)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value: &str, label: &str) -> Result<()> {
    validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} must be a canonical content ref: {error}")))
}

fn validate_optional_ref(value: Option<&str>, label: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, label)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_SECRET_REFS, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_diagnostics(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_SECRET_DIAGNOSTICS, label)
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    }
}

fn fixture_ref(label: &str) -> String {
    content_ref_from_bytes(label.as_bytes())
}

trait PushLimited<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}
