
fn parse_sealed_redaction(
    bundle: &Record<Value<IoValue>>,
    body: &ReportBundleBody,
    arity: usize,
) -> Result<SealedRedaction> {
    if arity != 21 {
        return Ok(SealedRedaction {
            redaction_policy_ref: None,
            redaction_gate_ref: None,
            seal_index: 16,
            receipt_index: 17,
            checks_index: 18,
            has_redaction: false,
        });
    }
    let redaction_policy_value = value_to_iovalue(&bundle[16]);
    let redaction_gate_value = value_to_iovalue(&bundle[17]);
    let (redaction_policy_ref, redaction_gate_ref) =
        validate_redaction_evidence(&body.report_value, &body.report, &redaction_policy_value, &redaction_gate_value)?;
    require_artifact_ref(&body.artifact_refs, "redaction-policy", &redaction_policy_ref)?;
    require_artifact_ref(&body.artifact_refs, "redaction-gate", &redaction_gate_ref)?;
    Ok(SealedRedaction {
        redaction_policy_ref: Some(redaction_policy_ref),
        redaction_gate_ref: Some(redaction_gate_ref),
        seal_index: 18,
        receipt_index: 19,
        checks_index: 20,
        has_redaction: true,
    })
}

fn embedded_gate_receipt(bundle: &Record<Value<IoValue>>, index: usize, expected_ref: &str) -> Result<IoValue> {
    let receipt_value = value_to_iovalue(&bundle[index]);
    let actual_gate_receipt_ref = canonical_hash(&receipt_value)?;
    if actual_gate_receipt_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle gate receipt ref mismatch: seal has {expected_ref}, embedded receipt hashes to {actual_gate_receipt_ref}"
        )));
    }
    Ok(receipt_value)
}

fn require_report_seal_checks(checks: &[String], has_redaction: bool) -> Result<()> {
    for expected in [
        "sealed-report",
        "embedded-gate-receipt",
        "report-ref-binding",
        "suite-ref-binding",
        "actor-registry-binding",
        "effect-log-binding",
        "policy-gate-ref-binding",
        "capability-gate-ref-binding",
        "budget-gate-ref-binding",
    ] {
        require_seal_check(checks, expected)?;
    }
    if has_redaction {
        require_seal_check(checks, "redaction-preflight")?;
        require_seal_check(checks, "redaction-gate-ref-binding")?;
        require_seal_check(checks, "no-sensitive-markers")?;
    }
    require_seal_check(checks, "replay-metadata-binding")
}

fn sealed_report_bundle(
    bundle_value: &IoValue,
    body: ReportBundleBody,
    seal: ReproSeal,
    receipt_value: IoValue,
    redaction: SealedRedaction,
) -> Result<ReproBundle> {
    Ok(ReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: ReproBundleKind::Report,
        artifact_ref: body.report_ref,
        report_value: Some(body.report_value),
        failure_value: None,
        gate_receipt_ref: Some(seal.gate_receipt_ref),
        receipt_value: Some(receipt_value),
        redaction_policy_ref: redaction.redaction_policy_ref,
        redaction_gate_ref: redaction.redaction_gate_ref,
        export_profile: Some(ReproExportProfile::DenySensitive.as_str().to_string()),
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
        loss_classification: Some(ReproExportProfile::DenySensitive.loss_classification().to_string()),
        encrypted_refs: Vec::new(),
    })
}

struct ProfiledBody {
    artifact_refs: Vec<(String, String)>,
    source_report_ref: String,
    source_suite_ref: String,
    output_report_ref: String,
    export_profile_value: IoValue,
    export_profile: ReproExportProfileEvidence,
    report_value: IoValue,
    report: Report,
}

struct ProfiledEvidence {
    policy_ref: String,
    manifest_ref: String,
    manifest_value: IoValue,
    transform_receipt: RedactionTransformReceiptEvidence,
}

struct ProfiledPrivate {
    private_bundle_profile_ref: Option<String>,
    private_bundle_profile_value: Option<IoValue>,
    checks_index: usize,
}

fn parse_profiled_report_repro_bundle(bundle_value: &IoValue, bundle: &Record<Value<IoValue>>) -> Result<ReproBundle> {
    let arity = bundle.fields_iter().count();
    let body = parse_profiled_body(bundle)?;
    require_report_artifact_refs(&body.artifact_refs, &body.report)?;
    let evidence = parse_profiled_evidence(bundle, &body)?;
    let private = parse_profiled_private(bundle, &body, &evidence, arity)?;
    let checks = parse_seal_checks(&bundle[private.checks_index])?;
    require_profiled_checks(&checks, body.export_profile.profile)?;
    profiled_report_bundle(bundle_value, body, evidence, private)
}

fn parse_profiled_body(bundle: &Record<Value<IoValue>>) -> Result<ProfiledBody> {
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(MoltenError::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let source_report_ref = required_hash(&bundle[6], "profiled repro source report ref")?;
    let source_suite_ref = required_hash(&bundle[7], "profiled repro source suite ref")?;
    let output_report_ref = required_hash(&bundle[8], "profiled repro output report ref")?;
    let output_suite_ref = required_hash(&bundle[9], "profiled repro output suite ref")?;
    require_artifact_ref(&artifact_refs, "source-report", &source_report_ref)?;
    require_artifact_ref(&artifact_refs, "source-suite", &source_suite_ref)?;
    require_artifact_ref(&artifact_refs, "report", &output_report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &output_suite_ref)?;
    let initial_state_hash = required_hash(&bundle[10], "profiled repro initial state hash")?;
    let final_state_hash = required_hash(&bundle[11], "profiled repro final state hash")?;
    let replay_status = required_string(&bundle[12], "profiled repro replay status")?;
    let run_profile = required_string(&bundle[13], "profiled repro harness profile")?;
    let export_profile_value = value_to_iovalue(&bundle[14]);
    let export_profile = parse_repro_export_profile(&export_profile_value)?;
    require_artifact_ref(&artifact_refs, "export-profile", &export_profile.profile_ref)?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[15]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[16]))?;
    let suite_value = value_to_iovalue(&bundle[17]);
    let report_value = value_to_iovalue(&bundle[18]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &output_report_ref,
        suite_ref: &output_suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &run_profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(ProfiledBody {
        artifact_refs,
        source_report_ref,
        source_suite_ref,
        output_report_ref,
        export_profile_value,
        export_profile,
        report_value,
        report,
    })
}

fn parse_profiled_evidence(bundle: &Record<Value<IoValue>>, body: &ProfiledBody) -> Result<ProfiledEvidence> {
    let policy_value = value_to_iovalue(&bundle[19]);
    parse_redaction_policy(&policy_value)?;
    let policy_ref = canonical_hash(&policy_value)?;
    require_artifact_ref(&body.artifact_refs, "redaction-policy", &policy_ref)?;
    let manifest_value = value_to_iovalue(&bundle[20]);
    let manifest_ref = canonical_hash(&manifest_value)?;
    require_artifact_ref(&body.artifact_refs, "redaction-transform-manifest", &manifest_ref)?;
    let transform_receipt_value = value_to_iovalue(&bundle[21]);
    let transform_receipt = parse_redaction_transform_receipt(&transform_receipt_value)?;
    require_artifact_ref(&body.artifact_refs, "redaction-transform", &transform_receipt.receipt_ref)?;
    require_transform_receipt_binding(body, &policy_ref, &manifest_ref, &transform_receipt)?;
    validate_redaction_transform_manifest(
        &manifest_value,
        &body.source_report_ref,
        &body.source_suite_ref,
        &body.report,
        body.export_profile.profile,
    )?;
    require_profiled_output_inventory(body, &transform_receipt)?;
    Ok(ProfiledEvidence {
        policy_ref,
        manifest_ref,
        manifest_value,
        transform_receipt,
    })
}

fn require_transform_receipt_binding(
    body: &ProfiledBody,
    policy_ref: &str,
    manifest_ref: &str,
    transform_receipt: &RedactionTransformReceiptEvidence,
) -> Result<()> {
    if transform_receipt.source_report_ref != body.source_report_ref
        || transform_receipt.source_suite_ref != body.source_suite_ref
        || transform_receipt.policy_ref != policy_ref
        || transform_receipt.profile != body.export_profile.profile
        || transform_receipt.manifest_ref != manifest_ref
        || transform_receipt.output_bundle_ref != body.output_report_ref
        || transform_receipt.loss_classification != body.export_profile.loss_classification
        || body.export_profile.is_gate_preserving
        || body.export_profile.requires_reveal != body.export_profile.profile.requires_reveal()
    {
        return Err(MoltenError::invalid_harness(
            "redaction transform receipt binding does not match profiled repro bundle",
        ));
    }
    Ok(())
}

fn require_profiled_output_inventory(
    body: &ProfiledBody,
    transform_receipt: &RedactionTransformReceiptEvidence,
) -> Result<()> {
    let output_encrypted_refs = validate_profiled_output(&body.report_value, body.export_profile.profile)?;
    if output_encrypted_refs != transform_receipt.encrypted_refs {
        return Err(MoltenError::invalid_harness(
            "redaction transform encrypted-ref inventory does not match output bundle",
        ));
    }
    let output_marker_refs = collect_redaction_marker_refs(&body.report_value)?;
    if output_marker_refs != transform_receipt.marker_refs {
        return Err(MoltenError::invalid_harness(
            "redaction transform marker manifest does not cover output bundle markers",
        ));
    }
    Ok(())
}

fn parse_profiled_private(
    bundle: &Record<Value<IoValue>>,
    body: &ProfiledBody,
    evidence: &ProfiledEvidence,
    arity: usize,
) -> Result<ProfiledPrivate> {
    if arity != 24 {
        if body.export_profile.profile == ReproExportProfile::EncryptedPrivate {
            return Err(MoltenError::invalid_harness(
                "encrypted-private repro bundle missing private bundle profile evidence",
            ));
        }
        return Ok(ProfiledPrivate {
            private_bundle_profile_ref: None,
            private_bundle_profile_value: None,
            checks_index: 22,
        });
    }
    let private_value = value_to_iovalue(&bundle[22]);
    let private = crate::secrets::parse_private_bundle_profile(&private_value)?;
    require_artifact_ref(&body.artifact_refs, "private-bundle-profile", &canonical_hash(&private_value)?)?;
    if body.export_profile.profile != ReproExportProfile::EncryptedPrivate {
        return Err(MoltenError::invalid_harness(
            "private bundle profile is only valid for encrypted-private repro exports",
        ));
    }
    if private.transform_receipt_ref != evidence.transform_receipt.receipt_ref
        || private.encrypted_refs != evidence.transform_receipt.encrypted_refs
        || private.is_gate_preserving
    {
        return Err(MoltenError::invalid_harness(
            "private bundle profile does not bind encrypted refs and diagnostic-only transform receipt",
        ));
    }
    Ok(ProfiledPrivate {
        private_bundle_profile_ref: Some(canonical_hash(&private_value)?),
        private_bundle_profile_value: Some(private_value),
        checks_index: 23,
    })
}
