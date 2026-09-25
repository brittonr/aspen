type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;

const CLAIM_SELECTOR_SCHEMA: &str = "molten.claim-authority.subject-selector.v1";
const AUTHORITY_CLAIM_SCHEMA: &str = "molten.claim-authority.claim.v1";
const CLAIM_ADMISSION_SCHEMA: &str = "molten.claim-authority.admission.v1";
const CLAIM_USE_SCHEMA: &str = "molten.claim-authority.use-decision.v1";
const CLAIM_ATTEST_ABILITY: &str = "claim:attest";
const CLAIM_TOKEN_KIND: &str = "external-claim-authority";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const SELECTOR_EXACT_REF: &str = "exact-ref";
const SELECTOR_REF_PREFIX: &str = "ref-prefix";
const SELECTOR_ARTIFACT_CLASS: &str = "artifact-class";
const SELECTOR_NAMESPACE: &str = "namespace";
const SELECTOR_SCHEMA_ID: &str = "schema-id";
const SELECTOR_RELEASE_CHANNEL: &str = "release-channel";
const SELECTOR_CLUSTER_ID: &str = "cluster-id";
const SELECTOR_POLICY_DEFINED: &str = "policy-defined";
const BROAD_SELECTOR_KINDS: &[&str] = &[
    SELECTOR_REF_PREFIX,
    SELECTOR_NAMESPACE,
    SELECTOR_ARTIFACT_CLASS,
    SELECTOR_SCHEMA_ID,
    SELECTOR_RELEASE_CHANNEL,
    SELECTOR_CLUSTER_ID,
    SELECTOR_POLICY_DEFINED,
];
const EVIDENCE_ONLY_CAVEAT: &str = "claim authority evidence is evidence-only until the exact subsystem gate consumes a matching admitted claim and still does not grant unrelated authority, provenance, source-gate, retention, execution, release, deployment, transport, or policy trust";
const MAX_REFS: usize = 128;
const MAX_DIAGNOSTICS: usize = 512;
const MAX_CAVEATS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimSubjectSelector {
    pub selector_kind: String,
    pub selector_value: String,
    pub subject_kind: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityClaim {
    pub issuer_ref: String,
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub subject_selector_ref: String,
    pub exact_subject_refs: Vec<String>,
    pub claim_kind: String,
    pub claim_value_ref: String,
    pub evidence_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub freshness_ref: String,
    pub revocation_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimAdmissionInput {
    pub selector: ClaimSubjectSelector,
    pub claim: AuthorityClaim,
    pub at_tick: u64,
    pub capability_admission: Option<crate::capability_tokens::CapabilityAdmissionReceipt>,
    pub ucan_verification_refs: Vec<String>,
    pub basalt_enforcement_refs: Vec<String>,
    pub local_policy_refs: Vec<String>,
    pub local_resource_refs: Vec<String>,
    pub freshness_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub peer_context_refs: Vec<String>,
    pub transport_observation_refs: Vec<String>,
    pub registry_discovery_refs: Vec<String>,
    pub local_fixture_grant_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimAdmission {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub claim_ref: String,
    pub selector_ref: String,
    pub value: IoValue,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimUseInput {
    pub admission: ClaimAdmission,
    pub required_selector_ref: String,
    pub required_claim_kind: String,
    pub subject_ref: String,
    pub subsystem: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub freshness_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimUseDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub decision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerClaimDiagnosticInput {
    pub peer_ref: String,
    pub bootstrap_ref: Option<String>,
    pub session_ref: Option<String>,
    pub transport_refs: Vec<String>,
    pub claim_admission: Option<ClaimAdmission>,
    pub claim_kind: String,
    pub selector_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerClaimDiagnostic {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub diagnostic_ref: String,
}

// r[impl molten.claim_authority.subject_selectors]
pub fn claim_subject_selector_value(selector: &ClaimSubjectSelector) -> Result<IoValue> {
    validate_selector(selector)?;
    Ok(record("claim-subject-selector-v1", vec![
        string(CLAIM_SELECTOR_SCHEMA),
        field_string("selector-kind", &selector.selector_kind),
        field_string("selector-value", &selector.selector_value),
        field_string("subject-kind", &selector.subject_kind),
        field_sequence("policy", ref_values(&selector.policy_refs)?),
        field_sequence("resource", ref_values(&selector.resource_refs)?),
        field_sequence("caveats", string_values(&selector.caveats)?),
        checks_value(&selector_checks(selector)),
    ]))
}

// r[impl molten.claim_authority.claim_records]
pub fn authority_claim_value(claim: &AuthorityClaim) -> Result<IoValue> {
    validate_claim(claim)?;
    Ok(record("authority-claim-v1", vec![
        string(AUTHORITY_CLAIM_SCHEMA),
        field_string("issuer", &claim.issuer_ref),
        field_string("holder", &claim.holder_ref),
        field_string("session", &claim.session_ref),
        field_string("context", &claim.context_ref),
        field_string("subject-selector", &claim.subject_selector_ref),
        field_sequence("subjects", ref_values(&claim.exact_subject_refs)?),
        field_string("claim-kind", &claim.claim_kind),
        field_string("claim-value", &claim.claim_value_ref),
        field_sequence("evidence", ref_values(&claim.evidence_refs)?),
        field_sequence("policy", ref_values(&claim.policy_refs)?),
        field_sequence("resource", ref_values(&claim.resource_refs)?),
        field_string("freshness", &claim.freshness_ref),
        field_sequence("revocations", ref_values(&claim.revocation_refs)?),
        field_sequence("caveats", string_values(&claim.caveats)?),
        checks_value(&[("evidence-only-until-admitted", "pass"), ("canonical-claim", "pass")]),
    ]))
}

pub struct RequestInput<'a> {
    pub holder_ref: &'a str,
    pub session_ref: &'a str,
    pub context_ref: &'a str,
    pub selector_ref: &'a str,
    pub claim_kind: &'a str,
    pub at_tick: u64,
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
}

// r[impl molten.claim_authority.capability_profile]
pub fn claim_capability_request(input: RequestInput<'_>) -> Result<crate::capability_tokens::CapabilityRequest> {
    let RequestInput {
        holder_ref,
        session_ref,
        context_ref,
        selector_ref,
        claim_kind,
        at_tick,
        policy_refs,
        resource_refs,
    } = input;
    validate_ref(holder_ref, "claim holder ref")?;
    validate_ref(session_ref, "claim session ref")?;
    validate_ref(context_ref, "claim context ref")?;
    validate_ref(selector_ref, "claim selector ref")?;
    validate_text("claim kind", claim_kind)?;
    validate_refs(policy_refs, "claim policy ref")?;
    validate_refs(resource_refs, "claim resource ref")?;
    Ok(crate::capability_tokens::CapabilityRequest {
        holder_ref: holder_ref.to_string(),
        session_ref: session_ref.to_string(),
        context_ref: context_ref.to_string(),
        resource_ref: selector_ref.to_string(),
        ability: CLAIM_ATTEST_ABILITY.to_string(),
        scope: claim_kind.to_string(),
        at_tick,
        required_policy_refs: policy_refs.to_vec(),
        required_resource_refs: resource_refs.to_vec(),
        required_token_kind: Some(CLAIM_TOKEN_KIND.to_string()),
        caveat_context: Vec::new(),
    })
}

// r[impl molten.claim_authority.claim_records]
// r[impl molten.claim_authority.no_parallel_trust]
// r[impl molten.claim_authority.peer_session_context]
pub fn admit_authority_claim(input: &ClaimAdmissionInput) -> Result<ClaimAdmission> {
    let selector_value = claim_subject_selector_value(&input.selector)?;
    let selector_ref = canonical_hash(&selector_value)?;
    if input.claim.subject_selector_ref != selector_ref {
        return denied_claim_admission(input, selector_ref, vec!["claim-selector-ref-mismatch".to_string()]);
    }
    let claim_value = authority_claim_value(&input.claim)?;
    let claim_ref = canonical_hash(&claim_value)?;
    let mut diagnostics = admission_diagnostics(input, &selector_ref)?;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let admission_value = claim_admission_value(input, decision, &diagnostics, &selector_ref, &claim_ref)?;
    let receipt_ref = canonical_hash(&admission_value)?;
    Ok(ClaimAdmission {
        decision: decision.to_string(),
        diagnostics,
        claim_ref,
        selector_ref,
        value: admission_value,
        receipt_ref,
    })
}
