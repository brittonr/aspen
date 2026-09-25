type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const CAPABILITY_TOKEN_SCHEMA: &str = "molten.capability-token.v1";
const CAPABILITY_PROOFSET_SCHEMA: &str = "molten.capability-proofset.v1";
const CAPABILITY_ADMISSION_SCHEMA: &str = "molten.capability-admission-receipt.v1";
const UCAN_VERIFICATION_RECEIPT_SCHEMA: &str = "molten.capability.ucan-verification-receipt.v1";
const WILDCARD_SCOPE: &str = "*";
const CHECK_STATUS_PASS: &str = "pass";
const CHECK_STATUS_FAIL: &str = "fail";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const MAX_UCAN_RECEIPT_REFS: u64 = 1024;

type UcanCheckEvaluator = fn(&UcanVerificationChecks) -> bool;
type UcanCheckSpec = (&'static str, UcanCheckEvaluator);

const UCAN_CHECKS: &[UcanCheckSpec] = &[
    ("signature-valid", |checks| checks.signature_valid),
    ("holder-bound", |checks| checks.holder_matches),
    ("audience-bound", |checks| checks.audience_matches),
    ("session-bound", |checks| checks.session_matches),
    ("context-bound", |checks| checks.context_matches),
    ("time-window-valid", |checks| checks.time_valid),
    ("proof-chain-present", |checks| checks.proofs_present),
    ("revocation-clean", |checks| checks.revocation_clean),
    ("caveats-satisfied", |checks| checks.caveats_satisfied),
    ("replay-fresh", |checks| checks.replay_fresh),
];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UcanVerificationChecks {
    pub signature_valid: bool,
    pub holder_matches: bool,
    pub audience_matches: bool,
    pub session_matches: bool,
    pub context_matches: bool,
    pub time_valid: bool,
    pub proofs_present: bool,
    pub revocation_clean: bool,
    pub caveats_satisfied: bool,
    pub replay_fresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcanVerificationInput {
    pub compact_token_ref: String,
    pub proofset_ref: String,
    pub proof_refs: Vec<String>,
    pub verification_key_refs: Vec<String>,
    pub caveat_decision_refs: Vec<String>,
    pub revocation_fact_refs: Vec<String>,
    pub replay_fact_refs: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub request_ref: String,
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub resource_ref: String,
    pub ability: String,
    pub scope: String,
    pub checks: UcanVerificationChecks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcanVerificationReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub compact_token_ref: String,
    pub proofset_ref: String,
    pub proof_refs: Vec<String>,
    pub verification_key_refs: Vec<String>,
    pub caveat_decision_refs: Vec<String>,
    pub revocation_fact_refs: Vec<String>,
    pub replay_fact_refs: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub request_ref: String,
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub resource_ref: String,
    pub ability: String,
    pub scope: String,
    pub value: IoValue,
    pub receipt_ref: String,
}

impl UcanVerificationChecks {
    pub fn all_pass() -> Self {
        Self {
            signature_valid: true,
            holder_matches: true,
            audience_matches: true,
            session_matches: true,
            context_matches: true,
            time_valid: true,
            proofs_present: true,
            revocation_clean: true,
            caveats_satisfied: true,
            replay_fresh: true,
        }
    }
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
    let token_outcomes = proofset
        .tokens
        .iter()
        .map(|token| {
            let token_ref = crate::preserves_rail::canonical_hash(&capability_token_value(token)?)?;
            Ok((token_ref, token_diagnostics(token, proofset, request)))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut admitted_token_refs = Vec::with_capacity(token_outcomes.len());
    diagnostics.reserve(token_outcomes.iter().map(|(_, token_diagnostics)| token_diagnostics.len()).sum());
    for (token_ref, token_diagnostics) in token_outcomes {
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

pub fn ucan_verification_receipt(input: &UcanVerificationInput) -> Result<UcanVerificationReceipt> {
    validate_ucan_verification_input(input)?;
    let diagnostics = ucan_verification_diagnostics(input);
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = ucan_verification_receipt_value(input, decision, &diagnostics);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(UcanVerificationReceipt {
        decision: decision.to_string(),
        diagnostics,
        compact_token_ref: input.compact_token_ref.clone(),
        proofset_ref: input.proofset_ref.clone(),
        proof_refs: input.proof_refs.clone(),
        verification_key_refs: input.verification_key_refs.clone(),
        caveat_decision_refs: input.caveat_decision_refs.clone(),
        revocation_fact_refs: input.revocation_fact_refs.clone(),
        replay_fact_refs: input.replay_fact_refs.clone(),
        derived_grant_refs: input.derived_grant_refs.clone(),
        request_ref: input.request_ref.clone(),
        holder_ref: input.holder_ref.clone(),
        session_ref: input.session_ref.clone(),
        context_ref: input.context_ref.clone(),
        resource_ref: input.resource_ref.clone(),
        ability: input.ability.clone(),
        scope: input.scope.clone(),
        value,
        receipt_ref,
    })
}
