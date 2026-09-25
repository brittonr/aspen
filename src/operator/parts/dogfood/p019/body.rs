
struct ReleaseWorkflowStageResult {
    name: &'static str,
    is_complete: bool,
    diagnostics: Vec<String>,
}

pub fn evaluate_release_workflow_state(
    input: &ReleaseWorkflowStateInput<'_>,
) -> Result<ReleaseWorkflowStateDecision> {
    validate_release_workflow_state_input(input)?;
    let mut completed_stages = Vec::new();
    let mut diagnostics = Vec::new();
    for stage in release_workflow_stage_results(input)? {
        if stage.is_complete {
            completed_stages.push_limited_value(
                stage.name.to_string(),
                RELEASE_WORKFLOW_STAGE_COUNT,
                "release workflow completed stages",
            )?;
        } else {
            for diagnostic in stage.diagnostics {
                diagnostics.push_limited_value(
                    diagnostic,
                    MAX_OPERATOR_DIAGNOSTICS,
                    "release workflow diagnostics",
                )?;
            }
        }
        if stage.name == input.required_stage {
            break;
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    Ok(ReleaseWorkflowStateDecision {
        decision,
        completed_stages,
        diagnostics,
    })
}

fn validate_release_workflow_state_input(input: &ReleaseWorkflowStateInput<'_>) -> Result<()> {
    validate_release_workflow_stage(input.required_stage)?;
    validate_decision(input.dogfood_report_decision)?;
    validate_decision(input.bundle_verify_decision)?;
    validate_decision(input.promotion_decision)?;
    validate_decision(input.summary_decision)?;
    validate_decision(input.export_verify_decision)?;
    validate_optional_ref(input.dogfood_report_ref, "release workflow dogfood report ref")?;
    validate_optional_ref(input.release_gate_ref, "release workflow release gate ref")?;
    validate_optional_ref(input.bundle_ref, "release workflow bundle ref")?;
    validate_optional_ref(input.bundle_verify_ref, "release workflow bundle verify ref")?;
    validate_refs(input.signed_member_refs, "release workflow signed member ref")?;
    validate_refs(input.required_signed_member_refs, "release workflow required signed member ref")?;
    validate_optional_ref(input.promotion_ref, "release workflow promotion ref")?;
    validate_optional_ref(input.signed_promotion_ref, "release workflow signed promotion ref")?;
    validate_optional_ref(
        input.signed_promotion_subject_ref,
        "release workflow signed promotion subject ref",
    )?;
    validate_optional_ref(input.summary_ref, "release workflow summary ref")?;
    validate_optional_ref(input.summary_promotion_ref, "release workflow summary promotion ref")?;
    validate_optional_ref(input.export_manifest_ref, "release workflow export manifest ref")?;
    validate_optional_ref(
        input.export_manifest_summary_ref,
        "release workflow export manifest summary ref",
    )?;
    validate_optional_ref(input.export_verify_ref, "release workflow export verify ref")?;
    validate_optional_ref(
        input.export_verify_manifest_ref,
        "release workflow export verify manifest ref",
    )
}

fn validate_release_workflow_stage(stage: &str) -> Result<()> {
    if RELEASE_WORKFLOW_STAGES.contains(&stage) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported release workflow required stage {stage}"
        )))
    }
}

fn release_workflow_stage_results(
    input: &ReleaseWorkflowStateInput<'_>,
) -> Result<Vec<ReleaseWorkflowStageResult>> {
    let [is_dogfood_complete, is_bundle_export_complete, is_bundle_verify_complete, is_signed_members_complete, is_promotion_complete, is_signed_promotion_complete, is_summary_complete, is_archive_export_complete, is_archive_verify_complete] = release_workflow_completion(input);
    Ok(vec![
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_DOGFOOD,
            is_dogfood_complete,
            dogfood_diagnostics(input),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_BUNDLE_EXPORT,
            is_bundle_export_complete,
            bundle_export_diagnostics(input, is_dogfood_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_BUNDLE_VERIFY,
            is_bundle_verify_complete,
            bundle_verify_diagnostics(input, is_bundle_export_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_SIGNED_MEMBERS,
            is_signed_members_complete,
            signed_member_diagnostics(input, is_bundle_verify_complete)?,
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_PROMOTION,
            is_promotion_complete,
            promotion_stage_diagnostics(input, is_signed_members_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_SIGNED_PROMOTION,
            is_signed_promotion_complete,
            signed_promotion_diagnostics(input, is_promotion_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_SUMMARY,
            is_summary_complete,
            summary_stage_diagnostics(input, is_signed_promotion_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_ARCHIVE_EXPORT,
            is_archive_export_complete,
            archive_export_diagnostics(input, is_summary_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_ARCHIVE_VERIFY,
            is_archive_verify_complete,
            archive_verify_diagnostics(input, is_archive_export_complete),
        )?,
    ])
}

/// Completion of each release workflow stage in order; a stage completes only after every earlier stage.
fn release_workflow_completion(input: &ReleaseWorkflowStateInput<'_>) -> [bool; 9] {
    let is_dogfood_complete = input.dogfood_report_ref.is_some() && input.dogfood_report_decision == "pass";
    let is_bundle_export_complete = is_dogfood_complete && input.release_gate_ref.is_some() && input.bundle_ref.is_some();
    let is_bundle_verify_complete = is_bundle_export_complete
        && input.bundle_verify_ref.is_some()
        && input.bundle_verify_decision == "pass";
    let is_signed_members_complete = is_bundle_verify_complete && signed_members_cover_required(input);
    let is_promotion_complete = is_signed_members_complete
        && input.promotion_ref.is_some()
        && input.promotion_decision == "pass";
    let is_signed_promotion_complete = is_promotion_complete
        && input.signed_promotion_ref.is_some()
        && input.signed_promotion_subject_ref == input.promotion_ref;
    let is_summary_complete = is_signed_promotion_complete
        && input.summary_ref.is_some()
        && input.summary_decision == "pass"
        && input.summary_promotion_ref == input.promotion_ref;
    let is_archive_export_complete = is_summary_complete
        && input.export_manifest_ref.is_some()
        && input.export_manifest_summary_ref == input.summary_ref;
    let is_archive_verify_complete = is_archive_export_complete
        && input.export_verify_ref.is_some()
        && input.export_verify_decision == "pass"
        && input.export_verify_manifest_ref == input.export_manifest_ref;
    [is_dogfood_complete, is_bundle_export_complete, is_bundle_verify_complete, is_signed_members_complete, is_promotion_complete, is_signed_promotion_complete, is_summary_complete, is_archive_export_complete, is_archive_verify_complete]
}

fn workflow_stage_result(
    name: &'static str,
    is_complete: bool,
    diagnostics: Vec<String>,
) -> Result<ReleaseWorkflowStageResult> {
    ensure_count_at_most(diagnostics.len(), MAX_OPERATOR_DIAGNOSTICS, "release workflow stage diagnostics")?;
    Ok(ReleaseWorkflowStageResult {
        name,
        is_complete,
        diagnostics,
    })
}

fn dogfood_diagnostics(input: &ReleaseWorkflowStateInput<'_>) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if input.dogfood_report_ref.is_none() {
        diagnostics.push("release workflow dogfood report evidence missing".to_string());
    }
    if input.dogfood_report_decision != "pass" {
        diagnostics.push(format!(
            "release workflow dogfood report decision is {}; expected pass",
            input.dogfood_report_decision
        ));
    }
    diagnostics
}

fn bundle_export_diagnostics(input: &ReleaseWorkflowStateInput<'_>, dogfood_complete: bool) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !dogfood_complete {
        diagnostics.push("release workflow bundle export requires passing dogfood evidence".to_string());
    }
    if input.release_gate_ref.is_none() {
        diagnostics.push("release workflow release gate evidence missing before bundle export".to_string());
    }
    if input.bundle_ref.is_none() {
        diagnostics.push("release workflow bundle export evidence missing".to_string());
    }
    diagnostics
}

fn bundle_verify_diagnostics(input: &ReleaseWorkflowStateInput<'_>, bundle_export_complete: bool) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !bundle_export_complete {
        diagnostics.push("release workflow bundle verification requires exported bundle evidence".to_string());
    }
    if input.bundle_verify_ref.is_none() {
        diagnostics.push("release workflow bundle verification receipt missing".to_string());
    }
    if input.bundle_verify_decision != "pass" {
        diagnostics.push(format!(
            "release workflow bundle verification decision is {}; expected pass",
            input.bundle_verify_decision
        ));
    }
    diagnostics
}

fn signed_member_diagnostics(
    input: &ReleaseWorkflowStateInput<'_>,
    bundle_verify_complete: bool,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if !bundle_verify_complete {
        diagnostics.push("release workflow signed members require current passing bundle verification".to_string());
    }
    if input.required_signed_member_refs.is_empty() {
        diagnostics.push("release workflow required signed-member class is empty".to_string());
    }
    for required_ref in input.required_signed_member_refs {
        if !input.signed_member_refs.iter().any(|signed_ref| signed_ref == required_ref) {
            diagnostics.push_limited_value(
                format!("release workflow missing signed member proof for {required_ref}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release workflow signed member diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}

fn signed_members_cover_required(input: &ReleaseWorkflowStateInput<'_>) -> bool {
    !input.required_signed_member_refs.is_empty()
        && input
            .required_signed_member_refs
            .iter()
            .all(|required_ref| input.signed_member_refs.iter().any(|signed_ref| signed_ref == required_ref))
}

fn promotion_stage_diagnostics(
    input: &ReleaseWorkflowStateInput<'_>,
    signed_members_complete: bool,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if !signed_members_complete {
        diagnostics.push("release promotion cannot pass before current passing bundle verification and signed members".to_string());
    }
    if input.promotion_ref.is_none() {
        diagnostics.push("release workflow promotion receipt missing".to_string());
    }
    if input.promotion_decision != "pass" {
        diagnostics.push(format!(
            "release workflow promotion decision is {}; expected pass",
            input.promotion_decision
        ));
    }
    diagnostics
}
