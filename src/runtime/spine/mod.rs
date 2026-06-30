use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::Envelope;
use super::RuntimeBoundaryError;
use crate::error::Result;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::validate_content_ref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractBackend {
    NickelStatic,
    SteelReviewed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeContractDecision {
    pub backend: ContractBackend,
    pub contract_id: String,
    pub contract_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasaltRuntimeRequest {
    pub contract_id: String,
    pub resource: String,
    pub ability: String,
    pub ucan_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePolicyGateReceipt {
    pub envelope_ref: String,
    pub decision: String,
    pub predicate: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValenceEvidenceRef {
    pub evidence_ref: String,
    pub claim: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeReceiptIndex {
    entries: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeIntegrationEvidence {
    pub config_ref: String,
    pub local_route_ref: String,
    pub remote_bridge_ref: String,
    pub policy_ref: String,
}

pub fn nickel_contract_decision(contract_id: impl Into<String>, source: &[u8]) -> RuntimeContractDecision {
    RuntimeContractDecision {
        backend: ContractBackend::NickelStatic,
        contract_id: contract_id.into(),
        contract_ref: content_ref_from_bytes(source),
    }
}

pub fn steel_contract_decision(contract_id: impl Into<String>, reviewed_script: &[u8]) -> RuntimeContractDecision {
    RuntimeContractDecision {
        backend: ContractBackend::SteelReviewed,
        contract_id: contract_id.into(),
        contract_ref: content_ref_from_bytes(reviewed_script),
    }
}

pub fn evaluate_basalt_runtime_request(
    request: &BasaltRuntimeRequest,
    expected_resource: &str,
    expected_ability: &str,
) -> std::result::Result<(), RuntimeBoundaryError> {
    validate_content_ref(&request.ucan_ref)
        .map_err(|error| RuntimeBoundaryError::invalid_input("basalt-contract", error.to_string()))?;
    if request.resource != expected_resource || request.ability != expected_ability {
        return Err(RuntimeBoundaryError::denied_operation(
            "basalt-contract",
            format!("request does not match resource={expected_resource} ability={expected_ability}"),
        ));
    }
    Ok(())
}

pub fn policy_gate_receipt(envelope: &Envelope, required_capability: &str) -> Result<RuntimePolicyGateReceipt> {
    let envelope_ref = envelope.canonical_hash()?;
    let has_capability = envelope.capabilities.iter().any(|capability| capability.as_str() == required_capability);
    let mut diagnostics = Vec::new();
    if !has_capability {
        diagnostics.push(format!("missing capability {required_capability}"));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(RuntimePolicyGateReceipt {
        envelope_ref,
        decision: decision.to_string(),
        predicate: "runtime-spine-policy-gate-v1".to_string(),
        diagnostics,
    })
}

pub fn validate_cairn_receipt_ref(reference: &str) -> std::result::Result<(), RuntimeBoundaryError> {
    validate_content_ref(reference)
        .map_err(|error| RuntimeBoundaryError::invalid_input("cairn-receipt", error.to_string()))
}

pub fn valence_evidence_ref(reference: impl Into<String>, claim: impl Into<String>) -> Result<ValenceEvidenceRef> {
    let evidence_ref = reference.into();
    validate_content_ref(&evidence_ref)?;
    Ok(ValenceEvidenceRef {
        evidence_ref,
        claim: claim.into(),
    })
}

impl RuntimeReceiptIndex {
    pub fn insert(&mut self, key: impl Into<String>, receipt_ref: impl Into<String>) -> Result<()> {
        let receipt_ref = receipt_ref.into();
        validate_content_ref(&receipt_ref)?;
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
) -> RuntimeIntegrationEvidence {
    RuntimeIntegrationEvidence {
        config_ref: content_ref_from_bytes(config),
        local_route_ref: content_ref_from_bytes(local_route),
        remote_bridge_ref: content_ref_from_bytes(remote_bridge),
        policy_ref: content_ref_from_bytes(policy),
    }
}

#[cfg(test)]
mod tests {
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
    fn basalt_runtime_request_matches_resource_and_ability() {
        let request = super::BasaltRuntimeRequest {
            contract_id: "contract:send".to_string(),
            resource: "subject:ready".to_string(),
            ability: "send".to_string(),
            ucan_ref: crate::preserves_rail::content_ref_from_bytes(b"ucan"),
        };
        super::evaluate_basalt_runtime_request(&request, "subject:ready", "send").expect("request admitted");
        let error = super::evaluate_basalt_runtime_request(&request, "subject:other", "send")
            .expect_err("wrong resource denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
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

        let mut index = super::RuntimeReceiptIndex::default();
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
