
fn validate_key_input(input: &KeyInput) -> Result<()> {
    validate_operation(&input.operation)?;
    validate_non_empty(&input.version, "eval cache key version")?;
    validate_ref(&input.input_ref, "eval cache input ref")?;
    validate_ref(&input.dependency_closure_hash, "eval cache dependency closure hash")?;
    validate_refs(&input.dependency_refs, "eval cache dependency ref")?;
    if let Some(handler_profile_ref) = input.handler_profile_ref.as_ref() {
        validate_ref(handler_profile_ref, "eval cache handler profile ref")?;
    }
    validate_refs(&input.policy_refs, "eval cache policy ref")?;
    validate_refs(&input.capability_refs, "eval cache capability ref")?;
    validate_refs(&input.revocation_refs, "eval cache revocation ref")?;
    validate_ref(&input.tool_ref, "eval cache tool ref")?;
    validate_non_empty(&input.tool_version, "eval cache tool version")?;
    validate_refs(&input.assumption_refs, "eval cache assumption ref")
}

fn validate_value_input(input: &ValueInput) -> Result<()> {
    validate_tier(&input.tier)?;
    validate_status(&input.status)?;
    validate_refs(&input.dependency_refs, "eval cache value dependency ref")?;
    validate_refs(&input.policy_refs, "eval cache value policy ref")?;
    validate_refs(&input.evidence_refs, "eval cache value evidence ref")?;
    if input.tier == TIER_PRODUCTION_TRACE_ONLY {
        if input.status != STATUS_TRACE_ONLY {
            return Err(MoltenError::invalid_harness(
                "production-effectful trace-only cache values must use trace-only status",
            ));
        }
        if input.output.is_some() {
            return Err(MoltenError::invalid_harness(
                "production-effectful trace-only cache values cannot store semantic output",
            ));
        }
    }
    if input.status == STATUS_PASS && input.output.is_none() {
        return Err(MoltenError::invalid_harness("passing eval cache values require output"));
    }
    Ok(())
}

fn validate_value_against_key(key: &Key, input: &ValueInput) -> Result<()> {
    if !input.dependency_refs.iter().all(|reference| key.dependency_refs.contains(reference)) {
        return Err(MoltenError::invalid_harness("eval cache value dependencies must be represented in key"));
    }
    if !input.policy_refs.iter().all(|reference| key.policy_refs.contains(reference)) {
        return Err(MoltenError::invalid_harness("eval cache value policy refs must be represented in key"));
    }
    if matches!(input.status.as_str(), STATUS_DENY | STATUS_ERROR) {
        if input.evidence_refs.is_empty() {
            return Err(MoltenError::invalid_harness("deterministic negative cache results require evidence refs"));
        }
        for evidence_ref in &input.evidence_refs {
            if !key.assumption_refs.contains(evidence_ref)
                && !key.policy_refs.contains(evidence_ref)
                && !key.capability_refs.contains(evidence_ref)
                && !key.revocation_refs.contains(evidence_ref)
            {
                return Err(MoltenError::invalid_harness(
                    "negative cache result evidence refs must be represented in key assumptions or policy inputs",
                ));
            }
        }
    }
    Ok(())
}

fn validate_output_ref(output: &OutputRef) -> Result<()> {
    match output {
        OutputRef::None => Ok(()),
        OutputRef::Inline { output_ref, .. } => validate_ref(output_ref, "eval cache inline output ref"),
        OutputRef::ContentRef {
            manifest_ref,
            output_ref,
            ..
        } => {
            validate_ref(manifest_ref, "eval cache content manifest ref")?;
            validate_ref(output_ref, "eval cache content output ref")
        }
    }
}

fn validate_invalidate_input(input: &InvalidateInput) -> Result<()> {
    if let Some(key_ref) = input.key_ref.as_ref() {
        validate_ref(key_ref, "invalidate key ref")?;
    }
    if let Some(dependency_ref) = input.dependency_ref.as_ref() {
        validate_ref(dependency_ref, "invalidate dependency ref")?;
    }
    if let Some(policy_ref) = input.policy_ref.as_ref() {
        validate_ref(policy_ref, "invalidate policy ref")?;
    }
    if let Some(capability_ref) = input.capability_ref.as_ref() {
        validate_ref(capability_ref, "invalidate capability ref")?;
    }
    if let Some(revocation_ref) = input.revocation_ref.as_ref() {
        validate_ref(revocation_ref, "invalidate revocation ref")?;
    }
    if let Some(operation) = input.operation.as_ref() {
        validate_operation(operation)?;
    }
    validate_refs(&input.apply_refs, "invalidate apply ref")?;
    crate::retention::validate_destructive_evidence(&input.retention_evidence)?;
    Ok(())
}

fn validate_operation(operation: &str) -> Result<()> {
    validate_non_empty(operation, "eval cache operation")?;
    if operation.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "eval cache operation {operation} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_tier(tier: &str) -> Result<()> {
    if matches!(tier, TIER_PURE | TIER_SIMULATED | TIER_POLICY_CURRENT | TIER_PRODUCTION_TRACE_ONLY) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported eval cache tier {tier}")))
    }
}

fn validate_status(status: &str) -> Result<()> {
    if matches!(status, STATUS_PASS | STATUS_DENY | STATUS_ERROR | STATUS_TRACE_ONLY) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported eval cache status {status}")))
    }
}

fn output_ref_value(output: &OutputRef) -> IoValue {
    record("output", vec![match output {
        OutputRef::None => record("none", Vec::new()),
        OutputRef::Inline { output_ref, length } => record("inline", vec![string(output_ref), u64_value(*length)]),
        OutputRef::ContentRef {
            manifest_ref,
            output_ref,
            length,
        } => record("content-ref", vec![string(manifest_ref), string(output_ref), u64_value(*length)]),
    }])
}

fn parse_output_ref(value: &PreservesValue<IoValue>) -> Result<OutputRef> {
    let value = value_to_iovalue(value);
    let output = simple_record(&value, "output", 1)?;
    let payload = value_to_iovalue(&output[0]);
    if payload.collect_simple_record("none", Some(0)).is_some() {
        return Ok(OutputRef::None);
    }
    if let Some(inline) = payload.collect_simple_record("inline", Some(2)) {
        return Ok(OutputRef::Inline {
            output_ref: required_ref(&inline[0], "inline output ref")?,
            length: required_u64(&inline[1], "inline output length")?,
        });
    }
    if let Some(content) = payload.collect_simple_record("content-ref", Some(3)) {
        return Ok(OutputRef::ContentRef {
            manifest_ref: required_ref(&content[0], "content output manifest ref")?,
            output_ref: required_ref(&content[1], "content output ref")?,
            length: required_u64(&content[2], "content output length")?,
        });
    }
    Err(MoltenError::invalid_harness("eval cache output must be none, inline, or content-ref"))
}

fn clear_derived_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> Result<()> {
    clear_str_table(write_txn, INDEX_OPERATION)?;
    clear_str_table(write_txn, INDEX_DEPENDENCY)?;
    clear_str_table(write_txn, INDEX_POLICY)?;
    clear_str_table(write_txn, INDEX_CAPABILITY)?;
    clear_str_table(write_txn, INDEX_REVOCATION)?;
    clear_str_table(write_txn, INDEX_EVIDENCE)?;
    clear_str_table(write_txn, INDEX_STATUS)?;
    clear_str_table(write_txn, INDEX_TIER)
}

fn clear_str_table(write_txn: &redb::WriteTransaction, table: TableDefinition<&str, &str>) -> Result<()> {
    let mut table = write_txn.open_table(table).map_err(index_error)?;
    let keys = str_table_keys(&table)?;
    for key in keys {
        table.remove(key.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn ensure_dirs(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    std::fs::create_dir_all(chunk_root(root)).map_err(MoltenError::from)
}

fn ensure_index_tables(root: &Path) -> Result<Database> {
    ensure_dirs(root)?;
    let db = Database::create(index_path(root)).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_KEYS).map_err(index_error)?;
        write_txn.open_table(INDEX_VALUES).map_err(index_error)?;
        write_txn.open_table(INDEX_OUTPUTS).map_err(index_error)?;
        write_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
        write_txn.open_table(INDEX_OPERATION).map_err(index_error)?;
        write_txn.open_table(INDEX_DEPENDENCY).map_err(index_error)?;
        write_txn.open_table(INDEX_POLICY).map_err(index_error)?;
        write_txn.open_table(INDEX_CAPABILITY).map_err(index_error)?;
        write_txn.open_table(INDEX_REVOCATION).map_err(index_error)?;
        write_txn.open_table(INDEX_EVIDENCE).map_err(index_error)?;
        write_txn.open_table(INDEX_STATUS).map_err(index_error)?;
        write_txn.open_table(INDEX_TIER).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn chunk_root(root: &Path) -> PathBuf {
    root.join("chunks")
}

fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<BtreeSet<_>>().into_iter().collect()
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn record_string(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_ref_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_string_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    items.iter().map(|item| required_string(item, label)).collect()
}

fn parse_ref_sequence_value(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}
