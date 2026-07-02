
pub fn validate_octet_source_gate(input: &OctetSourceGateValidationInput) -> Result<OctetSourceGateValidation> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let setup = prepare_source_validation(input, &mut checks, &mut diagnostics)?;
    let requirement_value = octet_source_gate_requirement_value(
        &input.consumer,
        &input.subject_ref,
        &setup.source_scope,
        setup.expected.as_ref(),
        &checks,
    );
    let requirement_ref = canonical_hash(&requirement_value)?;
    let parsed = parse_source_receipt(input.receipt_value.as_ref(), &mut checks, &mut diagnostics);
    let validation_refs = validate_source_receipt(
        ReceiptCheckInput {
            parsed: parsed.as_ref(),
            expected: setup.expected.as_ref(),
        },
        &mut checks,
        &mut diagnostics,
    );
    let gate_receipt_ref = validation_refs.receipt_ref.clone();
    let decision = if checks.iter().all(|check| check.status == "pass") {
        "pass"
    } else {
        "deny"
    };
    let value = octet_source_gate_validation_value(OctetSourceGateValidationValueInput {
        decision,
        requirement_ref: &requirement_ref,
        gate_receipt_ref: gate_receipt_ref.as_deref(),
        policy_ref: validation_refs.policy_ref.as_deref(),
        status_ref: validation_refs.status_ref.as_deref(),
        summary_ref: validation_refs.summary_ref.as_deref(),
        findings_ref: validation_refs.findings_ref.as_deref(),
        object_corpus_ref: validation_refs.object_corpus_ref.as_deref(),
        fingerprint_ref: validation_refs.fingerprint_ref.as_deref(),
        counts: &validation_refs.counts,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let validation_ref = canonical_hash(&value)?;
    Ok(OctetSourceGateValidation {
        decision: decision.to_string(),
        requirement_ref,
        validation_ref,
        gate_receipt_ref,
        value,
        diagnostics,
    })
}

fn normalized_source_scope(input: &OctetSourceGateValidationInput) -> Result<Vec<String>> {
    if input.source_scope.is_empty() {
        return default_source_scope(&input.consumer);
    }
    let mut scope = input.source_scope.clone();
    scope.sort();
    scope.dedup();
    Ok(scope)
}

fn prepare_source_validation(
    input: &OctetSourceGateValidationInput,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<SourceSetup> {
    let source_scope = normalized_source_scope(input)?;
    let is_consumer_supported = SOURCE_GATE_CONSUMERS.iter().any(|consumer| consumer == &input.consumer.as_str());
    push_check(checks, "source-gate-consumer-supported", is_consumer_supported);
    if !is_consumer_supported {
        push_diagnostic(diagnostics, format!("unsupported octet source-gate consumer {}", input.consumer));
    }
    let is_subject_ref_valid = is_content_ref(&input.subject_ref);
    push_check(checks, "source-gate-subject-ref", is_subject_ref_valid);
    if !is_subject_ref_valid {
        push_diagnostic(diagnostics, format!("invalid octet source-gate subject ref {}", input.subject_ref));
    }
    let expected = expected_metadata_for_command(DEFAULT_GATE_COMMAND).ok();
    push_check(checks, "current-octet-metadata", expected.is_some());
    if expected.is_none() {
        push_diagnostic(diagnostics, "cannot derive current Octet workspace metadata".to_string());
    }
    Ok(SourceSetup { source_scope, expected })
}

fn parse_source_receipt(
    value: Option<&IoValue>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<ParsedOctetGateReceipt> {
    match value {
        Some(value) => match parse_octet_gate_receipt(value) {
            Ok(parsed) => {
                push_check(checks, "gate-receipt-present", true);
                push_check(checks, "gate-receipt-parse", true);
                Some(parsed)
            }
            Err(error) => {
                push_check(checks, "gate-receipt-parse", false);
                push_diagnostic(diagnostics, format!("invalid octet gate receipt: {error}"));
                None
            }
        },
        None => {
            push_check(checks, "gate-receipt-present", false);
            push_diagnostic(diagnostics, "missing octet gate receipt value".to_string());
            None
        }
    }
}

fn validate_source_receipt(
    input: ReceiptCheckInput<'_>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> ValidationRefs {
    let Some(parsed) = input.parsed else {
        push_missing_receipt_checks(checks);
        return ValidationRefs::default();
    };
    let refs = receipt_refs(parsed);
    check_receipt_basics(parsed, checks, diagnostics);
    check_receipt_freshness(parsed, input.expected, checks, diagnostics);
    refs
}

fn receipt_refs(parsed: &ParsedOctetGateReceipt) -> ValidationRefs {
    ValidationRefs {
        counts: parsed.counts.clone(),
        receipt_ref: Some(parsed.receipt_ref.clone()),
        policy_ref: Some(parsed.policy_ref.clone()),
        status_ref: parsed.status_ref.clone(),
        summary_ref: parsed.summary_ref.clone(),
        findings_ref: parsed.findings_ref.clone(),
        object_corpus_ref: parsed.object_corpus_ref.clone(),
        fingerprint_ref: parsed.fingerprint_ref.clone(),
    }
}

fn check_receipt_basics(
    parsed: &ParsedOctetGateReceipt,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let is_receipt_pass = parsed.decision == "pass";
    push_check(checks, "gate-receipt-pass", is_receipt_pass);
    if !is_receipt_pass {
        push_diagnostic(diagnostics, format!("octet gate receipt decision is {}", parsed.decision));
    }
    let has_strict_profile_checks = parsed_check_pass(parsed, "profile-supported")
        && parsed_check_pass(parsed, "strict-status-clean")
        && parsed_check_pass(parsed, "no-critical-findings");
    push_check(checks, "strict-profile-required", has_strict_profile_checks);
    if !has_strict_profile_checks {
        push_diagnostic(diagnostics, "octet gate receipt is not strict clean source-gate pass evidence".to_string());
    }
    let has_required_artifact_refs = parsed.command_ref.is_some()
        && parsed.status_ref.is_some()
        && parsed.summary_ref.is_some()
        && parsed.findings_ref.is_some()
        && parsed.object_corpus_ref.is_some()
        && parsed.fingerprint_ref.is_some();
    push_check(checks, "required-artifact-refs", has_required_artifact_refs);
    if !has_required_artifact_refs {
        push_diagnostic(diagnostics, "octet gate receipt is missing required artifact refs".to_string());
    }
    let has_clean_finding_counts = parsed.counts.total == 0
        && parsed.counts.warnings == 0
        && parsed.counts.errors == 0
        && parsed.counts.critical == 0
        && parsed.counts.uncovered == 0;
    push_check(checks, "no-uncovered-findings", has_clean_finding_counts);
    if !has_clean_finding_counts {
        push_diagnostic(
            diagnostics,
            format!(
                "octet gate receipt has findings={} warnings={} errors={} critical={} uncovered={}",
                parsed.counts.total,
                parsed.counts.warnings,
                parsed.counts.errors,
                parsed.counts.critical,
                parsed.counts.uncovered
            ),
        );
    }
}

fn check_receipt_freshness(
    parsed: &ParsedOctetGateReceipt,
    expected: Option<&ExpectedMetadata>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let is_config_hash_current = expected
        .zip(parsed.config_hash.as_ref())
        .is_some_and(|(expected, actual)| expected.config_hash == *actual);
    let is_profile_hash_current = expected
        .zip(parsed.profile_hash.as_ref())
        .is_some_and(|(expected, actual)| expected.profile_hash == *actual);
    push_check(checks, "current-config-ref", is_config_hash_current);
    push_check(checks, "current-profile-ref", is_profile_hash_current);
    if !is_config_hash_current {
        push_diagnostic(diagnostics, "octet gate receipt config hash is stale or missing".to_string());
    }
    if !is_profile_hash_current {
        push_diagnostic(diagnostics, "octet gate receipt profile hash is stale or missing".to_string());
    }
    let has_scope_fingerprint_coverage = parsed.fingerprint_ref.as_deref().is_some_and(is_content_ref)
        && parsed.object_corpus_ref.as_deref().is_some_and(is_content_ref)
        && parsed_check_pass(parsed, "fingerprint-evidence-bound")
        && parsed_check_pass(parsed, "object-corpus-critical-paths")
        && parsed_check_pass(parsed, "object-corpus-fingerprint");
    push_check(checks, "scope-fingerprint-coverage", has_scope_fingerprint_coverage);
    if !has_scope_fingerprint_coverage {
        push_diagnostic(
            diagnostics,
            "octet gate receipt lacks object-corpus/fingerprint coverage for required source scope".to_string(),
        );
    }
    let has_toolchain = parsed.toolchain.is_some();
    push_check(checks, "toolchain-bound", has_toolchain);
    if !has_toolchain {
        push_diagnostic(diagnostics, "octet gate receipt missing toolchain metadata".to_string());
    }
}

fn push_missing_receipt_checks(checks: &mut impl crate::bounded::VecSink<Check>) {
    push_check(checks, "gate-receipt-pass", false);
    push_check(checks, "strict-profile-required", false);
    push_check(checks, "required-artifact-refs", false);
    push_check(checks, "no-uncovered-findings", false);
    push_check(checks, "current-config-ref", false);
    push_check(checks, "current-profile-ref", false);
    push_check(checks, "scope-fingerprint-coverage", false);
    push_check(checks, "toolchain-bound", false);
}

fn octet_source_gate_requirement_value(
    consumer: &str,
    subject_ref: &str,
    source_scope: &[String],
    expected: Option<&ExpectedMetadata>,
    checks: &[Check],
) -> IoValue {
    record("octet-source-gate-requirement-v1", vec![
        string(crate::preserves_rail::OCTET_SOURCE_GATE_REQUIREMENT_SCHEMA),
        record("consumer", vec![string(consumer)]),
        record("subject", vec![string(subject_ref)]),
        record("required-profile", vec![string(STRICT_PROFILE)]),
        record("source-scope", vec![sequence(source_scope.iter().map(string).collect())]),
        record("current-config", vec![optional_ref(expected.map(|metadata| metadata.config_hash.as_str()))]),
        record("current-profile", vec![optional_ref(expected.map(|metadata| metadata.profile_hash.as_str()))]),
        record("required-evidence", vec![sequence(vec![
            string("status"),
            string("summary"),
            string("structured-findings"),
            string("object-corpus"),
            string("fingerprint"),
        ])]),
        record("freshness", vec![string("same-workspace-metadata")]),
        checks_value(checks),
    ])
}

pub fn default_source_scope(consumer: &str) -> Result<Vec<String>> {
    let scope = match consumer {
        "node-startup" => vec!["src/main.rs", "src/node/runtime.rs", "src/octet/gate.rs"],
        "job-remote-admission" => vec!["src/job/dag.rs", "src/main.rs", "src/octet/gate.rs"],
        "upgrade-plan" => vec!["src/main.rs", "src/octet/gate.rs", "src/upgrades/mod.rs"],
        "node-control-gate" => vec![
            "src/main.rs",
            "src/node/daemon.rs",
            "src/node/runtime.rs",
            "src/octet/gate.rs",
        ],
        other => return Err(MoltenError::invalid_harness(format!("unsupported octet source-gate consumer {other}"))),
    };
    Ok(scope.into_iter().map(ToOwned::to_owned).collect())
}
