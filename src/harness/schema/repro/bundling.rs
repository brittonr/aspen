pub fn repro_bundle_value(report_value: &preserves::IOValue) -> crate::error::Result<preserves::IOValue> {
    repro_bundle_value_with_command(report_value, &default_report_bundle_command())
}

pub fn repro_bundle_value_with_command(
    report_value: &preserves::IOValue,
    command: &[String],
) -> crate::error::Result<preserves::IOValue> {
    let report = super::reading::parse_report(report_value)?;
    Ok(crate::preserves_rail::record("harness-repro-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        crate::preserves_rail::record("bundle-kind", vec![crate::preserves_rail::string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "validate", "report.preserves"][..],
            &["molten", "test", "replay", "report.preserves"][..],
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "gate", "check", "refs.preserves"][..],
        ]),
        super::manifest::report_artifact_refs_value(&report, None, None)?,
        crate::preserves_rail::string(&report.report_ref),
        crate::preserves_rail::string(&report.suite_ref),
        crate::preserves_rail::string(&report.initial_state_hash),
        crate::preserves_rail::string(&report.final_state_hash),
        crate::preserves_rail::string(&report.replay_status),
        crate::preserves_rail::string(&report.profile),
        super::roster::actor_registry_value(&report.actors),
        super::tape::effect_log_value(&report.effect_log),
        report.suite_value,
        report_value.clone(),
    ]))
}

pub fn sealed_repro_bundle_value_with_command_and_receipt(
    report_value: &preserves::IOValue,
    command: &[String],
    receipt_value: &preserves::IOValue,
) -> crate::error::Result<preserves::IOValue> {
    let report = super::reading::parse_report(report_value)?;
    let gate_receipt_ref = crate::preserves_rail::canonical_hash(receipt_value)?;
    let redaction_policy = super::disclosure::redaction_policy_value();
    let redaction_policy_ref = crate::preserves_rail::canonical_hash(&redaction_policy)?;
    let redaction_gate = super::disclosure::redaction_gate_value(report_value, &report)?;
    let redaction_gate_ref = crate::preserves_rail::canonical_hash(&redaction_gate)?;
    let seal = super::sealing::repro_seal_value(&report, &gate_receipt_ref);
    let sealed_checks = super::sealing::sealed_repro_checks_value();
    Ok(crate::preserves_rail::record("harness-repro-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        crate::preserves_rail::record("bundle-kind", vec![crate::preserves_rail::string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "validate", "report.preserves"][..],
            &["molten", "test", "replay", "report.preserves"][..],
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "gate", "check", "refs.preserves"][..],
        ]),
        super::manifest::report_artifact_refs_value(
            &report,
            Some(&gate_receipt_ref),
            Some((&redaction_policy_ref, &redaction_gate_ref)),
        )?,
        crate::preserves_rail::string(&report.report_ref),
        crate::preserves_rail::string(&report.suite_ref),
        crate::preserves_rail::string(&report.initial_state_hash),
        crate::preserves_rail::string(&report.final_state_hash),
        crate::preserves_rail::string(&report.replay_status),
        crate::preserves_rail::string(&report.profile),
        super::roster::actor_registry_value(&report.actors),
        super::tape::effect_log_value(&report.effect_log),
        report.suite_value,
        report_value.clone(),
        redaction_policy,
        redaction_gate,
        seal,
        receipt_value.clone(),
        sealed_checks,
    ]))
}

pub fn profiled_repro_bundle_value_with_command(
    report_value: &preserves::IOValue,
    command: &[String],
    profile: super::model::ReproExportProfile,
) -> crate::error::Result<preserves::IOValue> {
    if profile == super::model::ReproExportProfile::DenySensitive {
        return Err(crate::error::Failure::invalid_harness(
            "deny-sensitive repro export must use sealed pass bundle construction",
        ));
    }
    let source_report = super::reading::parse_report(report_value)?;
    let policy_value = super::disclosure::redaction_policy_value();
    let policy_ref = crate::preserves_rail::canonical_hash(&policy_value)?;
    let transform = super::masking::redacted_report_for_profile(report_value, &source_report, profile, &policy_ref)?;
    let output_report = super::reading::parse_report(&transform.report_value)?;
    let export_profile_value = super::audience::repro_export_profile_value(profile);
    let export_profile_ref = crate::preserves_rail::canonical_hash(&export_profile_value)?;
    let mut artifact_refs = super::manifest::report_artifact_refs(&output_report, None, None)?;
    artifact_refs.push(("source-report".to_string(), source_report.report_ref.clone()));
    artifact_refs.push(("source-suite".to_string(), source_report.suite_ref.clone()));
    artifact_refs.push(("redaction-policy".to_string(), policy_ref.clone()));
    artifact_refs.push(("export-profile".to_string(), export_profile_ref));
    artifact_refs.push(("redaction-transform-manifest".to_string(), transform.manifest_ref.clone()));
    artifact_refs.push(("redaction-transform".to_string(), transform.receipt_ref.clone()));
    let private_profile_value = if profile == super::model::ReproExportProfile::EncryptedPrivate {
        let value = crate::secrets::private_bundle_profile_value(&crate::secrets::PrivateBundleProfileInput {
            profile_ref: crate::preserves_rail::canonical_hash(&export_profile_value)?,
            encrypted_refs: transform.encrypted_refs.clone(),
            reveal_receipt_refs: Vec::new(),
            transform_receipt_ref: transform.receipt_ref.clone(),
            is_gate_preserving: false,
        })?;
        artifact_refs.push(("private-bundle-profile".to_string(), crate::preserves_rail::canonical_hash(&value)?));
        Some(value)
    } else {
        None
    };
    let mut fields = vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        crate::preserves_rail::record("bundle-kind", vec![crate::preserves_rail::string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "repro", "unpack", "refs.preserves"][..],
            &["molten", "test", "repro", "verify", "refs.preserves"][..],
        ]),
        artifact_refs_owned_value(&artifact_refs),
        crate::preserves_rail::string(&source_report.report_ref),
        crate::preserves_rail::string(&source_report.suite_ref),
        crate::preserves_rail::string(&output_report.report_ref),
        crate::preserves_rail::string(&output_report.suite_ref),
        crate::preserves_rail::string(&output_report.initial_state_hash),
        crate::preserves_rail::string(&output_report.final_state_hash),
        crate::preserves_rail::string(&output_report.replay_status),
        crate::preserves_rail::string(&output_report.profile),
        export_profile_value,
        super::roster::actor_registry_value(&output_report.actors),
        super::tape::effect_log_value(&output_report.effect_log),
        output_report.suite_value,
        transform.report_value,
        policy_value,
        transform.manifest_value,
        transform.receipt_value,
    ];
    if let Some(private_profile_value) = private_profile_value {
        fields.push(private_profile_value);
    }
    fields.push(super::audience::profiled_repro_checks_value(profile));
    Ok(crate::preserves_rail::record("harness-repro-bundle-v1", fields))
}

pub fn failure_repro_bundle_value(failure_value: &preserves::IOValue) -> crate::error::Result<preserves::IOValue> {
    failure_repro_bundle_value_with_command(failure_value, &default_failure_bundle_command())
}

pub fn failure_repro_bundle_value_with_command(
    failure_value: &preserves::IOValue,
    command: &[String],
) -> crate::error::Result<preserves::IOValue> {
    let failure = super::outcome::parse_failure(failure_value)?;
    Ok(crate::preserves_rail::record("harness-repro-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        crate::preserves_rail::record("bundle-kind", vec![crate::preserves_rail::string("failure")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "show", "failure.preserves"][..],
            &["molten", "test", "gate", "check", "failure.preserves"][..],
        ]),
        artifact_refs_value(&[("failure", failure.failure_ref.as_str())]),
        crate::preserves_rail::string(failure.failure_ref),
        failure_value.clone(),
    ]))
}

fn tool_value() -> preserves::IOValue {
    crate::preserves_rail::record("tool", vec![
        crate::preserves_rail::string("molten"),
        crate::preserves_rail::string(env!("CARGO_PKG_VERSION")),
    ])
}

fn command_value(command: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record("command", vec![crate::preserves_rail::sequence(
        command.iter().map(crate::preserves_rail::string).collect(),
    )])
}

fn replay_instructions_value(instructions: &[&[&str]]) -> preserves::IOValue {
    crate::preserves_rail::record("replay-instructions", vec![crate::preserves_rail::sequence(
        instructions
            .iter()
            .map(|instruction| {
                crate::preserves_rail::sequence(
                    instruction.iter().map(|part| crate::preserves_rail::string(*part)).collect(),
                )
            })
            .collect(),
    )])
}

fn artifact_refs_value(refs: &[(&str, &str)]) -> preserves::IOValue {
    crate::preserves_rail::record("artifact-refs", vec![crate::preserves_rail::sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| {
                crate::preserves_rail::record("artifact-ref", vec![
                    crate::preserves_rail::string(*kind),
                    crate::preserves_rail::string(*artifact_ref),
                ])
            })
            .collect(),
    )])
}

pub(super) fn artifact_refs_owned_value(refs: &[(String, String)]) -> preserves::IOValue {
    crate::preserves_rail::record("artifact-refs", vec![crate::preserves_rail::sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| {
                crate::preserves_rail::record("artifact-ref", vec![
                    crate::preserves_rail::string(kind),
                    crate::preserves_rail::string(artifact_ref),
                ])
            })
            .collect(),
    )])
}

fn default_report_bundle_command() -> Vec<String> {
    [
        "molten",
        "test",
        "repro",
        "export",
        "report.preserves",
        "--out",
        "repro",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

fn default_failure_bundle_command() -> Vec<String> {
    [
        "molten",
        "test",
        "repro",
        "export",
        "failure.preserves",
        "--out",
        "repro",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}
