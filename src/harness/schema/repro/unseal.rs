struct SealedRedaction {
    redaction_policy_ref: Option<String>,
    redaction_gate_ref: Option<String>,
    seal_index: usize,
    receipt_index: usize,
    checks_index: usize,
    has_redaction: bool,
}

pub(super) fn parse_sealed_report_repro_bundle(
    bundle_value: &preserves::IOValue,
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<super::model::ReproBundle> {
    let body = super::unpack::parse_report_body(bundle)?;
    super::sealing::require_report_artifact_refs(&body.artifact_refs, &body.report)?;
    let arity = bundle.fields_iter().count();
    let redaction = parse_sealed_redaction(bundle, &body, arity)?;
    let seal = super::sealing::parse_repro_seal(
        &bundle[redaction.seal_index],
        &body.report_ref,
        &body.suite_ref,
        &body.profile,
        &body.replay_status,
    )?;
    super::manifest::require_artifact_ref(&body.artifact_refs, "gate-receipt", &seal.gate_receipt_ref)?;
    let receipt_value = embedded_gate_receipt(bundle, redaction.receipt_index, &seal.gate_receipt_ref)?;
    let seal_checks = super::sealing::parse_seal_checks(&bundle[redaction.checks_index])?;
    require_report_seal_checks(&seal_checks, redaction.has_redaction)?;
    sealed_report_bundle(bundle_value, body, seal, receipt_value, redaction)
}

fn parse_sealed_redaction(
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
    body: &super::unpack::ReportBundleBody,
    arity: usize,
) -> crate::error::Result<SealedRedaction> {
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
    let redaction_policy_value = crate::preserves_rail::value_to_iovalue(&bundle[16]);
    let redaction_gate_value = crate::preserves_rail::value_to_iovalue(&bundle[17]);
    let (redaction_policy_ref, redaction_gate_ref) = super::disclosure::validate_redaction_evidence(
        &body.report_value,
        &body.report,
        &redaction_policy_value,
        &redaction_gate_value,
    )?;
    super::manifest::require_artifact_ref(&body.artifact_refs, "redaction-policy", &redaction_policy_ref)?;
    super::manifest::require_artifact_ref(&body.artifact_refs, "redaction-gate", &redaction_gate_ref)?;
    Ok(SealedRedaction {
        redaction_policy_ref: Some(redaction_policy_ref),
        redaction_gate_ref: Some(redaction_gate_ref),
        seal_index: 18,
        receipt_index: 19,
        checks_index: 20,
        has_redaction: true,
    })
}

fn embedded_gate_receipt(
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
    index: usize,
    expected_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    let receipt_value = crate::preserves_rail::value_to_iovalue(&bundle[index]);
    let actual_gate_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    if actual_gate_receipt_ref != expected_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "sealed repro bundle gate receipt ref mismatch: seal has {expected_ref}, embedded receipt hashes to {actual_gate_receipt_ref}"
        )));
    }
    Ok(receipt_value)
}

fn require_report_seal_checks(checks: &[String], has_redaction: bool) -> crate::error::Result<()> {
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
        super::sealing::require_seal_check(checks, expected)?;
    }
    if has_redaction {
        super::sealing::require_seal_check(checks, "redaction-preflight")?;
        super::sealing::require_seal_check(checks, "redaction-gate-ref-binding")?;
        super::sealing::require_seal_check(checks, "no-sensitive-markers")?;
    }
    super::sealing::require_seal_check(checks, "replay-metadata-binding")
}

fn sealed_report_bundle(
    bundle_value: &preserves::IOValue,
    body: super::unpack::ReportBundleBody,
    seal: super::sealing::ReproSeal,
    receipt_value: preserves::IOValue,
    redaction: SealedRedaction,
) -> crate::error::Result<super::model::ReproBundle> {
    Ok(super::model::ReproBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(bundle_value)?,
        kind: super::model::ReproBundleKind::Report,
        artifact_ref: body.report_ref,
        report_value: Some(body.report_value),
        failure_value: None,
        gate_receipt_ref: Some(seal.gate_receipt_ref),
        receipt_value: Some(receipt_value),
        redaction_policy_ref: redaction.redaction_policy_ref,
        redaction_gate_ref: redaction.redaction_gate_ref,
        export_profile: Some(super::model::ReproExportProfile::DenySensitive.as_str().to_string()),
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
        loss_classification: Some(super::model::ReproExportProfile::DenySensitive.loss_classification().to_string()),
        encrypted_refs: Vec::new(),
    })
}
