
fn boundary_record<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    arity: usize,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value.collect_simple_record(label, Some(arity)).ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} must be <{label} ...> with arity {arity} using schema {}",
            spec.family, schema_ref
        ))
    })
}

fn validate_boundary_schema_id(
    value: &Value<IoValue>,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let actual_schema = value.as_string().ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: schema field must be a string for schema {}",
            spec.family, schema_ref
        ))
    })?;
    if actual_schema.as_ref() == spec.schema_id {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{} schema validation deny: unsupported schema {} expected {} using schema {}",
            spec.family,
            actual_schema.as_ref(),
            spec.schema_id,
            schema_ref
        )))
    }
}

fn validate_string_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    ensure_string(&record[0], label, spec, schema_ref).map(|_| ())
}

fn validate_non_empty_string_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let text = ensure_string(&record[0], label, spec, schema_ref)?;
    if text.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} requires a non-empty string using schema {}",
            spec.family, schema_ref
        )));
    }
    Ok(())
}

fn validate_stable_id_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let text = ensure_string(&record[0], label, spec, schema_ref)?;
    validate_stable_id(text.as_ref(), label).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} expected stable id using schema {}: {error}",
            spec.family, schema_ref
        ))
    })
}

fn validate_decision_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let text = ensure_string(&record[0], label, spec, schema_ref)?;
    Decision::parse(text.as_ref()).map(|_| ()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} expected decision using schema {}: {error}",
            spec.family, schema_ref
        ))
    })
}

fn validate_u64_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    record[0]
        .as_u64()
        .ok_or_else(|| boundary_field_error(spec, label, "u64", schema_ref))?
        .map(|_| ())
        .map_err(|error| MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} u64 out of range using schema {}: {error}",
            spec.family, schema_ref
        )))
}

fn validate_ref_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    ensure_content_ref(&record[0], label, spec, schema_ref)
}

fn validate_ref_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    validate_ref_sequence_record_with_contract(RefSequenceContractInput { value, label, spec, schema_ref, require_non_empty: false, require_unique: false })
}

struct RefSequenceContractInput<'a> {
    value: &'a Value<IoValue>,
    label: &'a str,
    spec: &'a BoundarySchemaSpec,
    schema_ref: &'a ContentRef,
    require_non_empty: bool,
    require_unique: bool,
}

fn validate_ref_sequence_record_with_contract(input: RefSequenceContractInput<'_>) -> Result<()> {
    let RefSequenceContractInput { value, label, spec, schema_ref, require_non_empty, require_unique } = input;
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let sequence = ensure_sequence(&record[0], label, spec, schema_ref)?;
    if require_non_empty && sequence.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} requires a non-empty ref sequence using schema {}",
            spec.family, schema_ref
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for item in sequence.iter() {
        let reference = ensure_content_ref_string(item, label, spec, schema_ref)?;
        if require_unique && !seen.insert(reference.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: field {label} duplicate ref {reference} using schema {}",
                spec.family, schema_ref
            )));
        }
    }
    Ok(())
}

fn validate_string_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let sequence = ensure_sequence(&record[0], label, spec, schema_ref)?;
    for item in sequence.iter() {
        ensure_string(item, label, spec, schema_ref)?;
    }
    Ok(())
}

fn validate_unique_string_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let sequence = ensure_sequence(&record[0], label, spec, schema_ref)?;
    let mut seen = std::collections::BTreeSet::new();
    for item in sequence.iter() {
        let text = ensure_string(item, label, spec, schema_ref)?;
        if !seen.insert(text.to_string()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: field {label} duplicate string {text} using schema {}",
                spec.family, schema_ref
            )));
        }
    }
    Ok(())
}

fn validate_any_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    ensure_sequence(&record[0], label, spec, schema_ref).map(|_| ())
}

fn validate_optional_ref_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(FIELD_ARITY_ZERO)).is_some() {
        return Ok(());
    }
    if let Some(some) = optional.collect_simple_record("some", Some(FIELD_ARITY_ONE)) {
        return ensure_content_ref(&some[0], field_spec.label, spec, schema_ref);
    }
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)
}

fn validate_checks_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let checks = ensure_sequence(&record[0], field_spec.label, spec, schema_ref)?;
    let mut seen = std::collections::BTreeSet::new();
    for item in checks.iter() {
        let item = value_to_iovalue(item);
        let check = item.collect_simple_record("check", Some(FIELD_ARITY_TWO)).ok_or_else(|| {
            boundary_field_error(spec, field_spec.label, "<check string string>", schema_ref)
        })?;
        let name = ensure_string(&check[0], "check name", spec, schema_ref)?;
        let status = ensure_string(&check[1], "check status", spec, schema_ref)?;
        if !seen.insert(name.to_string()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: duplicate check {name} using schema {}",
                spec.family, schema_ref
            )));
        }
        CheckStatus::parse(status.as_ref()).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "{} schema validation deny: unsupported check status using schema {}: {error}",
                spec.family, schema_ref
            ))
        })?;
    }
    Ok(())
}

fn validate_chain_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let value = value_to_iovalue(value);
    let chain = value.collect_simple_record(field_spec.label, Some(FIELD_ARITY_THREE)).ok_or_else(|| {
        boundary_field_error(spec, field_spec.label, "chain record", schema_ref)
    })?;
    validate_string_record(&chain[0], "scope", spec, schema_ref)?;
    validate_string_record(&chain[1], "id", spec, schema_ref)?;
    validate_string_record(&chain[2], "epoch", spec, schema_ref)
}

fn validate_object_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_string(&record[1], field_spec.label, spec, schema_ref).map(|_| ())
}
