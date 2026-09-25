
pub fn parse_ucan_verification_receipt_value(value: &IoValue) -> Result<UcanVerificationReceipt> {
    let receipt = crate::preserves_rail::simple_record_fields(
        value,
        "ucan-verification-receipt-v1",
        UCAN_VERIFICATION_RECEIPT_ARITY,
    )?;
    require_string(
        &receipt[UCAN_VERIFICATION_SCHEMA_INDEX],
        UCAN_VERIFICATION_RECEIPT_SCHEMA,
        "UCAN verification schema",
    )?;
    let decision = record_string(&receipt[UCAN_VERIFICATION_DECISION_INDEX], "decision")?;
    if !matches!(decision.as_str(), DECISION_PASS | DECISION_DENY) {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported UCAN verification decision {decision}"
        )));
    }
    let diagnostics = record_string_sequence(&receipt[UCAN_VERIFICATION_DIAGNOSTICS_INDEX], "diagnostics")?;
    let compact_token_ref = record_ref(&receipt[UCAN_VERIFICATION_TOKEN_INDEX], "compact-token-ref")?;
    let proofset_ref = record_ref(&receipt[UCAN_VERIFICATION_PROOFSET_INDEX], "proofset-ref")?;
    let proof_refs = record_ref_sequence(&receipt[UCAN_VERIFICATION_PROOFS_INDEX], "proof-refs")?;
    let verification_key_refs = record_ref_sequence(&receipt[UCAN_VERIFICATION_KEYS_INDEX], "verification-key-refs")?;
    let caveat_decision_refs = record_ref_sequence(&receipt[UCAN_VERIFICATION_CAVEATS_INDEX], "caveat-decision-refs")?;
    let revocation_fact_refs =
        record_ref_sequence(&receipt[UCAN_VERIFICATION_REVOCATIONS_INDEX], "revocation-fact-refs")?;
    let replay_fact_refs = record_ref_sequence(&receipt[UCAN_VERIFICATION_REPLAYS_INDEX], "replay-fact-refs")?;
    let derived_grant_refs = record_ref_sequence(&receipt[UCAN_VERIFICATION_GRANTS_INDEX], "derived-grant-refs")?;
    let request_ref = record_ref(&receipt[UCAN_VERIFICATION_REQUEST_INDEX], "request-ref")?;
    let holder_ref = record_ref(&receipt[UCAN_VERIFICATION_HOLDER_INDEX], "holder-ref")?;
    let session_ref = record_ref(&receipt[UCAN_VERIFICATION_SESSION_INDEX], "session-ref")?;
    let context_ref = record_ref(&receipt[UCAN_VERIFICATION_CONTEXT_INDEX], "context-ref")?;
    let resource_ref = record_ref(&receipt[UCAN_VERIFICATION_RESOURCE_INDEX], "resource-ref")?;
    let ability = record_string(&receipt[UCAN_VERIFICATION_ABILITY_INDEX], "ability")?;
    let scope = record_string(&receipt[UCAN_VERIFICATION_SCOPE_INDEX], "scope")?;
    validate_checks(&receipt[UCAN_VERIFICATION_CHECKS_INDEX], decision.as_str())?;
    Ok(UcanVerificationReceipt {
        decision,
        diagnostics,
        compact_token_ref,
        proofset_ref,
        proof_refs,
        verification_key_refs,
        caveat_decision_refs,
        revocation_fact_refs,
        replay_fact_refs,
        derived_grant_refs,
        request_ref,
        holder_ref,
        session_ref,
        context_ref,
        resource_ref,
        ability,
        scope,
        value: value.clone(),
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
    })
}

pub fn imported_token_authority_denial(token_ref: &str, operation: &str) -> Result<CapabilityAdmissionReceipt> {
    crate::preserves_rail::validate_content_ref(token_ref)?;
    let diagnostics = vec![format!(
        "imported token {token_ref} is evidence-only until capability admission passes for {operation}"
    )];
    let value = crate::preserves_rail::record("capability-admission-receipt-v1", vec![
        crate::preserves_rail::string(CAPABILITY_ADMISSION_SCHEMA),
        field("decision", "deny"),
        string_list_field("diagnostics", &diagnostics),
        string_list_field("admitted-token-refs", &[]),
        field("operation", operation),
        field("evidence-only", "pass"),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CapabilityAdmissionReceipt {
        decision: "deny".to_string(),
        diagnostics,
        admitted_token_refs: Vec::new(),
        value,
        receipt_ref,
    })
}

pub fn capability_taxonomy() -> &'static [&'static str] {
    &[
        "identity-ref",
        "transport-receipt",
        "peer-session",
        "handoff-bundle",
        "bootstrap-ticket",
        "read-token",
        "write-token",
        "promotion-token",
        "authority-token",
        "membership-evidence",
    ]
}

fn proofset_boundary_diagnostics(
    proofset: &CapabilityProofset,
    request: &CapabilityRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    push_mismatch(diagnostics, "proofset holder", &proofset.holder_ref, &request.holder_ref);
    push_mismatch(diagnostics, "proofset session", &proofset.session_ref, &request.session_ref);
    push_mismatch(diagnostics, "proofset context", &proofset.context_ref, &request.context_ref);
    for required in &request.required_policy_refs {
        if !proofset.policy_refs.iter().any(|reference| reference == required) {
            diagnostics.push_item(format!("missing policy ref {required}"));
        }
    }
    for required in &request.required_resource_refs {
        if !proofset.resource_refs.iter().any(|reference| reference == required) {
            diagnostics.push_item(format!("missing resource ref {required}"));
        }
    }
}

fn token_diagnostics(
    token: &CapabilityToken,
    proofset: &CapabilityProofset,
    request: &CapabilityRequest,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if let Some(required_kind) = request.required_token_kind.as_deref() {
        push_mismatch(&mut diagnostics, "token kind", &token.token_kind, required_kind);
    }
    push_mismatch(&mut diagnostics, "holder", &token.holder_ref, &request.holder_ref);
    push_mismatch(&mut diagnostics, "session", &token.session_ref, &request.session_ref);
    push_mismatch(&mut diagnostics, "context", &token.context_ref, &request.context_ref);
    push_mismatch(&mut diagnostics, "resource", &token.resource_ref, &request.resource_ref);
    push_mismatch(&mut diagnostics, "ability", &token.ability, &request.ability);
    if token.scope != request.scope && token.scope != WILDCARD_SCOPE {
        diagnostics.push(format!("scope mismatch expected {} actual {}", request.scope, token.scope));
    }
    if token.scope == WILDCARD_SCOPE && token.attenuation != "attenuated" {
        diagnostics.push("over-broad scope lacks attenuation".to_string());
    }
    if request.at_tick > token.expires_at_tick {
        diagnostics.push("capability token expired".to_string());
    }
    if proofset.revocation_refs.iter().any(|reference| reference == &token.issuer_ref) {
        diagnostics.push("issuer revoked".to_string());
    }
    if token
        .delegation_refs
        .iter()
        .any(|reference| proofset.revocation_refs.iter().any(|revoked| revoked == reference))
    {
        diagnostics.push("delegation revoked".to_string());
    }
    diagnostics.extend(
        token
            .caveats
            .iter()
            .filter(|caveat| !request.caveat_context.iter().any(|available| available == *caveat))
            .map(|caveat| format!("caveat unsatisfied {caveat}")),
    );
    diagnostics
}

fn validate_ucan_verification_input(input: &UcanVerificationInput) -> Result<()> {
    validate_refs([
        input.compact_token_ref.as_str(),
        input.proofset_ref.as_str(),
        input.request_ref.as_str(),
        input.holder_ref.as_str(),
        input.session_ref.as_str(),
        input.context_ref.as_str(),
        input.resource_ref.as_str(),
    ])?;
    validate_ref_slice(&input.proof_refs)?;
    validate_ref_slice(&input.verification_key_refs)?;
    validate_ref_slice(&input.caveat_decision_refs)?;
    validate_ref_slice(&input.revocation_fact_refs)?;
    validate_ref_slice(&input.replay_fact_refs)?;
    validate_ref_slice(&input.derived_grant_refs)
}

fn validate_ref_slice(refs: &[String]) -> Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn ucan_verification_diagnostics(input: &UcanVerificationInput) -> Vec<String> {
    let mut diagnostics = UCAN_CHECKS
        .iter()
        .filter(|(_, predicate)| !predicate(&input.checks))
        .map(|(name, _)| format!("UCAN {name} check failed"))
        .collect::<Vec<_>>();
    if input.proof_refs.is_empty() {
        diagnostics.push("UCAN proof refs are required".to_string());
    }
    if input.verification_key_refs.is_empty() {
        diagnostics.push("UCAN verification key evidence is required".to_string());
    }
    if input.derived_grant_refs.is_empty() {
        diagnostics.push("UCAN derived grant refs are required".to_string());
    }
    diagnostics
}

const UCAN_VERIFICATION_RECEIPT_ARITY: u64 = 19;
const UCAN_VERIFICATION_SCHEMA_INDEX: usize = 0;
const UCAN_VERIFICATION_DECISION_INDEX: usize = 1;
const UCAN_VERIFICATION_DIAGNOSTICS_INDEX: usize = 2;
const UCAN_VERIFICATION_TOKEN_INDEX: usize = 3;
const UCAN_VERIFICATION_PROOFSET_INDEX: usize = 4;
const UCAN_VERIFICATION_PROOFS_INDEX: usize = 5;
const UCAN_VERIFICATION_KEYS_INDEX: usize = 6;
const UCAN_VERIFICATION_CAVEATS_INDEX: usize = 7;
const UCAN_VERIFICATION_REVOCATIONS_INDEX: usize = 8;
const UCAN_VERIFICATION_REPLAYS_INDEX: usize = 9;
const UCAN_VERIFICATION_GRANTS_INDEX: usize = 10;
const UCAN_VERIFICATION_REQUEST_INDEX: usize = 11;
const UCAN_VERIFICATION_HOLDER_INDEX: usize = 12;
const UCAN_VERIFICATION_SESSION_INDEX: usize = 13;
const UCAN_VERIFICATION_CONTEXT_INDEX: usize = 14;
const UCAN_VERIFICATION_RESOURCE_INDEX: usize = 15;
const UCAN_VERIFICATION_ABILITY_INDEX: usize = 16;
const UCAN_VERIFICATION_SCOPE_INDEX: usize = 17;
const UCAN_VERIFICATION_CHECKS_INDEX: usize = 18;

fn ucan_verification_receipt_value(input: &UcanVerificationInput, decision: &str, diagnostics: &[String]) -> IoValue {
    crate::preserves_rail::record("ucan-verification-receipt-v1", vec![
        crate::preserves_rail::string(UCAN_VERIFICATION_RECEIPT_SCHEMA),
        field("decision", decision),
        string_list_field("diagnostics", diagnostics),
        field("compact-token-ref", &input.compact_token_ref),
        field("proofset-ref", &input.proofset_ref),
        string_list_field("proof-refs", &input.proof_refs),
        string_list_field("verification-key-refs", &input.verification_key_refs),
        string_list_field("caveat-decision-refs", &input.caveat_decision_refs),
        string_list_field("revocation-fact-refs", &input.revocation_fact_refs),
        string_list_field("replay-fact-refs", &input.replay_fact_refs),
        string_list_field("derived-grant-refs", &input.derived_grant_refs),
        field("request-ref", &input.request_ref),
        field("holder-ref", &input.holder_ref),
        field("session-ref", &input.session_ref),
        field("context-ref", &input.context_ref),
        field("resource-ref", &input.resource_ref),
        field("ability", &input.ability),
        field("scope", &input.scope),
        checks_value(input, decision),
    ])
}

fn checks_value(input: &UcanVerificationInput, decision: &str) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        UCAN_CHECKS
            .iter()
            .map(|(name, predicate)| {
                let status = if predicate(&input.checks) {
                    CHECK_STATUS_PASS
                } else {
                    CHECK_STATUS_FAIL
                };
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(*name),
                    crate::preserves_rail::string(status),
                ])
            })
            .chain(std::iter::once(crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("decision-bound"),
                crate::preserves_rail::string(decision),
            ])))
            .collect(),
    )])
}
