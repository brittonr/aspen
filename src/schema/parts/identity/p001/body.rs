
pub fn parse_compatibility(value: &IoValue) -> Result<Compatibility> {
    let fields = value
        .collect_simple_record("schema-compatibility-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-compatibility-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SCHEMA_COMPATIBILITY_SCHEMA, "schema compatibility")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "unique-not-structural-by-default", "schema compatibility")?;
    let expected = parse_compatibility_identity(&fields[2], "expected")?;
    let actual = parse_compatibility_identity(&fields[3], "actual")?;
    Ok(Compatibility {
        compatibility_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        expected_identity_ref: expected.0,
        expected_schema_ref: expected.1,
        actual_identity_ref: actual.0,
        actual_schema_ref: actual.1,
        alias_ref: record_optional_ref(&fields[4], "alias")?,
        migration_ref: record_optional_ref(&fields[5], "migration")?,
        value: value.clone(),
    })
}

pub fn compatibility_admits_storage(
    value: &IoValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "typed storage request")
}

pub fn compatibility_admits_protocol_payload(
    value: &IoValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "protocol payload request")
}

pub fn compatibility_admits_effect_schema(
    value: &IoValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "effect schema request")
}

pub fn compatibility_admits_policy_contract_schema(
    value: &IoValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "policy contract schema request")
}

fn compatibility_admits_scope(
    value: &IoValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
    context: &str,
) -> Result<bool> {
    let parsed = parse_compatibility(value)?;
    if parsed.expected_schema_ref != expected_schema_ref || parsed.actual_schema_ref != actual_schema_ref {
        return Err(MoltenError::invalid_harness(format!("schema compatibility refs do not match {context}")));
    }
    Ok(matches!(
        parsed.decision.as_str(),
        DECISION_EXACT_ARTIFACT_MATCH
            | DECISION_STRUCTURAL_MATCH
            | DECISION_BRAND_MATCH
            | DECISION_ADMITTED_ALIAS
            | DECISION_MIGRATION_AVAILABLE
    ))
}

pub fn compatibility_receipt_value(operation: &str, compatibility_value: &IoValue) -> Result<IoValue> {
    validate_non_empty(operation, "schema compatibility receipt operation")?;
    let compatibility = parse_compatibility(compatibility_value)?;
    let decision = if matches!(
        compatibility.decision.as_str(),
        DECISION_EXACT_ARTIFACT_MATCH
            | DECISION_STRUCTURAL_MATCH
            | DECISION_BRAND_MATCH
            | DECISION_ADMITTED_ALIAS
            | DECISION_MIGRATION_AVAILABLE
    ) {
        "pass"
    } else {
        "deny"
    };
    Ok(record("schema-compatibility-receipt-v1", vec![
        string(crate::preserves_rail::SCHEMA_COMPATIBILITY_RECEIPT_SCHEMA),
        record("operation", vec![string(operation)]),
        record("decision", vec![string(decision)]),
        record("compatibility", vec![string(&compatibility.compatibility_ref)]),
        record("expected-schema", vec![string(&compatibility.expected_schema_ref)]),
        record("actual-schema", vec![string(&compatibility.actual_schema_ref)]),
        checks_value(&["schema-compatibility-recorded", "policy-denial-wins"]),
    ]))
}

pub fn parse_compatibility_receipt(value: &IoValue) -> Result<CompatibilityReceipt> {
    let fields = value
        .collect_simple_record("schema-compatibility-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-compatibility-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::SCHEMA_COMPATIBILITY_RECEIPT_SCHEMA,
        "schema compatibility receipt",
    )?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "schema-compatibility-recorded", "schema compatibility receipt")?;
    Ok(CompatibilityReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        compatibility_ref: record_ref(&fields[3], "compatibility")?,
        value: value.clone(),
    })
}

pub fn search_registry_by_fingerprint(registry_root: &std::path::Path, fingerprint: &str) -> Result<Vec<Identity>> {
    validate_ref(fingerprint, "schema structural fingerprint")?;
    let mut matches = Vec::new();
    for artifact in crate::artifacts::list_artifacts(registry_root, Some("schema-identity"))? {
        let payload = crate::artifacts::read_payload(registry_root, &artifact.artifact_ref)?;
        if let Ok(identity) = parse_identity(&payload)
            && identity.structural_fingerprint == fingerprint
        {
            push_bounded(&mut matches, identity, MAX_SEARCH_MATCHES, "schema search matches")?;
        }
    }
    matches.sort_by(|left, right| left.identity_ref.cmp(&right.identity_ref));
    Ok(matches)
}

fn compatibility_decision(input: &CompatibilityInput) -> Result<String> {
    if input.deny_by_policy {
        return Ok(DECISION_DENIED_BY_POLICY.to_string());
    }
    if input.expected.schema_ref == input.actual.schema_ref {
        return Ok(DECISION_EXACT_ARTIFACT_MATCH.to_string());
    }
    if let Some(alias) = input.alias.as_ref()
        && alias.from_schema_ref == input.actual.schema_ref
        && alias.to_schema_ref == input.expected.schema_ref
        && matches!(alias.scope.as_str(), "storage" | "effect" | "protocol" | "policy" | "global-local-fixture")
    {
        return Ok(DECISION_ADMITTED_ALIAS.to_string());
    }
    if input.expected.mode == MODE_STRUCTURAL
        && input.actual.mode == MODE_STRUCTURAL
        && input.expected.structural_fingerprint == input.actual.structural_fingerprint
    {
        return Ok(DECISION_STRUCTURAL_MATCH.to_string());
    }
    if input.expected.mode == MODE_BRANDED_STRUCTURAL
        && input.actual.mode == MODE_BRANDED_STRUCTURAL
        && input.expected.brand_ref == input.actual.brand_ref
        && input.expected.structural_fingerprint == input.actual.structural_fingerprint
    {
        return Ok(DECISION_BRAND_MATCH.to_string());
    }
    if input.migration_ref.is_some() {
        return Ok(DECISION_MIGRATION_AVAILABLE.to_string());
    }
    Ok(DECISION_MISMATCH_REQUIRES_MIGRATION.to_string())
}

fn normalize_record_shape(fields: &Record<Value<IoValue>>) -> Result<IoValue> {
    if fields.len() != 3 {
        return Err(MoltenError::invalid_harness("record shape expects label and field sequence"));
    }
    let label = required_string(&fields[1], "record shape label")?;
    let field_items = required_sequence(&fields[2], "record shape fields")?;
    let mut normalized_fields = Vec::with_capacity(field_items.len());
    for field in field_items.iter() {
        normalized_fields.push(normalize_shape(&value_to_iovalue(&field))?);
    }
    normalized_fields.sort_by_key(|field| canonical_hash(field).unwrap_or_else(|_| String::new()));
    Ok(record("shape", vec![string("record"), string(label), sequence(normalized_fields)]))
}

fn normalize_field_shape(fields: &Record<Value<IoValue>>) -> Result<IoValue> {
    if fields.len() != 3 {
        return Err(MoltenError::invalid_harness("field shape expects name and nested shape"));
    }
    Ok(record("shape", vec![
        string("field"),
        required_string(&fields[1], "field shape name").map(string)?,
        normalize_shape(&value_to_iovalue(&fields[2]))?,
    ]))
}

fn normalize_unary_shape(kind: &'static str, fields: &Record<Value<IoValue>>) -> Result<IoValue> {
    if fields.len() != 2 {
        return Err(MoltenError::invalid_harness(format!("{kind} shape expects one nested shape")));
    }
    Ok(record("shape", vec![string(kind), normalize_shape(&value_to_iovalue(&fields[1]))?]))
}

fn normalize_binary_shape(kind: &'static str, fields: &Record<Value<IoValue>>) -> Result<IoValue> {
    if fields.len() != 3 {
        return Err(MoltenError::invalid_harness(format!("{kind} shape expects two nested shapes")));
    }
    Ok(record("shape", vec![
        string(kind),
        normalize_shape(&value_to_iovalue(&fields[1]))?,
        normalize_shape(&value_to_iovalue(&fields[2]))?,
    ]))
}

fn compatibility_identity_record(label: &'static str, identity: &Identity) -> IoValue {
    record(label, vec![
        string(&identity.identity_ref),
        string(&identity.schema_ref),
        string(&identity.mode),
        string(&identity.structural_fingerprint),
        optional_ref_value(identity.brand_ref.as_deref()),
    ])
}

fn parse_compatibility_identity(value: &Value<IoValue>, label: &str) -> Result<(String, String)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 5)?;
    Ok((
        required_ref(&fields[0], "compatibility identity ref")?,
        required_ref(&fields[1], "compatibility schema ref")?,
    ))
}

fn validate_mode(mode: &str) -> Result<()> {
    if matches!(mode, MODE_STRUCTURAL | MODE_UNIQUE | MODE_BRANDED_STRUCTURAL) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported schema identity mode {mode}; expected structural, unique, or branded-structural"
        )))
    }
}

fn validate_alias_scope(scope: &str) -> Result<()> {
    if matches!(scope, "storage" | "effect" | "protocol" | "policy" | "global-local-fixture") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported schema alias scope {scope}; expected storage, effect, protocol, policy, or global-local-fixture"
        )))
    }
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}
