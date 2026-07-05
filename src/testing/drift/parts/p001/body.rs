fn normalize_input(input: &ComparisonInput) -> Result<NormalizedInput> {
    validate_text("left workflow", &input.left.workflow)?;
    validate_text("right workflow", &input.right.workflow)?;
    if input.left.workflow != input.right.workflow {
        return Err(MoltenError::invalid_harness(format!(
            "drift summaries compare different workflows: {} vs {}",
            input.left.workflow, input.right.workflow
        )));
    }
    let left_fields = field_map("left", &input.left.fields)?;
    let right_fields = field_map("right", &input.right.fields)?;
    let variances = variance_map(&input.allowed_variances, &left_fields, &right_fields)?;
    Ok(NormalizedInput {
        workflow: input.left.workflow.clone(),
        left_fields,
        right_fields,
        variances,
    })
}

fn field_map(label: &str, fields: &[EvidenceField]) -> Result<OrderedMap<String, EvidenceField>> {
    if fields.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} drift summary requires fields")));
    }
    if fields.len() > MAX_DRIFT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "{label} drift summary field count {} exceeds bound {MAX_DRIFT_FIELDS}",
            fields.len()
        )));
    }
    let mut map = OrderedMap::new();
    for field in fields {
        validate_field(field)?;
        if map.insert(field.path.clone(), field.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate {label} drift field path {}", field.path)));
        }
    }
    Ok(map)
}

fn variance_map(
    variances: &[AllowedVariance],
    left_fields: &OrderedMap<String, EvidenceField>,
    right_fields: &OrderedMap<String, EvidenceField>,
) -> Result<OrderedMap<String, String>> {
    if variances.len() > MAX_DRIFT_VARIANCES {
        return Err(MoltenError::invalid_harness(format!(
            "drift variance count {} exceeds bound {MAX_DRIFT_VARIANCES}",
            variances.len()
        )));
    }
    let mut map = OrderedMap::new();
    for variance in variances {
        validate_text("variance path", &variance.path)?;
        validate_variance_reason(&variance.reason)?;
        if !left_fields.contains_key(&variance.path) && !right_fields.contains_key(&variance.path) {
            return Err(MoltenError::invalid_harness(format!(
                "variance path {} does not name a compared field",
                variance.path
            )));
        }
        if map.insert(variance.path.clone(), variance.reason.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate drift variance path {}", variance.path)));
        }
    }
    Ok(map)
}

fn first_divergence(normalized: &NormalizedInput) -> Result<Vec<Diagnostic>> {
    let mut paths = OrderedSet::new();
    paths.extend(normalized.left_fields.keys().cloned());
    paths.extend(normalized.right_fields.keys().cloned());

    for path in paths {
        if normalized.variances.contains_key(&path) {
            continue;
        }
        match (normalized.left_fields.get(&path), normalized.right_fields.get(&path)) {
            (Some(left), Some(right)) => {
                if left.is_ref != right.is_ref {
                    return Ok(vec![diagnostic(
                        &path,
                        "field-kind-drift",
                        &left.is_ref.to_string(),
                        &right.is_ref.to_string(),
                    )]);
                }
                if left.value != right.value {
                    return Ok(vec![diagnostic(&path, "value-drift", &left.value, &right.value)]);
                }
            }
            (Some(left), None) => return Ok(vec![diagnostic(&path, "missing-right-field", &left.value, "<missing>")]),
            (None, Some(right)) => return Ok(vec![diagnostic(&path, "missing-left-field", "<missing>", &right.value)]),
            (None, None) => {
                return Err(MoltenError::invalid_harness(format!("drift path {path} disappeared during comparison")));
            }
        }
    }
    Ok(Vec::new())
}

fn diagnostic(path: &str, kind: &str, left: &str, right: &str) -> Diagnostic {
    Diagnostic {
        path: path.to_string(),
        kind: kind.to_string(),
        left: left.to_string(),
        right: right.to_string(),
    }
}

fn validate_field(field: &EvidenceField) -> Result<()> {
    validate_text("field path", &field.path)?;
    validate_text("field value", &field.value)?;
    if field.is_ref {
        crate::preserves_rail::validate_content_ref(&field.value).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid drift field ref {}: {error}", field.path))
        })?;
    }
    Ok(())
}

fn validate_variance_reason(reason: &str) -> Result<()> {
    match reason {
        "runtime-path" | "diagnostic-log" | "store-path" | "temporary-root" | "rendered-output" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported drift variance reason {other}"))),
    }
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("drift {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        other => {
            Err(MoltenError::invalid_harness(format!("unsupported drift decision {other}; expected pass or deny")))
        }
    }
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn check_value(name: &'static str, state: &'static str) -> IoValue {
    record("check", vec![string(name), string(state)])
}

fn status(is_denied: bool) -> &'static str {
    if is_denied { "deny" } else { "pass" }
}

fn field_values(fields: &[EvidenceField]) -> Result<Vec<IoValue>> {
    if fields.len() > MAX_DRIFT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "drift normalized field count {} exceeds bound {MAX_DRIFT_FIELDS}",
            fields.len()
        )));
    }
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        validate_field(field)?;
        values.push(record("field", vec![
            record("path", vec![string(&field.path)]),
            record("value", vec![string(&field.value)]),
            record("ref", vec![bool_value(field.is_ref)]),
        ]));
    }
    Ok(values)
}

fn diagnostic_values(diagnostics: &[Diagnostic]) -> Result<Vec<IoValue>> {
    if diagnostics.len() > MAX_DRIFT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "drift diagnostic count {} exceeds bound {MAX_DRIFT_FIELDS}",
            diagnostics.len()
        )));
    }
    let mut values = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        validate_text("diagnostic path", &diagnostic.path)?;
        validate_text("diagnostic kind", &diagnostic.kind)?;
        values.push(record("drift", vec![
            record("path", vec![string(&diagnostic.path)]),
            record("kind", vec![string(&diagnostic.kind)]),
            record("left", vec![string(&diagnostic.left)]),
            record("right", vec![string(&diagnostic.right)]),
        ]));
    }
    Ok(values)
}
