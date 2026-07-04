
fn tool_value() -> IoValue {
    record("tool", vec![string("molten"), string(env!("CARGO_PKG_VERSION"))])
}

fn command_value(command: &[String]) -> IoValue {
    record("command", vec![sequence(command.iter().map(string).collect())])
}

fn replay_instructions_value(instructions: &[&[&str]]) -> IoValue {
    record("replay-instructions", vec![sequence(
        instructions
            .iter()
            .map(|instruction| sequence(instruction.iter().map(|part| string(*part)).collect()))
            .collect(),
    )])
}

fn artifact_refs_value(refs: &[(&str, &str)]) -> IoValue {
    record("artifact-refs", vec![sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| record("artifact-ref", vec![string(*kind), string(*artifact_ref)]))
            .collect(),
    )])
}

fn artifact_refs_owned_value(refs: &[(String, String)]) -> IoValue {
    record("artifact-refs", vec![sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| record("artifact-ref", vec![string(kind), string(artifact_ref)]))
            .collect(),
    )])
}

fn report_artifact_refs_value(
    report: &Report,
    gate_receipt_ref: Option<&str>,
    redaction_refs: Option<(&str, &str)>,
) -> Result<IoValue> {
    Ok(artifact_refs_owned_value(&report_artifact_refs(report, gate_receipt_ref, redaction_refs)?))
}

fn report_artifact_refs(
    report: &Report,
    gate_receipt_ref: Option<&str>,
    redaction_refs: Option<(&str, &str)>,
) -> Result<Vec<(String, String)>> {
    let policy_gate = report
        .policy_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing policy gate evidence"))?;
    let capability_gate = report
        .capability_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing capability gate evidence"))?;
    let budget_gate = report
        .budget_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing budget gate evidence"))?;
    let executor_preflights = report
        .executor_preflights
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing executor preflight evidence"))?;
    let authority_refs = report_authority_aggregate_refs(report)?;
    let mut refs = vec![
        ("report".to_string(), report.report_ref.clone()),
        ("suite".to_string(), report.suite_ref.clone()),
        ("initial-state".to_string(), report.initial_state_hash.clone()),
        ("final-state".to_string(), report.final_state_hash.clone()),
        ("actor-registry".to_string(), canonical_hash(&actor_registry_value(&report.actors))?),
        ("executor-preflights".to_string(), canonical_hash(&executor_preflights.value)?),
        ("effect-log".to_string(), canonical_hash(&effect_log_value(&report.effect_log))?),
        ("policy".to_string(), policy_gate.policy_ref.clone()),
        ("policy-gate".to_string(), canonical_hash(&policy_gate.value)?),
        ("policy-nickel-source".to_string(), policy_gate.nickel_source_ref.clone()),
        ("policy-nickel-export".to_string(), policy_gate.nickel_export_ref.clone()),
        ("policy-basalt-preflight".to_string(), policy_gate.basalt_preflight_ref.clone()),
        ("budget".to_string(), budget_gate.budget_ref.clone()),
        ("budget-gate".to_string(), canonical_hash(&budget_gate.value)?),
        ("budget-nickel-source".to_string(), budget_gate.nickel_source_ref.clone()),
        ("budget-nickel-export".to_string(), budget_gate.nickel_export_ref.clone()),
        ("budget-basalt-preflight".to_string(), budget_gate.basalt_preflight_ref.clone()),
        ("capabilities".to_string(), capability_gate.capability_ref.clone()),
        ("capability-gate".to_string(), canonical_hash(&capability_gate.value)?),
        ("capability-authority-preflight".to_string(), capability_gate.authority_preflight_ref.clone()),
        ("ucan-proofset".to_string(), capability_gate.proofset_ref.clone()),
        (
            "ucan-verification-receipts".to_string(),
            authority_refs.ucan_verification_receipts_ref,
        ),
        ("derived-grants".to_string(), authority_refs.derived_grants_ref),
        (
            "basalt-enforcement-receipts".to_string(),
            authority_refs.authority_receipts_ref,
        ),
        ("authority-requests".to_string(), authority_refs.request_refs_ref),
    ];
    if let Some(gate_receipt_ref) = gate_receipt_ref {
        refs.push(("gate-receipt".to_string(), gate_receipt_ref.to_string()));
    }
    if let Some((redaction_policy_ref, redaction_gate_ref)) = redaction_refs {
        refs.push(("redaction-policy".to_string(), redaction_policy_ref.to_string()));
        refs.push(("redaction-gate".to_string(), redaction_gate_ref.to_string()));
    }
    Ok(refs)
}

struct ReportAuthorityAggregateRefs {
    ucan_verification_receipts_ref: String,
    derived_grants_ref: String,
    authority_receipts_ref: String,
    request_refs_ref: String,
}

fn report_authority_aggregate_refs(report: &Report) -> Result<ReportAuthorityAggregateRefs> {
    let mut ucan_verification_receipt_refs: Vec<String> = Vec::new();
    let mut derived_grant_refs: Vec<String> = Vec::new();
    let mut authority_receipt_refs: Vec<String> = Vec::new();
    let mut request_refs: Vec<String> = Vec::new();
    for observation in &report.observations {
        for event in &observation.events {
            if event_boundary(event) != EventBoundary::PolicyDecision {
                continue;
            }
            let admission = parse_admission_decision_event(event)?;
            let Some(authority) = admission.authority else {
                continue;
            };
            ucan_verification_receipt_refs.extend(authority.ucan_verification_receipt_refs);
            derived_grant_refs.extend(authority.derived_grant_refs);
            authority_receipt_refs.push(authority.basalt_enforcement_receipt_ref);
            request_refs.push(authority.request_ref);
        }
    }
    Ok(ReportAuthorityAggregateRefs {
        ucan_verification_receipts_ref: canonical_hash(&record(
            "ucan-verification-receipts",
            vec![sequence(ucan_verification_receipt_refs.as_slice().iter().map(string).collect())],
        ))?,
        derived_grants_ref: canonical_hash(&record(
            "derived-grants",
            vec![sequence(derived_grant_refs.as_slice().iter().map(string).collect())],
        ))?,
        authority_receipts_ref: canonical_hash(&record(
            "basalt-enforcement-receipts",
            vec![sequence(authority_receipt_refs.as_slice().iter().map(string).collect())],
        ))?,
        request_refs_ref: canonical_hash(&record(
            "authority-requests",
            vec![sequence(request_refs.as_slice().iter().map(string).collect())],
        ))?,
    })
}

const FORBIDDEN_REDACTION_MARKERS: &[&str] = &[
    "secret",
    "confidential",
    "credential",
    "private",
    "encrypted-ref",
    "encrypted-ref-v1",
    "secret-ref-v1",
];

struct ReproExportProfileEvidence {
    profile: ReproExportProfile,
    profile_ref: String,
    loss_classification: String,
    is_gate_preserving: bool,
    requires_reveal: bool,
}

struct RedactionTransformReceiptInput<'a> {
    source_report_ref: &'a str,
    source_suite_ref: &'a str,
    policy_ref: &'a str,
    profile: ReproExportProfile,
    manifest_ref: &'a str,
    output_bundle_ref: &'a str,
    marker_refs: &'a [String],
    encrypted_refs: &'a [String],
}

struct RedactionTransformReceiptEvidence {
    receipt_ref: String,
    source_report_ref: String,
    source_suite_ref: String,
    policy_ref: String,
    profile: ReproExportProfile,
    manifest_ref: String,
    output_bundle_ref: String,
    loss_classification: String,
    marker_refs: Vec<String>,
    encrypted_refs: Vec<String>,
    value: IoValue,
}

fn redaction_policy_value() -> IoValue {
    record("redaction-policy-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA),
        record("mode", vec![string("deny-sensitive-markers")]),
        record("forbidden-markers", vec![sequence(
            FORBIDDEN_REDACTION_MARKERS.iter().map(|marker| string(*marker)).collect(),
        )]),
    ])
}

fn repro_export_profile_value(profile: ReproExportProfile) -> IoValue {
    record("repro-export-profile-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA),
        record("name", vec![string(profile.as_str())]),
        record("loss-classification", vec![string(profile.loss_classification())]),
        record("gate-preserving", vec![bool_value(profile.is_gate_preserving())]),
        record("requires-reveal", vec![bool_value(profile.requires_reveal())]),
        checks_value_for_names(&[
            "explicit-export-profile",
            "loss-classification-bound",
            "gate-preserving-bound",
            "reveal-requirement-bound",
        ]),
    ])
}

fn parse_repro_export_profile(value: &IoValue) -> Result<ReproExportProfileEvidence> {
    let profile_value = simple_record(value, "repro-export-profile-v1", 6)?;
    let schema = required_string(&profile_value[0], "repro export profile schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro export profile schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA
        )));
    }
    let name = required_record_string(&profile_value[1], "name", "repro export profile name")?;
    let profile = ReproExportProfile::parse(&name)?;
    let loss_classification =
        required_record_string(&profile_value[2], "loss-classification", "repro export loss classification")?;
    if loss_classification != profile.loss_classification() {
        return Err(MoltenError::invalid_harness("repro export profile loss classification is not canonical"));
    }
    let is_gate_preserving =
        required_record_bool(&profile_value[3], "gate-preserving", "repro export gate preserving flag")?;
    if is_gate_preserving != profile.is_gate_preserving() {
        return Err(MoltenError::invalid_harness("repro export gate-preserving flag is not canonical"));
    }
    let is_requires_reveal = required_record_bool(&profile_value[4], "requires-reveal", "repro export reveal flag")?;
    if is_requires_reveal != profile.requires_reveal() {
        return Err(MoltenError::invalid_harness("repro export reveal flag is not canonical"));
    }
    let checks = parse_redaction_gate_checks(&profile_value[5])?;
    require_redaction_check(&checks, "explicit-export-profile")?;
    require_redaction_check(&checks, "loss-classification-bound")?;
    require_redaction_check(&checks, "gate-preserving-bound")?;
    require_redaction_check(&checks, "reveal-requirement-bound")?;
    Ok(ReproExportProfileEvidence {
        profile,
        profile_ref: canonical_hash(value)?,
        loss_classification,
        is_gate_preserving,
        requires_reveal: is_requires_reveal,
    })
}

fn redaction_transform_manifest_value(
    source_report: &Report,
    output_report: &Report,
    profile: ReproExportProfile,
    entries: &[RedactionManifestEntry],
    encrypted_refs: &[String],
) -> IoValue {
    record("redaction-transform-manifest-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA),
        record("source-report", vec![string(&source_report.report_ref)]),
        record("source-suite", vec![string(&source_report.suite_ref)]),
        record("output-report", vec![string(&output_report.report_ref)]),
        record("output-suite", vec![string(&output_report.suite_ref)]),
        record("profile", vec![string(profile.as_str())]),
        record("markers", vec![sequence(
            entries
                .iter()
                .map(|entry| {
                    record("redaction", vec![
                        string(&entry.path),
                        string(&entry.reason),
                        string(&entry.commitment_ref),
                        optional_ref_value(entry.marker_ref.as_deref()),
                        optional_ref_value(entry.encrypted_ref.as_deref()),
                    ])
                })
                .collect(),
        )]),
        record("encrypted-refs", vec![refs_sequence(encrypted_refs)]),
        checks_value_for_names(&[
            "source-report-bound",
            "output-report-bound",
            "deterministic-traversal-order",
            "marker-coverage-manifest",
            "encrypted-ref-inventory",
        ]),
    ])
}

fn redaction_transform_receipt_value(input: &RedactionTransformReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("redaction-transform-receipt-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("source-report", vec![string(input.source_report_ref)]),
        record("source-suite", vec![string(input.source_suite_ref)]),
        record("policy", vec![string(input.policy_ref)]),
        record("profile", vec![string(input.profile.as_str())]),
        record("transform-manifest", vec![string(input.manifest_ref)]),
        record("output-bundle", vec![string(input.output_bundle_ref)]),
        record("loss-classification", vec![string(input.profile.loss_classification())]),
        record("markers", vec![refs_sequence(input.marker_refs)]),
        record("encrypted-refs", vec![refs_sequence(input.encrypted_refs)]),
        checks_value_for_names(&redaction_transform_check_names(input.profile)),
    ]))
}
