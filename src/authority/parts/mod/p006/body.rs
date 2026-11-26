fn parse_revocations(revocation_values: &[IoValue]) -> Result<Vec<Revocation>> {
    revocation_values.iter().map(parse_revocation).collect()
}

/// Capability, scope, validity-window and revocation diagnostics shared by every admission path, in check
/// order.
fn scope_diagnostics(check: ScopeCheck<'_>) -> impl Iterator<Item = String> {
    let is_capability_denied = !check.context.capabilities.iter().any(|capability| {
        capability_allows_current_action(
            capability,
            check.requested_capability,
            check.requested_operation,
            check.requested_scope,
        )
    });
    let is_premature = check.context.not_before.is_some_and(|not_before| check.logical_time < not_before);
    let is_expired = check.context.expires_at.is_some_and(|expires_at| check.logical_time >= expires_at);
    let is_revoked = check
        .revocations
        .iter()
        .any(|revocation| revocation_hits_context(revocation, check.context, check.logical_time));
    [
        (is_capability_denied, "capability-denied"),
        (is_premature, "not-yet-valid"),
        (is_expired, "expired"),
        (is_revoked, "revoked"),
    ]
    .into_iter()
    .filter(|(is_failed, _)| *is_failed)
    .map(|(_, diagnostic)| diagnostic.to_string())
}

fn admission_with_receipt(input: AdmissionReceiptInput<'_>) -> Result<Admission> {
    let diagnostics = input.diagnostics.iter().map(String::as_str).collect::<Vec<_>>();
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: input.operation,
        decision: &input.decision,
        context_ref: Some(&input.context.context_ref),
        capability: input.capability,
        scope: input.scope,
        logical_time: input.logical_time,
        diagnostics: &diagnostics,
    });
    Ok(Admission {
        decision: input.decision.clone(),
        receipt: Receipt {
            receipt_ref: canonical_hash(&receipt_value)?,
            operation: input.operation.to_string(),
            decision: input.decision,
            context_ref: Some(input.context.context_ref.clone()),
            value: receipt_value,
        },
    })
}

pub fn replay_verify_receipt(receipt: &Receipt, context_value: &IoValue) -> Result<()> {
    let context = parse_context(context_value)?;
    if receipt.context_ref.as_deref() != Some(context.context_ref.as_str()) {
        return Err(MoltenError::invalid_harness("replay authority receipt does not bind recorded context"));
    }
    Ok(())
}

pub fn receipt_value(input: ReceiptValueInput<'_>) -> IoValue {
    record("authority-receipt-v1", vec![
        string(crate::preserves_rail::AUTHORITY_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("authority-context", vec![optional_ref_value(input.context_ref)]),
        record("request", vec![
            record("capability", vec![string(input.capability)]),
            record("scope", vec![string(input.scope)]),
            record("logical-time", vec![u64_value(input.logical_time)]),
        ]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("authority-context-recorded"), string("pass")]),
            record("check", vec![string("revocation-checked"), string("pass")]),
            record("check", vec![string("expiry-checked"), string("pass")]),
            record("check", vec![string("replay-does-not-mint-authority"), string("pass")]),
        ])]),
    ])
}

fn capability_value(capability: &Capability) -> IoValue {
    record("capability", vec![
        record("name", vec![string(&capability.capability)]),
        record("scope", vec![string(&capability.scope)]),
        record("attenuation", vec![string(&capability.attenuation)]),
    ])
}

fn parse_capability(value: &IoValue) -> Result<Capability> {
    let fields = value
        .collect_simple_record("capability", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected authority capability"))?;
    let capability = Capability {
        capability: record_string(&fields[0], "name")?,
        scope: record_string(&fields[1], "scope")?,
        attenuation: record_string(&fields[2], "attenuation")?,
    };
    validate_capability(&capability)?;
    Ok(capability)
}

fn parse_capability_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Capability>> {
    let values = field_sequence(value, label)?;
    values.iter().map(|value| parse_capability(&value_to_iovalue(value))).collect()
}
