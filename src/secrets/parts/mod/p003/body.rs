
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
        let is_decrypt_plaintext_mismatch = match input.decrypt.and_then(|decrypt| decrypt.plaintext_ref.as_deref()) {
            Some(actual) => actual != expected_plaintext_ref,
            None => false,
        };
        if reveal.plaintext_ref.as_deref() != Some(expected_plaintext_ref) || is_decrypt_plaintext_mismatch {
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
    let is_bundle_gate_preserving = match input.private_bundle {
        Some(profile) => profile.is_gate_preserving && profile.transform_receipt_ref == input.transform.receipt_ref,
        None => true,
    };
    let is_gate_preserving = input.transform.is_gate_preserving && is_bundle_gate_preserving;
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
            Err(_) => push_cleanup_diagnostic(&mut diagnostics, "secret cleanup retention tombstone invalid".to_string())?,
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
            Err(_) => push_cleanup_diagnostic(&mut diagnostics, "secret cleanup retention receipt invalid".to_string())?,
        }
    }
    let has_matching_pass = !matching_pass_refs.is_empty();
    let has_matching_tombstone = matching_pass_refs
        .iter()
        .any(|receipt_ref| tombstone_receipt_refs.contains(receipt_ref));
    if input.retention_receipts.is_empty() {
        push_cleanup_diagnostic(&mut diagnostics, "secret cleanup requires retention receipt evidence".to_string())?;
    }
    if expected_refs != actual_refs {
        push_cleanup_diagnostic(&mut diagnostics, "secret cleanup retention receipt refs mismatch".to_string())?;
    }
    if !has_matching_pass {
        push_cleanup_diagnostic(&mut diagnostics, "secret cleanup requires passing private-secret retention receipt".to_string())?;
    } else if !has_matching_tombstone {
        push_cleanup_diagnostic(&mut diagnostics, "secret cleanup retention tombstone mismatch".to_string())?;
    }
    Ok(diagnostics)
}

fn push_cleanup_diagnostic(
    diagnostics: &mut impl crate::bounded::PushLimited<String>,
    diagnostic: String,
) -> Result<()> {
    diagnostics.push_limited(diagnostic, MAX_SECRET_DIAGNOSTICS, "secret cleanup diagnostics")
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
