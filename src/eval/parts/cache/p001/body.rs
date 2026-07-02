
#[derive(Debug, Clone, Copy)]
pub struct ChoreographyProjectionKeyInput<'a> {
    pub protocol_artifact_ref: &'a str,
    pub role_ref: &'a str,
    pub closure_hash: &'a str,
    pub dependency_refs: &'a [String],
    pub projector_ref: &'a str,
    pub projector_version: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    key_ref: Option<&'a str>,
    value_ref: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

pub fn key_value(input: &KeyInput) -> Result<IoValue> {
    validate_key_input(input)?;
    Ok(record("eval-cache-key-v1", vec![
        string(EVAL_CACHE_KEY_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("version", vec![string(&input.version)]),
        record("input", vec![string(&input.input_ref)]),
        record("dependencies", vec![
            string(&input.dependency_closure_hash),
            refs_sequence(&sorted_unique(&input.dependency_refs)),
        ]),
        record("handler-profile", vec![optional_ref_value(input.handler_profile_ref.as_deref())]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(&input.capability_refs))]),
        record("revocation", vec![refs_sequence(&sorted_unique(&input.revocation_refs))]),
        record("tool", vec![string(&input.tool_ref), string(&input.tool_version)]),
        record("assumptions", vec![refs_sequence(&sorted_unique(&input.assumption_refs))]),
        checks_value(&["domain-separated-key", "no-name-key", "determinism-inputs-bound"]),
    ]))
}

pub fn parse_key(value: &IoValue) -> Result<Key> {
    let fields = value
        .collect_simple_record("eval-cache-key-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <eval-cache-key-v1 ...>"))?;
    require_schema(&fields[0], EVAL_CACHE_KEY_SCHEMA, "eval cache key")?;
    let deps = value_to_iovalue(&fields[4]);
    let dep_fields = simple_record(&deps, "dependencies", 2)?;
    let tool = value_to_iovalue(&fields[9]);
    let tool_fields = simple_record(&tool, "tool", 2)?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "no-name-key", "eval cache key")?;
    Ok(Key {
        key_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        version: record_string(&fields[2], "version")?,
        input_ref: record_ref(&fields[3], "input")?,
        dependency_closure_hash: required_ref(&dep_fields[0], "dependency closure hash")?,
        dependency_refs: parse_ref_sequence_value(&dep_fields[1], "dependency refs")?,
        handler_profile_ref: record_optional_ref(&fields[5], "handler-profile")?,
        policy_refs: record_ref_sequence(&fields[6], "policy")?,
        capability_refs: record_ref_sequence(&fields[7], "capability")?,
        revocation_refs: record_ref_sequence(&fields[8], "revocation")?,
        tool_ref: required_ref(&tool_fields[0], "tool ref")?,
        tool_version: required_string(&tool_fields[1], "tool version")?,
        assumption_refs: record_ref_sequence(&fields[10], "assumptions")?,
        value: value.clone(),
    })
}

pub fn value_value(key_ref: &str, input: &ValueInput, output_ref: &OutputRef) -> Result<IoValue> {
    validate_ref(key_ref, "eval cache key ref")?;
    validate_value_input(input)?;
    validate_output_ref(output_ref)?;
    Ok(record("eval-cache-value-v1", vec![
        string(EVAL_CACHE_VALUE_SCHEMA),
        record("key", vec![string(key_ref)]),
        record("tier", vec![string(&input.tier)]),
        record("status", vec![string(&input.status)]),
        output_ref_value(output_ref),
        record("dependencies", vec![refs_sequence(&sorted_unique(&input.dependency_refs))]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(&input.evidence_refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&["determinism-inputs-bound", "output-integrity", "negative-inputs-bound"]),
    ]))
}

pub fn parse_value(value: &IoValue) -> Result<Value> {
    let fields = value
        .collect_simple_record("eval-cache-value-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <eval-cache-value-v1 ...>"))?;
    require_schema(&fields[0], EVAL_CACHE_VALUE_SCHEMA, "eval cache value")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "determinism-inputs-bound", "eval cache value")?;
    Ok(Value {
        value_ref: canonical_hash(value)?,
        key_ref: record_ref(&fields[1], "key")?,
        tier: record_string(&fields[2], "tier")?,
        status: record_string(&fields[3], "status")?,
        output: parse_output_ref(&fields[4])?,
        dependency_refs: record_ref_sequence(&fields[5], "dependencies")?,
        policy_refs: record_ref_sequence(&fields[6], "policy")?,
        evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn put(root: &Path, key_input: &KeyInput, value_input: &ValueInput) -> Result<Put> {
    ensure_dirs(root)?;
    let key_value = key_value(key_input)?;
    let key = parse_key(&key_value)?;
    validate_value_against_key(&key, value_input)?;
    let output_bytes = value_input.output.as_ref().map(canonical_bytes).transpose()?;
    let output_ref = match (value_input.output.as_ref(), output_bytes.as_ref()) {
        (None, None) => OutputRef::None,
        (Some(output), Some(bytes)) if bytes.len() <= INLINE_OUTPUT_LIMIT => OutputRef::Inline {
            output_ref: canonical_hash(output)?,
            length: bytes.len() as u64,
        },
        (Some(output), Some(bytes)) => {
            let chunk = crate::chunk_store::put_bytes(
                &chunk_root(root),
                "eval-cache-output",
                bytes,
                DEFAULT_FIXED_V1_CHUNK_SIZE,
            )?;
            OutputRef::ContentRef {
                manifest_ref: chunk.manifest_ref,
                output_ref: canonical_hash(output)?,
                length: bytes.len() as u64,
            }
        }
        _ => {
            return Err(MoltenError::invalid_harness(
                "eval cache output bytes must be present whenever output value is present",
            ));
        }
    };
    let value_value = value_value(&key.key_ref, value_input, &output_ref)?;
    let value = parse_value(&value_value)?;
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "put",
        decision: "pass",
        key_ref: Some(&key.key_ref),
        value_ref: Some(&value.value_ref),
        refs: &refs_for_key_value(&key, &value),
        diagnostics: &[],
        checks: &[("cache-insert", "pass"), ("determinism-inputs-bound", "pass")],
    })?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_key_value_in_tx(&write_txn, &key, &value, output_bytes.as_deref())?;
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(Put {
        key,
        value,
        receipt_value,
    })
}

pub fn get(root: &Path, key_ref: &str, input: &GetInput) -> Result<Get> {
    validate_ref(key_ref, "eval cache key ref")?;
    validate_refs(&input.current_policy_refs, "current policy ref")?;
    validate_refs(&input.current_capability_refs, "current capability ref")?;
    validate_refs(&input.current_revocation_refs, "current revocation ref")?;
    ensure_dirs(root)?;
    if let Some(reason) = tombstone_reason(root, key_ref)? {
        return Err(denied_tombstone(root, key_ref, &reason)?);
    }
    let Some((key, value)) = read_key_value_pair(root, key_ref)? else {
        return Err(denied_missing(root, key_ref)?);
    };
    let refs = refs_for_key_value(&key, &value);
    if value.tier == TIER_PRODUCTION_TRACE_ONLY && input.semantic {
        return Err(denied_trace_only(root, &key.key_ref, &value.value_ref, &refs)?);
    }
    if value.tier == TIER_POLICY_CURRENT && !policy_current_refs_match(&key, input) {
        return Err(denied_stale(root, &key.key_ref, &value.value_ref, &refs)?);
    }
    let output = read_output(root, &key.key_ref, &value)?;
    let receipt_value = hit_receipt(root, &key.key_ref, &value.value_ref, &refs)?;
    Ok(Get {
        key,
        value,
        output,
        receipt_value,
    })
}

fn denied_tombstone(root: &Path, key_ref: &str, reason: &str) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "miss",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: None,
        refs: &[key_ref.to_string()],
        diagnostics: &[format!("cache key tombstoned: {reason}")],
        checks: &[("cache-miss", "pass"), ("tombstone", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache miss: key {key_ref} tombstoned ({})",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_missing(root: &Path, key_ref: &str) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "miss",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: None,
        refs: &[key_ref.to_string()],
        diagnostics: &["cache key not found".to_string()],
        checks: &[("cache-miss", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache miss: key {key_ref} not found ({})",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_trace_only(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "trace-only",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &["production trace-only cache value cannot be returned as semantic output".to_string()],
        checks: &[("trace-only-not-semantic", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache trace-only denial: {}",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_stale(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &ReceiptValueInput {
        operation: "stale-deny",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &["policy-current refs do not match current request refs".to_string()],
        checks: &[("policy-current-revalidation", "fail"), ("stale-deny", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache stale policy-current entry denied: {}",
        parse_receipt(&receipt)?.receipt_ref
    )))
}

fn hit_receipt(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<IoValue> {
    store_and_return_receipt(root, &ReceiptValueInput {
        operation: "hit",
        decision: "pass",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &[],
        checks: &[("cache-hit", "pass"), ("output-integrity", "pass")],
    })
}
