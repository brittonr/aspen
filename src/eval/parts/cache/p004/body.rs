
pub fn choreography_projection_key_input(input: &ChoreographyProjectionKeyInput<'_>) -> Result<KeyInput> {
    validate_ref(input.protocol_artifact_ref, "choreography protocol artifact ref")?;
    validate_ref(input.role_ref, "choreography role ref")?;
    validate_ref(input.closure_hash, "choreography closure hash")?;
    validate_refs(input.dependency_refs, "choreography dependency ref")?;
    validate_ref(input.projector_ref, "choreography projector ref")?;
    Ok(KeyInput {
        operation: "choreography-projection".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-choreography-projection-input", vec![
            string(input.protocol_artifact_ref),
            string(input.role_ref),
        ]))?,
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.projector_ref.to_string(),
        tool_version: input.projector_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn wasm_inspection_key_placeholder(
    module_artifact_ref: &str,
    inspector_ref: &str,
    inspector_version: &str,
) -> Result<KeyInput> {
    validate_ref(module_artifact_ref, "wasm module artifact ref")?;
    Ok(KeyInput {
        operation: "wasm-inspection".to_string(),
        version: "v1".to_string(),
        input_ref: module_artifact_ref.to_string(),
        dependency_closure_hash: canonical_hash(&record("eval-cache-wasm-closure", vec![string(module_artifact_ref)]))?,
        dependency_refs: vec![module_artifact_ref.to_string()],
        handler_profile_ref: None,
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: inspector_ref.to_string(),
        tool_version: inspector_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn transcript_run_key_placeholder(input: &TranscriptRunKeyInput<'_>) -> Result<KeyInput> {
    validate_ref(input.transcript_ref, "transcript ref")?;
    validate_ref(input.handler_profile_ref, "handler profile ref")?;
    Ok(KeyInput {
        operation: "transcript-run".to_string(),
        version: "v1".to_string(),
        input_ref: input.transcript_ref.to_string(),
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        handler_profile_ref: Some(input.handler_profile_ref.to_string()),
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.harness_ref.to_string(),
        tool_version: input.harness_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn parse_receipt(value: &IoValue) -> Result<Receipt> {
    let fields = value
        .collect_simple_record("eval-cache-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <eval-cache-receipt-v1 ...>"))?;
    require_schema(&fields[0], EVAL_CACHE_RECEIPT_SCHEMA, "eval cache receipt")?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("eval cache receipt missing checks"));
    }
    Ok(Receipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        key_ref: record_optional_ref(&fields[3], "key")?,
        value_ref: record_optional_ref(&fields[4], "value")?,
        value: value.clone(),
    })
}

fn read_key_value_pair(root: &Path, key_ref: &str) -> Result<Option<(Key, Value)>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    let Some(key_bytes) = keys.get(key_ref).map_err(index_error)? else {
        return Ok(None);
    };
    let Some(value_bytes) = values.get(key_ref).map_err(index_error)? else {
        return Ok(None);
    };
    let key = parse_key(&parse_canonical_bytes(key_bytes.value())?)?;
    let value = parse_value(&parse_canonical_bytes(value_bytes.value())?)?;
    Ok(Some((key, value)))
}

fn read_output(root: &Path, key_ref: &str, value: &Value) -> Result<Option<IoValue>> {
    match &value.output {
        OutputRef::None => Ok(None),
        OutputRef::Inline { output_ref, length } => {
            let db = ensure_index_tables(root)?;
            let read_txn = db.begin_read().map_err(index_error)?;
            let outputs = read_txn.open_table(INDEX_OUTPUTS).map_err(index_error)?;
            let Some(bytes) = outputs.get(key_ref).map_err(index_error)? else {
                return Err(MoltenError::invalid_harness(format!("missing inline eval cache output for {key_ref}")));
            };
            let bytes = bytes.value().to_vec();
            if bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness("eval cache inline output length mismatch"));
            }
            let output = parse_canonical_bytes(&bytes)?;
            let actual_ref = canonical_hash(&output)?;
            if &actual_ref != output_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "eval cache output hash mismatch: got {actual_ref}, expected {output_ref}"
                )));
            }
            Ok(Some(output))
        }
        OutputRef::ContentRef {
            manifest_ref,
            output_ref,
            length,
        } => {
            let read = crate::chunk_store::read_object(&chunk_root(root), manifest_ref)?;
            if read.bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness("eval cache content output length mismatch"));
            }
            let output = parse_canonical_bytes(&read.bytes)?;
            let actual_ref = canonical_hash(&output)?;
            if &actual_ref != output_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "eval cache output hash mismatch: got {actual_ref}, expected {output_ref}"
                )));
            }
            Ok(Some(output))
        }
    }
}

fn store_key_value_in_tx(
    write_txn: &redb::WriteTransaction,
    key: &Key,
    value: &Value,
    output_bytes: Option<&[u8]>,
) -> Result<()> {
    {
        let mut keys = write_txn.open_table(INDEX_KEYS).map_err(index_error)?;
        keys.insert(key.key_ref.as_str(), canonical_bytes(&key.value)?.as_slice()).map_err(index_error)?;
    }
    {
        let mut values = write_txn.open_table(INDEX_VALUES).map_err(index_error)?;
        values
            .insert(key.key_ref.as_str(), canonical_bytes(&value.value)?.as_slice())
            .map_err(index_error)?;
    }
    if let (OutputRef::Inline { .. }, Some(output_bytes)) = (&value.output, output_bytes) {
        let mut outputs = write_txn.open_table(INDEX_OUTPUTS).map_err(index_error)?;
        outputs.insert(key.key_ref.as_str(), output_bytes).map_err(index_error)?;
    }
    store_derived_indexes_in_tx(write_txn, key, value)
}

fn store_derived_indexes_in_tx(write_txn: &redb::WriteTransaction, key: &Key, value: &Value) -> Result<()> {
    insert_str_index(write_txn, INDEX_OPERATION, "operation", &key.operation, &key.key_ref)?;
    insert_str_index(write_txn, INDEX_STATUS, "status", &value.status, &key.key_ref)?;
    insert_str_index(write_txn, INDEX_TIER, "tier", &value.tier, &key.key_ref)?;
    for reference in &key.dependency_refs {
        insert_str_index(write_txn, INDEX_DEPENDENCY, "dependency", reference, &key.key_ref)?;
    }
    for reference in &value.dependency_refs {
        insert_str_index(write_txn, INDEX_DEPENDENCY, "dependency", reference, &key.key_ref)?;
    }
    for reference in &key.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", reference, &key.key_ref)?;
    }
    for reference in &value.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", reference, &key.key_ref)?;
    }
    for reference in &key.capability_refs {
        insert_str_index(write_txn, INDEX_CAPABILITY, "capability", reference, &key.key_ref)?;
    }
    for reference in &key.revocation_refs {
        insert_str_index(write_txn, INDEX_REVOCATION, "revocation", reference, &key.key_ref)?;
    }
    for reference in &value.evidence_refs {
        insert_str_index(write_txn, INDEX_EVIDENCE, "evidence", reference, &key.key_ref)?;
    }
    Ok(())
}

fn insert_str_index(
    write_txn: &redb::WriteTransaction,
    table: TableDefinition<&str, &str>,
    index_kind: &str,
    indexed: &str,
    key_ref: &str,
) -> Result<()> {
    let index_key =
        canonical_hash(&record("eval-cache-index-key", vec![string(index_kind), string(indexed), string(key_ref)]))?;
    let mut table = write_txn.open_table(table).map_err(index_error)?;
    table.insert(index_key.as_str(), key_ref).map_err(index_error)?;
    Ok(())
}

fn store_and_return_receipt(root: &Path, input: &ReceiptValueInput<'_>) -> Result<IoValue> {
    let receipt = receipt_value(input)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, &receipt)?;
    write_txn.commit().map_err(index_error)?;
    Ok(receipt)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt: &IoValue) -> Result<()> {
    let parsed = parse_receipt(receipt)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts
        .insert(parsed.receipt_ref.as_str(), canonical_bytes(receipt)?.as_slice())
        .map_err(index_error)?;
    Ok(())
}

fn tombstone_reason(root: &Path, key_ref: &str) -> Result<Option<String>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let tombstones = read_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
    Ok(tombstones.get(key_ref).map_err(index_error)?.map(|value| value.value().to_string()))
}

fn receipt_value(input: &ReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "eval cache receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported eval cache receipt decision {}",
            input.decision
        )));
    }
    if let Some(key_ref) = input.key_ref {
        validate_ref(key_ref, "eval cache receipt key ref")?;
    }
    if let Some(value_ref) = input.value_ref {
        validate_ref(value_ref, "eval cache receipt value ref")?;
    }
    validate_refs(input.refs, "eval cache receipt ref")?;
    Ok(record("eval-cache-receipt-v1", vec![
        string(EVAL_CACHE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("key", vec![optional_ref_value(input.key_ref)]),
        record("value", vec![optional_ref_value(input.value_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn refs_for_key_value(key: &Key, value: &Value) -> Vec<String> {
    let mut refs = vec![
        key.key_ref.clone(),
        value.value_ref.clone(),
        key.input_ref.clone(),
        key.dependency_closure_hash.clone(),
        key.tool_ref.clone(),
    ];
    refs.extend(key.dependency_refs.iter().cloned());
    refs.extend(key.policy_refs.iter().cloned());
    refs.extend(key.capability_refs.iter().cloned());
    refs.extend(key.revocation_refs.iter().cloned());
    refs.extend(key.assumption_refs.iter().cloned());
    refs.extend(value.dependency_refs.iter().cloned());
    refs.extend(value.policy_refs.iter().cloned());
    refs.extend(value.evidence_refs.iter().cloned());
    if let Some(handler) = key.handler_profile_ref.as_ref() {
        refs.push(handler.clone());
    }
    sorted_unique(&refs)
}

pub fn evaluate_cache_hit_validity(input: CacheHitValidityInput<'_>) -> CacheHitValidityDecision {
    let mut diagnostics = Vec::with_capacity(CACHE_HIT_VALIDITY_DIAGNOSTIC_CAPACITY);
    if input.value.key_ref != input.key.key_ref {
        diagnostics.push("cache-key-value-ref-mismatch".to_string());
    }
    if input.semantic && input.value.tier == TIER_PRODUCTION_TRACE_ONLY {
        diagnostics.push("trace-only-not-semantic".to_string());
    }
    if input.value.tier == TIER_POLICY_CURRENT && !policy_current_refs_match_parts(input.key, input) {
        diagnostics.push("policy-current-revalidation".to_string());
    }
    if !input.requested_dependency_refs.is_empty() {
        let requested = sorted_unique(input.requested_dependency_refs);
        let mut cached = input.key.dependency_refs.clone();
        cached.extend(input.value.dependency_refs.iter().cloned());
        if sorted_unique(&cached) != requested {
            diagnostics.push("dependency-refs-changed".to_string());
        }
    }
    if input
        .current_revocation_refs
        .iter()
        .any(|revocation_ref| input.key.capability_refs.iter().any(|capability_ref| capability_ref == revocation_ref))
    {
        diagnostics.push("capability-revoked".to_string());
    }
    if let Some(expected_output_ref) = input.expected_output_ref {
        let actual_output_ref = match &input.value.output {
            OutputRef::Inline { output_ref, .. } | OutputRef::ContentRef { output_ref, .. } => Some(output_ref.as_str()),
            OutputRef::None => None,
        };
        if actual_output_ref != Some(expected_output_ref) {
            diagnostics.push("output-ref-mismatch".to_string());
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    CacheHitValidityDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn policy_current_refs_match_parts(key: &Key, input: CacheHitValidityInput<'_>) -> bool {
    sorted_unique(&key.policy_refs) == sorted_unique(input.current_policy_refs)
        && sorted_unique(&key.capability_refs) == sorted_unique(input.current_capability_refs)
        && sorted_unique(&key.revocation_refs) == sorted_unique(input.current_revocation_refs)
}
