type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const CAPABILITY_TOKEN_SCHEMA: &str = "molten.capability-token.v1";
const CAPABILITY_PROOFSET_SCHEMA: &str = "molten.capability-proofset.v1";
const CAPABILITY_ADMISSION_SCHEMA: &str = "molten.capability-admission-receipt.v1";
const WILDCARD_SCOPE: &str = "*";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityToken {
    pub token_kind: String,
    pub issuer_ref: String,
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub resource_ref: String,
    pub ability: String,
    pub scope: String,
    pub attenuation: String,
    pub caveats: Vec<String>,
    pub expires_at_tick: u64,
    pub revoked_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub delegation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProofset {
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub tokens: Vec<CapabilityToken>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub resource_ref: String,
    pub ability: String,
    pub scope: String,
    pub at_tick: u64,
    pub required_policy_refs: Vec<String>,
    pub required_resource_refs: Vec<String>,
    pub required_token_kind: Option<String>,
    pub caveat_context: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAdmissionReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub admitted_token_refs: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn capability_token_value(token: &CapabilityToken) -> Result<IoValue> {
    validate_refs([
        token.issuer_ref.as_str(),
        token.holder_ref.as_str(),
        token.session_ref.as_str(),
        token.context_ref.as_str(),
        token.resource_ref.as_str(),
    ])?;
    Ok(crate::preserves_rail::record("capability-token-v1", vec![
        crate::preserves_rail::string(CAPABILITY_TOKEN_SCHEMA),
        field("token-kind", &token.token_kind),
        field("issuer-ref", &token.issuer_ref),
        field("holder-ref", &token.holder_ref),
        field("session-ref", &token.session_ref),
        field("context-ref", &token.context_ref),
        field("resource-ref", &token.resource_ref),
        field("ability", &token.ability),
        field("scope", &token.scope),
        field("attenuation", &token.attenuation),
        string_list_field("caveats", &token.caveats),
        crate::preserves_rail::record("expires-at-tick", vec![crate::preserves_rail::u64_value(token.expires_at_tick)]),
        string_list_field("revoked-refs", &token.revoked_refs),
        string_list_field("policy-refs", &token.policy_refs),
        string_list_field("resource-refs", &token.resource_refs),
        string_list_field("delegation-refs", &token.delegation_refs),
        string_list_field("evidence-refs", &token.evidence_refs),
    ]))
}

pub fn capability_proofset_value(proofset: &CapabilityProofset) -> Result<IoValue> {
    validate_refs([
        proofset.holder_ref.as_str(),
        proofset.session_ref.as_str(),
        proofset.context_ref.as_str(),
    ])?;
    let token_values = proofset.tokens.iter().map(capability_token_value).collect::<Result<Vec<_>>>()?;
    Ok(crate::preserves_rail::record("capability-proofset-v1", vec![
        crate::preserves_rail::string(CAPABILITY_PROOFSET_SCHEMA),
        field("holder-ref", &proofset.holder_ref),
        field("session-ref", &proofset.session_ref),
        field("context-ref", &proofset.context_ref),
        crate::preserves_rail::record("tokens", vec![crate::preserves_rail::sequence(token_values)]),
        string_list_field("policy-refs", &proofset.policy_refs),
        string_list_field("resource-refs", &proofset.resource_refs),
        string_list_field("revocation-refs", &proofset.revocation_refs),
        string_list_field("evidence-refs", &proofset.evidence_refs),
    ]))
}

pub fn admit_capability(
    proofset: &CapabilityProofset,
    request: &CapabilityRequest,
) -> Result<CapabilityAdmissionReceipt> {
    validate_request_refs(request)?;
    let mut diagnostics = Vec::new();
    proofset_boundary_diagnostics(proofset, request, &mut diagnostics);
    let mut admitted_token_refs = Vec::new();
    for token in &proofset.tokens {
        let token_ref = crate::preserves_rail::canonical_hash(&capability_token_value(token)?)?;
        let token_diagnostics = token_diagnostics(token, proofset, request);
        if token_diagnostics.is_empty() {
            admitted_token_refs.push(token_ref);
        } else {
            diagnostics.extend(token_diagnostics.into_iter().map(|diagnostic| format!("{token_ref}: {diagnostic}")));
        }
    }
    let decision = if diagnostics.is_empty() && !admitted_token_refs.is_empty() {
        "pass"
    } else {
        if admitted_token_refs.is_empty() {
            diagnostics.push("no admitted capability token for requested action".to_string());
        }
        "deny"
    };
    let value = admission_value(decision, &diagnostics, &admitted_token_refs, request);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CapabilityAdmissionReceipt {
        decision: decision.to_string(),
        diagnostics,
        admitted_token_refs,
        value,
        receipt_ref,
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
    diagnostics: &mut Vec<String>,
) {
    push_mismatch(diagnostics, "proofset holder", &proofset.holder_ref, &request.holder_ref);
    push_mismatch(diagnostics, "proofset session", &proofset.session_ref, &request.session_ref);
    push_mismatch(diagnostics, "proofset context", &proofset.context_ref, &request.context_ref);
    for required in &request.required_policy_refs {
        if !proofset.policy_refs.iter().any(|reference| reference == required) {
            diagnostics.push(format!("missing policy ref {required}"));
        }
    }
    for required in &request.required_resource_refs {
        if !proofset.resource_refs.iter().any(|reference| reference == required) {
            diagnostics.push(format!("missing resource ref {required}"));
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
    for caveat in &token.caveats {
        if !request.caveat_context.iter().any(|available| available == caveat) {
            diagnostics.push(format!("caveat unsatisfied {caveat}"));
        }
    }
    diagnostics
}

fn admission_value(
    decision: &str,
    diagnostics: &[String],
    admitted_token_refs: &[String],
    request: &CapabilityRequest,
) -> IoValue {
    crate::preserves_rail::record("capability-admission-receipt-v1", vec![
        crate::preserves_rail::string(CAPABILITY_ADMISSION_SCHEMA),
        field("decision", decision),
        field("holder-ref", &request.holder_ref),
        field("session-ref", &request.session_ref),
        field("resource-ref", &request.resource_ref),
        field("ability", &request.ability),
        field("scope", &request.scope),
        string_list_field("admitted-token-refs", admitted_token_refs),
        string_list_field("diagnostics", diagnostics),
        field("evidence-only", "capability-admission-does-not-grant-subsystem-trust"),
    ])
}

fn validate_request_refs(request: &CapabilityRequest) -> Result<()> {
    validate_refs([
        request.holder_ref.as_str(),
        request.session_ref.as_str(),
        request.context_ref.as_str(),
        request.resource_ref.as_str(),
    ])
}

fn validate_refs<'a>(refs: impl IntoIterator<Item = &'a str>) -> Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn push_mismatch(diagnostics: &mut Vec<String>, label: &str, actual: &str, expected: &str) {
    if actual != expected {
        diagnostics.push(format!("{label} mismatch expected {expected} actual {actual}"));
    }
}

fn field(label: &'static str, value: &str) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::string(value)])
}

fn string_list_field(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(
        values.iter().map(crate::preserves_rail::string).collect(),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TICK: u64 = 8;
    const EXPIRED_TICK: u64 = 9;

    #[test]
    fn capability_admission_accepts_scoped_token() {
        let proofset = proofset(token("write-token", "publish", "topic:alerts", VALID_TICK));
        let receipt = admit_capability(&proofset, &request("publish", "topic:alerts", VALID_TICK)).expect("admission");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("hash"));
        assert!(capability_taxonomy().contains(&"handoff-bundle"));
    }

    #[test]
    fn capability_admission_denies_wrong_holder_expiry_and_missing_caveat() {
        let mut proofset = proofset(token("write-token", "publish", "topic:alerts", VALID_TICK));
        proofset.tokens[0].holder_ref = test_ref("other-holder");
        proofset.tokens[0].caveats = vec!["mfa".to_string()];
        let receipt =
            admit_capability(&proofset, &request("publish", "topic:alerts", EXPIRED_TICK)).expect("admission");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("holder mismatch")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("expired")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("caveat unsatisfied")));
    }

    #[test]
    fn imported_token_is_not_operation_authority() {
        let denial = imported_token_authority_denial(&test_ref("token"), "node-control").expect("denial");
        assert_eq!(denial.decision, "deny");
        assert!(denial.diagnostics[0].contains("evidence-only"));
    }

    fn proofset(token: CapabilityToken) -> CapabilityProofset {
        CapabilityProofset {
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            tokens: vec![token],
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource-policy")],
            revocation_refs: Vec::new(),
            evidence_refs: vec![test_ref("evidence")],
        }
    }

    fn token(kind: &str, ability: &str, scope: &str, expires_at_tick: u64) -> CapabilityToken {
        CapabilityToken {
            token_kind: kind.to_string(),
            issuer_ref: test_ref("issuer"),
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            resource_ref: test_ref("resource"),
            ability: ability.to_string(),
            scope: scope.to_string(),
            attenuation: "attenuated".to_string(),
            caveats: Vec::new(),
            expires_at_tick,
            revoked_refs: Vec::new(),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource-policy")],
            delegation_refs: vec![test_ref("delegation")],
            evidence_refs: vec![test_ref("token-evidence")],
        }
    }

    fn request(ability: &str, scope: &str, at_tick: u64) -> CapabilityRequest {
        CapabilityRequest {
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            resource_ref: test_ref("resource"),
            ability: ability.to_string(),
            scope: scope.to_string(),
            at_tick,
            required_policy_refs: vec![test_ref("policy")],
            required_resource_refs: vec![test_ref("resource-policy")],
            required_token_kind: Some("write-token".to_string()),
            caveat_context: Vec::new(),
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("capability-test-ref", vec![
            crate::preserves_rail::string(label),
        ]))
        .expect("test ref")
    }
}
