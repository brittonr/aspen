use super::super::*;
use super::support::ComponentFixture;
use super::support::fixture_ref;

const OVER_BOUND_RECEIPT_REF_COUNT: usize = 129;

fn receipt_input(
    fixture: &ComponentFixture,
    stage: ComponentReceiptStage,
    decision: ComponentReceiptDecision,
) -> ComponentReceiptInput {
    let denial_class = (decision == ComponentReceiptDecision::Deny)
        .then(|| ComponentDenialClass::ComponentAdmissionDenial.as_str().to_string());
    ComponentReceiptInput {
        stage,
        decision,
        evidence_scope: EvidenceScope::Production,
        consumer: ComponentConsumer::Actor,
        component_ref: fixture.bundle.component.content_ref.clone(),
        wit_ref: fixture.bundle.wit.content_ref.clone(),
        profile_ref: component_profile_ref(&fixture.profile),
        runtime_configuration_ref: fixture_ref("runtime-configuration"),
        bundle_ref: Some(fixture.bundle.bundle_ref.clone()),
        imports: Vec::new(),
        capabilities: Vec::new(),
        mantle_evidence_refs: vec![fixture_ref("mantle-evidence")],
        valence_evidence_refs: fixture.envelope.valence_sidecar_refs.clone(),
        cairn_evidence_refs: fixture.envelope.cairn_acceptance_refs.clone(),
        policy_refs: fixture.envelope.policy_refs.clone(),
        authority_refs: fixture.envelope.authority_refs.clone(),
        resource_refs: fixture.envelope.resource_refs.clone(),
        recorded_effect_refs: Vec::new(),
        input_ref: None,
        output_ref: None,
        fuel_limit: None,
        fuel_remaining: None,
        trap_class: denial_class.clone(),
        parent_refs: Vec::new(),
        diagnostics: denial_class.into_iter().collect(),
    }
}

#[test]
fn every_component_stage_emits_a_canonical_profile_bound_receipt() {
    // r[verify molten.wasm_component.receipts]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    for stage in [
        ComponentReceiptStage::Inspection,
        ComponentReceiptStage::Instantiation,
        ComponentReceiptStage::Execution,
        ComponentReceiptStage::Hostcall,
        ComponentReceiptStage::Denial,
        ComponentReceiptStage::Migration,
    ] {
        let decision = if stage == ComponentReceiptStage::Denial {
            ComponentReceiptDecision::Deny
        } else {
            ComponentReceiptDecision::Pass
        };
        let mut input = receipt_input(&fixture, stage, decision);
        if matches!(
            stage,
            ComponentReceiptStage::Instantiation | ComponentReceiptStage::Execution | ComponentReceiptStage::Hostcall
        ) {
            input.parent_refs = vec![fixture_ref("parent-stage")];
        }
        if stage == ComponentReceiptStage::Execution {
            input.input_ref = Some(fixture_ref("input"));
            input.output_ref = Some(fixture_ref("output"));
            input.fuel_limit = Some(fixture.profile.resources.fuel);
            input.fuel_remaining = Some(fixture.profile.resources.fuel);
        }
        if stage == ComponentReceiptStage::Hostcall {
            input.imports = vec!["molten:fixture/effect@1.0.0".to_string()];
            input.capabilities = vec!["fixture-effect".to_string()];
            input.recorded_effect_refs = vec![fixture_ref("recorded-effect")];
            input.input_ref = Some(fixture_ref("hostcall-input"));
            input.output_ref = Some(fixture_ref("hostcall-output"));
        }
        let receipt = build_component_receipt(input).expect("component receipt");
        validate_component_receipt(&receipt).expect("receipt validates");
        assert!(super::super::model::valid_content_ref(&receipt.receipt_ref));
    }
}

#[test]
fn stale_cross_profile_and_overclaiming_receipts_fail_closed() {
    // r[verify molten.wasm_component.receipts]
    // r[verify molten.wasm_component.nonclaims]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    let expected_input = receipt_input(&fixture, ComponentReceiptStage::Inspection, ComponentReceiptDecision::Pass);
    let receipt = build_component_receipt(expected_input.clone()).expect("receipt");

    let mut missing_evidence =
        receipt_input(&fixture, ComponentReceiptStage::Inspection, ComponentReceiptDecision::Pass);
    missing_evidence.valence_evidence_refs.clear();
    assert!(build_component_receipt(missing_evidence).is_err());

    let mut over_bound = receipt_input(&fixture, ComponentReceiptStage::Inspection, ComponentReceiptDecision::Pass);
    over_bound.policy_refs = vec![fixture_ref("over-bound-policy"); OVER_BOUND_RECEIPT_REF_COUNT];
    assert!(build_component_receipt(over_bound).is_err());

    let missing_hostcall = receipt_input(&fixture, ComponentReceiptStage::Hostcall, ComponentReceiptDecision::Pass);
    assert!(build_component_receipt(missing_hostcall).is_err());

    let mut unknown_denial = receipt_input(&fixture, ComponentReceiptStage::Denial, ComponentReceiptDecision::Deny);
    unknown_denial.trap_class = Some("guest-controlled-class".to_string());
    unknown_denial.diagnostics = vec!["guest-controlled-class".to_string()];
    assert!(build_component_receipt(unknown_denial).is_err());

    let mut raw_denial = receipt_input(&fixture, ComponentReceiptStage::Denial, ComponentReceiptDecision::Deny);
    raw_denial.diagnostics = vec!["raw runtime diagnostic".to_string()];
    assert!(build_component_receipt(raw_denial).is_err());

    let mut self_consistent_stale = receipt.clone();
    self_consistent_stale.input.profile_ref = fixture_ref("other-profile");
    self_consistent_stale.receipt_ref =
        crate::preserves_rail::canonical_hash(&component_receipt_value(&self_consistent_stale))
            .expect("self-consistent stale receipt hash");
    assert!(validate_component_receipt_against(&self_consistent_stale, &expected_input).is_err());

    let mut stale = receipt.clone();
    stale.input.profile_ref = fixture_ref("other-profile");
    assert!(validate_component_receipt(&stale).is_err());

    let mut wrong_parent_input =
        receipt_input(&fixture, ComponentReceiptStage::Instantiation, ComponentReceiptDecision::Pass);
    wrong_parent_input.parent_refs = vec![fixture_ref("wrong-parent")];
    let wrong_parent = build_component_receipt(wrong_parent_input).expect("self-consistent wrong parent receipt");
    assert!(validate_component_receipt_chain(&[receipt.clone(), wrong_parent]).is_err());
    assert!(!replay_receipts_match(&[], &[]));

    let mut overclaim = receipt;
    overclaim.non_claims.retain(|claim| claim != "not-behavioral-correctness");
    overclaim.receipt_ref =
        crate::preserves_rail::canonical_hash(&component_receipt_value(&overclaim)).expect("tampered receipt hash");
    assert!(validate_component_receipt(&overclaim).is_err());
}

#[test]
fn operator_readback_and_replay_keep_component_evidence_distinct() {
    // r[verify molten.wasm_component.receipts]
    // r[verify molten.wasm_component.migration]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    let receipt = build_component_receipt(receipt_input(
        &fixture,
        ComponentReceiptStage::Migration,
        ComponentReceiptDecision::Pass,
    ))
    .expect("migration receipt");
    let summary = component_receipt_summary(&receipt);
    assert!(summary.contains("wasm component migration pass"));
    assert!(summary.contains("non-normative"));
    assert!(!summary.contains("molten.wasm.abi.v1"));
    assert!(replay_receipts_match(std::slice::from_ref(&receipt), std::slice::from_ref(&receipt)));
}
