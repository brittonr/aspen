fn validate_basalt_ucan_input(
    input: &BasaltUcanAuthorityInput,
) -> std::result::Result<(), super::RuntimeBoundaryError> {
    validate_ref(&input.holder_ref)?;
    validate_ref(&input.session_ref)?;
    validate_ref(&input.context_ref)?;
    validate_ref(&input.request_ref)?;
    validate_ref(&input.basalt_policy_ref)?;
    validate_ref(&input.basalt_policy_source_ref)?;
    validate_ref(&input.basalt_policy_export_ref)?;
    validate_ref(&input.proofset_ref)?;
    validate_refs(&input.ucan_verification_receipt_refs)?;
    for grant in &input.verified_grants {
        validate_ref(&grant.grant_ref)?;
        validate_ref(&grant.verification_receipt_ref)?;
        validate_ref(&grant.holder_ref)?;
        validate_ref(&grant.session_ref)?;
        validate_ref(&grant.context_ref)?;
    }
    Ok(())
}

fn validate_ref(reference: &str) -> std::result::Result<(), super::RuntimeBoundaryError> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| super::RuntimeBoundaryError::invalid_input(BASALT_AUTHORITY_COMPONENT, error.to_string()))
}

fn validate_refs(refs: &[String]) -> std::result::Result<(), super::RuntimeBoundaryError> {
    for reference in refs {
        validate_ref(reference)?;
    }
    Ok(())
}

fn basalt_ucan_diagnostics(
    input: &BasaltUcanAuthorityInput,
) -> std::result::Result<Vec<String>, super::RuntimeBoundaryError> {
    let mut diagnostics = Vec::new();
    if input.ucan_verification_receipt_refs.is_empty() {
        push_diagnostic(&mut diagnostics, "missing UCAN verification receipt refs".to_string())?;
    }
    if input.verified_grants.is_empty() {
        push_diagnostic(&mut diagnostics, "missing verified UCAN-derived grants".to_string())?;
    }
    if !input.policy_allows {
        push_diagnostic(&mut diagnostics, "Basalt policy denied requested resource or ability".to_string())?;
    }
    for grant in &input.verified_grants {
        if !input
            .ucan_verification_receipt_refs
            .iter()
            .any(|receipt_ref| receipt_ref == &grant.verification_receipt_ref)
        {
            push_diagnostic(
                &mut diagnostics,
                format!("grant {} is not bound to a supplied UCAN verification receipt", grant.grant_ref),
            )?;
        }
        authority_mismatch(&mut diagnostics, "holder", &grant.holder_ref, &input.holder_ref)?;
        authority_mismatch(&mut diagnostics, "session", &grant.session_ref, &input.session_ref)?;
        authority_mismatch(&mut diagnostics, "context", &grant.context_ref, &input.context_ref)?;
        authority_mismatch(&mut diagnostics, "resource", &grant.resource, &input.resource)?;
        authority_mismatch(&mut diagnostics, "ability", &grant.ability, &input.ability)?;
    }
    Ok(diagnostics)
}

trait DiagnosticSink {
    fn push_bounded(&mut self, diagnostic: String) -> std::result::Result<(), super::RuntimeBoundaryError>;
}

impl DiagnosticSink for Vec<String> {
    fn push_bounded(&mut self, diagnostic: String) -> std::result::Result<(), super::RuntimeBoundaryError> {
        let next = self.len().checked_add(1).ok_or_else(|| {
            super::RuntimeBoundaryError::invalid_input(BASALT_AUTHORITY_COMPONENT, "diagnostic count overflow")
        })?;
        if next > MAX_SPINE_DIAGNOSTICS {
            return Err(super::RuntimeBoundaryError::invalid_input(
                BASALT_AUTHORITY_COMPONENT,
                "diagnostics exceeded bound",
            ));
        }
        self.push(diagnostic);
        Ok(())
    }
}

fn push_diagnostic(
    diagnostics: &mut impl DiagnosticSink,
    diagnostic: String,
) -> std::result::Result<(), super::RuntimeBoundaryError> {
    diagnostics.push_bounded(diagnostic)
}

fn authority_mismatch(
    diagnostics: &mut impl DiagnosticSink,
    label: &str,
    actual: &str,
    expected: &str,
) -> std::result::Result<(), super::RuntimeBoundaryError> {
    if actual != expected {
        push_diagnostic(diagnostics, format!("verified grant {label} mismatch expected {expected} actual {actual}"))?;
    }
    Ok(())
}

fn basalt_ucan_receipt_value(
    input: &BasaltUcanAuthorityInput,
    decision: &str,
    diagnostics: &[String],
    derived_grant_refs: &[String],
) -> IoValue {
    crate::preserves_rail::record("basalt-ucan-authority-receipt-v1", vec![
        crate::preserves_rail::string(BASALT_UCAN_AUTHORITY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("contract-id", vec![crate::preserves_rail::string(&input.contract_id)]),
        crate::preserves_rail::record("resource", vec![crate::preserves_rail::string(&input.resource)]),
        crate::preserves_rail::record("ability", vec![crate::preserves_rail::string(&input.ability)]),
        crate::preserves_rail::record("holder-ref", vec![crate::preserves_rail::string(&input.holder_ref)]),
        crate::preserves_rail::record("session-ref", vec![crate::preserves_rail::string(&input.session_ref)]),
        crate::preserves_rail::record("context-ref", vec![crate::preserves_rail::string(&input.context_ref)]),
        crate::preserves_rail::record("request-ref", vec![crate::preserves_rail::string(&input.request_ref)]),
        crate::preserves_rail::record("basalt-policy-ref", vec![crate::preserves_rail::string(
            &input.basalt_policy_ref,
        )]),
        crate::preserves_rail::record("basalt-policy-source-ref", vec![crate::preserves_rail::string(
            &input.basalt_policy_source_ref,
        )]),
        crate::preserves_rail::record("basalt-policy-export-ref", vec![crate::preserves_rail::string(
            &input.basalt_policy_export_ref,
        )]),
        crate::preserves_rail::record("ucan-proofset-ref", vec![crate::preserves_rail::string(&input.proofset_ref)]),
        string_sequence_record("ucan-verification-receipt-refs", &input.ucan_verification_receipt_refs),
        string_sequence_record("derived-grant-refs", derived_grant_refs),
        string_sequence_record("diagnostics", diagnostics),
        crate::preserves_rail::record("basalt-enforcement-result", vec![crate::preserves_rail::string(
            if input.policy_allows {
                DECISION_PASS
            } else {
                DECISION_DENY
            },
        )]),
        crate::preserves_rail::record("evidence-only", vec![crate::preserves_rail::string(
            "authority-receipt-does-not-grant-future-authority",
        )]),
    ])
}

fn string_sequence_record(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(
        values.iter().map(crate::preserves_rail::string).collect(),
    )])
}

pub fn policy_gate_receipt(
    envelope: &super::Envelope,
    required_capability: &str,
) -> crate::error::Result<PolicyGateReceipt> {
    let envelope_ref = envelope.canonical_hash()?;
    let has_capability = envelope.capabilities.iter().any(|capability| capability.as_str() == required_capability);
    let mut diagnostics = Vec::new();
    if !has_capability {
        diagnostics.push(format!("missing capability {required_capability}"));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(PolicyGateReceipt {
        envelope_ref,
        decision: decision.to_string(),
        predicate: "runtime-spine-policy-gate-v1".to_string(),
        diagnostics,
    })
}

pub fn validate_cairn_receipt_ref(reference: &str) -> std::result::Result<(), super::RuntimeBoundaryError> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| super::RuntimeBoundaryError::invalid_input("cairn-receipt", error.to_string()))
}

pub fn valence_evidence_ref(
    reference: impl Into<String>,
    claim: impl Into<String>,
) -> crate::error::Result<ValenceEvidenceRef> {
    let evidence_ref = reference.into();
    crate::preserves_rail::validate_content_ref(&evidence_ref)?;
    Ok(ValenceEvidenceRef {
        evidence_ref,
        claim: claim.into(),
    })
}

impl ReceiptIndex {
    pub fn insert(&mut self, key: impl Into<String>, receipt_ref: impl Into<String>) -> crate::error::Result<()> {
        let receipt_ref = receipt_ref.into();
        crate::preserves_rail::validate_content_ref(&receipt_ref)?;
        self.entries.insert(key.into(), receipt_ref);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }
}

pub fn integration_evidence(
    config: &[u8],
    local_route: &[u8],
    remote_bridge: &[u8],
    policy: &[u8],
) -> IntegrationEvidence {
    IntegrationEvidence {
        config_ref: crate::preserves_rail::content_ref_from_bytes(config),
        local_route_ref: crate::preserves_rail::content_ref_from_bytes(local_route),
        remote_bridge_ref: crate::preserves_rail::content_ref_from_bytes(remote_bridge),
        policy_ref: crate::preserves_rail::content_ref_from_bytes(policy),
    }
}
