
fn ensure_store_tables(root: &std::path::Path) -> Result<redb::Database> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    let db = redb::Database::create(store_path(root)).map_err(store_error)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        write_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
        write_txn.open_table(STORE_ENTRIES).map_err(store_error)?;
        write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        write_txn.open_table(STORE_PINS).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)?;
    Ok(db)
}

fn store_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(STORE_FILE)
}

fn store_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("delivery idempotency redb store error: {error}"))
}

fn strings_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![crate::preserves_rail::sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected checks record"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected checks sequence"))?;
    let mut checks = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(entry);
        let check_fields = check_value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
        checks.push((
            required_string(&check_fields[0], "check name")?,
            required_string(&check_fields[1], "check status")?,
        ));
    }
    Ok(checks)
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} missing pass check {name}")))
    }
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

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
    entries.iter().map(|entry| required_string(entry, label)).collect()
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
