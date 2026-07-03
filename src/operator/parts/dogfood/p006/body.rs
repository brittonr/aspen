
fn summary_record(
    output_path_string: &str,
    output_path_ref: &str,
    facts: &SummaryFacts,
    refs: &SummaryRefs,
) -> IoValue {
    let decision = if facts.diagnostics.is_empty() { "pass" } else { "deny" };
    crate::preserves_rail::record("release-promotion-summary-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("output", vec![
            crate::preserves_rail::string(output_path_string),
            crate::preserves_rail::string(output_path_ref),
        ]),
        crate::preserves_rail::record("promotion", vec![
            crate::preserves_rail::string(&refs.promotion_ref),
            crate::preserves_rail::string(&refs.promotion_decision),
            crate::preserves_rail::string(&refs.bundle_verify_ref),
            crate::preserves_rail::string(&refs.bundle_ref),
        ]),
        crate::preserves_rail::record("signed-promotion", vec![
            crate::preserves_rail::string(&refs.signed_envelope_ref),
            crate::preserves_rail::string(&refs.signed_subject_ref),
            crate::preserves_rail::string(&refs.signed_key_ref),
            crate::preserves_rail::string(RELEASE_PROMOTION_SIGNING_PURPOSE),
        ]),
        crate::preserves_rail::record("evidence", vec![
            crate::preserves_rail::record("source", vec![crate::preserves_rail::string(&refs.source_ref)]),
            crate::preserves_rail::record("octet", vec![crate::preserves_rail::string(&refs.octet_ref)]),
            crate::preserves_rail::record("cairn", vec![crate::preserves_rail::string(&refs.cairn_ref)]),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&facts.diagnostics)]),
        checks_value_from_pairs(&[
            (
                "release-promotion-pass",
                status(facts.promotion.as_ref().is_some_and(|promotion| promotion.decision == "pass")),
            ),
            (
                "release-promotion-output-bound",
                status(facts.promotion.as_ref().is_some_and(|promotion| promotion.output_path_ref == output_path_ref)),
            ),
            ("signed-promotion-present", status(facts.signed.is_some())),
            (
                "signed-promotion-subject-bound",
                status(facts.signed.as_ref().is_some_and(|signed| signed.subject_ref == refs.promotion_ref)),
            ),
            ("signed-promotion-keyring-current", status(facts.signed.is_some())),
            ("release-promotion-summary-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ])
}

pub fn parse_release_promotion_summary(value: &IoValue) -> Result<ReleasePromotionSummary> {
    let fields = value
        .collect_simple_record("release-promotion-summary-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-promotion-summary-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA,
        "release promotion summary",
    )?;
    let promotion_value = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let promotion_fields = simple_record(&promotion_value, "promotion", 4)?;
    let signed_value = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let signed_fields = simple_record(&signed_value, "signed-promotion", 4)?;
    let evidence_value = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let evidence_fields = simple_record(&evidence_value, "evidence", 3)?;
    let source_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[0]);
    let source_fields = simple_record(&source_value, "source", 1)?;
    let octet_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[1]);
    let octet_fields = simple_record(&octet_value, "octet", 1)?;
    let cairn_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[2]);
    let cairn_fields = simple_record(&cairn_value, "cairn", 1)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "release-promotion-pass", "release promotion summary")?;
    require_check(&checks, "release-promotion-output-bound", "release promotion summary")?;
    require_check(&checks, "signed-promotion-present", "release promotion summary")?;
    require_check(&checks, "signed-promotion-subject-bound", "release promotion summary")?;
    require_check(&checks, "signed-promotion-keyring-current", "release promotion summary")?;
    require_check(&checks, "release-promotion-summary-is-evidence-only", "release promotion summary")?;
    require_check(&checks, "no-release-authority-granted", "release promotion summary")?;
    Ok(ReleasePromotionSummary {
        summary_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        promotion_ref: required_ref(&promotion_fields[0], "release promotion summary promotion ref")?,
        bundle_verify_ref: required_ref(&promotion_fields[2], "release promotion summary bundle verify ref")?,
        signed_envelope_ref: required_ref(&signed_fields[0], "release promotion summary signed envelope ref")?,
        signed_subject_ref: required_ref(&signed_fields[1], "release promotion summary signed subject ref")?,
        signed_key_ref: required_ref(&signed_fields[2], "release promotion summary signed key ref")?,
        source_ref: required_ref(&source_fields[0], "release promotion summary source ref")?,
        octet_ref: required_ref(&octet_fields[0], "release promotion summary Octet ref")?,
        cairn_ref: required_ref(&cairn_fields[0], "release promotion summary Cairn ref")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_export_manifest_value(input: &ReleaseExportManifestInput<'_>) -> Result<ReleaseExportManifest> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let summary_value = crate::preserves_rail::parse_text(&read_output_text(
        input.output_path,
        "release-promotion-summary.preserves",
    )?)?;
    let summary = parse_release_promotion_summary(&summary_value)?;
    if summary.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release export requires pass promotion summary {}; decision is {}",
            summary.summary_ref, summary.decision
        )));
    }
    let member_refs = observe_release_export_members(input.output_path)?;
    let value = crate::preserves_rail::record("release-export-manifest-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA),
        crate::preserves_rail::record("output", vec![
            crate::preserves_rail::string(&output_path_string),
            crate::preserves_rail::string(&output_path_ref),
        ]),
        crate::preserves_rail::record("promotion-summary", vec![crate::preserves_rail::string(&summary.summary_ref)]),
        crate::preserves_rail::record("members", vec![file_refs_sequence(&member_refs)]),
        checks_value_from_pairs(&[
            ("release-promotion-summary-pass", "pass"),
            ("release-export-members-bound", "pass"),
            ("deterministic-archive-layout", "pass"),
            ("release-export-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ]);
    parse_release_export_manifest(&value)
}

pub fn parse_release_export_manifest(value: &IoValue) -> Result<ReleaseExportManifest> {
    let fields = value
        .collect_simple_record("release-export-manifest-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-export-manifest-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA,
        "release export manifest",
    )?;
    let output_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_value, "output", 2)?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "release-promotion-summary-pass", "release export manifest")?;
    require_check(&checks, "release-export-members-bound", "release export manifest")?;
    require_check(&checks, "deterministic-archive-layout", "release export manifest")?;
    require_check(&checks, "release-export-is-evidence-only", "release export manifest")?;
    require_check(&checks, "no-release-authority-granted", "release export manifest")?;
    Ok(ReleaseExportManifest {
        manifest_ref: crate::preserves_rail::canonical_hash(value)?,
        output_path_ref: required_ref(&output_fields[1], "release export output path ref")?,
        promotion_summary_ref: record_ref(&fields[2], "promotion-summary")?,
        member_refs: record_file_refs(&fields[3], "members")?,
        checks,
        value: value.clone(),
    })
}

pub fn verify_release_export(input: &ReleaseExportVerifyInput<'_>) -> Result<ReleaseExportVerifyReceipt> {
    let mut diagnostics = input.archive_diagnostics.to_vec();
    let parsed_manifest = match input.manifest_value {
        Some(value) => Some(parse_release_export_manifest(value)?),
        None => {
            diagnostics.push_limited_value(
                "release export archive is missing manifest".to_string(),
                MAX_OPERATOR_DIAGNOSTICS,
                "release export verify diagnostics",
            )?;
            None
        }
    };
    if let Some(manifest) = parsed_manifest.as_ref() {
        for diagnostic in file_ref_mismatch_diagnostics(&manifest.member_refs, input.member_refs)? {
            diagnostics.push_limited_value(
                diagnostic,
                MAX_OPERATOR_DIAGNOSTICS,
                "release export verify diagnostics",
            )?;
        }
    }
    if input.member_refs.iter().any(|(name, _)| name == "release-export-manifest.preserves") {
        diagnostics.push_limited_value(
            "release export archive must not list manifest as a payload member".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release export verify diagnostics",
        )?;
    }
    let manifest_ref = parsed_manifest
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-release-export-manifest"), |manifest| Ok(manifest.manifest_ref.clone()))?;
    let promotion_summary_ref = parsed_manifest.as_ref().map_or_else(
        || dogfood_ref("missing-release-promotion-summary"),
        |manifest| Ok(manifest.promotion_summary_ref.clone()),
    )?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("release-export-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("manifest", vec![
            crate::preserves_rail::string(&manifest_ref),
            crate::preserves_rail::string(&promotion_summary_ref),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("release-export-members-bound", status(diagnostics.is_empty())),
            ("release-promotion-summary-bound", status(parsed_manifest.is_some() && diagnostics.is_empty())),
            ("release-export-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ]);
    parse_release_export_verify_receipt(&value)
}

pub fn parse_release_export_verify_receipt(value: &IoValue) -> Result<ReleaseExportVerifyReceipt> {
    let fields = value
        .collect_simple_record("release-export-verify-receipt-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-export-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA,
        "release export verify receipt",
    )?;
    let manifest_value = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let manifest_fields = simple_record(&manifest_value, "manifest", 2)?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "release-export-members-bound", "release export verify receipt")?;
    require_check(&checks, "release-promotion-summary-bound", "release export verify receipt")?;
    require_check(&checks, "release-export-is-evidence-only", "release export verify receipt")?;
    require_check(&checks, "no-release-authority-granted", "release export verify receipt")?;
    Ok(ReleaseExportVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        manifest_ref: required_ref(&manifest_fields[0], "release export manifest ref")?,
        promotion_summary_ref: required_ref(&manifest_fields[1], "release export promotion summary ref")?,
        diagnostics: record_string_sequence(&fields[3], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

fn select_release_promotion_key<'a>(input: &'a ReleasePromotionGateInput<'_>) -> Result<&'a SignedReceiptKey> {
    let mut matches = Vec::new();
    for key in input.signed_keys {
        if key.trust_root != input.signed_trust_root {
            continue;
        }
        if let Some(signer) = input.signed_signer
            && key.signer != signer
        {
            continue;
        }
        if let Some(key_ref) = input.signed_key_ref
            && key.key_ref != key_ref
        {
            continue;
        }
        if let Some(key_id) = input.signed_key_id
            && key.key_id != key_id
        {
            continue;
        }
        matches.push_limited_value(key, MAX_OPERATOR_REFS, "release promotion signed key matches")?;
    }
    if matches.is_empty() {
        return Err(MoltenError::invalid_harness("no signed receipt key matched promotion policy"));
    }
    let mut current = Vec::new();
    for key in matches {
        if key.status != crate::evidence::SIGNED_RECEIPT_KEY_STATUS_CURRENT {
            continue;
        }
        if input.signed_key_revocations.iter().any(|revocation| revocation.key_ref == key.key_ref) {
            continue;
        }
        current.push_limited_value(key, MAX_OPERATOR_REFS, "release promotion current signed keys")?;
    }
    if current.is_empty() {
        Err(MoltenError::invalid_harness("matching signed receipt keys are stale or revoked"))
    } else if current.len() > 1 {
        Err(MoltenError::invalid_harness(
            "multiple current signed receipt keys matched promotion policy; specify key ref or key id",
        ))
    } else {
        Ok(current[0])
    }
}

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
    let dogfood_complete = input.dogfood_report_ref.is_some() && input.dogfood_report_decision == "pass";
    let bundle_export_complete = dogfood_complete && input.release_gate_ref.is_some() && input.bundle_ref.is_some();
    let bundle_verify_complete = bundle_export_complete
        && input.bundle_verify_ref.is_some()
        && input.bundle_verify_decision == "pass";
    let signed_members_complete = bundle_verify_complete && signed_members_cover_required(input);
    let promotion_complete = signed_members_complete
        && input.promotion_ref.is_some()
        && input.promotion_decision == "pass";
    let signed_promotion_complete = promotion_complete
        && input.signed_promotion_ref.is_some()
        && input.signed_promotion_subject_ref == input.promotion_ref;
    let summary_complete = signed_promotion_complete
        && input.summary_ref.is_some()
        && input.summary_decision == "pass"
        && input.summary_promotion_ref == input.promotion_ref;
    let archive_export_complete = summary_complete
        && input.export_manifest_ref.is_some()
        && input.export_manifest_summary_ref == input.summary_ref;
    let archive_verify_complete = archive_export_complete
        && input.export_verify_ref.is_some()
        && input.export_verify_decision == "pass"
        && input.export_verify_manifest_ref == input.export_manifest_ref;

    Ok(vec![
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_DOGFOOD,
            dogfood_complete,
            dogfood_diagnostics(input),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_BUNDLE_EXPORT,
            bundle_export_complete,
            bundle_export_diagnostics(input, dogfood_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_BUNDLE_VERIFY,
            bundle_verify_complete,
            bundle_verify_diagnostics(input, bundle_export_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_SIGNED_MEMBERS,
            signed_members_complete,
            signed_member_diagnostics(input, bundle_verify_complete)?,
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_PROMOTION,
            promotion_complete,
            promotion_stage_diagnostics(input, signed_members_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_SIGNED_PROMOTION,
            signed_promotion_complete,
            signed_promotion_diagnostics(input, promotion_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_SUMMARY,
            summary_complete,
            summary_stage_diagnostics(input, signed_promotion_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_ARCHIVE_EXPORT,
            archive_export_complete,
            archive_export_diagnostics(input, summary_complete),
        )?,
        workflow_stage_result(
            RELEASE_WORKFLOW_STAGE_ARCHIVE_VERIFY,
            archive_verify_complete,
            archive_verify_diagnostics(input, archive_export_complete),
        )?,
    ])
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

fn push_release_boundary_diagnostic(diagnostics: &mut Vec<String>, operation: &str, refs: &[String], gate: &str) {
    if refs.is_empty() {
        diagnostics.push(format!(
            "release evidence for operation {operation} remains evidence-only and does not grant {gate} trust"
        ));
    }
}
