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
const MAX_UCAN_RECEIPT_REFS: usize = 1024;

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
    let mut diagnostics = Vec::new();
    for (name, predicate) in UCAN_CHECKS {
        if !predicate(&input.checks) {
            diagnostics.push(format!("UCAN {name} check failed"));
        }
    }
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

const UCAN_VERIFICATION_RECEIPT_ARITY: usize = 19;
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

fn validate_checks(value: &preserves::Value<IoValue>, decision: &str) -> Result<()> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks = crate::preserves_rail::simple_record_fields(&value, "checks", 1)?;
    let entries = crate::preserves_rail::required_sequence_field(&checks[0], "UCAN verification check sequence")?;
    let mut saw_decision = false;
    for entry in entries.as_ref() {
        let entry_value = crate::preserves_rail::value_to_iovalue(entry);
        let check = crate::preserves_rail::simple_record_fields(&entry_value, "check", 2)?;
        let name = required_string(&check[0], "UCAN verification check name")?;
        let status = required_string(&check[1], "UCAN verification check status")?;
        if name == "decision-bound" {
            saw_decision = status == decision;
        } else if decision == DECISION_PASS && status != CHECK_STATUS_PASS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "passing UCAN verification receipt has failing check {name}"
            )));
        }
    }
    if saw_decision {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness("UCAN verification receipt missing decision-bound check"))
    }
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

fn record_string(value: &preserves::Value<IoValue>, label: &str) -> Result<String> {
    crate::preserves_rail::record_string_field(value, label, label)
}

fn record_ref(value: &preserves::Value<IoValue>, label: &str) -> Result<String> {
    crate::preserves_rail::record_content_ref_string(value, label, label)
}

fn record_string_sequence(value: &preserves::Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = crate::preserves_rail::simple_record_fields(&value, label, 1)?;
    let sequence = crate::preserves_rail::required_sequence_field(&record[0], label)?;
    sequence
        .as_ref()
        .iter()
        .map(|entry| crate::preserves_rail::required_string_field(entry, label))
        .collect()
}

fn record_ref_sequence(value: &preserves::Value<IoValue>, label: &str) -> Result<Vec<String>> {
    crate::preserves_rail::record_content_ref_strings(value, label, label, MAX_UCAN_RECEIPT_REFS)
}

fn require_string(value: &preserves::Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported {label} {actual}; expected {expected}"
        )))
    }
}

fn required_string(value: &preserves::Value<IoValue>, label: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(value, label)
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
    fn ucan_verification_receipt_binds_request_and_derived_grants() {
        let input = ucan_input(UcanVerificationChecks::all_pass());
        let receipt = ucan_verification_receipt(&input).expect("UCAN verification receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("hash"));
        let parsed = parse_ucan_verification_receipt_value(&receipt.value).expect("parse UCAN receipt");
        assert_eq!(parsed.request_ref, input.request_ref);
        assert_eq!(parsed.proof_refs, input.proof_refs);
        assert_eq!(parsed.derived_grant_refs, input.derived_grant_refs);
    }

    #[test]
    fn ucan_verification_receipt_denies_invalid_signature_holder_replay_and_missing_proofs() {
        let mut checks = UcanVerificationChecks::all_pass();
        checks.signature_valid = false;
        checks.holder_matches = false;
        checks.replay_fresh = false;
        let mut input = ucan_input(checks);
        input.proof_refs.clear();
        let receipt = ucan_verification_receipt(&input).expect("UCAN denial receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("signature-valid")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("holder-bound")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("replay-fresh")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("proof refs are required")));
    }

    #[test]
    fn imported_token_is_not_operation_authority() {
        let denial = imported_token_authority_denial(&test_ref("token"), "node-control").expect("denial");
        assert_eq!(denial.decision, "deny");
        assert!(denial.diagnostics[0].contains("evidence-only"));
    }

    fn ucan_input(checks: UcanVerificationChecks) -> UcanVerificationInput {
        UcanVerificationInput {
            compact_token_ref: test_ref("compact-token"),
            proofset_ref: test_ref("proofset"),
            proof_refs: vec![test_ref("proof")],
            verification_key_refs: vec![test_ref("key")],
            caveat_decision_refs: vec![test_ref("caveat")],
            revocation_fact_refs: vec![test_ref("revocation")],
            replay_fact_refs: vec![test_ref("replay")],
            derived_grant_refs: vec![test_ref("derived-grant")],
            request_ref: test_ref("request"),
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            resource_ref: test_ref("resource"),
            ability: "publish".to_string(),
            scope: "topic:alerts".to_string(),
            checks,
        }
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
