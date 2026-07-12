use super::super::model::ComponentConsumer;
use super::super::model::ComponentDenial;
use super::super::model::ComponentResult;
use super::super::model::EvidenceScope;
use super::super::model::sorted_unique;
use super::super::profile::COMPONENT_NON_CLAIMS;

mod validation;

pub const COMPONENT_RECEIPT_SCHEMA: &str = "molten.wasm-component-receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentReceiptStage {
    Inspection,
    Instantiation,
    Execution,
    Hostcall,
    Denial,
    Migration,
}

impl ComponentReceiptStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inspection => "inspection",
            Self::Instantiation => "instantiation",
            Self::Execution => "execution",
            Self::Hostcall => "hostcall",
            Self::Denial => "denial",
            Self::Migration => "migration",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentReceiptDecision {
    Pass,
    Deny,
}

impl ComponentReceiptDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentReceiptInput {
    pub stage: ComponentReceiptStage,
    pub decision: ComponentReceiptDecision,
    pub evidence_scope: EvidenceScope,
    pub consumer: ComponentConsumer,
    pub component_ref: String,
    pub wit_ref: String,
    pub profile_ref: String,
    pub runtime_configuration_ref: String,
    pub bundle_ref: Option<String>,
    pub imports: Vec<String>,
    pub capabilities: Vec<String>,
    pub mantle_evidence_refs: Vec<String>,
    pub valence_evidence_refs: Vec<String>,
    pub cairn_evidence_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub recorded_effect_refs: Vec<String>,
    pub input_ref: Option<String>,
    pub output_ref: Option<String>,
    pub fuel_limit: Option<u64>,
    pub fuel_remaining: Option<u64>,
    pub trap_class: Option<String>,
    pub parent_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentReceipt {
    pub input: ComponentReceiptInput,
    pub non_claims: Vec<String>,
    pub receipt_ref: String,
}

pub fn build_component_receipt(input: ComponentReceiptInput) -> ComponentResult<ComponentReceipt> {
    validation::validate_input_bounds(&input)?;
    let non_claims = COMPONENT_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    let mut receipt = ComponentReceipt {
        input: normalize_input(input),
        non_claims,
        receipt_ref: String::new(),
    };
    validation::validate_receipt_shape(&receipt)?;
    receipt.receipt_ref = crate::preserves_rail::canonical_hash(&component_receipt_value(&receipt))
        .map_err(|error| ComponentDenial::new(format!("component receipt hashing failed: {error}")))?;
    Ok(receipt)
}

pub fn validate_component_receipt(receipt: &ComponentReceipt) -> ComponentResult<()> {
    validation::validate_input_bounds(&receipt.input)?;
    validation::validate_receipt_shape(receipt)?;
    let expected = crate::preserves_rail::canonical_hash(&component_receipt_value(receipt))
        .map_err(|error| ComponentDenial::new(format!("component receipt hashing failed: {error}")))?;
    if receipt.receipt_ref != expected {
        return Err(ComponentDenial::new("component receipt identity is stale or cross-profile"));
    }
    Ok(())
}

pub fn validate_component_receipt_against(
    receipt: &ComponentReceipt,
    expected_input: &ComponentReceiptInput,
) -> ComponentResult<()> {
    validate_component_receipt(receipt)?;
    let expected = build_component_receipt(expected_input.clone())?;
    if receipt != &expected {
        return Err(ComponentDenial::new(
            "component receipt differs from the expected inspected bytes or execution plan",
        ));
    }
    Ok(())
}

pub fn validate_component_receipt_chain(receipts: &[ComponentReceipt]) -> ComponentResult<()> {
    if receipts.is_empty() {
        return Err(ComponentDenial::new("component receipt chain cannot be empty"));
    }
    let mut previous_ref = None;
    for receipt in receipts {
        validate_component_receipt(receipt)?;
        let expected_parent_refs = previous_ref.iter().cloned().collect::<Vec<_>>();
        if receipt.input.parent_refs != expected_parent_refs {
            return Err(ComponentDenial::new("component receipt parent does not match the preceding canonical stage"));
        }
        previous_ref = Some(receipt.receipt_ref.clone());
    }
    Ok(())
}

pub fn component_receipt_value(receipt: &ComponentReceipt) -> preserves::IOValue {
    use crate::preserves_rail::optional_ref_value;
    use crate::preserves_rail::record;
    use crate::preserves_rail::sequence;
    use crate::preserves_rail::string;
    use crate::preserves_rail::u64_value;

    record("wasm-component-receipt-v1", vec![
        record("schema", vec![string(COMPONENT_RECEIPT_SCHEMA)]),
        record("stage", vec![string(receipt.input.stage.as_str())]),
        record("decision", vec![string(receipt.input.decision.as_str())]),
        record("evidence-scope", vec![string(receipt.input.evidence_scope.as_str())]),
        record("consumer", vec![string(receipt.input.consumer.as_str())]),
        record("component-ref", vec![string(&receipt.input.component_ref)]),
        record("wit-ref", vec![string(&receipt.input.wit_ref)]),
        record("profile-ref", vec![string(&receipt.input.profile_ref)]),
        record("runtime-configuration-ref", vec![string(&receipt.input.runtime_configuration_ref)]),
        record("bundle-ref", vec![optional_ref_value(receipt.input.bundle_ref.as_deref())]),
        record("imports", vec![strings(&receipt.input.imports)]),
        record("capabilities", vec![strings(&receipt.input.capabilities)]),
        record("mantle-evidence-refs", vec![strings(&receipt.input.mantle_evidence_refs)]),
        record("valence-evidence-refs", vec![strings(&receipt.input.valence_evidence_refs)]),
        record("cairn-evidence-refs", vec![strings(&receipt.input.cairn_evidence_refs)]),
        record("policy-refs", vec![strings(&receipt.input.policy_refs)]),
        record("authority-refs", vec![strings(&receipt.input.authority_refs)]),
        record("resource-refs", vec![strings(&receipt.input.resource_refs)]),
        record("recorded-effect-refs", vec![strings(&receipt.input.recorded_effect_refs)]),
        record("input-ref", vec![optional_ref_value(receipt.input.input_ref.as_deref())]),
        record("output-ref", vec![optional_ref_value(receipt.input.output_ref.as_deref())]),
        record("fuel-limit", vec![optional_u64(receipt.input.fuel_limit)]),
        record("fuel-remaining", vec![optional_u64(receipt.input.fuel_remaining)]),
        record("trap-class", vec![
            receipt
                .input
                .trap_class
                .as_deref()
                .map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)])),
        ]),
        record("parent-refs", vec![strings(&receipt.input.parent_refs)]),
        record("diagnostics", vec![strings(&receipt.input.diagnostics)]),
        record("non-claims", vec![sequence(receipt.non_claims.iter().map(string).collect())]),
        record("fuel-observation", vec![
            u64_value(receipt.input.fuel_limit.unwrap_or(0)),
            u64_value(receipt.input.fuel_remaining.unwrap_or(0)),
        ]),
    ])
}

pub fn component_receipt_summary(receipt: &ComponentReceipt) -> String {
    format!(
        "wasm component {} {} profile={} component={} receipt={} (non-normative)",
        receipt.input.stage.as_str(),
        receipt.input.decision.as_str(),
        receipt.input.profile_ref,
        receipt.input.component_ref,
        receipt.receipt_ref
    )
}

pub fn replay_receipts_match(left: &[ComponentReceipt], right: &[ComponentReceipt]) -> bool {
    left == right && validate_component_receipt_chain(left).is_ok() && validate_component_receipt_chain(right).is_ok()
}

fn normalize_input(mut input: ComponentReceiptInput) -> ComponentReceiptInput {
    input.imports = sorted_unique(&input.imports);
    input.capabilities = sorted_unique(&input.capabilities);
    input.mantle_evidence_refs = sorted_unique(&input.mantle_evidence_refs);
    input.valence_evidence_refs = sorted_unique(&input.valence_evidence_refs);
    input.cairn_evidence_refs = sorted_unique(&input.cairn_evidence_refs);
    input.policy_refs = sorted_unique(&input.policy_refs);
    input.authority_refs = sorted_unique(&input.authority_refs);
    input.resource_refs = sorted_unique(&input.resource_refs);
    input.recorded_effect_refs = sorted_unique(&input.recorded_effect_refs);
    input.parent_refs = sorted_unique(&input.parent_refs);
    input.diagnostics = sorted_unique(&input.diagnostics);
    input
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
    )
}
