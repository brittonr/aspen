
fn signed_promotion_diagnostics(
    input: &ReleaseWorkflowStateInput<'_>,
    promotion_complete: bool,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !promotion_complete {
        diagnostics.push("signed release promotion requires passing promotion receipt".to_string());
    }
    if input.signed_promotion_ref.is_none() {
        diagnostics.push("signed release promotion receipt missing".to_string());
    }
    if input.signed_promotion_subject_ref != input.promotion_ref {
        diagnostics.push("signed release promotion subject ref does not match promotion receipt".to_string());
    }
    diagnostics
}

fn summary_stage_diagnostics(input: &ReleaseWorkflowStateInput<'_>, signed_promotion_complete: bool) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !signed_promotion_complete {
        diagnostics.push("release summary requires verified signed promotion receipt".to_string());
    }
    if input.summary_ref.is_none() {
        diagnostics.push("release promotion summary receipt missing".to_string());
    }
    if input.summary_decision != "pass" {
        diagnostics.push(format!(
            "release workflow summary decision is {}; expected pass",
            input.summary_decision
        ));
    }
    if input.summary_promotion_ref != input.promotion_ref {
        diagnostics.push("release promotion summary does not bind promotion receipt".to_string());
    }
    diagnostics
}

fn archive_export_diagnostics(input: &ReleaseWorkflowStateInput<'_>, summary_complete: bool) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !summary_complete {
        diagnostics.push("release archive export requires passing release summary".to_string());
    }
    if input.export_manifest_ref.is_none() {
        diagnostics.push("release export manifest missing".to_string());
    }
    if input.export_manifest_summary_ref != input.summary_ref {
        diagnostics.push("release export manifest does not bind promotion summary".to_string());
    }
    diagnostics
}

fn archive_verify_diagnostics(input: &ReleaseWorkflowStateInput<'_>, archive_export_complete: bool) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !archive_export_complete {
        diagnostics.push("release archive verification requires deterministic archive export manifest".to_string());
    }
    if input.export_verify_ref.is_none() {
        diagnostics.push("release export verification receipt missing".to_string());
    }
    if input.export_verify_decision != "pass" {
        diagnostics.push(format!(
            "release workflow export verification decision is {}; expected pass",
            input.export_verify_decision
        ));
    }
    if input.export_verify_manifest_ref != input.export_manifest_ref {
        diagnostics.push("release export verification does not bind export manifest".to_string());
    }
    diagnostics
}

pub fn evaluate_release_evidence_only_boundary(
    input: &ReleaseEvidenceBoundaryInput<'_>,
) -> Result<ReleaseEvidenceBoundaryDecision> {
    validate_non_empty(input.operation, "release evidence boundary operation")?;
    validate_refs(input.release_receipt_refs, "release evidence receipt ref")?;
    validate_refs(input.authority_refs, "release evidence boundary authority ref")?;
    validate_refs(input.policy_refs, "release evidence boundary policy ref")?;
    validate_refs(input.provenance_refs, "release evidence boundary provenance ref")?;
    validate_refs(input.source_gate_refs, "release evidence boundary source-gate ref")?;
    validate_refs(input.retention_refs, "release evidence boundary retention ref")?;
    validate_refs(input.resource_refs, "release evidence boundary resource ref")?;
    validate_refs(input.transport_refs, "release evidence boundary transport ref")?;
    validate_refs(
        input.destructive_operation_refs,
        "release evidence boundary destructive-operation ref",
    )?;
    debug_assert_eq!(RELEASE_EVIDENCE_BOUNDARY_GATES.len(), RELEASE_EVIDENCE_BOUNDARY_GATE_COUNT);

    let mut diagnostics = Vec::new();
    if input.release_receipt_refs.is_empty() {
        diagnostics.push(format!(
            "release evidence receipt missing for operation {}",
            input.operation
        ));
    }
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.authority_refs, "authority");
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.policy_refs, "policy");
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.provenance_refs, "provenance");
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.source_gate_refs, "source-gate");
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.retention_refs, "retention");
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.resource_refs, "resource");
    push_release_boundary_diagnostic(&mut diagnostics, input.operation, input.transport_refs, "transport");
    push_release_boundary_diagnostic(
        &mut diagnostics,
        input.operation,
        input.destructive_operation_refs,
        "destructive-operation",
    );
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    Ok(ReleaseEvidenceBoundaryDecision { decision, diagnostics })
}

fn push_release_boundary_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, operation: &str, refs: &[String], gate: &str) {
    if refs.is_empty() {
        diagnostics.push_item(format!(
            "release evidence for operation {operation} remains evidence-only and does not grant {gate} trust"
        ));
    }
}
