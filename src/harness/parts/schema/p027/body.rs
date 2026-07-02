
fn parse_redaction_transform_receipt(value: &IoValue) -> Result<RedactionTransformReceiptEvidence> {
    let receipt = simple_record(value, "redaction-transform-receipt-v1", 12)?;
    let schema = required_string(&receipt[0], "redaction transform schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction transform schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "redaction transform decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported redaction transform decision {decision}")));
    }
    let source_report_ref = required_record_hash(&receipt[2], "source-report", "redaction source report")?;
    let source_suite_ref = required_record_hash(&receipt[3], "source-suite", "redaction source suite")?;
    let policy_ref = required_record_hash(&receipt[4], "policy", "redaction policy")?;
    let profile_name = required_record_string(&receipt[5], "profile", "redaction profile")?;
    let profile = ReproExportProfile::parse(&profile_name)?;
    let manifest_ref = required_record_hash(&receipt[6], "transform-manifest", "redaction transform manifest")?;
    let output_bundle_ref = required_record_hash(&receipt[7], "output-bundle", "redaction output bundle")?;
    let loss_classification =
        required_record_string(&receipt[8], "loss-classification", "redaction loss classification")?;
    if loss_classification != profile.loss_classification() {
        return Err(MoltenError::invalid_harness("redaction transform loss classification is not canonical"));
    }
    let marker_refs = required_record_hash_sequence(&receipt[9], "markers", "redaction marker refs")?;
    let encrypted_refs = required_record_hash_sequence(&receipt[10], "encrypted-refs", "redaction encrypted refs")?;
    let checks = parse_redaction_gate_checks(&receipt[11])?;
    for check in redaction_transform_check_names(profile) {
        require_redaction_check(&checks, check)?;
    }
    Ok(RedactionTransformReceiptEvidence {
        receipt_ref: canonical_hash(value)?,
        source_report_ref,
        source_suite_ref,
        policy_ref,
        profile,
        manifest_ref,
        output_bundle_ref,
        loss_classification,
        marker_refs,
        encrypted_refs,
        value: value.clone(),
    })
}

fn redaction_gate_value(report_value: &IoValue, report: &Report) -> Result<IoValue> {
    if let Some(marker) = first_sensitive_marker(report_value) {
        return Err(MoltenError::invalid_harness(format!(
            "redaction preflight found sensitive marker {marker}; sealed pass repro bundles require explicit redaction before export"
        )));
    }
    let policy = redaction_policy_value();
    let policy_ref = canonical_hash(&policy)?;
    Ok(record("redaction-gate-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("policy-ref", vec![string(policy_ref)]),
        record("report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        record("scan-root-ref", vec![string(canonical_hash(report_value)?)]),
        redaction_gate_checks_value(),
    ]))
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    match reference {
        Some(reference) => record("some", vec![string(reference)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value_for_names(names: &[&str]) -> IoValue {
    record("checks", vec![sequence(
        names.iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

fn redaction_transform_check_names(profile: ReproExportProfile) -> Vec<&'static str> {
    let mut checks = vec![
        "source-report-ref-bound",
        "source-suite-ref-bound",
        "policy-ref-bound",
        "profile-ref-bound",
        "transform-manifest-bound",
        "output-bundle-ref-bound",
        "marker-coverage",
        "deterministic-traversal-order",
        "forbidden-cleartext-absent",
    ];
    match profile {
        ReproExportProfile::DenySensitive => checks.push("gate-preserving"),
        ReproExportProfile::RedactedDiagnostic => checks.push("diagnostic-only"),
        ReproExportProfile::EncryptedPrivate => {
            checks.push("requires-reveal");
            checks.push("encrypted-ref-validation");
        }
    }
    checks
}

fn profiled_repro_checks_value(profile: ReproExportProfile) -> IoValue {
    let mut checks = vec![
        "profile-schema",
        "redaction-transform-receipt",
        "transform-manifest-bound",
        "source-report-ref-binding",
        "output-report-ref-binding",
        "no-forbidden-cleartext",
    ];
    match profile {
        ReproExportProfile::DenySensitive => checks.push("gate-preserving"),
        ReproExportProfile::RedactedDiagnostic => checks.push("diagnostic-only"),
        ReproExportProfile::EncryptedPrivate => {
            checks.push("requires-reveal");
            checks.push("encrypted-ref-validation");
        }
    }
    record("seal-checks", vec![sequence(
        checks.as_slice().iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

fn redaction_gate_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "redaction-policy",
            "canonical-report-scan",
            "no-secret-markers",
            "no-confidential-markers",
            "no-credential-markers",
            "no-private-markers",
            "no-unvalidated-encrypted-refs",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn validate_redaction_evidence(
    report_value: &IoValue,
    report: &Report,
    policy_value: &IoValue,
    gate_value: &IoValue,
) -> Result<(String, String)> {
    let expected_policy = redaction_policy_value();
    let expected_policy_ref = canonical_hash(&expected_policy)?;
    let actual_policy_ref = canonical_hash(policy_value)?;
    if actual_policy_ref != expected_policy_ref || policy_value != &expected_policy {
        return Err(MoltenError::invalid_harness(format!(
            "redaction policy evidence mismatch: policy hashes to {actual_policy_ref}, expected {expected_policy_ref}"
        )));
    }
    parse_redaction_policy(policy_value)?;
    let expected_gate = redaction_gate_value(report_value, report)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(gate_value)?;
    if actual_gate_ref != expected_gate_ref || gate_value != &expected_gate {
        return Err(MoltenError::invalid_harness(format!(
            "redaction gate evidence mismatch: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    parse_redaction_gate(gate_value, report, &expected_policy_ref, report_value)?;
    Ok((actual_policy_ref, actual_gate_ref))
}

fn parse_redaction_policy(value: &IoValue) -> Result<()> {
    let policy = simple_record(value, "redaction-policy-v1", 3)?;
    let schema = required_string(&policy[0], "redaction policy schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction policy schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA
        )));
    }
    let mode = required_record_string(&policy[1], "mode", "redaction policy mode")?;
    if mode != "deny-sensitive-markers" {
        return Err(MoltenError::invalid_harness(format!("unsupported redaction policy mode {mode}")));
    }
    let markers = required_record_sequence(&policy[2], "forbidden-markers", "redaction forbidden markers")?;
    let actual = markers
        .iter()
        .map(|marker| required_string(&marker, "redaction marker"))
        .collect::<Result<Vec<_>>>()?;
    if actual != FORBIDDEN_REDACTION_MARKERS {
        return Err(MoltenError::invalid_harness("redaction policy forbidden marker set is not canonical"));
    }
    Ok(())
}

fn parse_redaction_gate(value: &IoValue, report: &Report, policy_ref: &str, report_value: &IoValue) -> Result<()> {
    let gate = simple_record(value, "redaction-gate-v1", 7)?;
    let schema = required_string(&gate[0], "redaction gate schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "redaction gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported redaction gate decision {decision}")));
    }
    let actual_policy_ref = required_record_hash(&gate[2], "policy-ref", "redaction policy ref")?;
    if actual_policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("redaction gate policy ref does not match policy evidence"));
    }
    let report_ref = required_record_hash(&gate[3], "report-ref", "redaction gate report ref")?;
    if report_ref != report.report_ref {
        return Err(MoltenError::invalid_harness("redaction gate report ref does not match embedded report"));
    }
    let suite_ref = required_record_hash(&gate[4], "suite-ref", "redaction gate suite ref")?;
    if suite_ref != report.suite_ref {
        return Err(MoltenError::invalid_harness("redaction gate suite ref does not match embedded report"));
    }
    let scan_root_ref = required_record_hash(&gate[5], "scan-root-ref", "redaction gate scan root ref")?;
    let actual_scan_root_ref = canonical_hash(report_value)?;
    if scan_root_ref != actual_scan_root_ref {
        return Err(MoltenError::invalid_harness("redaction gate scan root ref does not match embedded report"));
    }
    let checks = parse_redaction_gate_checks(&gate[6])?;
    require_redaction_check(&checks, "redaction-policy")?;
    require_redaction_check(&checks, "canonical-report-scan")?;
    require_redaction_check(&checks, "no-secret-markers")?;
    require_redaction_check(&checks, "no-confidential-markers")?;
    require_redaction_check(&checks, "no-credential-markers")?;
    require_redaction_check(&checks, "no-private-markers")?;
    require_redaction_check(&checks, "no-unvalidated-encrypted-refs")?;
    Ok(())
}

fn parse_redaction_gate_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "redaction gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "redaction gate check name")?;
        let status = required_string(&check[1], "redaction gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("redaction gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_redaction_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("redaction gate missing {expected} check")))
    }
}

fn is_sensitive_record_label(label: &str) -> bool {
    FORBIDDEN_REDACTION_MARKERS.iter().any(|marker| marker == &label)
}
