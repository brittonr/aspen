
fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional raft ref").map(Some);
    }
    required_ref(value, "optional raft ref").map(Some)
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, label)?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    ensure_count_at_most(values.len(), MAX_RAFT_COMMANDS, "raft checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected raft check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{context} missing passing {name} check")))
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")))
    }
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn ensure_store_tables(root: &Path) -> Result<Database> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    let db = Database::create(root.join(STORE_FILE)).map_err(store_error)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        write_txn.open_table(STORE_LOGS).map_err(store_error)?;
        write_txn.open_table(STORE_SNAPSHOTS).map_err(store_error)?;
        write_txn.open_table(STORE_SESSIONS).map_err(store_error)?;
        write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)?;
    Ok(db)
}

fn store_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("control registry redb store error: {error}"))
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("raft-control-fixture-ref", vec![string(label)]))
}
