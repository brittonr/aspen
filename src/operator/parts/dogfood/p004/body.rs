
pub fn verify_release_evidence_bundle(
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
) -> Result<ReleaseEvidenceBundleVerifyReceipt> {
    let bundle = parse_release_evidence_bundle(input.bundle_value)?;
    let mut diagnostics = Vec::new();
    let BundleObservation {
        observed,
        is_output_observed,
    } = observed_or_fallback(input.output_path, &bundle, &mut diagnostics)?;
    for diagnostic in release_bundle_mismatch_diagnostics(&bundle, &observed)? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    let signature_diagnostics = release_bundle_signature_diagnostics(&bundle, input)?;
    let is_signed_member_receipts_ok = signature_diagnostics.is_empty();
    for diagnostic in signature_diagnostics {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("release-evidence-bundle-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&bundle.bundle_ref)]),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("dogfood", vec![
            crate::preserves_rail::string(&observed.report_ref),
            crate::preserves_rail::string(&observed.release_gate_ref),
        ]),
        crate::preserves_rail::record("replay", vec![
            crate::preserves_rail::string(&observed.replay_verify_ref),
            crate::preserves_rail::string(&observed.replay_index_ref),
        ]),
        crate::preserves_rail::record("nix", vec![
            crate::preserves_rail::string(&observed.nix_evidence_ref),
            crate::preserves_rail::string(&observed.nix_verify_ref),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", status(is_output_observed)),
            ("release-gate-pass", status(is_output_observed)),
            ("replay-verify-bound", status(is_output_observed)),
            ("replay-index-bound", status(is_output_observed)),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-verify-pass", status(is_output_observed)),
            ("bundle-members-bound", status(diagnostics.is_empty())),
            ("signed-member-receipts", status(is_signed_member_receipts_ok)),
            ("signed-receipts-evidence-only", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]);
    parse_release_evidence_bundle_verify_receipt(&value)
}

pub fn parse_release_evidence_bundle_verify_receipt(value: &IoValue) -> Result<ReleaseEvidenceBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("release-evidence-bundle-verify-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-evidence-bundle-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA,
        "release evidence bundle verify receipt",
    )?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let dogfood = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let dogfood_fields = simple_record(&dogfood, "dogfood", 2)?;
    let replay = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let replay_fields = simple_record(&replay, "replay", 2)?;
    let nix = crate::preserves_rail::value_to_iovalue(&fields[6]);
    let nix_fields = simple_record(&nix, "nix", 2)?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "bundle-members-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "replay-verify-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "replay-index-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "replay-index-is-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "signed-member-receipts", "release evidence bundle verify receipt")?;
    require_check(&checks, "signed-receipts-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "release-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "no-text-oracle", "release evidence bundle verify receipt")?;
    Ok(ReleaseEvidenceBundleVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        bundle_ref: record_ref(&fields[2], "bundle")?,
        output_path_ref: required_ref(&output_fields[1], "release evidence verify output path ref")?,
        report_ref: required_ref(&dogfood_fields[0], "release evidence verify report ref")?,
        release_gate_ref: required_ref(&dogfood_fields[1], "release evidence verify release gate ref")?,
        replay_verify_ref: required_ref(&replay_fields[0], "release evidence verify replay verify ref")?,
        replay_index_ref: required_ref(&replay_fields[1], "release evidence verify replay index ref")?,
        nix_evidence_ref: required_ref(&nix_fields[0], "release evidence verify Nix evidence ref")?,
        nix_verify_ref: required_ref(&nix_fields[1], "release evidence verify Nix verify ref")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

struct PromotionFacts {
    output_path_ref: String,
    source_ref: String,
    octet_ref: String,
    cairn_ref: String,
    diagnostics: Vec<String>,
    key: PromotionKeyFacts,
    key_revocation_refs: Vec<String>,
}

struct PromotionKeyFacts {
    selected_key_ref: String,
    selected_key_id: String,
    selected_signer: String,
    selected_trust_root: String,
    selected_generation: u64,
    has_selected_key: bool,
    diagnostic: Option<String>,
}

pub fn release_promotion_gate_receipt_value(
    input: &ReleasePromotionGateInput<'_>,
) -> Result<ReleasePromotionGateReceipt> {
    let bundle_verify = parse_release_evidence_bundle_verify_receipt(input.bundle_verify_value)?;
    let facts = promotion_facts(input, &bundle_verify)?;
    let decision = if facts.diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("release-promotion-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("bundle-verify", vec![
            crate::preserves_rail::string(&bundle_verify.receipt_ref),
            crate::preserves_rail::string(&bundle_verify.bundle_ref),
            crate::preserves_rail::string(&bundle_verify.output_path_ref),
            crate::preserves_rail::string(&bundle_verify.report_ref),
            crate::preserves_rail::string(&bundle_verify.release_gate_ref),
            crate::preserves_rail::string(&bundle_verify.nix_evidence_ref),
            crate::preserves_rail::string(&bundle_verify.nix_verify_ref),
        ]),
        crate::preserves_rail::record("signed-keyring", vec![
            crate::preserves_rail::record("selected-key", vec![
                crate::preserves_rail::string(&facts.key.selected_key_ref),
                crate::preserves_rail::string(&facts.key.selected_key_id),
                crate::preserves_rail::string(&facts.key.selected_signer),
                crate::preserves_rail::string(&facts.key.selected_trust_root),
                crate::preserves_rail::u64_value(facts.key.selected_generation),
            ]),
            refs_sequence(&facts.key_revocation_refs),
        ]),
        crate::preserves_rail::record("evidence", vec![
            crate::preserves_rail::record("source", vec![
                crate::preserves_rail::string(input.source_evidence),
                crate::preserves_rail::string(&facts.source_ref),
            ]),
            crate::preserves_rail::record("octet", vec![
                crate::preserves_rail::string(input.octet_evidence),
                crate::preserves_rail::string(&facts.octet_ref),
            ]),
            crate::preserves_rail::record("cairn", vec![
                crate::preserves_rail::string(input.cairn_evidence),
                crate::preserves_rail::string(&facts.cairn_ref),
            ]),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&facts.diagnostics)]),
        checks_value_from_pairs(&[
            ("release-bundle-verify-pass", status(bundle_verify.decision == "pass")),
            ("promotion-output-path-bound", status(facts.output_path_ref == bundle_verify.output_path_ref)),
            ("signed-keyring-current", status(facts.key.has_selected_key)),
            ("source-evidence-bound", status(!input.source_evidence.trim().is_empty())),
            ("octet-evidence-bound", status(!input.octet_evidence.trim().is_empty())),
            ("cairn-evidence-bound", status(!input.cairn_evidence.trim().is_empty())),
            ("release-promotion-is-evidence-only", "pass"),
            ("no-subsystem-authority-granted", "pass"),
        ]),
    ]);
    parse_release_promotion_gate_receipt(&value)
}

fn promotion_facts(
    input: &ReleasePromotionGateInput<'_>,
    bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
) -> Result<PromotionFacts> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let source_ref = raw_text_ref("molten.operator.release-promotion.source-evidence.v1", input.source_evidence);
    let octet_ref = raw_text_ref("molten.operator.release-promotion.octet-evidence.v1", input.octet_evidence);
    let cairn_ref = raw_text_ref("molten.operator.release-promotion.cairn-evidence.v1", input.cairn_evidence);
    let key = promotion_key_facts(input)?;
    let mut diagnostics = promotion_diagnostics(input, bundle_verify, &output_path_ref)?;
    if let Some(diagnostic) = key.diagnostic.as_ref() {
        diagnostics.push_limited_value(
            diagnostic.clone(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    let key_revocation_refs = input
        .signed_key_revocations
        .iter()
        .map(|revocation| revocation.revocation_ref.clone())
        .collect::<Vec<_>>();
    Ok(PromotionFacts {
        output_path_ref,
        source_ref,
        octet_ref,
        cairn_ref,
        diagnostics,
        key,
        key_revocation_refs,
    })
}

fn promotion_diagnostics(
    input: &ReleasePromotionGateInput<'_>,
    bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
    output_path_ref: &str,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if bundle_verify.decision != "pass" {
        diagnostics.push_limited_value(
            format!(
                "release evidence bundle verify receipt {} decision is {}",
                bundle_verify.receipt_ref, bundle_verify.decision
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if output_path_ref != bundle_verify.output_path_ref {
        diagnostics.push_limited_value(
            format!(
                "promotion output-path-ref mismatch: receipt={} observed={}",
                bundle_verify.output_path_ref, output_path_ref
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if input.source_evidence.trim().is_empty() {
        diagnostics.push_limited_value(
            "source evidence marker must not be empty".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if input.octet_evidence.trim().is_empty() {
        diagnostics.push_limited_value(
            "Octet evidence marker must not be empty".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if input.cairn_evidence.trim().is_empty() {
        diagnostics.push_limited_value(
            "Cairn evidence marker must not be empty".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn promotion_key_facts(input: &ReleasePromotionGateInput<'_>) -> Result<PromotionKeyFacts> {
    match select_release_promotion_key(input) {
        Ok(key) => Ok(PromotionKeyFacts {
            selected_key_ref: key.key_ref.clone(),
            selected_key_id: key.key_id.clone(),
            selected_signer: key.signer.clone(),
            selected_trust_root: key.trust_root.clone(),
            selected_generation: key.generation,
            has_selected_key: true,
            diagnostic: None,
        }),
        Err(error) => Ok(PromotionKeyFacts {
            selected_key_ref: dogfood_ref("missing-signed-key")?,
            selected_key_id: "missing".to_string(),
            selected_signer: input.signed_signer.unwrap_or("missing").to_string(),
            selected_trust_root: input.signed_trust_root.to_string(),
            selected_generation: 0,
            has_selected_key: false,
            diagnostic: Some(format!("signed keyring currentness failed: {error}")),
        }),
    }
}
