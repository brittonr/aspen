struct ProfiledBody {
    artifact_refs: Vec<(String, String)>,
    source_report_ref: String,
    source_suite_ref: String,
    output_report_ref: String,
    export_profile_value: preserves::IOValue,
    export_profile: super::audience::ReproExportProfileEvidence,
    report_value: preserves::IOValue,
    report: super::model::Report,
}

struct ProfiledEvidence {
    policy_ref: String,
    manifest_ref: String,
    manifest_value: preserves::IOValue,
    transform_receipt: super::ledgering::RedactionTransformReceiptEvidence,
}

struct ProfiledPrivate {
    private_bundle_profile_ref: Option<String>,
    private_bundle_profile_value: Option<preserves::IOValue>,
    checks_index: usize,
}

pub(super) fn parse_profiled_report_repro_bundle(
    bundle_value: &preserves::IOValue,
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<super::model::ReproBundle> {
    let arity = bundle.fields_iter().count();
    let body = parse_profiled_body(bundle)?;
    super::sealing::require_report_artifact_refs(&body.artifact_refs, &body.report)?;
    let evidence = parse_profiled_evidence(bundle, &body)?;
    let private = parse_profiled_private(bundle, &body, &evidence, arity)?;
    let checks = super::sealing::parse_seal_checks(&bundle[private.checks_index])?;
    require_profiled_checks(&checks, body.export_profile.profile)?;
    profiled_report_bundle(bundle_value, body, evidence, private)
}

fn parse_profiled_body(
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<ProfiledBody> {
    let kind = super::fields::required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(crate::error::Failure::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    super::manifest::validate_tool_record(&bundle[2])?;
    super::manifest::validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    super::manifest::validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = super::manifest::parse_artifact_refs(&bundle[5])?;
    let source_report_ref = super::fields::required_hash(&bundle[6], "profiled repro source report ref")?;
    let source_suite_ref = super::fields::required_hash(&bundle[7], "profiled repro source suite ref")?;
    let output_report_ref = super::fields::required_hash(&bundle[8], "profiled repro output report ref")?;
    let output_suite_ref = super::fields::required_hash(&bundle[9], "profiled repro output suite ref")?;
    super::manifest::require_artifact_ref(&artifact_refs, "source-report", &source_report_ref)?;
    super::manifest::require_artifact_ref(&artifact_refs, "source-suite", &source_suite_ref)?;
    super::manifest::require_artifact_ref(&artifact_refs, "report", &output_report_ref)?;
    super::manifest::require_artifact_ref(&artifact_refs, "suite", &output_suite_ref)?;
    let initial_state_hash = super::fields::required_hash(&bundle[10], "profiled repro initial state hash")?;
    let final_state_hash = super::fields::required_hash(&bundle[11], "profiled repro final state hash")?;
    let replay_status = super::fields::required_string(&bundle[12], "profiled repro replay status")?;
    let run_profile = super::fields::required_string(&bundle[13], "profiled repro harness profile")?;
    let export_profile_value = crate::preserves_rail::value_to_iovalue(&bundle[14]);
    let export_profile = super::audience::parse_repro_export_profile(&export_profile_value)?;
    super::manifest::require_artifact_ref(&artifact_refs, "export-profile", &export_profile.profile_ref)?;
    let actors = super::roster::parse_actor_registry(&crate::preserves_rail::value_to_iovalue(&bundle[15]))?;
    let effect_log = super::tape::parse_effect_log(&crate::preserves_rail::value_to_iovalue(&bundle[16]))?;
    let suite_value = crate::preserves_rail::value_to_iovalue(&bundle[17]);
    let report_value = crate::preserves_rail::value_to_iovalue(&bundle[18]);
    let report = super::reading::parse_report(&report_value)?;
    super::sealing::require_repro_report_matches(&super::sealing::ReproReportMatchInput {
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

fn parse_profiled_evidence(
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
    body: &ProfiledBody,
) -> crate::error::Result<ProfiledEvidence> {
    let policy_value = crate::preserves_rail::value_to_iovalue(&bundle[19]);
    super::disclosure::parse_redaction_policy(&policy_value)?;
    let policy_ref = crate::preserves_rail::canonical_hash(&policy_value)?;
    super::manifest::require_artifact_ref(&body.artifact_refs, "redaction-policy", &policy_ref)?;
    let manifest_value = crate::preserves_rail::value_to_iovalue(&bundle[20]);
    let manifest_ref = crate::preserves_rail::canonical_hash(&manifest_value)?;
    super::manifest::require_artifact_ref(&body.artifact_refs, "redaction-transform-manifest", &manifest_ref)?;
    let transform_receipt_value = crate::preserves_rail::value_to_iovalue(&bundle[21]);
    let transform_receipt = super::ledgering::parse_redaction_transform_receipt(&transform_receipt_value)?;
    super::manifest::require_artifact_ref(&body.artifact_refs, "redaction-transform", &transform_receipt.receipt_ref)?;
    require_transform_receipt_binding(body, &policy_ref, &manifest_ref, &transform_receipt)?;
    super::audience::validate_redaction_transform_manifest(
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
    transform_receipt: &super::ledgering::RedactionTransformReceiptEvidence,
) -> crate::error::Result<()> {
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
        return Err(crate::error::Failure::invalid_harness(
            "redaction transform receipt binding does not match profiled repro bundle",
        ));
    }
    Ok(())
}

fn require_profiled_output_inventory(
    body: &ProfiledBody,
    transform_receipt: &super::ledgering::RedactionTransformReceiptEvidence,
) -> crate::error::Result<()> {
    let output_encrypted_refs =
        super::audience::validate_profiled_output(&body.report_value, body.export_profile.profile)?;
    if output_encrypted_refs != transform_receipt.encrypted_refs {
        return Err(crate::error::Failure::invalid_harness(
            "redaction transform encrypted-ref inventory does not match output bundle",
        ));
    }
    let output_marker_refs = super::audience::collect_redaction_marker_refs(&body.report_value)?;
    if output_marker_refs != transform_receipt.marker_refs {
        return Err(crate::error::Failure::invalid_harness(
            "redaction transform marker manifest does not cover output bundle markers",
        ));
    }
    Ok(())
}

fn parse_profiled_private(
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
    body: &ProfiledBody,
    evidence: &ProfiledEvidence,
    arity: usize,
) -> crate::error::Result<ProfiledPrivate> {
    if arity != 24 {
        if body.export_profile.profile == super::model::ReproExportProfile::EncryptedPrivate {
            return Err(crate::error::Failure::invalid_harness(
                "encrypted-private repro bundle missing private bundle profile evidence",
            ));
        }
        return Ok(ProfiledPrivate {
            private_bundle_profile_ref: None,
            private_bundle_profile_value: None,
            checks_index: 22,
        });
    }
    let private_value = crate::preserves_rail::value_to_iovalue(&bundle[22]);
    let private = crate::secrets::parse_private_bundle_profile(&private_value)?;
    super::manifest::require_artifact_ref(
        &body.artifact_refs,
        "private-bundle-profile",
        &crate::preserves_rail::canonical_hash(&private_value)?,
    )?;
    if body.export_profile.profile != super::model::ReproExportProfile::EncryptedPrivate {
        return Err(crate::error::Failure::invalid_harness(
            "private bundle profile is only valid for encrypted-private repro exports",
        ));
    }
    if private.transform_receipt_ref != evidence.transform_receipt.receipt_ref
        || private.encrypted_refs != evidence.transform_receipt.encrypted_refs
        || private.is_gate_preserving
    {
        return Err(crate::error::Failure::invalid_harness(
            "private bundle profile does not bind encrypted refs and diagnostic-only transform receipt",
        ));
    }
    Ok(ProfiledPrivate {
        private_bundle_profile_ref: Some(crate::preserves_rail::canonical_hash(&private_value)?),
        private_bundle_profile_value: Some(private_value),
        checks_index: 23,
    })
}

fn require_profiled_checks(checks: &[String], profile: super::model::ReproExportProfile) -> crate::error::Result<()> {
    for expected in [
        "profile-schema",
        "redaction-transform-receipt",
        "transform-manifest-bound",
        "source-report-ref-binding",
        "output-report-ref-binding",
        "no-forbidden-cleartext",
    ] {
        super::sealing::require_seal_check(checks, expected)?;
    }
    match profile {
        super::model::ReproExportProfile::DenySensitive => {
            super::sealing::require_seal_check(checks, "gate-preserving")?
        }
        super::model::ReproExportProfile::RedactedDiagnostic => {
            super::sealing::require_seal_check(checks, "diagnostic-only")?
        }
        super::model::ReproExportProfile::EncryptedPrivate => {
            super::sealing::require_seal_check(checks, "requires-reveal")?;
            super::sealing::require_seal_check(checks, "encrypted-ref-validation")?;
        }
    }
    Ok(())
}

fn profiled_report_bundle(
    bundle_value: &preserves::IOValue,
    body: ProfiledBody,
    evidence: ProfiledEvidence,
    private: ProfiledPrivate,
) -> crate::error::Result<super::model::ReproBundle> {
    let transform_receipt = evidence.transform_receipt;
    Ok(super::model::ReproBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(bundle_value)?,
        kind: super::model::ReproBundleKind::Report,
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
