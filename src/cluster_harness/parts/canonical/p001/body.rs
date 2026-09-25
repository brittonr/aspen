
// r[impl molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
// r[impl molten.testing.fixture_driven_cluster_execution.observation_gate]
pub fn build_cluster_harness_parent(
    input: &ClusterHarnessParentInput,
) -> crate::error::Result<ClusterHarnessParentReceipt> {
    let bound_refs = [
        &input.fixture_ref,
        &input.command_plan_ref,
        &input.local_plan_ref,
        &input.local_run_ref,
        &input.lifecycle_ref,
        &input.drift_summary_ref,
        &input.cleanup_ref,
    ];
    for reference in bound_refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    validate_refs("cluster run child receipt", &input.child_receipt_refs)?;
    validate_refs("cluster run diagnostic log", &input.diagnostic_log_refs)?;
    validate_non_empty_strings("cluster run required artifact kind", &input.required_artifact_kinds)?;
    validate_non_empty_strings("cluster run observed artifact kind", &input.observed_artifact_kinds)?;
    validate_non_empty_strings("cluster run caveat", &input.caveats)?;

    let diagnostics = parent_diagnostics(input);
    let is_all_required_kinds_observed =
        !diagnostics.iter().any(|diagnostic| diagnostic.contains("missing-required-artifact-kind"));
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = crate::preserves_rail::record("cluster-harness-run-v1", vec![
        crate::preserves_rail::string(CLUSTER_RUN_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("fixture", vec![crate::preserves_rail::string(&input.fixture_ref)]),
        crate::preserves_rail::record("command-plan", vec![crate::preserves_rail::string(&input.command_plan_ref)]),
        crate::preserves_rail::record("local-plan", vec![crate::preserves_rail::string(&input.local_plan_ref)]),
        crate::preserves_rail::record("local-run", vec![crate::preserves_rail::string(&input.local_run_ref)]),
        crate::preserves_rail::record("lifecycle", vec![crate::preserves_rail::string(&input.lifecycle_ref)]),
        crate::preserves_rail::record("drift-summary", vec![crate::preserves_rail::string(&input.drift_summary_ref)]),
        crate::preserves_rail::record("cleanup", vec![crate::preserves_rail::string(&input.cleanup_ref)]),
        crate::preserves_rail::record("child-receipts", vec![refs_sequence(&input.child_receipt_refs)]),
        crate::preserves_rail::record("diagnostic-logs", vec![refs_sequence(&input.diagnostic_log_refs)]),
        crate::preserves_rail::record("required-artifact-kinds", vec![strings_sequence(
            &input.required_artifact_kinds,
        )]),
        crate::preserves_rail::record("observed-artifact-kinds", vec![strings_sequence(
            &input.observed_artifact_kinds,
        )]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        crate::preserves_rail::record("caveats", vec![strings_sequence(&input.caveats)]),
        crate::preserves_rail::checks_value(&[
            ("child-receipts-bound", status(!input.child_receipt_refs.is_empty())),
            ("required-artifact-kinds-observed", status(is_all_required_kinds_observed)),
            ("unsupported-is-not-pass", status(!input.unsupported_pass_claim)),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ClusterHarnessParentReceipt {
        decision,
        diagnostics,
        receipt_ref,
        value,
    })
}

fn parent_diagnostics(input: &ClusterHarnessParentInput) -> Vec<String> {
    let mut diagnostics = input.diagnostics.clone();
    if input.unsupported_pass_claim {
        diagnostics.push("cluster-run-unsupported-pass-claim".to_string());
    }
    for required in &input.required_artifact_kinds {
        if !input.observed_artifact_kinds.contains(required) {
            diagnostics.push(format!("cluster-run-missing-required-artifact-kind:{required}"));
        }
    }
    if input.child_receipt_refs.is_empty() {
        diagnostics.push("cluster-run-missing-child-receipts".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

// r[impl molten.testing.receipt_first_cluster_harness.run_artifact_directory]
// r[impl molten.testing.receipt_first_cluster_harness.failure_triage]
pub fn cluster_run_verification_value(
    index_ref: &str,
    assessment: &molten_core::cluster_harness::RunDirectoryAssessment,
) -> crate::error::Result<ClusterRunVerificationReceipt> {
    crate::preserves_rail::validate_content_ref(index_ref)?;
    let divergence = first_divergence_value(assessment.first_divergence.as_ref());
    let value = crate::preserves_rail::record("cluster-run-verification-v1", vec![
        crate::preserves_rail::string(VERIFICATION_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&assessment.decision)]),
        crate::preserves_rail::record("artifact-index", vec![crate::preserves_rail::string(index_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&assessment.diagnostics)]),
        crate::preserves_rail::record("first-divergence", vec![divergence]),
        crate::preserves_rail::record("evidence-scope", vec![crate::preserves_rail::string(DIAGNOSTIC_ONLY)]),
        crate::preserves_rail::checks_value(&[
            ("offline-content-verification", status(assessment.decision == PASS_DECISION)),
            ("first-divergence-diagnostic-only", PASS_DECISION),
        ]),
    ]);
    let verification_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ClusterRunVerificationReceipt {
        decision: assessment.decision.clone(),
        diagnostics: assessment.diagnostics.clone(),
        verification_ref,
        value,
    })
}

pub fn artifact_decision(value: &IoValue, artifact_kind: &str) -> crate::error::Result<Option<String>> {
    let (record_label, arity) = match artifact_kind {
        CLUSTER_RUN_KIND => ("cluster-harness-run-v1", CLUSTER_RUN_RECORD_ARITY),
        LOCAL_PLAN_KIND => ("local-multiprocess-plan-v1", LOCAL_PLAN_RECORD_ARITY),
        LOCAL_EXECUTABLE_RUN_KIND => ("local-multiprocess-executable-run-v1", LOCAL_EXECUTABLE_RUN_RECORD_ARITY),
        CLUSTER_LIFECYCLE_KIND => ("cluster-lifecycle-run-v1", CLUSTER_LIFECYCLE_RECORD_ARITY),
        CHILD_PROCESS_KIND => ("cluster-harness-child-process-v1", CHILD_PROCESS_RECORD_ARITY),
        CLEANUP_KIND => ("cluster-harness-cleanup-v1", CLEANUP_RECORD_ARITY),
        _ => return Ok(None),
    };
    let fields = crate::preserves_rail::simple_record_fields(value, record_label, arity)?;
    let decision_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let decision = crate::preserves_rail::simple_record_fields(&decision_value, "decision", 1)?;
    Ok(Some(crate::preserves_rail::required_string_field(&decision[0], "cluster artifact decision")?))
}

pub fn content_ref_for_text(domain: &str, text: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(&[0]);
    hasher.update(text.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn first_divergence_value(divergence: Option<&molten_core::cluster_harness::FirstDivergence>) -> IoValue {
    let Some(divergence) = divergence else {
        return crate::preserves_rail::record("none", Vec::new());
    };
    crate::preserves_rail::record("first-divergence", vec![
        crate::preserves_rail::record("path", vec![crate::preserves_rail::string(&divergence.relative_path)]),
        crate::preserves_rail::record("artifact-kind", vec![crate::preserves_rail::string(&divergence.artifact_kind)]),
        crate::preserves_rail::record("expected", vec![crate::preserves_rail::string(&divergence.expected)]),
        crate::preserves_rail::record("observed", vec![crate::preserves_rail::string(&divergence.observed)]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string(&divergence.reason)]),
        crate::preserves_rail::record("evidence-scope", vec![crate::preserves_rail::string(DIAGNOSTIC_ONLY)]),
    ])
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn validate_refs(label: &str, refs: &[String]) -> crate::error::Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_non_empty_strings(label: &str, values: &[String]) -> crate::error::Result<()> {
    if values.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} values must not be empty")));
    }
    if values.iter().any(|value| value.trim().is_empty() || value.trim() != value) {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} values must be non-empty and unpadded"
        )));
    }
    Ok(())
}

fn status(condition: bool) -> &'static str {
    if condition { PASS_DECISION } else { DENY_DECISION }
}

pub fn no_divergence_marker() -> &'static str {
    NONE_VALUE
}
