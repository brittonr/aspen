
pub fn verify_build(input: &BuildVerificationInput<'_>) -> Result<BuildVerification> {
    validate_ref(input.actual_artifact_ref, "provenance actual artifact ref")?;
    let build_record = parse_build_record(input.build_record_value)?;
    let mut diagnostics = Vec::with_capacity(input.prior_diagnostics.len().saturating_add(2));
    diagnostics.extend(input.prior_diagnostics.iter().cloned());
    if build_record.expected_artifact_ref != input.actual_artifact_ref {
        diagnostics.push(format!(
            "build artifact mismatch: expected {}, got {}",
            build_record.expected_artifact_ref, input.actual_artifact_ref
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = build_verify_receipt_value(&BuildVerifyReceiptValueInput {
        decision,
        expected_artifact_ref: &build_record.expected_artifact_ref,
        actual_artifact_ref: input.actual_artifact_ref,
        build_record_ref: &build_record.record_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(BuildVerification {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        build_record_ref: build_record.record_ref,
        expected_artifact_ref: build_record.expected_artifact_ref,
        actual_artifact_ref: input.actual_artifact_ref.to_string(),
        diagnostics,
    })
}

pub fn evaluate(input: &EvaluationInput<'_>) -> Result<Evaluation> {
    validate_evaluation_input(input)?;
    let mut diagnostics = Vec::with_capacity(evaluation_diagnostic_capacity(input));
    diagnostics.extend(input.prior_diagnostics.iter().cloned());

    let build_checks = parse_build_checks(input.build_verification_values);
    diagnostics.extend(build_checks.diagnostics);

    let record_match = find_matching_record(input.provenance_values, input.artifact_ref);
    diagnostics.extend(record_match.diagnostics);
    let matched = record_match.record;
    let matched_record_ref = matched.as_ref().map(|record| record.record_ref.clone());
    let (trust_state, trust_admission) = trust_status(matched.as_ref(), input.profile);

    if trust_admission == TrustAdmission::Denied {
        diagnostics.push(format!("provenance trust state {trust_state} is not admitted for profile {}", input.profile));
    }
    if let Some(record) = matched.as_ref() {
        diagnostics.extend(stronger_diagnostics(record, input.operation, input.profile));
    }
    if let Some(record) = matched.as_ref().filter(|record| record.trust_state == TRUST_STATE_REPRODUCIBLE_VERIFIED) {
        diagnostics.extend(reproducible_build_binding_diagnostics(record, input.artifact_ref, &build_checks.receipts));
    }

    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = receipt_value(&ReceiptValueInput {
        decision,
        operation: input.operation,
        profile: input.profile,
        artifact_ref: input.artifact_ref,
        trust_state,
        record_ref: matched_record_ref.as_deref(),
        build_verification_refs: &build_checks.refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(Evaluation {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        matched_record_ref,
        diagnostics,
    })
}

fn validate_evaluation_input(input: &EvaluationInput<'_>) -> Result<()> {
    validate_ref(input.artifact_ref, "provenance evaluation artifact ref")?;
    validate_profile(input.profile)?;
    ensure_ref_bound(input.provenance_values.len(), MAX_PROVENANCE_REFS, "provenance values")?;
    ensure_ref_bound(
        input.build_verification_values.len(),
        MAX_PROVENANCE_REFS,
        "provenance build verification values",
    )?;
    Ok(())
}

fn evaluation_diagnostic_capacity(input: &EvaluationInput<'_>) -> usize {
    input
        .prior_diagnostics
        .len()
        .saturating_add(input.provenance_values.len())
        .saturating_add(input.build_verification_values.len())
        .saturating_add(4)
}

fn parse_build_checks(values: &[IoValue]) -> BuildChecks {
    let mut receipts = Vec::with_capacity(values.len());
    let mut refs = Vec::with_capacity(values.len());
    let mut diagnostics = Vec::with_capacity(values.len());
    for value in values {
        match parse_build_verification_receipt(value) {
            Ok(receipt) => {
                refs.push(receipt.receipt_ref.clone());
                receipts.push(receipt);
            }
            Err(error) => diagnostics.push(format!("malformed provenance build verification receipt: {error}")),
        }
    }
    BuildChecks {
        receipts,
        refs,
        diagnostics,
    }
}

fn find_matching_record(values: &[IoValue], artifact_ref: &str) -> RecordMatch {
    let mut diagnostics = Vec::with_capacity(values.len().saturating_add(1));
    let mut matched: Option<Record> = None;
    let mut has_mismatched_record = false;
    for value in values {
        let record = match parse_record(value) {
            Ok(record) => record,
            Err(error) => {
                diagnostics.push(format!("malformed provenance record: {error}"));
                continue;
            }
        };
        if record.artifact_ref == artifact_ref {
            matched = Some(record);
            break;
        }
        has_mismatched_record = true;
    }
    if values.is_empty() {
        diagnostics.push(format!("missing provenance evidence for {artifact_ref}"));
    } else if matched.is_none() && has_mismatched_record {
        diagnostics.push(format!("no provenance record matches artifact {artifact_ref}"));
    }
    RecordMatch {
        record: matched,
        diagnostics,
    }
}

fn trust_status<'a>(record: Option<&'a Record>, profile: &str) -> (&'a str, TrustAdmission) {
    let trust_state = record.map(|record| record.trust_state.as_str()).unwrap_or(TRUST_STATE_UNKNOWN);
    let has_admitted_trust_state = is_trust_state_admitted(trust_state, profile);
    let admission = if record.is_some() && has_admitted_trust_state {
        TrustAdmission::Admitted
    } else if record.is_some() {
        TrustAdmission::Denied
    } else {
        TrustAdmission::Missing
    };
    (trust_state, admission)
}

pub fn parse_record(value: &IoValue) -> Result<Record> {
    if let Some(fields) = value.collect_simple_record("provenance-record-v1", Some(12)) {
        require_schema(&fields[0], crate::preserves_rail::PROVENANCE_RECORD_SCHEMA, "provenance record")?;
        let trust_state = record_string(&fields[2], "trust-state")?;
        validate_trust_state(&trust_state)?;
        return Ok(Record {
            record_ref: canonical_hash(value)?,
            artifact_ref: record_ref(&fields[1], "artifact")?,
            trust_state,
            source_refs: record_ref_sequence(&fields[3], "source")?,
            dependency_closure_ref: record_ref(&fields[4], "dependency-closure")?,
            toolchain_refs: record_ref_sequence(&fields[5], "toolchain")?,
            builder_ref: record_ref(&fields[6], "builder")?,
            review_refs: record_ref_sequence(&fields[7], "review")?,
            test_refs: record_ref_sequence(&fields[8], "tests")?,
            source_gate_refs: record_ref_sequence(&fields[9], "source-gates")?,
            policy_refs: record_ref_sequence(&fields[10], "policy")?,
            build_record_refs: record_ref_sequence(&fields[11], "build-records")?,
            value: value.clone(),
        });
    }
    let fields = value
        .collect_simple_record("provenance-record-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <provenance-record-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::PROVENANCE_RECORD_SCHEMA, "provenance record")?;
    let trust_state = record_string(&fields[2], "trust-state")?;
    validate_trust_state(&trust_state)?;
    Ok(Record {
        record_ref: canonical_hash(value)?,
        artifact_ref: record_ref(&fields[1], "artifact")?,
        trust_state,
        source_refs: record_ref_sequence(&fields[3], "source")?,
        dependency_closure_ref: record_ref(&fields[4], "dependency-closure")?,
        toolchain_refs: record_ref_sequence(&fields[5], "toolchain")?,
        builder_ref: record_ref(&fields[6], "builder")?,
        review_refs: record_ref_sequence(&fields[7], "review")?,
        test_refs: record_ref_sequence(&fields[8], "tests")?,
        source_gate_refs: record_ref_sequence(&fields[9], "source-gates")?,
        policy_refs: record_ref_sequence(&fields[10], "policy")?,
        build_record_refs: Vec::new(),
        value: value.clone(),
    })
}

pub fn parse_build_record(value: &IoValue) -> Result<BuildRecord> {
    let fields = value
        .collect_simple_record("provenance-build-record-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <provenance-build-record-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::PROVENANCE_BUILD_RECORD_SCHEMA, "provenance build record")?;
    let build_params = record_build_params_sequence(&fields[5], "build-params")?;
    Ok(BuildRecord {
        record_ref: canonical_hash(value)?,
        expected_artifact_ref: record_ref(&fields[1], "expected-artifact")?,
        source_refs: record_ref_sequence(&fields[2], "source")?,
        dependency_closure_ref: record_ref(&fields[3], "dependency-closure")?,
        toolchain_refs: record_ref_sequence(&fields[4], "toolchain")?,
        build_params,
        builder_ref: record_ref(&fields[6], "builder")?,
        nix_derivation_refs: record_ref_sequence(&fields[7], "nix-derivations")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    })
}

pub fn parse_build_verification_receipt(value: &IoValue) -> Result<BuildVerificationReceipt> {
    let fields = value
        .collect_simple_record("provenance-build-verify-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <provenance-build-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA,
        "provenance build verification receipt",
    )?;
    let decision = record_string(&fields[1], "decision")?;
    if !matches!(decision.as_str(), "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!(
            "invalid provenance build verification decision `{decision}`"
        )));
    }
    Ok(BuildVerificationReceipt {
        decision,
        receipt_ref: canonical_hash(value)?,
        expected_artifact_ref: record_ref(&fields[2], "expected-artifact")?,
        actual_artifact_ref: record_ref(&fields[3], "actual-artifact")?,
        build_record_ref: record_ref(&fields[4], "build-record")?,
        diagnostics: record_string_sequence(&fields[5], "diagnostics")?,
        value: value.clone(),
    })
}
