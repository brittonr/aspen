
pub fn parse_release_promotion_gate_receipt(value: &IoValue) -> Result<ReleasePromotionGateReceipt> {
    let fields = value
        .collect_simple_record("release-promotion-gate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-promotion-gate-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA,
        "release promotion gate receipt",
    )?;
    let bundle_value = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let bundle_fields = simple_record(&bundle_value, "bundle-verify", 7)?;
    let keyring_value = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let keyring_fields = simple_record(&keyring_value, "signed-keyring", 2)?;
    let selected_key_value = crate::preserves_rail::value_to_iovalue(&keyring_fields[0]);
    let selected_key_fields = simple_record(&selected_key_value, "selected-key", 5)?;
    let evidence_value = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let evidence_fields = simple_record(&evidence_value, "evidence", 3)?;
    let source_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[0]);
    let source_fields = simple_record(&source_value, "source", 2)?;
    let octet_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[1]);
    let octet_fields = simple_record(&octet_value, "octet", 2)?;
    let cairn_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[2]);
    let cairn_fields = simple_record(&cairn_value, "cairn", 2)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "release-bundle-verify-pass", "release promotion gate receipt")?;
    require_check(&checks, "promotion-output-path-bound", "release promotion gate receipt")?;
    require_check(&checks, "signed-keyring-current", "release promotion gate receipt")?;
    require_check(&checks, "source-evidence-bound", "release promotion gate receipt")?;
    require_check(&checks, "octet-evidence-bound", "release promotion gate receipt")?;
    require_check(&checks, "cairn-evidence-bound", "release promotion gate receipt")?;
    require_check(&checks, "release-promotion-is-evidence-only", "release promotion gate receipt")?;
    require_check(&checks, "no-subsystem-authority-granted", "release promotion gate receipt")?;
    Ok(ReleasePromotionGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        bundle_verify_ref: required_ref(&bundle_fields[0], "release promotion bundle verify receipt ref")?,
        bundle_ref: required_ref(&bundle_fields[1], "release promotion bundle ref")?,
        output_path_ref: required_ref(&bundle_fields[2], "release promotion output path ref")?,
        selected_key_ref: required_ref(&selected_key_fields[0], "release promotion signed key ref")?,
        source_ref: required_ref(&source_fields[1], "release promotion source evidence ref")?,
        octet_ref: required_ref(&octet_fields[1], "release promotion Octet evidence ref")?,
        cairn_ref: required_ref(&cairn_fields[1], "release promotion Cairn evidence ref")?,
        diagnostics: record_string_sequence(&fields[5], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

struct GateReadback {
    promotion: Option<ReleasePromotionGateReceipt>,
    diagnostics: Vec<String>,
}

struct SummarySigned {
    envelope_ref: String,
    subject_ref: String,
    key_ref: String,
}

struct SignedReadback {
    signed: Option<SummarySigned>,
    diagnostics: Vec<String>,
}

struct SummaryFacts {
    promotion: Option<ReleasePromotionGateReceipt>,
    signed: Option<SummarySigned>,
    diagnostics: Vec<String>,
}

struct SummaryRefs {
    promotion_ref: String,
    promotion_decision: String,
    bundle_verify_ref: String,
    bundle_ref: String,
    source_ref: String,
    octet_ref: String,
    cairn_ref: String,
    signed_envelope_ref: String,
    signed_subject_ref: String,
    signed_key_ref: String,
}

pub fn release_promotion_summary_value(input: &ReleasePromotionSummaryInput<'_>) -> Result<ReleasePromotionSummary> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let facts = summary_facts(input, &output_path_ref)?;
    let refs = summary_refs(&facts)?;
    let value = summary_record(&output_path_string, &output_path_ref, &facts, &refs);
    parse_release_promotion_summary(&value)
}

fn summary_facts(input: &ReleasePromotionSummaryInput<'_>, output_path_ref: &str) -> Result<SummaryFacts> {
    let gate = read_summary_gate(input, output_path_ref)?;
    let expected_subject_ref = gate.promotion.as_ref().map(|promotion| promotion.receipt_ref.as_str());
    let signed = read_signed_summary(input, expected_subject_ref)?;
    let mut diagnostics = gate.diagnostics;
    for diagnostic in signed.diagnostics {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion summary diagnostics",
        )?;
    }
    Ok(SummaryFacts {
        promotion: gate.promotion,
        signed: signed.signed,
        diagnostics,
    })
}

fn read_summary_gate(input: &ReleasePromotionSummaryInput<'_>, output_path_ref: &str) -> Result<GateReadback> {
    let mut diagnostics = Vec::new();
    let promotion_result = read_output_text(input.output_path, "release-promotion-gate.preserves")
        .and_then(|text| crate::preserves_rail::parse_text(&text))
        .and_then(|value| parse_release_promotion_gate_receipt(&value));
    let promotion = match promotion_result {
        Ok(promotion) => Some(promotion),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("release promotion gate receipt readback failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release promotion summary diagnostics",
            )?;
            None
        }
    };
    if let Some(promotion) = promotion.as_ref() {
        for diagnostic in summary_gate_diagnostics(promotion, output_path_ref)? {
            diagnostics.push_limited_value(
                diagnostic,
                MAX_OPERATOR_DIAGNOSTICS,
                "release promotion summary diagnostics",
            )?;
        }
    }
    Ok(GateReadback { promotion, diagnostics })
}

fn summary_gate_diagnostics(promotion: &ReleasePromotionGateReceipt, output_path_ref: &str) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if promotion.decision != "pass" {
        diagnostics.push_limited_value(
            format!("release promotion gate receipt {} decision is {}", promotion.receipt_ref, promotion.decision),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion summary diagnostics",
        )?;
    }
    if promotion.output_path_ref != output_path_ref {
        diagnostics.push_limited_value(
            format!(
                "release promotion summary output-path-ref mismatch: receipt={} observed={}",
                promotion.output_path_ref, output_path_ref
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion summary diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn read_signed_summary(
    input: &ReleasePromotionSummaryInput<'_>,
    expected_subject_ref: Option<&str>,
) -> Result<SignedReadback> {
    let signed_result = read_output_text(input.output_path, "release-promotion-gate.signed.preserves")
        .and_then(|text| crate::preserves_rail::parse_text(&text))
        .and_then(|value| {
            verify_signed_receipt_with_keyring_policy(&value, &VerifySignedReceiptKeyringPolicy {
                required_purpose: RELEASE_PROMOTION_SIGNING_PURPOSE,
                trust_root: input.signed_trust_root,
                expected_signer: input.signed_signer,
                expected_subject_ref,
                required_key_ref: input.signed_key_ref,
                required_key_id: input.signed_key_id,
                keys: input.signed_keys,
                revocations: input.signed_key_revocations,
            })
        });
    match signed_result {
        Ok(signed) => Ok(SignedReadback {
            signed: Some(SummarySigned {
                envelope_ref: signed.receipt.envelope_ref,
                subject_ref: signed.receipt.subject_ref,
                key_ref: signed.key_ref,
            }),
            diagnostics: Vec::new(),
        }),
        Err(error) => Ok(SignedReadback {
            signed: None,
            diagnostics: vec![format!("signed promotion receipt verification failed: {error}")],
        }),
    }
}

fn summary_refs(facts: &SummaryFacts) -> Result<SummaryRefs> {
    let promotion_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-release-promotion-gate"), |promotion| Ok(promotion.receipt_ref.clone()))?;
    let promotion_decision = facts
        .promotion
        .as_ref()
        .map_or_else(|| "missing".to_string(), |promotion| promotion.decision.clone());
    let bundle_verify_ref = facts.promotion.as_ref().map_or_else(
        || dogfood_ref("missing-release-bundle-verify"),
        |promotion| Ok(promotion.bundle_verify_ref.clone()),
    )?;
    let bundle_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-release-evidence-bundle"), |promotion| Ok(promotion.bundle_ref.clone()))?;
    let source_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-source-evidence"), |promotion| Ok(promotion.source_ref.clone()))?;
    let octet_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-octet-evidence"), |promotion| Ok(promotion.octet_ref.clone()))?;
    let cairn_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-cairn-evidence"), |promotion| Ok(promotion.cairn_ref.clone()))?;
    let signed_envelope_ref = facts
        .signed
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-signed-release-promotion"), |signed| Ok(signed.envelope_ref.clone()))?;
    let signed_subject_ref = facts.signed.as_ref().map_or_else(
        || dogfood_ref("missing-signed-release-promotion-subject"),
        |signed| Ok(signed.subject_ref.clone()),
    )?;
    let signed_key_ref = facts
        .signed
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-signed-release-key"), |signed| Ok(signed.key_ref.clone()))?;
    Ok(SummaryRefs {
        promotion_ref,
        promotion_decision,
        bundle_verify_ref,
        bundle_ref,
        source_ref,
        octet_ref,
        cairn_ref,
        signed_envelope_ref,
        signed_subject_ref,
        signed_key_ref,
    })
}
