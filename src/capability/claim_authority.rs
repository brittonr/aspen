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

// r[impl molten.claim_authority.capability_profile]
pub fn claim_capability_request(
    holder_ref: &str,
    session_ref: &str,
    context_ref: &str,
    selector_ref: &str,
    claim_kind: &str,
    at_tick: u64,
    policy_refs: &[String],
    resource_refs: &[String],
) -> Result<crate::capability_tokens::CapabilityRequest> {
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
    let request = claim_capability_request(
        &input.claim.holder_ref,
        &input.claim.session_ref,
        &input.claim.context_ref,
        selector_ref,
        &input.claim.claim_kind,
        input.at_tick,
        &input.local_policy_refs,
        &input.local_resource_refs,
    )?;
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
    validate_refs(&input.ucan_verification_refs, "claim UCAN verification ref")?;
    validate_refs(&input.basalt_enforcement_refs, "claim Basalt enforcement ref")?;
    validate_refs(&input.freshness_refs, "claim freshness ref")?;
    validate_refs(&input.revocation_refs, "claim revocation ref")?;
    validate_refs(&input.peer_context_refs, "claim peer context ref")?;
    validate_refs(&input.transport_observation_refs, "claim transport observation ref")?;
    validate_refs(&input.registry_discovery_refs, "claim registry discovery ref")?;
    validate_refs(&input.local_fixture_grant_refs, "claim local fixture grant ref")?;
    Ok(diagnostics)
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

fn validate_selector(selector: &ClaimSubjectSelector) -> Result<()> {
    match selector.selector_kind.as_str() {
        SELECTOR_EXACT_REF => validate_ref(&selector.selector_value, "exact claim subject ref")?,
        SELECTOR_REF_PREFIX
        | SELECTOR_ARTIFACT_CLASS
        | SELECTOR_NAMESPACE
        | SELECTOR_SCHEMA_ID
        | SELECTOR_RELEASE_CHANNEL
        | SELECTOR_CLUSTER_ID
        | SELECTOR_POLICY_DEFINED => validate_text("claim selector value", &selector.selector_value)?,
        other => return Err(MoltenError::invalid_harness(format!("unsupported claim selector kind {other}"))),
    }
    validate_text("claim selector subject kind", &selector.subject_kind)?;
    validate_refs(&selector.policy_refs, "claim selector policy ref")?;
    validate_refs(&selector.resource_refs, "claim selector resource ref")?;
    validate_caveats(&selector.caveats)
}

fn validate_claim(claim: &AuthorityClaim) -> Result<()> {
    validate_ref(&claim.issuer_ref, "claim issuer ref")?;
    validate_ref(&claim.holder_ref, "claim holder ref")?;
    validate_ref(&claim.session_ref, "claim session ref")?;
    validate_ref(&claim.context_ref, "claim context ref")?;
    validate_ref(&claim.subject_selector_ref, "claim selector ref")?;
    validate_refs(&claim.exact_subject_refs, "claim exact subject ref")?;
    validate_text("claim kind", &claim.claim_kind)?;
    validate_ref(&claim.claim_value_ref, "claim value ref")?;
    validate_refs(&claim.evidence_refs, "claim evidence ref")?;
    validate_refs(&claim.policy_refs, "claim policy ref")?;
    validate_refs(&claim.resource_refs, "claim resource ref")?;
    validate_ref(&claim.freshness_ref, "claim freshness ref")?;
    validate_refs(&claim.revocation_refs, "claim revocation ref")?;
    validate_caveats(&claim.caveats)
}

fn validate_caveats(caveats: &[String]) -> Result<()> {
    crate::bounded::ensure_count_at_most(caveats.len(), MAX_CAVEATS, "claim caveats")?;
    for caveat in caveats {
        validate_text("claim caveat", caveat)?;
    }
    Ok(())
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(refs.len(), MAX_REFS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} {reference}: {error}")))
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn admission_text_contains(value: &IoValue, needle: &str) -> Result<bool> {
    Ok(crate::preserves_rail::to_text(value)?.contains(needle))
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_refs(refs, "claim ref")?;
    Ok(refs.iter().map(|reference| string(reference)).collect())
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn checks_value(checks: &[(&'static str, &'static str)]) -> IoValue {
    record("checks", vec![crate::preserves_rail::sequence(
        checks.iter().map(|(name, state)| record("check", vec![string(name), string(state)])).collect(),
    )])
}

fn status(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "claim authority diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn selector() -> ClaimSubjectSelector {
        ClaimSubjectSelector {
            selector_kind: SELECTOR_EXACT_REF.to_string(),
            selector_value: local_ref("subject"),
            subject_kind: "artifact".to_string(),
            policy_refs: refs("selector-policy"),
            resource_refs: refs("selector-resource"),
            caveats: Vec::new(),
        }
    }

    fn claim(selector_ref: String) -> AuthorityClaim {
        AuthorityClaim {
            issuer_ref: local_ref("issuer"),
            holder_ref: local_ref("holder"),
            session_ref: local_ref("session"),
            context_ref: local_ref("context"),
            subject_selector_ref: selector_ref,
            exact_subject_refs: refs("subject"),
            claim_kind: "class-membership".to_string(),
            claim_value_ref: local_ref("claim-value"),
            evidence_refs: refs("claim-evidence"),
            policy_refs: refs("claim-policy"),
            resource_refs: refs("claim-resource"),
            freshness_ref: local_ref("claim-freshness"),
            revocation_refs: refs("revocation-clean"),
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn admitted_capability(selector_ref: &str) -> crate::capability_tokens::CapabilityAdmissionReceipt {
        let token = crate::capability_tokens::CapabilityToken {
            token_kind: CLAIM_TOKEN_KIND.to_string(),
            issuer_ref: local_ref("issuer"),
            holder_ref: local_ref("holder"),
            session_ref: local_ref("session"),
            context_ref: local_ref("context"),
            resource_ref: selector_ref.to_string(),
            ability: CLAIM_ATTEST_ABILITY.to_string(),
            scope: "class-membership".to_string(),
            attenuation: "selector-exact".to_string(),
            caveats: Vec::new(),
            expires_at_tick: 10,
            revoked_refs: Vec::new(),
            policy_refs: refs("local-policy"),
            resource_refs: refs("local-resource"),
            delegation_refs: refs("delegation"),
            evidence_refs: refs("token-evidence"),
        };
        let proofset = crate::capability_tokens::CapabilityProofset {
            holder_ref: local_ref("holder"),
            session_ref: local_ref("session"),
            context_ref: local_ref("context"),
            tokens: vec![token],
            policy_refs: refs("local-policy"),
            resource_refs: refs("local-resource"),
            revocation_refs: Vec::new(),
            evidence_refs: refs("proofset-evidence"),
        };
        let request = claim_capability_request(
            &local_ref("holder"),
            &local_ref("session"),
            &local_ref("context"),
            selector_ref,
            "class-membership",
            1,
            &refs("local-policy"),
            &refs("local-resource"),
        )
        .expect("request");
        crate::capability_tokens::admit_capability(&proofset, &request).expect("capability admission")
    }

    fn admission_input() -> ClaimAdmissionInput {
        let selector = selector();
        let selector_ref =
            canonical_hash(&claim_subject_selector_value(&selector).expect("selector")).expect("selector ref");
        ClaimAdmissionInput {
            selector,
            claim: claim(selector_ref.clone()),
            at_tick: 1,
            capability_admission: Some(admitted_capability(&selector_ref)),
            ucan_verification_refs: refs("ucan"),
            basalt_enforcement_refs: refs("basalt"),
            local_policy_refs: refs("local-policy"),
            local_resource_refs: refs("local-resource"),
            freshness_refs: refs("freshness"),
            revocation_refs: refs("revocation-clean"),
            peer_context_refs: refs("peer-session"),
            transport_observation_refs: Vec::new(),
            registry_discovery_refs: Vec::new(),
            local_fixture_grant_refs: Vec::new(),
        }
    }

    // r[verify molten.claim_authority.subject_selectors]
    // r[verify molten.claim_authority.claim_records]
    // r[verify molten.claim_authority.capability_profile]
    // r[verify molten.claim_authority.positive_negative_tests]
    #[test]
    fn admitted_external_claim_binds_capability_ucan_and_basalt_path() {
        let admission = admit_authority_claim(&admission_input()).expect("claim admission");
        assert_eq!(admission.decision, DECISION_PASS);
        assert!(
            crate::preserves_rail::to_text(&admission.value)
                .expect("admission text")
                .contains("authority-claim-admission-v1")
        );
    }

    #[test]
    fn missing_proof_transport_registry_and_fixture_fallback_deny() {
        let mut input = admission_input();
        input.capability_admission = None;
        input.ucan_verification_refs.clear();
        input.basalt_enforcement_refs.clear();
        input.transport_observation_refs = refs("transport");
        input.registry_discovery_refs = refs("registry");
        input.local_fixture_grant_refs = refs("fixture-grant");
        let admission = admit_authority_claim(&input).expect("claim admission");
        assert_eq!(admission.decision, DECISION_DENY);
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic == "missing-claim-capability-admission"));
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "transport-evidence-is-not-claim-authority")
        );
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "registry-discovery-is-not-claim-authority")
        );
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "local-fixture-grant-cannot-satisfy-ucan-basalt-claim")
        );
    }

    #[test]
    fn broad_selector_without_attenuation_denies_visibly() {
        let mut input = admission_input();
        input.selector.selector_kind = SELECTOR_NAMESPACE.to_string();
        input.selector.selector_value = "cluster:friend".to_string();
        input.selector.caveats.clear();
        let selector_ref =
            canonical_hash(&claim_subject_selector_value(&input.selector).expect("selector")).expect("selector ref");
        input.claim.subject_selector_ref = selector_ref;
        let admission = admit_authority_claim(&input).expect("claim admission");
        assert_eq!(admission.decision, DECISION_DENY);
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "broad-selector-without-visible-attenuation")
        );
    }

    // r[verify molten.claim_authority.downstream_consumption]
    #[test]
    fn downstream_claim_use_is_exact_and_wrong_kind_denies() {
        let admission = admit_authority_claim(&admission_input()).expect("claim admission");
        let use_decision = decide_claim_use(&ClaimUseInput {
            admission: admission.clone(),
            required_selector_ref: admission.selector_ref.clone(),
            required_claim_kind: "class-membership".to_string(),
            subject_ref: local_ref("subject"),
            subsystem: "release-gate".to_string(),
            policy_refs: refs("subsystem-policy"),
            resource_refs: refs("subsystem-resource"),
            freshness_ref: local_ref("freshness"),
        })
        .expect("use decision");
        assert_eq!(use_decision.decision, DECISION_PASS);

        let required_selector_ref = admission.selector_ref.clone();
        let wrong_kind = decide_claim_use(&ClaimUseInput {
            admission,
            required_selector_ref,
            required_claim_kind: "release-channel-attestation".to_string(),
            subject_ref: local_ref("subject"),
            subsystem: "release-gate".to_string(),
            policy_refs: refs("subsystem-policy"),
            resource_refs: refs("subsystem-resource"),
            freshness_ref: local_ref("freshness"),
        })
        .expect("wrong kind");
        assert_eq!(wrong_kind.decision, DECISION_DENY);
        assert!(wrong_kind.diagnostics.iter().any(|diagnostic| diagnostic == "claim-kind-mismatch"));
    }

    // r[verify molten.claim_authority.registry_readback]
    // r[verify molten.claim_authority.registry_tests]
    #[test]
    fn registry_readback_classifies_without_authority() {
        let selector_value = claim_subject_selector_value(&selector()).expect("selector");
        assert_eq!(crate::ledger::artifact_kind(&selector_value), "claim-subject-selector");
        let summary = claim_readback_summary(&selector_value).expect("summary").expect("some summary");
        assert!(summary.contains("evidence candidate only"));
    }

    // r[verify molten.claim_authority.peer_diagnostics]
    // r[verify molten.claim_authority.peer_diagnostic_tests]
    #[test]
    fn peer_claim_diagnostics_separate_transport_from_claim_authority() {
        let admission = admit_authority_claim(&admission_input()).expect("claim admission");
        let pass = peer_claim_authority_diagnostic(&PeerClaimDiagnosticInput {
            peer_ref: local_ref("peer"),
            bootstrap_ref: Some(local_ref("bootstrap")),
            session_ref: Some(local_ref("session")),
            transport_refs: refs("transport"),
            claim_admission: Some(admission.clone()),
            claim_kind: "class-membership".to_string(),
            selector_ref: admission.selector_ref,
        })
        .expect("peer diagnostic");
        assert_eq!(pass.decision, DECISION_PASS);
        assert!(pass.diagnostics.iter().any(|diagnostic| diagnostic == "peer-transport-observed-context-only"));

        let deny = peer_claim_authority_diagnostic(&PeerClaimDiagnosticInput {
            peer_ref: local_ref("peer"),
            bootstrap_ref: Some(local_ref("bootstrap")),
            session_ref: Some(local_ref("session")),
            transport_refs: refs("transport"),
            claim_admission: None,
            claim_kind: "class-membership".to_string(),
            selector_ref: local_ref("selector"),
        })
        .expect("peer diagnostic deny");
        assert_eq!(deny.decision, DECISION_DENY);
        assert!(
            deny.diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "peer-claim-authority-missing-capability-ucan-basalt-proof")
        );
    }
}
