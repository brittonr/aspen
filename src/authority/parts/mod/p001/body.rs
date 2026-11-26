
pub fn revocation_value(input: RevocationValueInput<'_>) -> Result<IoValue> {
    validate_revocation_target(input.target_kind)?;
    require_ref(input.target_ref, "authority revocation target ref")?;
    validate_non_empty(input.reason, "authority revocation reason")?;
    require_ref(input.issuer_ref, "authority revocation issuer ref")?;
    validate_refs(input.evidence_refs, "authority revocation evidence ref")?;
    Ok(record("authority-revocation-v1", vec![
        string(crate::preserves_rail::AUTHORITY_REVOCATION_SCHEMA),
        record("target", vec![
            record("kind", vec![string(input.target_kind)]),
            record("ref", vec![string(input.target_ref)]),
        ]),
        record("reason", vec![string(input.reason)]),
        record("effective-at", vec![u64_value(input.effective_at)]),
        record("issuer", vec![string(input.issuer_ref)]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("revocation-target-bound"), string("pass")]),
            record("check", vec![string("authority-cleanup-required"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_revocation(value: &IoValue) -> Result<Revocation> {
    let fields = value
        .collect_simple_record("authority-revocation-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <authority-revocation-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::AUTHORITY_REVOCATION_SCHEMA, "authority revocation schema")?;
    let target = value_to_iovalue(&fields[1]);
    let target_fields = target
        .collect_simple_record("target", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("authority revocation missing target"))?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "authority-cleanup-required")?;
    Ok(Revocation {
        revocation_ref: canonical_hash(value)?,
        target_kind: record_string(&target_fields[0], "kind")?,
        target_ref: record_string(&target_fields[1], "ref")?,
        reason: record_string(&fields[2], "reason")?,
        effective_at: record_u64(&fields[3], "effective-at")?,
        issuer_ref: record_string(&fields[4], "issuer")?,
        evidence_refs: parse_ref_sequence(&fields[5], "evidence")?,
        value: value.clone(),
    })
}

// r[impl molten.authority.independent_currentness_and_revocation_bounds]
pub fn authority_grant_currentness(input: AuthorityGrantCurrentnessInput<'_>) -> Result<AuthorityGrantCurrentness> {
    validate_non_empty(input.requested_principal_ref, "authority requested principal ref")?;
    validate_non_empty(input.requested_capability, "authority requested capability")?;
    validate_non_empty(input.requested_operation, "authority requested operation")?;
    validate_non_empty(input.requested_scope, "authority requested scope")?;
    let context_refs = nominal::admit_context_refs(input.context)?;
    let request_refs = nominal::admit_currentness_request(
        input.requested_principal_ref,
        input.requested_operation,
        input.current_key_refs,
    )?;

    let mut diagnostics = Vec::with_capacity(AUTHORITY_CURRENTNESS_DIAGNOSTIC_CAPACITY);
    if context_refs.subject != request_refs.principal {
        diagnostics.push("principal-mismatch".to_string());
    }
    if input.grant_epoch < input.minimum_epoch {
        diagnostics.push("stale-epoch".to_string());
    }
    if input.grant_epoch > input.current_epoch {
        diagnostics.push("not-yet-current-epoch".to_string());
    }
    // A context must name its signing keys, and every one of them must be current: one current key
    // never vouches for a stale sibling.
    if context_refs.keys.is_empty() {
        diagnostics.push("no-signing-keys".to_string());
    } else if context_refs.keys.iter().any(|key| !request_refs.current_keys.contains(key)) {
        diagnostics.push("key-not-current".to_string());
    }
    push_scope_diagnostics(
        ScopeCheck {
            context: input.context,
            requested_capability: input.requested_capability,
            requested_operation: input.requested_operation,
            requested_scope: input.requested_scope,
            logical_time: input.logical_time,
            revocations: input.revocations,
        },
        &mut diagnostics,
    );
    let decision = if diagnostics.is_empty() { "pass" } else { "fail" };
    Ok(AuthorityGrantCurrentness {
        decision: decision.to_string(),
        diagnostics,
    })
}

/// Admits a context against caller-supplied, context-independent facts: the expected principal,
/// minimum grant epoch and current key set never come from the context being judged.
// r[impl molten.authority.independent_currentness_and_revocation_bounds]
pub fn admit_authority(input: AuthorityAdmissionInput<'_>) -> Result<Admission> {
    let context = parse_context(input.context_value)?;
    let revocations = parse_revocations(input.revocation_values)?;
    let currentness = authority_grant_currentness(AuthorityGrantCurrentnessInput {
        context: &context,
        requested_principal_ref: input.expected_principal_ref,
        requested_capability: input.requested_capability,
        requested_operation: input.requested_capability,
        requested_scope: input.requested_scope,
        logical_time: input.logical_time,
        grant_epoch: context.not_before.unwrap_or(AUTHORITY_DEFAULT_GRANT_EPOCH),
        minimum_epoch: input.minimum_epoch,
        current_epoch: input.logical_time,
        current_key_refs: input.current_key_refs,
        revocations: &revocations,
    })?;
    admission_with_receipt(AdmissionReceiptInput {
        operation: "admission",
        decision: currentness.decision,
        context: &context,
        capability: input.requested_capability,
        scope: input.requested_scope,
        logical_time: input.logical_time,
        diagnostics: &currentness.diagnostics,
    })
}

/// Capability-scope admission for callers with no independent principal or key source (job DAG
/// admission). It checks capability, scope, attenuation, validity window and revocation only. Its
/// receipt always records `principal-and-key-currentness-not-claimed`, so a pass is never evidence of
/// caller identity or key currentness.
// r[impl molten.authority.independent_currentness_and_revocation_bounds]
pub fn admit_capability_scope(
    context_value: &IoValue,
    requested_capability: &str,
    requested_scope: &str,
    logical_time: u64,
    revocation_values: &[IoValue],
) -> Result<Admission> {
    validate_non_empty(requested_capability, "authority requested capability")?;
    validate_non_empty(requested_scope, "authority requested scope")?;
    let context = parse_context(context_value)?;
    let revocations = parse_revocations(revocation_values)?;
    let mut diagnostics = Vec::with_capacity(AUTHORITY_CURRENTNESS_DIAGNOSTIC_CAPACITY);
    push_scope_diagnostics(
        ScopeCheck {
            context: &context,
            requested_capability,
            requested_operation: requested_capability,
            requested_scope,
            logical_time,
            revocations: &revocations,
        },
        &mut diagnostics,
    );
    let decision = if diagnostics.is_empty() { "pass" } else { "fail" };
    diagnostics.push(CAPABILITY_SCOPE_NON_CLAIM.to_string());
    admission_with_receipt(AdmissionReceiptInput {
        operation: "capability-scope-admission",
        decision: decision.to_string(),
        context: &context,
        capability: requested_capability,
        scope: requested_scope,
        logical_time,
        diagnostics: &diagnostics,
    })
}

// r[impl molten.authority.independent_currentness_and_revocation_bounds]
pub fn gatekeeper_resolve_live_ref(input: AuthorityAdmissionInput<'_>) -> Result<LiveRef> {
    let admission = admit_authority(input)?;
    if admission.decision != "pass" {
        return Err(MoltenError::invalid_harness("gatekeeper resolution denied by authority context"));
    }
    let context = parse_context(input.context_value)?;
    let scope = input.requested_scope;
    let requested_capability = input.requested_capability;
    let expires_at = context.expires_at;
    let evidence_refs = vec![admission.receipt.receipt_ref];
    let value = record("authority-live-ref-v1", vec![
        string(crate::preserves_rail::AUTHORITY_LIVE_REF_SCHEMA),
        record("authority-context", vec![string(&context.context_ref)]),
        record("scope", vec![string(scope)]),
        record("capability", vec![string(requested_capability)]),
        record("attenuation", vec![string("scoped")]),
        record("expires-at", vec![optional_u64_value(expires_at)]),
        record("evidence", vec![sequence(evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("gatekeeper-resolution"), string("pass")]),
            record("check", vec![string("scoped-live-ref"), string("pass")]),
            record("check", vec![string("expiry-bound"), string("pass")]),
        ])]),
    ]);
    Ok(LiveRef {
        live_ref: canonical_hash(&value)?,
        context_ref: context.context_ref,
        scope: scope.to_string(),
        attenuation: "scoped".to_string(),
        expires_at,
        evidence_refs,
        value,
    })
}

/// Removes assertions bound to the revoked authority object once the revocation is effective. Only
/// `authority-context` and `live-ref` targets name the authority recorded in an
/// `authority-bound-assertion`; other target kinds never match by reference coincidence.
// r[impl molten.authority.independent_currentness_and_revocation_bounds]
pub fn cleanup_for_revocation(
    assertions: &[RuntimeAssertion],
    revocation_value: &IoValue,
    logical_time: u64,
) -> Result<(Vec<RuntimeAssertion>, Receipt)> {
    let revocation = parse_revocation(revocation_value)?;
    let is_effective = revocation.effective_at <= logical_time;
    let is_assertion_bound_kind = matches!(revocation.target_kind.as_str(), "authority-context" | "live-ref");
    let remaining = assertions
        .iter()
        .filter(|assertion| {
            if !is_effective || !is_assertion_bound_kind {
                return true;
            }
            let value = assertion.value.as_iovalue();
            let Some(record) = value.collect_simple_record("authority-bound-assertion", Some(2)) else {
                return true;
            };
            let Ok(authority_ref) = record_string(&record[0], "authority") else {
                return true;
            };
            authority_ref != revocation.target_ref
        })
        .cloned()
        .collect::<Vec<_>>();
    let removed = assertions.len().saturating_sub(remaining.len());
    let removed_diagnostic = format!("cleanup-removed:{removed}");
    let mut diagnostics = vec![removed_diagnostic.as_str()];
    let decision = if is_effective { "pass" } else { "fail" };
    if !is_effective {
        diagnostics.push("revocation-not-yet-effective");
    } else if !is_assertion_bound_kind {
        diagnostics.push("target-kind-not-assertion-bound");
    }
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "cleanup",
        decision,
        context_ref: Some(&revocation.target_ref),
        capability: "cleanup",
        scope: &revocation.target_kind,
        logical_time,
        diagnostics: &diagnostics,
    });
    Ok((remaining, Receipt {
        receipt_ref: canonical_hash(&receipt_value)?,
        operation: "cleanup".to_string(),
        decision: decision.to_string(),
        context_ref: Some(revocation.target_ref),
        value: receipt_value,
    }))
}

fn parse_revocations(revocation_values: &[IoValue]) -> Result<Vec<Revocation>> {
    revocation_values.iter().map(parse_revocation).collect()
}

/// Capability, scope, validity-window and revocation diagnostics shared by every admission path.
fn push_scope_diagnostics(check: ScopeCheck<'_>, diagnostics: &mut Vec<String>) {
    if !check.context.capabilities.iter().any(|capability| {
        capability_allows_current_action(
            capability,
            check.requested_capability,
            check.requested_operation,
            check.requested_scope,
        )
    }) {
        diagnostics.push("capability-denied".to_string());
    }
    if check.context.not_before.is_some_and(|not_before| check.logical_time < not_before) {
        diagnostics.push("not-yet-valid".to_string());
    }
    if check.context.expires_at.is_some_and(|expires_at| check.logical_time >= expires_at) {
        diagnostics.push("expired".to_string());
    }
    if check
        .revocations
        .iter()
        .any(|revocation| revocation_hits_context(revocation, check.context, check.logical_time))
    {
        diagnostics.push("revoked".to_string());
    }
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
