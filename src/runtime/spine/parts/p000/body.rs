type IoValue = preserves::IOValue;

const BASALT_UCAN_AUTHORITY_RECEIPT_SCHEMA: &str = "molten.runtime.basalt-ucan-authority-receipt.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const BASALT_AUTHORITY_COMPONENT: &str = "basalt-contract";
const MAX_SPINE_DIAGNOSTICS: usize = 256;

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
    let diagnostics = basalt_ucan_diagnostics(input)?;
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

