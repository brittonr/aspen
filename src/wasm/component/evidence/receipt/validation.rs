use super::super::super::model::ComponentDenial;
use super::super::super::model::ComponentDenialClass;
use super::super::super::model::ComponentResult;
use super::super::super::model::EvidenceScope;
use super::super::super::model::sorted_unique;
use super::super::super::model::valid_content_ref;
use super::super::super::profile::COMPONENT_NON_CLAIMS;
use super::ComponentReceipt;
use super::ComponentReceiptDecision;
use super::ComponentReceiptInput;
use super::ComponentReceiptStage;

const MAX_COMPONENT_RECEIPT_REFS: usize = 128;
const MAX_COMPONENT_RECEIPT_DIAGNOSTICS: usize = 64;

pub(super) fn validate_input_bounds(input: &ComponentReceiptInput) -> ComponentResult<()> {
    let mut blockers = Vec::new();
    for (label, count) in [
        ("imports", input.imports.len()),
        ("capabilities", input.capabilities.len()),
        ("Mantle evidence", input.mantle_evidence_refs.len()),
        ("Valence evidence", input.valence_evidence_refs.len()),
        ("Cairn evidence", input.cairn_evidence_refs.len()),
        ("policy", input.policy_refs.len()),
        ("authority", input.authority_refs.len()),
        ("resource", input.resource_refs.len()),
        ("recorded effect", input.recorded_effect_refs.len()),
        ("parent", input.parent_refs.len()),
    ] {
        if count > MAX_COMPONENT_RECEIPT_REFS {
            blockers.push(format!("component receipt {label} refs exceed the canonical bound"));
        }
    }
    if input.diagnostics.len() > MAX_COMPONENT_RECEIPT_DIAGNOSTICS {
        blockers.push("component receipt diagnostics exceed the canonical bound".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(ComponentDenial::from_blockers(blockers))
    }
}

pub(super) fn validate_receipt_shape(receipt: &ComponentReceipt) -> ComponentResult<()> {
    let mut blockers = Vec::new();
    for (label, value) in [
        ("component", receipt.input.component_ref.as_str()),
        ("WIT", receipt.input.wit_ref.as_str()),
        ("profile", receipt.input.profile_ref.as_str()),
        ("runtime configuration", receipt.input.runtime_configuration_ref.as_str()),
    ] {
        if !valid_content_ref(value) {
            blockers.push(format!("component receipt {label} ref is malformed"));
        }
    }
    if receipt.input.evidence_scope == EvidenceScope::Production
        && receipt.input.decision == ComponentReceiptDecision::Pass
        && receipt.input.bundle_ref.as_deref().is_none_or(|value| !valid_content_ref(value))
    {
        blockers.push("production component receipt is missing its Mantle bundle ref".to_string());
    }
    for (label, refs) in [
        ("Mantle evidence", &receipt.input.mantle_evidence_refs),
        ("Valence evidence", &receipt.input.valence_evidence_refs),
        ("Cairn evidence", &receipt.input.cairn_evidence_refs),
        ("policy", &receipt.input.policy_refs),
        ("authority", &receipt.input.authority_refs),
        ("resource", &receipt.input.resource_refs),
        ("recorded effect", &receipt.input.recorded_effect_refs),
        ("parent", &receipt.input.parent_refs),
    ] {
        if refs.iter().any(|value| !valid_content_ref(value)) || sorted_unique(refs) != *refs {
            blockers.push(format!("component receipt {label} refs are malformed, duplicate, or unsorted"));
        }
    }
    if receipt.input.evidence_scope == EvidenceScope::Production
        && receipt.input.decision == ComponentReceiptDecision::Pass
        && (receipt.input.mantle_evidence_refs.is_empty()
            || receipt.input.valence_evidence_refs.is_empty()
            || receipt.input.cairn_evidence_refs.is_empty())
    {
        blockers.push("production component receipt is missing Mantle, Valence, or Cairn evidence refs".to_string());
    }
    validate_stage_fields(receipt, &mut blockers);
    let expected_non_claims = COMPONENT_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    if receipt.non_claims != expected_non_claims {
        blockers.push("component receipt omits or changes required non-claims".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(ComponentDenial::from_blockers(blockers))
    }
}

fn validate_stage_fields(receipt: &ComponentReceipt, blockers: &mut Vec<String>) {
    match receipt.input.decision {
        ComponentReceiptDecision::Deny => validate_denial_fields(receipt, blockers),
        ComponentReceiptDecision::Pass => {
            if receipt.input.trap_class.is_some() || !receipt.input.diagnostics.is_empty() {
                blockers.push("passing component receipt cannot carry denial class or diagnostics".to_string());
            }
        }
    }
    if receipt.input.decision == ComponentReceiptDecision::Pass
        && matches!(
            receipt.input.stage,
            ComponentReceiptStage::Instantiation | ComponentReceiptStage::Execution | ComponentReceiptStage::Hostcall
        )
        && receipt.input.parent_refs.len() != 1
    {
        blockers.push("passing component stage receipt requires exactly one parent stage ref".to_string());
    }
    if receipt.input.stage == ComponentReceiptStage::Execution
        && receipt.input.decision == ComponentReceiptDecision::Pass
        && (receipt.input.input_ref.as_deref().is_none_or(|value| !valid_content_ref(value))
            || receipt.input.output_ref.as_deref().is_none_or(|value| !valid_content_ref(value))
            || receipt.input.fuel_limit.is_none()
            || receipt.input.fuel_remaining.is_none())
    {
        blockers.push("passing component execution receipt is missing input, output, or fuel evidence".to_string());
    }
    if receipt.input.stage == ComponentReceiptStage::Hostcall
        && receipt.input.decision == ComponentReceiptDecision::Pass
        && (receipt.input.imports.is_empty()
            || receipt.input.capabilities.is_empty()
            || receipt.input.policy_refs.is_empty()
            || receipt.input.authority_refs.is_empty()
            || receipt.input.resource_refs.is_empty()
            || receipt.input.recorded_effect_refs.is_empty()
            || receipt.input.input_ref.as_deref().is_none_or(|value| !valid_content_ref(value))
            || receipt.input.output_ref.as_deref().is_none_or(|value| !valid_content_ref(value)))
    {
        blockers.push(
            "passing component hostcall receipt is missing import, authority, effect, or outcome evidence".to_string(),
        );
    }
    if let (Some(limit), Some(remaining)) = (receipt.input.fuel_limit, receipt.input.fuel_remaining)
        && remaining > limit
    {
        blockers.push("component receipt fuel remaining exceeds its admitted limit".to_string());
    }
}

fn validate_denial_fields(receipt: &ComponentReceipt, blockers: &mut Vec<String>) {
    if receipt.input.stage != ComponentReceiptStage::Denial {
        blockers.push("denied component receipt must use the denial stage".to_string());
    }
    let Some(trap_class) = receipt.input.trap_class.as_deref() else {
        blockers.push("component denial receipt requires a canonical denial class".to_string());
        return;
    };
    if ComponentDenialClass::parse(trap_class).is_none() {
        blockers.push("component denial receipt class is not recognized".to_string());
    }
    if !matches!(receipt.input.diagnostics.as_slice(), [diagnostic] if diagnostic == trap_class) {
        blockers.push("component denial receipt diagnostics must contain only its canonical class".to_string());
    }
}
