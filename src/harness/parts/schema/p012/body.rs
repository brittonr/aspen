
pub fn repro_bundle_value_with_command(report_value: &IoValue, command: &[String]) -> Result<IoValue> {
    let report = parse_report(report_value)?;
    Ok(record("harness-repro-bundle-v1", vec![
        string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "validate", "report.preserves"][..],
            &["molten", "test", "replay", "report.preserves"][..],
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "gate", "check", "refs.preserves"][..],
        ]),
        report_artifact_refs_value(&report, None, None)?,
        string(&report.report_ref),
        string(&report.suite_ref),
        string(&report.initial_state_hash),
        string(&report.final_state_hash),
        string(&report.replay_status),
        string(&report.profile),
        actor_registry_value(&report.actors),
        effect_log_value(&report.effect_log),
        report.suite_value,
        report_value.clone(),
    ]))
}

pub fn sealed_repro_bundle_value_with_command_and_receipt(
    report_value: &IoValue,
    command: &[String],
    receipt_value: &IoValue,
) -> Result<IoValue> {
    let report = parse_report(report_value)?;
    let gate_receipt_ref = canonical_hash(receipt_value)?;
    let redaction_policy = redaction_policy_value();
    let redaction_policy_ref = canonical_hash(&redaction_policy)?;
    let redaction_gate = redaction_gate_value(report_value, &report)?;
    let redaction_gate_ref = canonical_hash(&redaction_gate)?;
    let seal = repro_seal_value(&report, &gate_receipt_ref);
    let sealed_checks = sealed_repro_checks_value();
    Ok(record("harness-repro-bundle-v1", vec![
        string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "validate", "report.preserves"][..],
            &["molten", "test", "replay", "report.preserves"][..],
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "gate", "check", "refs.preserves"][..],
        ]),
        report_artifact_refs_value(
            &report,
            Some(&gate_receipt_ref),
            Some((&redaction_policy_ref, &redaction_gate_ref)),
        )?,
        string(&report.report_ref),
        string(&report.suite_ref),
        string(&report.initial_state_hash),
        string(&report.final_state_hash),
        string(&report.replay_status),
        string(&report.profile),
        actor_registry_value(&report.actors),
        effect_log_value(&report.effect_log),
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
    report_value: &IoValue,
    command: &[String],
    profile: ReproExportProfile,
) -> Result<IoValue> {
    if profile == ReproExportProfile::DenySensitive {
        return Err(MoltenError::invalid_harness(
            "deny-sensitive repro export must use sealed pass bundle construction",
        ));
    }
    let source_report = parse_report(report_value)?;
    let policy_value = redaction_policy_value();
    let policy_ref = canonical_hash(&policy_value)?;
    let transform = redacted_report_for_profile(report_value, &source_report, profile, &policy_ref)?;
    let output_report = parse_report(&transform.report_value)?;
    let export_profile_value = repro_export_profile_value(profile);
    let export_profile_ref = canonical_hash(&export_profile_value)?;
    let mut artifact_refs = report_artifact_refs(&output_report, None, None)?;
    artifact_refs.push(("source-report".to_string(), source_report.report_ref.clone()));
    artifact_refs.push(("source-suite".to_string(), source_report.suite_ref.clone()));
    artifact_refs.push(("redaction-policy".to_string(), policy_ref.clone()));
    artifact_refs.push(("export-profile".to_string(), export_profile_ref));
    artifact_refs.push(("redaction-transform-manifest".to_string(), transform.manifest_ref.clone()));
    artifact_refs.push(("redaction-transform".to_string(), transform.receipt_ref.clone()));
    let private_profile_value = if profile == ReproExportProfile::EncryptedPrivate {
        let value = crate::secrets::private_bundle_profile_value(&crate::secrets::PrivateBundleProfileInput {
            profile_ref: canonical_hash(&export_profile_value)?,
            encrypted_refs: transform.encrypted_refs.clone(),
            reveal_receipt_refs: Vec::new(),
            transform_receipt_ref: transform.receipt_ref.clone(),
            is_gate_preserving: false,
        })?;
        artifact_refs.push(("private-bundle-profile".to_string(), canonical_hash(&value)?));
        Some(value)
    } else {
        None
    };
    let mut fields = vec![
        string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "repro", "unpack", "refs.preserves"][..],
            &["molten", "test", "repro", "verify", "refs.preserves"][..],
        ]),
        artifact_refs_owned_value(&artifact_refs),
        string(&source_report.report_ref),
        string(&source_report.suite_ref),
        string(&output_report.report_ref),
        string(&output_report.suite_ref),
        string(&output_report.initial_state_hash),
        string(&output_report.final_state_hash),
        string(&output_report.replay_status),
        string(&output_report.profile),
        export_profile_value,
        actor_registry_value(&output_report.actors),
        effect_log_value(&output_report.effect_log),
        output_report.suite_value,
        transform.report_value,
        policy_value,
        transform.manifest_value,
        transform.receipt_value,
    ];
    if let Some(private_profile_value) = private_profile_value {
        fields.push(private_profile_value);
    }
    fields.push(profiled_repro_checks_value(profile));
    Ok(record("harness-repro-bundle-v1", fields))
}

struct ProfiledTransformOutput {
    report_value: IoValue,
    manifest_ref: String,
    manifest_value: IoValue,
    receipt_ref: String,
    receipt_value: IoValue,
    encrypted_refs: Vec<String>,
}

struct RedactionTransformState {
    profile: ReproExportProfile,
    policy_ref: String,
    marker_refs: Vec<String>,
    marker_entries: Vec<RedactionManifestEntry>,
    encrypted_refs: Vec<String>,
}

struct RedactionManifestEntry {
    path: String,
    reason: String,
    commitment_ref: String,
    marker_ref: Option<String>,
    encrypted_ref: Option<String>,
}

fn redacted_report_for_profile(
    report_value: &IoValue,
    report: &Report,
    profile: ReproExportProfile,
    policy_ref: &str,
) -> Result<ProfiledTransformOutput> {
    let mut state = RedactionTransformState {
        profile,
        policy_ref: policy_ref.to_string(),
        marker_refs: Vec::new(),
        marker_entries: Vec::new(),
        encrypted_refs: Vec::new(),
    };
    let redacted_report_value = rebind_report_suite_ref(&transform_sensitive_value(report_value, "/", &mut state)?)?;
    state.marker_refs.sort();
    state.marker_refs.dedup();
    state.encrypted_refs.sort();
    state.encrypted_refs.dedup();
    let redacted_report = parse_report(&redacted_report_value)?;
    validate_profiled_output(&redacted_report_value, profile)?;
    let manifest_value = redaction_transform_manifest_value(
        report,
        &redacted_report,
        profile,
        &state.marker_entries,
        &state.encrypted_refs,
    );
    let manifest_ref = canonical_hash(&manifest_value)?;
    let receipt_value = redaction_transform_receipt_value(&RedactionTransformReceiptInput {
        source_report_ref: &report.report_ref,
        source_suite_ref: &report.suite_ref,
        policy_ref,
        profile,
        manifest_ref: &manifest_ref,
        output_bundle_ref: &redacted_report.report_ref,
        marker_refs: &state.marker_refs,
        encrypted_refs: &state.encrypted_refs,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(ProfiledTransformOutput {
        report_value: redacted_report_value,
        manifest_ref,
        manifest_value,
        receipt_ref,
        receipt_value,
        encrypted_refs: state.encrypted_refs,
    })
}

fn rebind_report_suite_ref(report_value: &IoValue) -> Result<IoValue> {
    let report = simple_record(report_value, "harness-report-v1", 17)
        .or_else(|_| simple_record(report_value, "harness-report-v1", 16))
        .or_else(|_| simple_record(report_value, "harness-report-v1", 15))
        .or_else(|_| simple_record(report_value, "harness-report-v1", 14))
        .or_else(|_| simple_record(report_value, "harness-report-v1", 13))?;
    let suite_value = value_to_iovalue(&report[8]);
    let suite_ref = canonical_hash(&suite_value)?;
    let field_count = report.fields_iter().count();
    let mut fields = Vec::with_capacity(field_count);
    for (index, field) in report.fields_iter().enumerate() {
        if index == 5 {
            fields.push(string(&suite_ref));
        } else {
            fields.push(value_to_iovalue(field));
        }
    }
    Ok(record("harness-report-v1", fields))
}

enum RedactionTraversalFrame {
    Enter {
        value: IoValue,
        path: String,
    },
    ExitRecord {
        original: IoValue,
        label: IoValue,
        field_count: usize,
    },
    ExitSequence {
        original: IoValue,
        item_count: usize,
    },
}

fn ensure_redaction_bound(count: usize, limit: usize, context: &str) -> Result<()> {
    if count > limit {
        return Err(MoltenError::invalid_harness(format!("{context} exceeds redaction transform bound {limit}")));
    }
    Ok(())
}

struct RedactionFrameStack {
    frames: Vec<RedactionTraversalFrame>,
}

impl RedactionFrameStack {
    fn new() -> Self {
        Self {
            frames: Vec::with_capacity(1),
        }
    }

    fn push(&mut self, frame: RedactionTraversalFrame) -> Result<()> {
        ensure_redaction_bound(self.frames.len() + 1, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal stack")?;
        self.frames.push(frame);
        Ok(())
    }

    fn pop(&mut self) -> Option<RedactionTraversalFrame> {
        self.frames.pop()
    }

    fn push_children(&mut self, child_entries: Vec<(IoValue, String)>) -> Result<()> {
        ensure_redaction_bound(
            self.frames.len() + child_entries.len(),
            MAX_REDACTION_TRANSFORM_NODES,
            "redaction traversal stack",
        )?;
        for (child_value, child_path) in child_entries.into_iter().rev() {
            self.push(RedactionTraversalFrame::Enter {
                value: child_value,
                path: child_path,
            })?;
        }
        Ok(())
    }
}
