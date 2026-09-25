
// r[impl molten.claim_authority.downstream_consumption]
pub fn decide_claim_use(input: &ClaimUseInput) -> Result<ClaimUseDecision> {
    validate_ref(&input.required_selector_ref, "claim use selector ref")?;
    validate_ref(&input.subject_ref, "claim use subject ref")?;
    validate_ref(&input.freshness_ref, "claim use freshness ref")?;
    validate_text("claim use required kind", &input.required_claim_kind)?;
    validate_text("claim use subsystem", &input.subsystem)?;
    validate_refs(&input.policy_refs, "claim use policy ref")?;
    validate_refs(&input.resource_refs, "claim use resource ref")?;
    let mut diagnostics = Vec::new();
    if input.admission.decision != DECISION_PASS {
        diagnostics.push("claim-admission-not-pass".to_string());
    }
    if input.admission.selector_ref != input.required_selector_ref {
        diagnostics.push("claim-selector-mismatch".to_string());
    }
    if !admission_text_contains(&input.admission.value, &input.required_claim_kind)? {
        diagnostics.push("claim-kind-mismatch".to_string());
    }
    if input.policy_refs.is_empty() {
        diagnostics.push("missing-subsystem-claim-policy".to_string());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("missing-subsystem-claim-resource".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = record("authority-claim-use-decision-v1", vec![
        string(CLAIM_USE_SCHEMA),
        field_string("decision", decision),
        field_string("claim-admission", &input.admission.receipt_ref),
        field_string("selector", &input.required_selector_ref),
        field_string("claim-kind", &input.required_claim_kind),
        field_string("subject", &input.subject_ref),
        field_string("subsystem", &input.subsystem),
        field_sequence("policy", ref_values(&input.policy_refs)?),
        field_sequence("resource", ref_values(&input.resource_refs)?),
        field_string("freshness", &input.freshness_ref),
        field_sequence("diagnostics", string_values(&diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]);
    let decision_ref = canonical_hash(&value)?;
    Ok(ClaimUseDecision {
        decision: decision.to_string(),
        diagnostics,
        value,
        decision_ref,
    })
}

// r[impl molten.claim_authority.peer_diagnostics]
// r[impl molten.claim_authority.peer_session_context]
pub fn peer_claim_authority_diagnostic(input: &PeerClaimDiagnosticInput) -> Result<PeerClaimDiagnostic> {
    validate_ref(&input.peer_ref, "peer claim diagnostic peer ref")?;
    if let Some(bootstrap_ref) = input.bootstrap_ref.as_ref() {
        validate_ref(bootstrap_ref, "peer claim diagnostic bootstrap ref")?;
    }
    if let Some(session_ref) = input.session_ref.as_ref() {
        validate_ref(session_ref, "peer claim diagnostic session ref")?;
    }
    validate_refs(&input.transport_refs, "peer claim diagnostic transport ref")?;
    validate_ref(&input.selector_ref, "peer claim diagnostic selector ref")?;
    validate_text("peer claim diagnostic claim kind", &input.claim_kind)?;
    let mut diagnostics = Vec::new();
    if input.bootstrap_ref.is_none() {
        diagnostics.push("peer-bootstrap-missing".to_string());
    }
    if input.session_ref.is_none() {
        diagnostics.push("peer-session-missing".to_string());
    }
    if !input.transport_refs.is_empty() {
        diagnostics.push("peer-transport-observed-context-only".to_string());
    }
    match input.claim_admission.as_ref() {
        Some(admission) if admission.decision == DECISION_PASS && admission.selector_ref == input.selector_ref => {}
        Some(admission) if admission.decision != DECISION_PASS => {
            diagnostics.push("peer-claim-admission-denied".to_string())
        }
        Some(_) => diagnostics.push("peer-claim-selector-mismatch".to_string()),
        None => diagnostics.push("peer-claim-authority-missing-capability-ucan-basalt-proof".to_string()),
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.iter().any(|diagnostic| diagnostic.contains("claim")) {
        DECISION_DENY
    } else {
        DECISION_PASS
    };
    let value = record("peer-claim-authority-diagnostic-v1", vec![
        field_string("decision", decision),
        field_string("peer", &input.peer_ref),
        field_string("bootstrap", input.bootstrap_ref.as_deref().unwrap_or("none")),
        field_string("session", input.session_ref.as_deref().unwrap_or("none")),
        field_sequence("transport", ref_values(&input.transport_refs)?),
        field_string("claim-kind", &input.claim_kind),
        field_string("selector", &input.selector_ref),
        field_sequence("diagnostics", string_values(&diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]);
    let diagnostic_ref = canonical_hash(&value)?;
    Ok(PeerClaimDiagnostic {
        decision: decision.to_string(),
        diagnostics,
        value,
        diagnostic_ref,
    })
}

// r[impl molten.claim_authority.registry_readback]
pub fn claim_readback_summary(value: &IoValue) -> Result<Option<String>> {
    if value.collect_simple_record("claim-subject-selector-v1", Some(8)).is_some() {
        return Ok(Some("claim subject selector (evidence candidate only)".to_string()));
    }
    if value.collect_simple_record("authority-claim-v1", Some(16)).is_some() {
        return Ok(Some("authority claim (not admitted by discovery)".to_string()));
    }
    if value.collect_simple_record("authority-claim-admission-v1", Some(17)).is_some() {
        return Ok(Some("authority claim admission (policy-selected use still required)".to_string()));
    }
    Ok(None)
}

fn admission_diagnostics(input: &ClaimAdmissionInput, selector_ref: &str) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if is_broad_selector(&input.selector) && input.selector.caveats.is_empty() {
        diagnostics.push("broad-selector-without-visible-attenuation".to_string());
    }
    let request = claim_capability_request(RequestInput {
        holder_ref: &input.claim.holder_ref,
        session_ref: &input.claim.session_ref,
        context_ref: &input.claim.context_ref,
        selector_ref,
        claim_kind: &input.claim.claim_kind,
        at_tick: input.at_tick,
        policy_refs: &input.local_policy_refs,
        resource_refs: &input.local_resource_refs,
    })?;
    match input.capability_admission.as_ref() {
        Some(admission) if admission.decision == DECISION_PASS => {
            if admission.admitted_token_refs.is_empty() {
                diagnostics.push("claim-capability-admission-has-no-token".to_string());
            }
        }
        Some(_) => diagnostics.push("claim-capability-admission-denied".to_string()),
        None => diagnostics.push("missing-claim-capability-admission".to_string()),
    }
    if input.ucan_verification_refs.is_empty() {
        diagnostics.push("missing-ucan-verification-receipt".to_string());
    }
    if input.basalt_enforcement_refs.is_empty() {
        diagnostics.push("missing-basalt-enforcement-receipt".to_string());
    }
    if input.local_policy_refs.is_empty() {
        diagnostics.push("missing-local-claim-policy".to_string());
    }
    if input.local_resource_refs.is_empty() {
        diagnostics.push("missing-local-claim-resource".to_string());
    }
    if input.freshness_refs.is_empty() {
        diagnostics.push("missing-claim-freshness".to_string());
    }
    if input.revocation_refs.iter().any(|revoked| revoked == &input.claim.issuer_ref) {
        diagnostics.push("claim-issuer-revoked".to_string());
    }
    if !input.transport_observation_refs.is_empty() && input.capability_admission.is_none() {
        diagnostics.push("transport-evidence-is-not-claim-authority".to_string());
    }
    if !input.registry_discovery_refs.is_empty() && input.capability_admission.is_none() {
        diagnostics.push("registry-discovery-is-not-claim-authority".to_string());
    }
    if !input.local_fixture_grant_refs.is_empty()
        && (input.ucan_verification_refs.is_empty() || input.basalt_enforcement_refs.is_empty())
    {
        diagnostics.push("local-fixture-grant-cannot-satisfy-ucan-basalt-claim".to_string());
    }
    if input.claim.holder_ref != request.holder_ref
        || input.claim.session_ref != request.session_ref
        || input.claim.context_ref != request.context_ref
        || input.claim.subject_selector_ref != request.resource_ref
        || request.ability != CLAIM_ATTEST_ABILITY
        || request.scope != input.claim.claim_kind
    {
        diagnostics.push("claim-capability-request-mismatch".to_string());
    }
    validate_admission_refs(input)?;
    Ok(diagnostics)
}

fn validate_admission_refs(input: &ClaimAdmissionInput) -> Result<()> {
    validate_refs(&input.ucan_verification_refs, "claim UCAN verification ref")?;
    validate_refs(&input.basalt_enforcement_refs, "claim Basalt enforcement ref")?;
    validate_refs(&input.freshness_refs, "claim freshness ref")?;
    validate_refs(&input.revocation_refs, "claim revocation ref")?;
    validate_refs(&input.peer_context_refs, "claim peer context ref")?;
    validate_refs(&input.transport_observation_refs, "claim transport observation ref")?;
    validate_refs(&input.registry_discovery_refs, "claim registry discovery ref")?;
    validate_refs(&input.local_fixture_grant_refs, "claim local fixture grant ref")
}

fn denied_claim_admission(
    input: &ClaimAdmissionInput,
    selector_ref: String,
    diagnostics: Vec<String>,
) -> Result<ClaimAdmission> {
    let claim_value = authority_claim_value(&input.claim)?;
    let claim_ref = canonical_hash(&claim_value)?;
    let value = claim_admission_value(input, DECISION_DENY, &diagnostics, &selector_ref, &claim_ref)?;
    let receipt_ref = canonical_hash(&value)?;
    Ok(ClaimAdmission {
        decision: DECISION_DENY.to_string(),
        diagnostics,
        claim_ref,
        selector_ref,
        value,
        receipt_ref,
    })
}

fn claim_admission_value(
    input: &ClaimAdmissionInput,
    decision: &str,
    diagnostics: &[String],
    selector_ref: &str,
    claim_ref: &str,
) -> Result<IoValue> {
    Ok(record("authority-claim-admission-v1", vec![
        string(CLAIM_ADMISSION_SCHEMA),
        field_string("decision", decision),
        field_string("claim", claim_ref),
        field_string("subject-selector", selector_ref),
        field_string("holder", &input.claim.holder_ref),
        field_string("session", &input.claim.session_ref),
        field_string("context", &input.claim.context_ref),
        field_string("resource", selector_ref),
        field_string("ability", CLAIM_ATTEST_ABILITY),
        field_string("scope", &input.claim.claim_kind),
        field_sequence(
            "capability-admission",
            ref_values(
                &input
                    .capability_admission
                    .as_ref()
                    .map(|admission| vec![admission.receipt_ref.clone()])
                    .unwrap_or_default(),
            )?,
        ),
        field_sequence("ucan", ref_values(&input.ucan_verification_refs)?),
        field_sequence("basalt", ref_values(&input.basalt_enforcement_refs)?),
        field_sequence("policy", ref_values(&input.local_policy_refs)?),
        field_sequence("resource-evidence", ref_values(&input.local_resource_refs)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        checks_value(&[
            ("capability-path-required", status(input.capability_admission.is_some())),
            ("ucan-proof-required", status(!input.ucan_verification_refs.is_empty())),
            ("basalt-proof-required", status(!input.basalt_enforcement_refs.is_empty())),
            ("peer-session-is-context-not-authority", "pass"),
            ("registry-readback-is-not-authority", "pass"),
            ("evidence-only", "pass"),
        ]),
    ]))
}

fn selector_checks(selector: &ClaimSubjectSelector) -> Vec<(&'static str, &'static str)> {
    vec![
        ("hash-agnostic-selector", "pass"),
        ("broad-selector-visible", status(!is_broad_selector(selector) || !selector.caveats.is_empty())),
        (
            "policy-or-resource-bound",
            status(!selector.policy_refs.is_empty() || !selector.resource_refs.is_empty()),
        ),
    ]
}

fn is_broad_selector(selector: &ClaimSubjectSelector) -> bool {
    BROAD_SELECTOR_KINDS.iter().any(|kind| *kind == selector.selector_kind)
}
