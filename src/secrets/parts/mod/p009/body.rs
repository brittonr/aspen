
pub fn reveal_receipt_value(input: &RevealReceiptInput) -> Result<IoValue> {
    validate_ref(&input.secret_ref, "reveal secret ref")?;
    validate_optional_ref(input.encrypted_ref.as_deref(), "reveal encrypted ref")?;
    validate_ref(&input.requester_ref, "reveal requester ref")?;
    validate_purpose(&input.purpose)?;
    validate_optional_ref(input.plaintext_ref.as_deref(), "reveal plaintext ref")?;
    validate_ref(&input.commitment_ref, "reveal commitment ref")?;
    validate_refs(&input.authority_refs, "reveal authority ref")?;
    validate_refs(&input.policy_refs, "reveal policy ref")?;
    validate_refs(&input.resource_refs, "reveal resource ref")?;
    validate_refs(&input.effect_handle_refs, "reveal effect handle ref")?;
    validate_refs(&input.revocation_refs, "reveal revocation ref")?;
    let mut diagnostics = Vec::new();
    collect_gate_diagnostics(
        AccessGateInput {
            authority_refs: &input.authority_refs,
            policy_refs: &input.policy_refs,
            resource_refs: &input.resource_refs,
            effect_handle_refs: &input.effect_handle_refs,
            revocation_refs: &input.revocation_refs,
            operation: "reveal",
        },
        &mut diagnostics,
    )?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let plaintext_ref = if decision == "pass" {
        input.plaintext_ref.as_deref()
    } else {
        None
    };
    Ok(record("reveal-receipt-v1", vec![
        string(SECRET_REVEAL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("secret", vec![string(&input.secret_ref)]),
        record("encrypted-ref", vec![optional_ref_value(input.encrypted_ref.as_deref())]),
        record("requester", vec![string(&input.requester_ref)]),
        record("purpose", vec![string(&input.purpose)]),
        record("plaintext-ref", vec![optional_ref_value(plaintext_ref)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        diagnostics_value(&diagnostics),
        checks_value(&reveal_checks(decision, input.encrypted_ref.is_some())),
    ]))
}
