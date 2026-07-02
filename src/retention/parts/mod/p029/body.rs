
fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    let reference = required_string(&some[0], label)?;
    require_ref(&reference, label)?;
    Ok(Some(reference))
}

fn record_optional_u64(value: &Value<IoValue>, label: &str) -> Result<Option<u64>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional u64 for {label}")))?;
    let number = some[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))?;
    Ok(Some(number))
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let refs = record_string_sequence(value, label)?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        push_bounded(&mut values, required_string(entry, label)?, MAX_RETENTION_REFS, "retention string sequence")?;
    }
    Ok(values)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_pass_bool(value: &Value<IoValue>, label: &str) -> Result<bool> {
    match record_string(value, label)?.as_str() {
        "pass" => Ok(true),
        "deny" => Ok(false),
        other => Err(MoltenError::invalid_harness(format!("expected pass or deny for {label}, got {other}"))),
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {label} {expected}, got {actual}")))
    }
}

fn required_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RETENTION_REFS, label)?;
    for value in values {
        require_ref(value, label)?;
    }
    Ok(())
}

fn validate_diagnostics(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RETENTION_DIAGNOSTICS, label)?;
    for value in values {
        validate_name(value, label)?;
    }
    Ok(())
}

fn require_ref(value: &str, label: &str) -> Result<()> {
    validate_name(value, label)?;
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} must be a canonical content ref: {error}")))
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} cannot be empty")));
    }
    ensure_count_at_most(value.len(), MAX_RETENTION_TEXT_LEN, label)
}

fn ensure_count_at_most(count: usize, limit: usize, label: &str) -> Result<()> {
    if count > limit {
        Err(MoltenError::invalid_harness(format!("{label} exceeds limit {limit}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T, S>(values: &mut S, value: T, limit: usize, label: &str) -> Result<()>
where S: VecSink<T> {
    ensure_count_at_most(values.item_count() + 1, limit, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_bounded<T, S, I>(values: &mut S, items: I, limit: usize, label: &str) -> Result<()>
where
    S: VecSink<T>,
    I: IntoIterator<Item = T>,
{
    for item in items {
        push_bounded(values, item, limit, label)?;
    }
    Ok(())
}

fn refs_with_extra(base_refs: &[String], extra_refs: &[String], label: &str) -> Result<Vec<String>> {
    validate_refs(base_refs, label)?;
    validate_refs(extra_refs, label)?;
    let mut refs = base_refs.to_vec();
    extend_bounded(&mut refs, extra_refs.iter().cloned(), MAX_RETENTION_REFS, label)?;
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn push_named<S>(values: &mut S, name: &str, value: IoValue) -> Result<()>
where S: VecSink<(String, IoValue)> {
    push_bounded(values, (name.to_string(), value), MAX_RETENTION_REFS, "retention fixture artifacts")
}

fn pass_or_deny(value: bool) -> &'static str {
    if value { "pass" } else { "deny" }
}

fn synthetic_ref(label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("retention-synthetic-ref", vec![
        crate::preserves_rail::string(label),
    ]))
}

const RETENTION_CLASSES: &[&str] = &[
    CLASS_EPHEMERAL_CACHE,
    CLASS_DEBUG_TRACE,
    CLASS_REPLAY_SNAPSHOT,
    CLASS_AUDIT_RECEIPT,
    CLASS_DURABLE_VALUE,
    CLASS_PUBLIC_ARTIFACT,
    CLASS_PRIVATE_SECRET_REF,
    CLASS_UPGRADE_ROLLBACK,
    CLASS_LEGAL_HOLD,
];

const PIN_SOURCES: &[&str] = &[
    SOURCE_ACTIVE_SESSION,
    SOURCE_ARTIFACT,
    SOURCE_BLOB,
    SOURCE_RECEIPT,
    SOURCE_SNAPSHOT,
    SOURCE_TRANSCRIPT,
    SOURCE_DOC,
    SOURCE_POLICY,
    SOURCE_UPGRADE,
    SOURCE_STORAGE_REF,
    SOURCE_REMOTE_CACHE,
    SOURCE_EVALUATION_CACHE,
    SOURCE_OPERATOR_HOLD,
    SOURCE_LEGAL_HOLD,
    SOURCE_SECRET_REDACTION,
];

const RETENTION_ACTIONS: &[&str] = &[
    ACTION_PIN,
    ACTION_UNPIN,
    ACTION_RETAIN,
    ACTION_ELIGIBILITY,
    ACTION_DELETE,
    ACTION_TOMBSTONE,
    ACTION_REDACT,
    ACTION_COMPACT,
];

const ADMISSION_KINDS: &[&str] = &[
    ADMISSION_KIND_POLICY,
    ADMISSION_KIND_AUTHORITY,
    ADMISSION_KIND_SUPPORTING_EVIDENCE,
    ADMISSION_KIND_REFERENCE_INDEX,
    ADMISSION_KIND_REMOTE_GC,
];

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p004/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p005/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p006/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p007/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retention/parts/mod/tests/m000/p008/body.rs"));
}
