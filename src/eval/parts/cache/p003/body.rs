
fn invalidate_diagnostics(
    input: &InvalidateInput,
    decision: &str,
    invalidated_key_refs: &[String],
    run: &InvalRun,
) -> Vec<String> {
    let mut diagnostics = if decision == "pass" {
        vec![format!("invalidated {} keys", invalidated_key_refs.len())]
    } else {
        vec![format!("retention denied {} keys", run.denials.len())]
    };
    diagnostics.push(format!(
        "retention evidence requester={} policy={} authority={} evidence={} retained={} remote_peers={} remote={} reference_index={} remote_gc={} remote_clearance={} index_complete={}",
        input.retention_evidence.requester_ref.is_some(),
        input.retention_evidence.policy_refs.len(),
        input.retention_evidence.authority_refs.len(),
        input.retention_evidence.evidence_refs.len(),
        input.retention_evidence.retained_refs.len(),
        input.retention_evidence.remote_peer_refs.len(),
        input.retention_evidence.remote_refs.len(),
        input.retention_evidence.reference_index_refs.len(),
        input.retention_evidence.remote_gc_refs.len(),
        input.retention_evidence.remote_clearance_refs.len(),
        input.retention_evidence.is_reference_index_complete
    ));
    diagnostics.extend(run.admission_diagnostics.iter().cloned());
    diagnostics.extend(run.execution_diagnostics.iter().cloned());
    diagnostics
}

fn invalidate_receipt(decision: &str, refs: &[String], diagnostics: &[String], run: &InvalRun) -> Result<IoValue> {
    receipt_value(&ReceiptValueInput {
        operation: "invalidate",
        decision,
        key_ref: None,
        value_ref: None,
        refs,
        diagnostics,
        checks: &[
            ("cache-invalidation", if decision == "pass" { "pass" } else { "fail" }),
            ("tombstone", if decision == "pass" { "pass" } else { "fail" }),
            ("retention-receipt-bound", "pass"),
            ("retention-execution-gate", if run.has_execution_denial() { "fail" } else { "pass" }),
            ("retention-authority-evidence", if run.has_admission_denial() { "fail" } else { "pass" }),
            ("deny-before-tombstone", if decision == "pass" { "pass" } else { "fail" }),
        ],
    })
}

pub fn status(root: &Path) -> Result<Status> {
    ensure_dirs(root)?;
    let mut status = Status::default();
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    status.keys = checked_table_len(
        read_txn.open_table(INDEX_KEYS).map_err(index_error)?.len().map_err(index_error)?,
        "cache keys",
    )?;
    status.values = checked_table_len(
        read_txn.open_table(INDEX_VALUES).map_err(index_error)?.len().map_err(index_error)?,
        "cache values",
    )?;
    status.tombstones = checked_table_len(
        read_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?.len().map_err(index_error)?,
        "cache tombstones",
    )?;
    status.receipts = checked_table_len(
        read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?.len().map_err(index_error)?,
        "cache receipts",
    )?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    for item in values.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        let value = parse_value(&parse_canonical_bytes(bytes.value())?)?;
        match value.tier.as_str() {
            TIER_PURE => status.pure += 1,
            TIER_SIMULATED => status.simulated += 1,
            TIER_POLICY_CURRENT => status.policy_current += 1,
            TIER_PRODUCTION_TRACE_ONLY => status.trace_only_tier += 1,
            _ => {}
        }
        match value.status.as_str() {
            STATUS_PASS => status.pass += 1,
            STATUS_DENY => status.deny += 1,
            STATUS_ERROR => status.error += 1,
            STATUS_TRACE_ONLY => status.trace_only_status += 1,
            _ => {}
        }
    }
    Ok(status)
}

pub fn list(root: &Path, filter: &ListFilter) -> Result<Vec<EntrySummary>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    let tombstones = read_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
    let mut entries = Vec::new();
    for item in values.iter().map_err(index_error)? {
        let (key_ref, bytes) = item.map_err(index_error)?;
        let key_ref = key_ref.value().to_string();
        let value = parse_value(&parse_canonical_bytes(bytes.value())?)?;
        let Some(key_bytes) = keys.get(key_ref.as_str()).map_err(index_error)? else {
            continue;
        };
        let key = parse_key(&parse_canonical_bytes(key_bytes.value())?)?;
        if filter.operation.as_ref().is_some_and(|operation| operation != &key.operation)
            || filter.tier.as_ref().is_some_and(|tier| tier != &value.tier)
            || filter.status.as_ref().is_some_and(|status| status != &value.status)
            || filter.dependency_ref.as_ref().is_some_and(|reference| {
                !key.dependency_refs.contains(reference) && !value.dependency_refs.contains(reference)
            })
            || filter
                .policy_ref
                .as_ref()
                .is_some_and(|reference| !key.policy_refs.contains(reference) && !value.policy_refs.contains(reference))
            || filter.capability_ref.as_ref().is_some_and(|reference| !key.capability_refs.contains(reference))
            || filter.revocation_ref.as_ref().is_some_and(|reference| !key.revocation_refs.contains(reference))
            || filter.evidence_ref.as_ref().is_some_and(|reference| !value.evidence_refs.contains(reference))
        {
            continue;
        }
        push_bounded(
            &mut entries,
            EntrySummary {
                key_ref: key_ref.clone(),
                operation: key.operation,
                tier: value.tier,
                status: value.status,
                value_ref: value.value_ref,
                tombstoned: tombstones.get(key_ref.as_str()).map_err(index_error)?.is_some(),
            },
            MAX_EVAL_CACHE_SCAN_ENTRIES,
            "eval cache entries",
        )?;
    }
    entries.sort_by(|left, right| left.key_ref.cmp(&right.key_ref));
    Ok(entries)
}

pub fn read_key(root: &Path, key_ref: &str) -> Result<Key> {
    validate_ref(key_ref, "eval cache key ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
    let Some(bytes) = keys.get(key_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("eval cache key {key_ref} not found")));
    };
    parse_key(&parse_canonical_bytes(bytes.value())?)
}

pub fn read_value(root: &Path, key_ref: &str) -> Result<Value> {
    validate_ref(key_ref, "eval cache key ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    let Some(bytes) = values.get(key_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("eval cache value for key {key_ref} not found")));
    };
    parse_value(&parse_canonical_bytes(bytes.value())?)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<Receipt> {
    validate_ref(receipt_ref, "eval cache receipt ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = receipts.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("eval cache receipt {receipt_ref} not found")));
    };
    parse_receipt(&parse_canonical_bytes(bytes.value())?)
}

pub fn rebuild_index(root: &Path) -> Result<IoValue> {
    ensure_dirs(root)?;
    let keys_values = {
        let db = ensure_index_tables(root)?;
        let read_txn = db.begin_read().map_err(index_error)?;
        let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
        let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
        let mut pairs = Vec::new();
        for item in keys.iter().map_err(index_error)? {
            let (key_ref, key_bytes) = item.map_err(index_error)?;
            if let Some(value_bytes) = values.get(key_ref.value()).map_err(index_error)? {
                let key = parse_key(&parse_canonical_bytes(key_bytes.value())?)?;
                let value = parse_value(&parse_canonical_bytes(value_bytes.value())?)?;
                push_bounded(&mut pairs, (key, value), MAX_EVAL_CACHE_SCAN_ENTRIES, "eval cache index pairs")?;
            }
        }
        pairs
    };
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_derived_index_tables_in_tx(&write_txn)?;
    for (key, value) in &keys_values {
        store_derived_indexes_in_tx(&write_txn, key, value)?;
    }
    let refs = keys_values.iter().map(|(key, _value)| key.key_ref.clone()).collect::<Vec<_>>();
    let receipt = receipt_value(&ReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        key_ref: None,
        value_ref: None,
        refs: &refs,
        diagnostics: &[format!("rebuilt {} cache entries", keys_values.len())],
        checks: &[("redb-index-rebuild", "pass"), ("derived-index-ready", "pass")],
    })?;
    store_receipt_in_tx(&write_txn, &receipt)?;
    write_txn.commit().map_err(index_error)?;
    Ok(receipt)
}

pub fn schema_fingerprint_key_input(
    normalized_shape_ref: &str,
    tool_ref: &str,
    tool_version: &str,
    policy_refs: &[String],
) -> Result<KeyInput> {
    validate_ref(normalized_shape_ref, "schema fingerprint shape ref")?;
    Ok(KeyInput {
        operation: "schema-fingerprint".to_string(),
        version: "v1".to_string(),
        input_ref: normalized_shape_ref.to_string(),
        dependency_closure_hash: canonical_hash(&record("eval-cache-empty-closure", Vec::new()))?,
        dependency_refs: Vec::new(),
        handler_profile_ref: None,
        policy_refs: policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: tool_ref.to_string(),
        tool_version: tool_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn schema_compatibility_key_input(input: &SchemaCompatibilityKeyInput<'_>) -> Result<KeyInput> {
    let mut dependencies = vec![
        input.expected_identity_ref.to_string(),
        input.actual_identity_ref.to_string(),
    ];
    if let Some(alias_ref) = input.alias_ref {
        validate_ref(alias_ref, "schema compatibility alias ref")?;
        dependencies.push(alias_ref.to_string());
    }
    if let Some(migration_ref) = input.migration_ref {
        validate_ref(migration_ref, "schema compatibility migration ref")?;
        dependencies.push(migration_ref.to_string());
    }
    dependencies.sort();
    let closure_hash = canonical_hash(&record("eval-cache-schema-compat-closure", vec![refs_sequence(&dependencies)]))?;
    Ok(KeyInput {
        operation: "schema-compat".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-schema-compat-input", vec![
            string(input.expected_identity_ref),
            string(input.actual_identity_ref),
            optional_ref_value(input.alias_ref),
            optional_ref_value(input.migration_ref),
        ]))?,
        dependency_closure_hash: closure_hash,
        dependency_refs: dependencies,
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.tool_ref.to_string(),
        tool_version: input.tool_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn artifact_closure_key_input(input: &ArtifactClosureKeyInput<'_>) -> Result<KeyInput> {
    validate_refs(input.root_refs, "artifact closure root ref")?;
    validate_ref(input.closure_hash, "artifact closure hash")?;
    validate_refs(input.dependency_refs, "artifact closure dependency ref")?;
    Ok(KeyInput {
        operation: "artifact-closure".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-artifact-closure-input", vec![refs_sequence(&sorted_unique(
            input.root_refs,
        ))]))?,
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.tool_ref.to_string(),
        tool_version: input.tool_version.to_string(),
        assumption_refs: Vec::new(),
    })
}
