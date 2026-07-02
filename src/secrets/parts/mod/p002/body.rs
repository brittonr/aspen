
pub fn parse_reveal_receipt(value: &IoValue) -> Result<RevealReceipt> {
    let fields =
        simple_record(value, "reveal-receipt-v1", 10).or_else(|_| simple_record(value, "reveal-receipt-v1", 9))?;
    let arity = fields.fields_iter().count();
    require_schema(&fields[0], SECRET_REVEAL_RECEIPT_SCHEMA, "reveal receipt")?;
    let decision = record_decision(&fields[1])?;
    let secret_ref = record_ref(&fields[2], "secret", "reveal secret ref")?;
    let (encrypted_ref, requester_ref, purpose, plaintext_ref, commitment_ref, diagnostics, checks_index) =
        if arity == 10 {
            (
                record_optional_ref(&fields[3], "encrypted-ref", "reveal encrypted ref")?,
                record_ref(&fields[4], "requester", "reveal requester ref")?,
                record_string(&fields[5], "purpose", "reveal purpose")?,
                record_optional_ref(&fields[6], "plaintext-ref", "reveal plaintext ref")?,
                record_ref(&fields[7], "commitment", "reveal commitment")?,
                parse_diagnostics(&fields[8])?,
                9usize,
            )
        } else {
            (
                None,
                record_ref(&fields[3], "requester", "reveal requester ref")?,
                record_string(&fields[4], "purpose", "reveal purpose")?,
                record_optional_ref(&fields[5], "plaintext-ref", "reveal plaintext ref")?,
                record_ref(&fields[6], "commitment", "reveal commitment")?,
                parse_diagnostics(&fields[7])?,
                8usize,
            )
        };
    validate_purpose(&purpose)?;
    let required = if decision == "pass" {
        [
            "authorized-reveal",
            "policy-bound",
            "resource-bound",
            "effect-handle-bound",
        ]
    } else {
        [
            "deny-without-authority",
            "no-plaintext-on-deny",
            "ciphertext-not-authority",
            "audit-receipt",
        ]
    };
    require_checks(&fields[checks_index], &required)?;
    if encrypted_ref.is_some() {
        require_checks(&fields[checks_index], &["encrypted-ref-bound"])?;
    }
    Ok(RevealReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        secret_ref,
        encrypted_ref,
        requester_ref,
        purpose,
        plaintext_ref,
        commitment_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn decrypt_receipt_value(input: &DecryptReceiptInput) -> Result<IoValue> {
    validate_ref(&input.encrypted_ref, "decrypt encrypted ref")?;
    validate_ref(&input.requester_ref, "decrypt requester ref")?;
    validate_purpose(&input.purpose)?;
    validate_optional_ref(input.plaintext_ref.as_deref(), "decrypt plaintext ref")?;
    validate_ref(&input.commitment_ref, "decrypt commitment ref")?;
    validate_ref(&input.expected_commitment_ref, "decrypt expected commitment ref")?;
    validate_optional_ref(input.reveal_receipt_ref.as_deref(), "decrypt reveal receipt ref")?;
    validate_refs(&input.authority_refs, "decrypt authority ref")?;
    validate_refs(&input.policy_refs, "decrypt policy ref")?;
    validate_refs(&input.resource_refs, "decrypt resource ref")?;
    validate_refs(&input.effect_handle_refs, "decrypt effect handle ref")?;
    let mut diagnostics = Vec::new();
    collect_gate_diagnostics(
        AccessGateInput {
            authority_refs: &input.authority_refs,
            policy_refs: &input.policy_refs,
            resource_refs: &input.resource_refs,
            effect_handle_refs: &input.effect_handle_refs,
            revocation_refs: &[],
            operation: "decrypt",
        },
        &mut diagnostics,
    )?;
    if !input.has_reveal_authority || input.reveal_receipt_ref.is_none() {
        diagnostics.push_limited(
            "decrypt requires a passing reveal receipt; encrypted refs alone are not authority".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "decrypt diagnostics",
        )?;
    }
    if input.commitment_ref != input.expected_commitment_ref {
        diagnostics.push_limited(
            "decrypt commitment does not match encrypted ref commitment".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "decrypt diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let plaintext_ref = if decision == "pass" {
        input.plaintext_ref.as_deref()
    } else {
        None
    };
    Ok(record("decrypt-receipt-v1", vec![
        string(SECRET_DECRYPT_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("encrypted-ref", vec![string(&input.encrypted_ref)]),
        record("requester", vec![string(&input.requester_ref)]),
        record("purpose", vec![string(&input.purpose)]),
        record("plaintext-ref", vec![optional_ref_value(plaintext_ref)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        record("reveal-receipt", vec![optional_ref_value(input.reveal_receipt_ref.as_deref())]),
        diagnostics_value(&diagnostics),
        checks_value(&decrypt_checks(decision)),
    ]))
}

pub fn parse_decrypt_receipt(value: &IoValue) -> Result<DecryptReceipt> {
    let fields = simple_record(value, "decrypt-receipt-v1", 10)?;
    require_schema(&fields[0], SECRET_DECRYPT_RECEIPT_SCHEMA, "decrypt receipt")?;
    let decision = record_decision(&fields[1])?;
    let encrypted_ref = record_ref(&fields[2], "encrypted-ref", "decrypt encrypted ref")?;
    let requester_ref = record_ref(&fields[3], "requester", "decrypt requester ref")?;
    let purpose = record_string(&fields[4], "purpose", "decrypt purpose")?;
    validate_purpose(&purpose)?;
    let plaintext_ref = record_optional_ref(&fields[5], "plaintext-ref", "decrypt plaintext ref")?;
    let commitment_ref = record_ref(&fields[6], "commitment", "decrypt commitment")?;
    let reveal_receipt_ref = record_optional_ref(&fields[7], "reveal-receipt", "decrypt reveal receipt ref")?;
    let diagnostics = parse_diagnostics(&fields[8])?;
    let required = if decision == "pass" {
        [
            "authorized-decrypt",
            "reveal-receipt-bound",
            "commitment-match",
            "effect-handle-bound",
        ]
    } else {
        [
            "deny-without-reveal",
            "no-plaintext-on-deny",
            "ciphertext-not-authority",
            "audit-receipt",
        ]
    };
    require_checks(&fields[9], &required)?;
    Ok(DecryptReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        encrypted_ref,
        requester_ref,
        purpose,
        plaintext_ref,
        commitment_ref,
        reveal_receipt_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn redaction_transform_receipt_value(input: &RedactionTransformInput) -> Result<IoValue> {
    validate_ref(&input.source_ref, "redaction source ref")?;
    validate_ref(&input.output_ref, "redaction output ref")?;
    validate_refs(&input.policy_refs, "redaction policy ref")?;
    validate_ref(&input.profile_ref, "redaction profile ref")?;
    validate_refs(&input.marker_refs, "redaction marker ref")?;
    validate_diagnostics(&input.diagnostics, "redaction diagnostics")?;
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("redaction-transform-receipt-v1", vec![
        string(SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("source", vec![string(&input.source_ref)]),
        record("output", vec![string(&input.output_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("profile", vec![string(&input.profile_ref)]),
        record("markers", vec![refs_sequence(&input.marker_refs)]),
        record("gate-preserving", vec![bool_value(input.is_gate_preserving)]),
        diagnostics_value(&input.diagnostics),
        checks_value(&redaction_transform_checks(decision, input.is_gate_preserving)),
    ]))
}

pub fn parse_redaction_transform_receipt(value: &IoValue) -> Result<RedactionTransformReceipt> {
    let fields = simple_record(value, "redaction-transform-receipt-v1", 10)?;
    require_schema(&fields[0], SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA, "redaction transform")?;
    let decision = record_decision(&fields[1])?;
    let source_ref = record_ref(&fields[2], "source", "redaction source")?;
    let output_ref = record_ref(&fields[3], "output", "redaction output")?;
    let _policy_refs = record_refs(&fields[4], "policy", "redaction policy")?;
    let _profile_ref = record_ref(&fields[5], "profile", "redaction profile")?;
    let marker_refs = record_refs(&fields[6], "markers", "redaction markers")?;
    let is_gate_preserving = record_bool(&fields[7], "gate-preserving", "redaction gate preserving")?;
    let diagnostics = parse_diagnostics(&fields[8])?;
    if is_gate_preserving {
        require_checks(&fields[9], &[
            "source-ref-bound",
            "output-ref-bound",
            "marker-ref-bound",
            "semantic-evidence-preserved",
        ])?;
    } else {
        require_checks(&fields[9], &[
            "source-ref-bound",
            "output-ref-bound",
            "marker-ref-bound",
            "diagnostic-only",
        ])?;
    }
    Ok(RedactionTransformReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        source_ref,
        output_ref,
        marker_refs,
        is_gate_preserving,
        diagnostics,
        value: value.clone(),
    })
}

pub fn commitment_replay_receipt_value(input: &CommitmentReplayInput) -> Result<IoValue> {
    validate_ref(&input.expected_commitment_ref, "expected commitment")?;
    validate_ref(&input.actual_commitment_ref, "actual commitment")?;
    validate_optional_ref(input.reveal_receipt_ref.as_deref(), "commitment replay reveal receipt")?;
    let mut diagnostics = Vec::new();
    if input.expected_commitment_ref != input.actual_commitment_ref {
        diagnostics.push_limited(
            "secret commitment mismatch during replay".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "commitment replay diagnostics",
        )?;
    }
    if input.is_plaintext_required && input.reveal_receipt_ref.is_none() {
        diagnostics.push_limited(
            "plaintext-required replay needs recorded effect response or reveal receipt".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "commitment replay diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("commitment-replay-receipt-v1", vec![
        string(SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("expected", vec![string(&input.expected_commitment_ref)]),
        record("actual", vec![string(&input.actual_commitment_ref)]),
        record("reveal-receipt", vec![optional_ref_value(input.reveal_receipt_ref.as_deref())]),
        record("plaintext-required", vec![bool_value(input.is_plaintext_required)]),
        diagnostics_value(&diagnostics),
        checks_value(&commitment_replay_checks(decision, input.is_plaintext_required)),
    ]))
}

pub fn parse_commitment_replay_receipt(value: &IoValue) -> Result<CommitmentReplayReceipt> {
    let fields = simple_record(value, "commitment-replay-receipt-v1", 8)?;
    require_schema(&fields[0], SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA, "commitment replay")?;
    let decision = record_decision(&fields[1])?;
    let expected_commitment_ref = record_ref(&fields[2], "expected", "expected commitment")?;
    let actual_commitment_ref = record_ref(&fields[3], "actual", "actual commitment")?;
    let reveal_receipt_ref = record_optional_ref(&fields[4], "reveal-receipt", "commitment replay reveal")?;
    let is_plaintext_required = record_bool(&fields[5], "plaintext-required", "plaintext required")?;
    let diagnostics = parse_diagnostics(&fields[6])?;
    if decision == "pass" {
        require_checks(&fields[7], &["commitment-match", "plaintext-not-required", "replay-without-plaintext"])?;
    } else if is_plaintext_required {
        require_checks(&fields[7], &["commitment-comparison", "plaintext-required", "diagnostic-only"])?;
    } else {
        require_checks(&fields[7], &["commitment-mismatch", "fail-closed", "audit-receipt"])?;
    }
    Ok(CommitmentReplayReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        expected_commitment_ref,
        actual_commitment_ref,
        reveal_receipt_ref,
        diagnostics,
        value: value.clone(),
    })
}
