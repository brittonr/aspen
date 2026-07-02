
fn require_profiled_checks(checks: &[String], profile: ReproExportProfile) -> Result<()> {
    for expected in [
        "profile-schema",
        "redaction-transform-receipt",
        "transform-manifest-bound",
        "source-report-ref-binding",
        "output-report-ref-binding",
        "no-forbidden-cleartext",
    ] {
        require_seal_check(checks, expected)?;
    }
    match profile {
        ReproExportProfile::DenySensitive => require_seal_check(checks, "gate-preserving")?,
        ReproExportProfile::RedactedDiagnostic => require_seal_check(checks, "diagnostic-only")?,
        ReproExportProfile::EncryptedPrivate => {
            require_seal_check(checks, "requires-reveal")?;
            require_seal_check(checks, "encrypted-ref-validation")?;
        }
    }
    Ok(())
}

fn profiled_report_bundle(
    bundle_value: &IoValue,
    body: ProfiledBody,
    evidence: ProfiledEvidence,
    private: ProfiledPrivate,
) -> Result<ReproBundle> {
    let transform_receipt = evidence.transform_receipt;
    Ok(ReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: ReproBundleKind::Report,
        artifact_ref: body.output_report_ref,
        report_value: Some(body.report_value),
        failure_value: None,
        gate_receipt_ref: None,
        receipt_value: None,
        redaction_policy_ref: Some(evidence.policy_ref),
        redaction_gate_ref: None,
        export_profile: Some(body.export_profile.profile.as_str().to_string()),
        export_profile_ref: Some(body.export_profile.profile_ref),
        export_profile_value: Some(body.export_profile_value),
        source_report_ref: Some(body.source_report_ref),
        source_suite_ref: Some(body.source_suite_ref),
        redaction_transform_manifest_ref: Some(evidence.manifest_ref),
        redaction_transform_manifest_value: Some(evidence.manifest_value),
        redaction_transform_receipt_ref: Some(transform_receipt.receipt_ref.clone()),
        redaction_transform_receipt_value: Some(transform_receipt.value.clone()),
        private_bundle_profile_ref: private.private_bundle_profile_ref,
        private_bundle_profile_value: private.private_bundle_profile_value,
        loss_classification: Some(transform_receipt.loss_classification.clone()),
        encrypted_refs: transform_receipt.encrypted_refs.clone(),
    })
}

fn validate_redaction_transform_manifest(
    value: &IoValue,
    source_report_ref: &str,
    source_suite_ref: &str,
    report: &Report,
    profile: ReproExportProfile,
) -> Result<()> {
    let manifest = simple_record(value, "redaction-transform-manifest-v1", 9)?;
    let schema = required_string(&manifest[0], "redaction transform manifest schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction transform manifest schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA
        )));
    }
    let manifest_source_report = required_record_hash(&manifest[1], "source-report", "manifest source report")?;
    let manifest_source_suite = required_record_hash(&manifest[2], "source-suite", "manifest source suite")?;
    let manifest_output_report = required_record_hash(&manifest[3], "output-report", "manifest output report")?;
    let manifest_output_suite = required_record_hash(&manifest[4], "output-suite", "manifest output suite")?;
    let manifest_profile = required_record_string(&manifest[5], "profile", "manifest profile")?;
    if manifest_source_report != source_report_ref
        || manifest_source_suite != source_suite_ref
        || manifest_output_report != report.report_ref
        || manifest_output_suite != report.suite_ref
        || manifest_profile != profile.as_str()
    {
        return Err(MoltenError::invalid_harness("redaction transform manifest binding mismatch"));
    }
    validate_sequence_record(&manifest[6], "markers", "redaction transform manifest markers")?;
    let manifest_encrypted_refs =
        required_record_hash_sequence(&manifest[7], "encrypted-refs", "manifest encrypted refs")?;
    if profile == ReproExportProfile::EncryptedPrivate && manifest_encrypted_refs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "encrypted-private repro bundle transform manifest missing encrypted refs",
        ));
    }
    let checks = parse_redaction_gate_checks(&manifest[8])?;
    require_redaction_check(&checks, "source-report-bound")?;
    require_redaction_check(&checks, "output-report-bound")?;
    require_redaction_check(&checks, "deterministic-traversal-order")?;
    require_redaction_check(&checks, "marker-coverage-manifest")?;
    require_redaction_check(&checks, "encrypted-ref-inventory")?;
    Ok(())
}

fn collect_redaction_marker_refs(value: &IoValue) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(8);
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.collect_simple_record("redaction-marker-v1", None).is_some() {
            ensure_redaction_bound(refs.len() + 1, MAX_REDACTION_MARKER_REFS, "redaction marker refs")?;
            refs.push(crate::secrets::parse_redaction_marker(&current)?.marker_ref);
            continue;
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record)
            | ValueClass::Compound(CompoundClass::Sequence)
            | ValueClass::Compound(CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(value_to_iovalue(&child));
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(value_to_iovalue(&key));
                    stack.push(value_to_iovalue(&value));
                }
            }
        }
    }
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn parse_failure_repro_bundle(bundle_value: &IoValue, bundle: &Record<Value<IoValue>>) -> Result<ReproBundle> {
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "failure" {
        return Err(MoltenError::invalid_harness(format!("expected failure repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let failure_ref = required_string(&bundle[6], "repro bundle failure ref")?;
    require_artifact_ref(&artifact_refs, "failure", &failure_ref)?;
    let failure_value = value_to_iovalue(&bundle[7]);
    let failure = parse_failure(&failure_value)?;
    if failure.failure_ref != failure_ref {
        return Err(MoltenError::invalid_harness(format!(
            "failure repro bundle ref mismatch: bundle has {failure_ref}, embedded failure hashes to {}",
            failure.failure_ref
        )));
    }
    Ok(ReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: ReproBundleKind::Failure,
        artifact_ref: failure_ref,
        report_value: None,
        failure_value: Some(failure_value),
        gate_receipt_ref: None,
        receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: Some("diagnostic-only".to_string()),
        encrypted_refs: Vec::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReproSeal {
    gate_receipt_ref: String,
}

fn parse_repro_seal(
    value: &Value<IoValue>,
    report_ref: &str,
    suite_ref: &str,
    profile: &str,
    replay_status: &str,
) -> Result<ReproSeal> {
    let value = value_to_iovalue(value);
    let seal = simple_record(&value, "repro-seal", 7)?;
    let schema = required_string(&seal[0], "repro seal schema")?;
    if schema != crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro seal schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA
        )));
    }
    let decision = required_record_string(&seal[1], "decision", "repro seal decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported repro seal decision {decision}")));
    }
    let gate_receipt_ref = required_record_hash(&seal[2], "gate-receipt-ref", "repro seal gate receipt ref")?;
    let sealed_report_ref = required_record_hash(&seal[3], "report-ref", "repro seal report ref")?;
    if sealed_report_ref != report_ref {
        return Err(MoltenError::invalid_harness("repro seal report ref does not match bundle report ref"));
    }
    let sealed_suite_ref = required_record_hash(&seal[4], "suite-ref", "repro seal suite ref")?;
    if sealed_suite_ref != suite_ref {
        return Err(MoltenError::invalid_harness("repro seal suite ref does not match bundle suite ref"));
    }
    let sealed_profile = required_record_string(&seal[5], "profile", "repro seal profile")?;
    if sealed_profile != profile {
        return Err(MoltenError::invalid_harness("repro seal profile does not match bundle profile"));
    }
    let sealed_replay_status = required_record_string(&seal[6], "replay-status", "repro seal replay status")?;
    if sealed_replay_status != replay_status {
        return Err(MoltenError::invalid_harness("repro seal replay status does not match bundle replay status"));
    }
    Ok(ReproSeal { gate_receipt_ref })
}

fn parse_seal_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "seal-checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "repro seal checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "repro seal check name")?;
        let status = required_string(&check[1], "repro seal check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("repro seal check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_seal_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("repro seal missing {expected} check")))
    }
}

struct ReproReportMatchInput<'a> {
    report: &'a Report,
    report_ref: &'a str,
    suite_ref: &'a str,
    initial_state_hash: &'a str,
    final_state_hash: &'a str,
    replay_status: &'a str,
    profile: &'a str,
    actors: &'a [ActorDecl],
    effect_log: &'a [EffectLogEntry],
    suite_value: &'a IoValue,
}

fn require_report_artifact_refs(refs: &[(String, String)], report: &Report) -> Result<()> {
    for (kind, artifact_ref) in report_artifact_refs(report, None, None)? {
        require_artifact_ref(refs, &kind, &artifact_ref)?;
    }
    Ok(())
}

fn require_repro_report_matches(input: &ReproReportMatchInput<'_>) -> Result<()> {
    if input.report.report_ref != input.report_ref {
        return Err(MoltenError::invalid_harness(format!(
            "repro bundle report ref mismatch: bundle has {}, embedded report hashes to {}",
            input.report_ref, input.report.report_ref
        )));
    }
    if input.report.suite_ref != input.suite_ref {
        return Err(MoltenError::invalid_harness("repro bundle suite ref does not match embedded report"));
    }
    if input.report.initial_state_hash != input.initial_state_hash
        || input.report.final_state_hash != input.final_state_hash
    {
        return Err(MoltenError::invalid_harness("repro bundle state refs do not match embedded report"));
    }
    if input.report.replay_status != input.replay_status || input.report.profile != input.profile {
        return Err(MoltenError::invalid_harness(
            "repro bundle replay/profile metadata does not match embedded report",
        ));
    }
    if input.report.actors != input.actors {
        return Err(MoltenError::invalid_harness("repro bundle actor registry does not match embedded report"));
    }
    if input.report.effect_log != input.effect_log {
        return Err(MoltenError::invalid_harness("repro bundle effect log does not match embedded report"));
    }
    if &input.report.suite_value != input.suite_value {
        return Err(MoltenError::invalid_harness("repro bundle suite value does not match embedded report"));
    }
    Ok(())
}
