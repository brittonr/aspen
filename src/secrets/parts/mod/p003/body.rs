
pub fn evaluate_secret_access_binding(input: SecretAccessBindingInput<'_>) -> Result<SecretStateDecision> {
    if let Some(expected_plaintext_ref) = input.expected_plaintext_ref {
        validate_ref(expected_plaintext_ref, "secret access expected plaintext ref")?;
    }
    let mut diagnostics = Vec::new();
    let Some(reveal) = input.reveal else {
        diagnostics.push_limited(
            SECRET_ACCESS_REVEAL_MISSING.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
        return Ok(secret_decision(diagnostics, false, false, false));
    };
    if reveal.decision != "pass" {
        diagnostics.push_limited(
            SECRET_ACCESS_REVEAL_FAILED.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if reveal.secret_ref != input.secret.secret_ref {
        diagnostics.push_limited(
            SECRET_ACCESS_REVEAL_SECRET_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if reveal.encrypted_ref.as_deref() != Some(input.encrypted.encrypted_ref.as_str()) {
        diagnostics.push_limited(
            SECRET_ACCESS_REVEAL_ENCRYPTED_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if reveal.commitment_ref != input.secret.commitment_ref || reveal.commitment_ref != input.encrypted.commitment_ref {
        diagnostics.push_limited(
            SECRET_ACCESS_REVEAL_COMMITMENT_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if let Some(decrypt) = input.decrypt {
        collect_decrypt_binding_diagnostics(input, reveal, decrypt, &mut diagnostics)?;
    }
    if let Some(expected_plaintext_ref) = input.expected_plaintext_ref {
        let decrypt_plaintext_mismatch = match input.decrypt.and_then(|decrypt| decrypt.plaintext_ref.as_deref()) {
            Some(actual) => actual != expected_plaintext_ref,
            None => false,
        };
        if reveal.plaintext_ref.as_deref() != Some(expected_plaintext_ref) || decrypt_plaintext_mismatch {
            diagnostics.push_limited(
                SECRET_ACCESS_PLAINTEXT_MISMATCH.to_string(),
                MAX_SECRET_DIAGNOSTICS,
                "secret access diagnostics",
            )?;
        }
    }
    Ok(secret_decision(diagnostics, true, false, false))
}

fn collect_decrypt_binding_diagnostics(
    input: SecretAccessBindingInput<'_>,
    reveal: &RevealReceipt,
    decrypt: &DecryptReceipt,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    if decrypt.decision != "pass" {
        diagnostics.push_limited(
            SECRET_ACCESS_DECRYPT_FAILED.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if decrypt.encrypted_ref != input.encrypted.encrypted_ref {
        diagnostics.push_limited(
            SECRET_ACCESS_DECRYPT_ENCRYPTED_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if decrypt.reveal_receipt_ref.as_deref() != Some(reveal.receipt_ref.as_str()) {
        diagnostics.push_limited(
            SECRET_ACCESS_DECRYPT_REVEAL_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if decrypt.commitment_ref != input.encrypted.commitment_ref || decrypt.commitment_ref != input.secret.commitment_ref {
        diagnostics.push_limited(
            SECRET_ACCESS_DECRYPT_COMMITMENT_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    if decrypt.plaintext_ref != reveal.plaintext_ref {
        diagnostics.push_limited(
            SECRET_ACCESS_PLAINTEXT_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret access diagnostics",
        )?;
    }
    Ok(())
}

pub fn evaluate_secret_redaction_gate(input: SecretRedactionGateInput<'_>) -> Result<SecretStateDecision> {
    validate_ref(input.required_source_ref, "secret redaction required source ref")?;
    validate_ref(input.required_output_ref, "secret redaction required output ref")?;
    let mut diagnostics = Vec::new();
    if input.transform.decision != "pass"
        || input.transform.source_ref != input.required_source_ref
        || input.transform.output_ref != input.required_output_ref
    {
        diagnostics.push_limited(
            SECRET_REDACTION_PROFILE_TRANSFORM_MISMATCH.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret redaction diagnostics",
        )?;
    }
    let bundle_gate_preserving = match input.private_bundle {
        Some(profile) => profile.is_gate_preserving && profile.transform_receipt_ref == input.transform.receipt_ref,
        None => true,
    };
    let is_gate_preserving = input.transform.is_gate_preserving && bundle_gate_preserving;
    if input.requires_gate_preserving && !is_gate_preserving {
        diagnostics.push_limited(
            SECRET_REDACTION_PROFILE_DIAGNOSTIC_ONLY.to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret redaction diagnostics",
        )?;
    }
    Ok(secret_decision(diagnostics, false, is_gate_preserving, false))
}

pub fn evaluate_secret_cleanup_admission(input: &SecretCleanupInput) -> Result<SecretStateDecision> {
    validate_ref(&input.secret_ref, "cleanup secret ref")?;
    validate_ref(&input.revocation_ref, "cleanup revocation ref")?;
    validate_ref(&input.tombstone_ref, "cleanup tombstone ref")?;
    validate_refs(&input.retention_refs, "cleanup retention ref")?;
    validate_refs(&input.authority_refs, "cleanup authority ref")?;
    validate_refs(&input.policy_refs, "cleanup policy ref")?;
    ensure_count_at_most(input.retention_receipts.len(), MAX_SECRET_REFS, "cleanup retention receipts")?;
    ensure_count_at_most(input.retention_tombstones.len(), MAX_SECRET_REFS, "cleanup retention tombstones")?;
    let mut diagnostics = cleanup_retention_diagnostics(input)?;
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            "secret cleanup requires authority evidence".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() {
        diagnostics.push_limited(
            "secret cleanup requires policy evidence".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    Ok(secret_decision(diagnostics, false, false, true))
}

fn secret_decision(
    diagnostics: Vec<String>,
    plaintext_requested: bool,
    gate_preserving: bool,
    cleanup_requested: bool,
) -> SecretStateDecision {
    let is_pass = diagnostics.is_empty();
    SecretStateDecision {
        decision: if is_pass { "pass" } else { "deny" }.to_string(),
        diagnostics,
        plaintext_authorized: is_pass && plaintext_requested,
        gate_preserving: is_pass && gate_preserving,
        cleanup_authorized: is_pass && cleanup_requested,
    }
}

pub fn secret_cleanup_receipt_value(input: &SecretCleanupInput) -> Result<IoValue> {
    let cleanup_decision = evaluate_secret_cleanup_admission(input)?;
    let decision = cleanup_decision.decision.as_str();
    let diagnostics = cleanup_decision.diagnostics;
    Ok(record("secret-cleanup-receipt-v1", vec![
        string(SECRET_CLEANUP_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("secret", vec![string(&input.secret_ref)]),
        record("revocation", vec![string(&input.revocation_ref)]),
        record("tombstone", vec![string(&input.tombstone_ref)]),
        record("retention", vec![refs_sequence(&input.retention_refs)]),
        diagnostics_value(&diagnostics),
        checks_value(&secret_cleanup_checks(decision)),
    ]))
}

fn cleanup_retention_diagnostics(input: &SecretCleanupInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let expected_refs = input.retention_refs.iter().cloned().collect::<BtreeSet<_>>();
    let mut actual_refs = BtreeSet::new();
    let mut matching_pass_refs = BtreeSet::new();
    let mut tombstone_receipt_refs = BtreeSet::new();
    for tombstone_value in &input.retention_tombstones {
        match crate::retention::parse_tombstone(tombstone_value) {
            Ok(tombstone) => {
                if tombstone.tombstone_ref == input.tombstone_ref {
                    tombstone_receipt_refs.insert(tombstone.receipt_ref.clone());
                }
            }
            Err(_) => diagnostics.push_limited(
                "secret cleanup retention tombstone invalid".to_string(),
                MAX_SECRET_DIAGNOSTICS,
                "secret cleanup diagnostics",
            )?,
        }
    }
    for receipt_value in &input.retention_receipts {
        match crate::retention::parse_receipt(receipt_value) {
            Ok(receipt) => {
                actual_refs.insert(receipt.receipt_ref.clone());
                let is_cleanup_action = matches!(
                    receipt.action.as_str(),
                    crate::retention::ACTION_DELETE
                        | crate::retention::ACTION_TOMBSTONE
                        | crate::retention::ACTION_REDACT
                );
                if receipt.decision == "pass"
                    && receipt.object_ref == input.secret_ref
                    && receipt.retention_class == crate::retention::CLASS_PRIVATE_SECRET_REF
                    && is_cleanup_action
                {
                    matching_pass_refs.insert(receipt.receipt_ref.clone());
                }
            }
            Err(_) => diagnostics.push_limited(
                "secret cleanup retention receipt invalid".to_string(),
                MAX_SECRET_DIAGNOSTICS,
                "secret cleanup diagnostics",
            )?,
        }
    }
    let has_matching_pass = !matching_pass_refs.is_empty();
    let has_matching_tombstone = matching_pass_refs
        .iter()
        .any(|receipt_ref| tombstone_receipt_refs.contains(receipt_ref));
    if input.retention_receipts.is_empty() {
        diagnostics.push_limited(
            "secret cleanup requires retention receipt evidence".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    if expected_refs != actual_refs {
        diagnostics.push_limited(
            "secret cleanup retention receipt refs mismatch".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    if !has_matching_pass {
        diagnostics.push_limited(
            "secret cleanup requires passing private-secret retention receipt".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    } else if !has_matching_tombstone {
        diagnostics.push_limited(
            "secret cleanup retention tombstone mismatch".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    Ok(diagnostics)
}

pub fn parse_secret_cleanup_receipt(value: &IoValue) -> Result<SecretCleanupReceipt> {
    let fields = simple_record(value, "secret-cleanup-receipt-v1", 8)?;
    require_schema(&fields[0], SECRET_CLEANUP_RECEIPT_SCHEMA, "secret cleanup")?;
    let decision = record_decision(&fields[1])?;
    let secret_ref = record_ref(&fields[2], "secret", "cleanup secret")?;
    let revocation_ref = record_ref(&fields[3], "revocation", "cleanup revocation")?;
    let tombstone_ref = record_ref(&fields[4], "tombstone", "cleanup tombstone")?;
    let retention_refs = record_refs(&fields[5], "retention", "cleanup retention")?;
    let diagnostics = parse_diagnostics(&fields[6])?;
    if decision == "pass" {
        require_checks(&fields[7], &[
            "revocation-bound",
            "tombstone-bound",
            "retention-gc-bound",
            "idempotent-cleanup",
        ])?;
    } else {
        require_checks(&fields[7], &[
            "cleanup-denied",
            "no-plaintext-default",
            "audit-receipt",
            "retention-preserved",
        ])?;
    }
    Ok(SecretCleanupReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        secret_ref,
        revocation_ref,
        tombstone_ref,
        retention_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn private_bundle_profile_value(input: &PrivateBundleProfileInput) -> Result<IoValue> {
    validate_ref(&input.profile_ref, "private bundle profile ref")?;
    validate_refs(&input.encrypted_refs, "private bundle encrypted ref")?;
    validate_refs(&input.reveal_receipt_refs, "private bundle reveal receipt")?;
    validate_ref(&input.transform_receipt_ref, "private bundle transform receipt")?;
    let checks = if input.is_gate_preserving {
        [
            ("encrypted-ref-validation", "pass"),
            ("reveal-receipts-bound", "pass"),
            ("redaction-transform-bound", "pass"),
            ("gate-preserving-redaction", "pass"),
        ]
    } else {
        [
            ("encrypted-ref-validation", "pass"),
            ("reveal-receipts-bound", "pass"),
            ("redaction-transform-bound", "pass"),
            ("diagnostic-only", "pass"),
        ]
    };
    Ok(record("private-bundle-profile-v1", vec![
        string(PRIVATE_BUNDLE_PROFILE_SCHEMA),
        record("profile", vec![string(&input.profile_ref)]),
        record("encrypted-refs", vec![refs_sequence(&input.encrypted_refs)]),
        record("reveal-receipts", vec![refs_sequence(&input.reveal_receipt_refs)]),
        record("transform-receipt", vec![string(&input.transform_receipt_ref)]),
        record("gate-preserving", vec![bool_value(input.is_gate_preserving)]),
        checks_value(&checks),
    ]))
}

pub fn parse_private_bundle_profile(value: &IoValue) -> Result<PrivateBundleProfile> {
    let fields = simple_record(value, "private-bundle-profile-v1", 7)?;
    require_schema(&fields[0], PRIVATE_BUNDLE_PROFILE_SCHEMA, "private bundle profile")?;
    let profile_ref = record_ref(&fields[1], "profile", "private bundle profile ref")?;
    let encrypted_refs = record_refs(&fields[2], "encrypted-refs", "private bundle encrypted refs")?;
    let reveal_receipt_refs = record_refs(&fields[3], "reveal-receipts", "private bundle reveal receipts")?;
    let transform_receipt_ref = record_ref(&fields[4], "transform-receipt", "private bundle transform receipt")?;
    let is_gate_preserving = record_bool(&fields[5], "gate-preserving", "private bundle gate preserving")?;
    if is_gate_preserving {
        require_checks(&fields[6], &[
            "encrypted-ref-validation",
            "reveal-receipts-bound",
            "redaction-transform-bound",
            "gate-preserving-redaction",
        ])?;
    } else {
        require_checks(&fields[6], &[
            "encrypted-ref-validation",
            "reveal-receipts-bound",
            "redaction-transform-bound",
            "diagnostic-only",
        ])?;
    }
    Ok(PrivateBundleProfile {
        profile_ref,
        encrypted_refs,
        reveal_receipt_refs,
        transform_receipt_ref,
        is_gate_preserving,
        value: value.clone(),
    })
}

pub fn contains_secret_marker(value: &IoValue) -> Result<bool> {
    let text = to_text(value)?;
    Ok(SENSITIVE_RECORD_LABELS.iter().any(|label| text.contains(&format!("<{label}"))))
}

pub fn redacted_value(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<IoValue> {
    Ok(redacted_view(value, redaction_profile_ref)?.value)
}

pub fn redacted_view(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<RedactedValue> {
    if !contains_secret_marker(value)? {
        return Ok(RedactedValue {
            value: value.clone(),
            marker: None,
            transform_receipt: None,
        });
    }
    let source_ref = canonical_hash(value)?;
    let policy_refs = vec![DEFAULT_REDACTION_POLICY.to_string()];
    let profile_ref = redaction_profile_ref.unwrap_or(DEFAULT_REDACTION_PROFILE).to_string();
    validate_ref(&profile_ref, "redaction profile ref")?;
    let reason = first_redaction_reason(value)?;
    let path_ref = fixture_ref(&format!("redaction-path:{source_ref}"));
    let marker_receipt_ref = redaction_seed_ref(&source_ref, &profile_ref, &policy_refs)?;
    let marker_value = redaction_marker_value(&RedactionMarkerInput {
        reason,
        commitment_ref: source_ref.clone(),
        schema_ref: fixture_ref("redacted-source-schema"),
        path_ref,
        policy_refs: policy_refs.clone(),
        receipt_ref: marker_receipt_ref,
    })?;
    let marker = parse_redaction_marker(&marker_value)?;
    let output_ref = canonical_hash(&marker.value)?;
    let transform_value = redaction_transform_receipt_value(&RedactionTransformInput {
        source_ref,
        output_ref,
        policy_refs,
        profile_ref,
        marker_refs: vec![marker.marker_ref.clone()],
        is_gate_preserving: true,
        diagnostics: Vec::new(),
    })?;
    let transform_receipt = parse_redaction_transform_receipt(&transform_value)?;
    Ok(RedactedValue {
        value: marker.value.clone(),
        marker: Some(marker),
        transform_receipt: Some(transform_receipt),
    })
}

pub fn redacted_text(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<String> {
    to_text(&redacted_value(value, redaction_profile_ref)?)
}

pub fn secrets_summary(value: &IoValue) -> Result<String> {
    let kind = crate::ledger::artifact_kind(value);
    if let Some(line) = summary_core(kind, value)? {
        return Ok(line);
    }
    if let Some(line) = summary_receipts(kind, value)? {
        return Ok(line);
    }
    if let Some(line) = summary_profiles(kind, value)? {
        return Ok(line);
    }
    Err(MoltenError::invalid_harness("not a secrets artifact"))
}
