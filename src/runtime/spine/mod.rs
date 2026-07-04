type IoValue = preserves::IOValue;

const BASALT_UCAN_AUTHORITY_RECEIPT_SCHEMA: &str = "molten.runtime.basalt-ucan-authority-receipt.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const BASALT_AUTHORITY_COMPONENT: &str = "basalt-contract";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractBackend {
    NickelStatic,
    SteelReviewed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ContractDecision {
    pub backend: ContractBackend,
    pub contract_id: String,
    pub contract_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BasaltRequest {
    pub contract_id: String,
    pub resource: String,
    pub ability: String,
    pub ucan_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct VerifiedBasaltGrant {
    pub grant_ref: String,
    pub verification_receipt_ref: String,
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub resource: String,
    pub ability: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BasaltUcanAuthorityInput {
    pub contract_id: String,
    pub resource: String,
    pub ability: String,
    pub holder_ref: String,
    pub session_ref: String,
    pub context_ref: String,
    pub request_ref: String,
    pub basalt_policy_ref: String,
    pub basalt_policy_source_ref: String,
    pub basalt_policy_export_ref: String,
    pub proofset_ref: String,
    pub ucan_verification_receipt_refs: Vec<String>,
    pub verified_grants: Vec<VerifiedBasaltGrant>,
    pub policy_allows: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasaltUcanAuthorityReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PolicyGateReceipt {
    pub envelope_ref: String,
    pub decision: String,
    pub predicate: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ValenceEvidenceRef {
    pub evidence_ref: String,
    pub claim: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiptIndex {
    entries: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct IntegrationEvidence {
    pub config_ref: String,
    pub local_route_ref: String,
    pub remote_bridge_ref: String,
    pub policy_ref: String,
}

pub fn nickel_contract_decision(contract_id: impl Into<String>, source: &[u8]) -> ContractDecision {
    ContractDecision {
        backend: ContractBackend::NickelStatic,
        contract_id: contract_id.into(),
        contract_ref: crate::preserves_rail::content_ref_from_bytes(source),
    }
}

pub fn steel_contract_decision(contract_id: impl Into<String>, reviewed_script: &[u8]) -> ContractDecision {
    ContractDecision {
        backend: ContractBackend::SteelReviewed,
        contract_id: contract_id.into(),
        contract_ref: crate::preserves_rail::content_ref_from_bytes(reviewed_script),
    }
}

pub fn evaluate_basalt_request(
    request: &BasaltRequest,
    expected_resource: &str,
    expected_ability: &str,
) -> std::result::Result<(), super::RuntimeBoundaryError> {
    crate::preserves_rail::validate_content_ref(&request.ucan_ref)
        .map_err(|error| super::RuntimeBoundaryError::invalid_input(BASALT_AUTHORITY_COMPONENT, error.to_string()))?;
    if request.resource != expected_resource || request.ability != expected_ability {
        return Err(super::RuntimeBoundaryError::denied_operation(
            BASALT_AUTHORITY_COMPONENT,
            format!("request does not match resource={expected_resource} ability={expected_ability}"),
        ));
    }
    Err(super::RuntimeBoundaryError::denied_operation(
        BASALT_AUTHORITY_COMPONENT,
        "bare ucan_ref is not current authority; verified grant refs and UCAN verification receipt refs are required",
    ))
}

pub fn evaluate_basalt_ucan_authority(
    input: &BasaltUcanAuthorityInput,
) -> std::result::Result<BasaltUcanAuthorityReceipt, super::RuntimeBoundaryError> {
    validate_basalt_ucan_input(input)?;
    let diagnostics = basalt_ucan_diagnostics(input);
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let derived_grant_refs = input.verified_grants.iter().map(|grant| grant.grant_ref.clone()).collect::<Vec<_>>();
    let value = basalt_ucan_receipt_value(input, decision, &diagnostics, &derived_grant_refs);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)
        .map_err(|error| super::RuntimeBoundaryError::invalid_input(BASALT_AUTHORITY_COMPONENT, error.to_string()))?;
    Ok(BasaltUcanAuthorityReceipt {
        decision: decision.to_string(),
        diagnostics,
        derived_grant_refs,
        value,
        receipt_ref,
    })
}

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

fn basalt_ucan_diagnostics(input: &BasaltUcanAuthorityInput) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if input.ucan_verification_receipt_refs.is_empty() {
        diagnostics.push("missing UCAN verification receipt refs".to_string());
    }
    if input.verified_grants.is_empty() {
        diagnostics.push("missing verified UCAN-derived grants".to_string());
    }
    if !input.policy_allows {
        diagnostics.push("Basalt policy denied requested resource or ability".to_string());
    }
    for grant in &input.verified_grants {
        if !input
            .ucan_verification_receipt_refs
            .iter()
            .any(|receipt_ref| receipt_ref == &grant.verification_receipt_ref)
        {
            diagnostics.push(format!("grant {} is not bound to a supplied UCAN verification receipt", grant.grant_ref));
        }
        push_authority_mismatch(&mut diagnostics, "holder", &grant.holder_ref, &input.holder_ref);
        push_authority_mismatch(&mut diagnostics, "session", &grant.session_ref, &input.session_ref);
        push_authority_mismatch(&mut diagnostics, "context", &grant.context_ref, &input.context_ref);
        push_authority_mismatch(&mut diagnostics, "resource", &grant.resource, &input.resource);
        push_authority_mismatch(&mut diagnostics, "ability", &grant.ability, &input.ability);
    }
    diagnostics
}

fn push_authority_mismatch(diagnostics: &mut Vec<String>, label: &str, actual: &str, expected: &str) {
    if actual != expected {
        diagnostics.push(format!("verified grant {label} mismatch expected {expected} actual {actual}"));
    }
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

#[cfg(test)]
mod tests {
    fn authority_input(resource: &str, ability: &str, policy_allows: bool) -> super::BasaltUcanAuthorityInput {
        super::BasaltUcanAuthorityInput {
            contract_id: "contract:send".to_string(),
            resource: resource.to_string(),
            ability: ability.to_string(),
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            request_ref: test_ref("request"),
            basalt_policy_ref: test_ref("policy"),
            basalt_policy_source_ref: test_ref("policy-source"),
            basalt_policy_export_ref: test_ref("policy-export"),
            proofset_ref: test_ref("proofset"),
            ucan_verification_receipt_refs: vec![test_ref("ucan-verification")],
            verified_grants: vec![super::VerifiedBasaltGrant {
                grant_ref: test_ref("grant"),
                verification_receipt_ref: test_ref("ucan-verification"),
                holder_ref: test_ref("holder"),
                session_ref: test_ref("session"),
                context_ref: test_ref("context"),
                resource: resource.to_string(),
                ability: ability.to_string(),
                scope: "topic".to_string(),
            }],
            policy_allows,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn envelope(capability: &str) -> crate::runtime::Envelope {
        crate::runtime::Envelope::new(crate::runtime::EnvelopeInput {
            sender: crate::runtime::ActorId::parse("actor:policy").expect("sender"),
            subject: crate::runtime::RuntimeValue::string("policy.subject").expect("subject"),
            body: crate::runtime::RuntimeValue::string("body").expect("body"),
            blob_refs: Vec::new(),
            capabilities: vec![crate::runtime::Capability::parse(capability).expect("capability")],
            evidence_refs: Vec::new(),
        })
        .expect("envelope")
    }

    #[test]
    fn contract_selection_records_static_nickel_and_reviewed_steel() {
        let nickel = super::nickel_contract_decision("static-policy", b"{ allowed = true }");
        let steel = super::steel_contract_decision("dynamic-review", b"(lambda (x) x)");
        assert_eq!(nickel.backend, super::ContractBackend::NickelStatic);
        assert_eq!(steel.backend, super::ContractBackend::SteelReviewed);
        assert_ne!(nickel.contract_ref, steel.contract_ref);
    }

    #[test]
    fn basalt_request_requires_verified_authority() {
        let request = super::BasaltRequest {
            contract_id: "contract:send".to_string(),
            resource: "subject:ready".to_string(),
            ability: "send".to_string(),
            ucan_ref: crate::preserves_rail::content_ref_from_bytes(b"ucan"),
        };
        let error =
            super::evaluate_basalt_request(&request, "subject:ready", "send").expect_err("bare UCAN ref denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
        assert!(error.to_string().contains("bare ucan_ref"));
        let error =
            super::evaluate_basalt_request(&request, "subject:other", "send").expect_err("wrong resource denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
    }

    #[test]
    fn basalt_ucan_authority_admits_verified_grant_and_denies_mismatches() {
        let input = authority_input("subject:ready", "send", true);
        let receipt = super::evaluate_basalt_ucan_authority(&input).expect("authority receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("hash"));

        let mut denied = input.clone();
        denied.policy_allows = false;
        denied.verified_grants[0].resource = "subject:other".to_string();
        let receipt = super::evaluate_basalt_ucan_authority(&denied).expect("deny receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("Basalt policy denied")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource mismatch")));
    }

    #[test]
    fn policy_gate_records_pass_and_deny_receipts() {
        let pass =
            super::policy_gate_receipt(&envelope("send:policy.subject"), "send:policy.subject").expect("pass receipt");
        let deny = super::policy_gate_receipt(&envelope("send:other"), "send:policy.subject").expect("deny receipt");
        assert_eq!(pass.decision, "pass");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics[0].contains("missing capability"));
    }

    #[test]
    fn cairn_valence_and_receipt_index_refs_are_canonical() {
        let reference = crate::preserves_rail::content_ref_from_bytes(b"receipt");
        super::validate_cairn_receipt_ref(&reference).expect("cairn receipt ref");
        let evidence = super::valence_evidence_ref(reference.clone(), "function-object").expect("valence evidence");
        assert_eq!(evidence.evidence_ref, reference);

        let mut index = super::ReceiptIndex::default();
        index.insert("turn:1", evidence.evidence_ref.clone()).expect("insert receipt");
        assert_eq!(index.get("turn:1"), Some(evidence.evidence_ref.as_str()));
    }

    #[test]
    fn integration_evidence_binds_config_route_remote_and_policy_refs() {
        let evidence = super::integration_evidence(b"config", b"local", b"remote", b"policy");
        assert_eq!(evidence.config_ref, crate::preserves_rail::content_ref_from_bytes(b"config"));
        assert_eq!(evidence.local_route_ref, crate::preserves_rail::content_ref_from_bytes(b"local"));
        assert_eq!(evidence.remote_bridge_ref, crate::preserves_rail::content_ref_from_bytes(b"remote"));
        assert_eq!(evidence.policy_ref, crate::preserves_rail::content_ref_from_bytes(b"policy"));
    }

    #[hegel::test(test_cases = 8)]
    fn hegel_envelope_policy_identity_is_stable(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let capability = format!("send:policy.subject.{salt}");
        let envelope = envelope(&capability);
        let left = super::policy_gate_receipt(&envelope, &capability).expect("left");
        let right = super::policy_gate_receipt(&envelope, &capability).expect("right");
        assert_eq!(left.envelope_ref, right.envelope_ref);
        assert_eq!(left.decision, "pass");
    }
}
