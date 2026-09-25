
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
