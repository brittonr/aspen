use preserves::ValueImpl;

pub fn parse_repro_bundle(value: &preserves::IOValue) -> crate::error::Result<super::model::ReproBundle> {
    let bundle = value
        .collect_simple_record("harness-repro-bundle-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <harness-repro-bundle-v1 ...>"))?;
    let arity = bundle.fields_iter().count();
    let schema = super::fields::required_string(&bundle[0], "repro bundle schema")?;
    if schema != crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported repro bundle schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA
        )));
    }

    if arity == 11 {
        return parse_legacy_report_repro_bundle(value, &bundle);
    }
    if arity == 16 {
        return parse_report_repro_bundle(value, &bundle);
    }
    if arity == 19 || arity == 21 {
        return super::unseal::parse_sealed_report_repro_bundle(value, &bundle);
    }
    if arity == 23 || arity == 24 {
        return super::tailored::parse_profiled_report_repro_bundle(value, &bundle);
    }
    if arity == 8 {
        return super::sealing::parse_failure_repro_bundle(value, &bundle);
    }
    Err(crate::error::Failure::invalid_harness(format!(
        "expected <harness-repro-bundle-v1 ...> with arity 8, 11, 16, 19, 21, 23, or 24, got {arity}"
    )))
}

pub fn repro_bundle_report_value(bundle_value: &preserves::IOValue) -> crate::error::Result<preserves::IOValue> {
    let bundle = parse_repro_bundle(bundle_value)?;
    match (bundle.kind, bundle.report_value) {
        (super::model::ReproBundleKind::Report, Some(report_value)) => Ok(report_value),
        (super::model::ReproBundleKind::Failure, _) => Err(crate::error::Failure::invalid_harness(format!(
            "failure repro bundle {} cannot satisfy pass evidence gate",
            bundle.bundle_ref
        ))),
        (super::model::ReproBundleKind::Report, None) => {
            Err(crate::error::Failure::invalid_harness("report repro bundle missing report value"))
        }
    }
}

pub fn repro_bundle_summary(bundle_value: &preserves::IOValue) -> crate::error::Result<String> {
    let bundle = parse_repro_bundle(bundle_value)?;
    let gate_receipt = bundle.gate_receipt_ref.as_deref().unwrap_or("none");
    let export_profile = bundle.export_profile.as_deref().unwrap_or("legacy");
    let loss_classification = bundle.loss_classification.as_deref().unwrap_or("unknown");
    Ok(format!(
        "repro bundle {}\nkind={}\nartifact={}\ngate_receipt={}\nprofile={}\nloss_classification={}",
        bundle.bundle_ref,
        match bundle.kind {
            super::model::ReproBundleKind::Report => "report",
            super::model::ReproBundleKind::Failure => "failure",
        },
        bundle.artifact_ref,
        gate_receipt,
        export_profile,
        loss_classification
    ))
}

fn parse_legacy_report_repro_bundle(
    bundle_value: &preserves::IOValue,
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<super::model::ReproBundle> {
    let report_ref = super::fields::required_string(&bundle[1], "repro bundle report ref")?;
    let suite_ref = super::fields::required_string(&bundle[2], "repro bundle suite ref")?;
    let initial_state_hash = super::fields::required_hash(&bundle[3], "repro bundle initial state hash")?;
    let final_state_hash = super::fields::required_hash(&bundle[4], "repro bundle final state hash")?;
    let replay_status = super::fields::required_string(&bundle[5], "repro bundle replay status")?;
    let profile = super::fields::required_string(&bundle[6], "repro bundle profile")?;
    let actors = super::roster::parse_actor_registry(&crate::preserves_rail::value_to_iovalue(&bundle[7]))?;
    let effect_log = super::tape::parse_effect_log(&crate::preserves_rail::value_to_iovalue(&bundle[8]))?;
    let suite_value = crate::preserves_rail::value_to_iovalue(&bundle[9]);
    let report_value = crate::preserves_rail::value_to_iovalue(&bundle[10]);
    let report = super::reading::parse_report(&report_value)?;
    super::sealing::require_repro_report_matches(&super::sealing::ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(super::model::ReproBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(bundle_value)?,
        kind: super::model::ReproBundleKind::Report,
        artifact_ref: report_ref,
        report_value: Some(report_value),
        failure_value: None,
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
        loss_classification: None,
        encrypted_refs: Vec::new(),
    })
}

pub(super) struct ReportBundleBody {
    pub(super) artifact_refs: Vec<(String, String)>,
    pub(super) report_ref: String,
    pub(super) suite_ref: String,
    pub(super) replay_status: String,
    pub(super) profile: String,
    pub(super) report_value: preserves::IOValue,
    pub(super) report: super::model::Report,
}

fn parse_report_repro_bundle(
    bundle_value: &preserves::IOValue,
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<super::model::ReproBundle> {
    let body = parse_report_body(bundle)?;
    report_bundle_from_body(bundle_value, body)
}

pub(super) fn parse_report_body(
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<ReportBundleBody> {
    let kind = super::fields::required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(crate::error::Failure::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    super::manifest::validate_tool_record(&bundle[2])?;
    super::manifest::validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    super::manifest::validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = super::manifest::parse_artifact_refs(&bundle[5])?;
    let report_ref = super::fields::required_string(&bundle[6], "repro bundle report ref")?;
    let suite_ref = super::fields::required_string(&bundle[7], "repro bundle suite ref")?;
    super::manifest::require_artifact_ref(&artifact_refs, "report", &report_ref)?;
    super::manifest::require_artifact_ref(&artifact_refs, "suite", &suite_ref)?;
    let initial_state_hash = super::fields::required_hash(&bundle[8], "repro bundle initial state hash")?;
    let final_state_hash = super::fields::required_hash(&bundle[9], "repro bundle final state hash")?;
    let replay_status = super::fields::required_string(&bundle[10], "repro bundle replay status")?;
    let profile = super::fields::required_string(&bundle[11], "repro bundle profile")?;
    let actors = super::roster::parse_actor_registry(&crate::preserves_rail::value_to_iovalue(&bundle[12]))?;
    let effect_log = super::tape::parse_effect_log(&crate::preserves_rail::value_to_iovalue(&bundle[13]))?;
    let suite_value = crate::preserves_rail::value_to_iovalue(&bundle[14]);
    let report_value = crate::preserves_rail::value_to_iovalue(&bundle[15]);
    let report = super::reading::parse_report(&report_value)?;
    super::sealing::require_repro_report_matches(&super::sealing::ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(ReportBundleBody {
        artifact_refs,
        report_ref,
        suite_ref,
        replay_status,
        profile,
        report_value,
        report,
    })
}

fn report_bundle_from_body(
    bundle_value: &preserves::IOValue,
    body: ReportBundleBody,
) -> crate::error::Result<super::model::ReproBundle> {
    Ok(super::model::ReproBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(bundle_value)?,
        kind: super::model::ReproBundleKind::Report,
        artifact_ref: body.report_ref,
        report_value: Some(body.report_value),
        failure_value: None,
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
        loss_classification: None,
        encrypted_refs: Vec::new(),
    })
}
