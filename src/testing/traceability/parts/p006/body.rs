fn deny_path_diagnostics(input: &DenyPathMatrixInput) -> Result<Vec<String>> {
    ensure_count_at_most(input.cases.len(), MAX_COVERAGE_ITEMS, "deny path cases")?;
    let max_diagnostics = deny_path_diagnostic_bound(input.cases.len(), required_deny_classes().len())?;
    let mut diagnostics = Vec::with_capacity(input.cases.len());
    let mut classes = OrderedSet::new();
    for case in &input.cases {
        validate_deny_path_case(case)?;
        if !classes.insert(case.class.clone()) {
            diagnostics.push_limited(
                format!("duplicate-deny-class:{}", case.class),
                max_diagnostics,
                "deny path diagnostics",
            )?;
        }
        if case.expected_decision != "deny" {
            diagnostics.push_limited(
                format!("wrong-deny-decision:{}", case.class),
                max_diagnostics,
                "deny path diagnostics",
            )?;
        }
        if case.class == "denied-mutation" {
            if mutation_absence_evidence_present(case) {
                continue;
            }
            diagnostics.push_limited(
                "missing-no-mutation-evidence:denied-mutation".to_string(),
                max_diagnostics,
                "deny path diagnostics",
            )?;
        }
    }
    for required in required_deny_classes() {
        if !classes.contains(*required) {
            diagnostics.push_limited(
                format!("missing-deny-class:{required}"),
                max_diagnostics,
                "deny path diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}

fn validate_deny_path_case(case: &DenyPathCaseInput) -> Result<()> {
    validate_deny_class(&case.class)?;
    validate_ref(&case.fixture_ref, "deny fixture")?;
    validate_decision(&case.expected_decision)?;
    if let Some(reference) = &case.before_state_ref {
        validate_ref(reference, "deny before state")?;
    }
    if let Some(reference) = &case.after_state_ref {
        validate_ref(reference, "deny after state")?;
    }
    if let Some(reference) = &case.no_mutation_ref {
        validate_ref(reference, "deny no-mutation receipt")?;
    }
    Ok(())
}

fn mutation_absence_evidence_present(case: &DenyPathCaseInput) -> bool {
    case.no_mutation_ref.is_some()
        || matches!((&case.before_state_ref, &case.after_state_ref), (Some(before), Some(after)) if before == after)
}

fn deny_path_matrix_value(input: &DenyPathMatrixInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("proof-deny-path-matrix-v1", vec![
        string(DENY_PATH_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("gate", vec![string(&input.gate)]),
        record("subject", vec![string(&input.subject_ref)]),
        record("cases", vec![sequence(deny_case_values(&input.cases)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
        record("caveats", vec![sequence(vec![
            string("deny-path evidence proves only the declared gate scope"),
            string("logs are diagnostic-only"),
        ])]),
    ]))
}

fn deny_case_values(cases: &[DenyPathCaseInput]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(cases.len());
    for case in cases {
        values.push(record("case", vec![
            record("class", vec![string(&case.class)]),
            record("fixture", vec![string(&case.fixture_ref)]),
            record("expected-decision", vec![string(&case.expected_decision)]),
            record("before-state", vec![optional_ref_value(case.before_state_ref.as_deref())]),
            record("after-state", vec![optional_ref_value(case.after_state_ref.as_deref())]),
            record("no-mutation", vec![optional_ref_value(case.no_mutation_ref.as_deref())]),
        ]));
    }
    Ok(values)
}

fn required_deny_classes() -> &'static [&'static str] {
    &[
        "missing-artifact",
        "stale-ref",
        "malformed-schema",
        "wrong-signer",
        "wrong-purpose",
        "tampered-bytes",
        "duplicate-receipt",
        "denied-mutation",
        "diagnostic-only-not-pass",
    ]
}

fn receipt_refs(evidence: &[VerificationEvidence]) -> Vec<String> {
    evidence.iter().filter_map(|item| item.receipt_ref.clone()).collect()
}

fn artifact_refs_for_entry(entry: &TraceabilityEntry) -> Result<Vec<String>> {
    let mut refs = OrderedSet::new();
    for evidence in entry.positive.iter().chain(entry.negative.iter()) {
        for artifact_ref in &evidence.artifact_refs {
            validate_ref(artifact_ref, "readback artifact")?;
            refs.insert(artifact_ref.clone());
        }
    }
    Ok(refs.into_iter().collect())
}

fn extract_requirement_ids(markdown: &str) -> Result<Vec<String>> {
    let mut ids = OrderedSet::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("r[") {
        let after_marker = &rest[start + "r[".len()..];
        let Some(end) = after_marker.find(']') else {
            return Err(MoltenError::invalid_harness("unterminated requirement marker r[...]"));
        };
        let id = &after_marker[..end];
        validate_requirement_id(id)?;
        ids.insert(id.to_string());
        rest = &after_marker[end + "]".len()..];
    }
    Ok(ids.into_iter().collect())
}

fn validate_requirement(requirement: &RequirementInput) -> Result<()> {
    validate_requirement_id(&requirement.id)?;
    validate_text("requirement source", &requirement.source)?;
    validate_kind(&requirement.kind)
}

fn validate_requirement_id(id: &str) -> Result<()> {
    validate_text("requirement id", id)?;
    if id.chars().any(char::is_whitespace) {
        return Err(MoltenError::invalid_harness(format!(
            "traceability requirement id {id} must not contain whitespace"
        )));
    }
    Ok(())
}

fn validate_kind(kind: &str) -> Result<()> {
    match kind {
        "evidence" | "documentation" | "operator" | "other" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported traceability requirement kind {other}"))),
    }
}

fn validate_coverage_kind(kind: &str) -> Result<()> {
    match kind {
        "positive" | "negative" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("coverage kind {other} must be positive or negative"))),
    }
}

fn expected_decision(kind: &str) -> Result<&'static str> {
    match kind {
        "positive" => Ok("pass"),
        "negative" => Ok("deny"),
        other => Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported traceability decision {other}; expected pass or deny"
        ))),
    }
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("traceability {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_ref_list(label: &str, values: &[String], maximum: usize) -> Result<()> {
    ensure_count_at_most(values.len(), maximum, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_string_list(label: &str, values: &[String], maximum: usize) -> Result<()> {
    ensure_count_at_most(values.len(), maximum, label)?;
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn validate_obligation_class(class: &str) -> Result<()> {
    match class {
        "input-validation"
        | "canonicalization"
        | "admission"
        | "mutation-boundary"
        | "replay-determinism"
        | "fail-closed-negative" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported proof obligation class {other}"))),
    }
}

fn validate_layer_role(role: &str) -> Result<()> {
    match role {
        "pure-core" | "gate" | "replay" | "release" | "operator-readback" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported proof layer role {other}"))),
    }
}

fn validate_deny_class(class: &str) -> Result<()> {
    if required_deny_classes().contains(&class) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported deny-path class {class}")))
    }
}

