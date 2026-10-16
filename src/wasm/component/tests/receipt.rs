use super::super::*;

const OVER_BOUND_RECEIPT_REF_COUNT: usize = 129;

fn input(fixture: &super::support::Fixture, stage: Stage, decision: Decision) -> Input {
    let denial_class = (decision == Decision::Deny).then(|| DenialClass::ComponentAdmissionDenial.as_str().to_string());
    Input {
        stage,
        decision,
        evidence_scope: EvidenceScope::Production,
        consumer: Consumer::Actor,
        component_ref: fixture.bundle.component.content_ref.clone(),
        wit_ref: fixture.bundle.wit.content_ref.clone(),
        profile_ref: r#ref(&fixture.profile),
        runtime_configuration_ref: super::support::fixture_ref("runtime-configuration"),
        bundle_ref: Some(fixture.bundle.bundle_ref.clone()),
        imports: Vec::new(),
        capabilities: Vec::new(),
        mantle_evidence_refs: vec![super::support::fixture_ref("mantle-evidence")],
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
fn every_stage_emits_a_canonical_profile_bound() {
    // r[verify molten.wasm_component.receipts]
    let fixture = super::support::Fixture::new(Consumer::Actor);
    for stage in [
        Stage::Inspection,
        Stage::Instantiation,
        Stage::Execution,
        Stage::Hostcall,
        Stage::Denial,
        Stage::Migration,
    ] {
        let decision = if stage == Stage::Denial {
            Decision::Deny
        } else {
            Decision::Pass
        };
        let mut input = input(&fixture, stage, decision);
        if matches!(stage, Stage::Instantiation | Stage::Execution | Stage::Hostcall) {
            input.parent_refs = vec![super::support::fixture_ref("parent-stage")];
        }
        if stage == Stage::Execution {
            input.input_ref = Some(super::support::fixture_ref("input"));
            input.output_ref = Some(super::support::fixture_ref("output"));
            input.fuel_limit = Some(fixture.profile.resources.fuel);
            input.fuel_remaining = Some(fixture.profile.resources.fuel);
        }
        if stage == Stage::Hostcall {
            input.imports = vec!["molten:fixture/effect@1.0.0".to_string()];
            input.capabilities = vec!["fixture-effect".to_string()];
            input.recorded_effect_refs = vec![super::support::fixture_ref("recorded-effect")];
            input.input_ref = Some(super::support::fixture_ref("hostcall-input"));
            input.output_ref = Some(super::support::fixture_ref("hostcall-output"));
        }
        let receipt = build(input).expect("component receipt");
        validate(&receipt).expect("receipt validates");
        assert!(super::super::model::valid_content_ref(&receipt.receipt_ref));
    }
}

#[test]
fn stale_cross_profile_and_overclaiming_receipts_fail_closed() {
    // r[verify molten.wasm_component.receipts]
    // r[verify molten.wasm_component.nonclaims]
    let fixture = super::support::Fixture::new(Consumer::Actor);
    let expected_input = input(&fixture, Stage::Inspection, Decision::Pass);
    let receipt = build(expected_input.clone()).expect("receipt");

    let mut missing_evidence = input(&fixture, Stage::Inspection, Decision::Pass);
    missing_evidence.valence_evidence_refs.clear();
    assert!(build(missing_evidence).is_err());

    let mut over_bound = input(&fixture, Stage::Inspection, Decision::Pass);
    over_bound.policy_refs = vec![super::support::fixture_ref("over-bound-policy"); OVER_BOUND_RECEIPT_REF_COUNT];
    assert!(build(over_bound).is_err());

    let missing_hostcall = input(&fixture, Stage::Hostcall, Decision::Pass);
    assert!(build(missing_hostcall).is_err());

    let mut unknown_denial = input(&fixture, Stage::Denial, Decision::Deny);
    unknown_denial.trap_class = Some("guest-controlled-class".to_string());
    unknown_denial.diagnostics = vec!["guest-controlled-class".to_string()];
    assert!(build(unknown_denial).is_err());

    let mut raw_denial = input(&fixture, Stage::Denial, Decision::Deny);
    raw_denial.diagnostics = vec!["raw runtime diagnostic".to_string()];
    assert!(build(raw_denial).is_err());

    let mut self_consistent_stale = receipt.clone();
    self_consistent_stale.input.profile_ref = super::support::fixture_ref("other-profile");
    self_consistent_stale.receipt_ref = crate::preserves_rail::canonical_hash(&value(&self_consistent_stale))
        .expect("self-consistent stale receipt hash");
    assert!(validate_against(&self_consistent_stale, &expected_input).is_err());

    let mut stale = receipt.clone();
    stale.input.profile_ref = super::support::fixture_ref("other-profile");
    assert!(validate(&stale).is_err());

    let mut wrong_parent_input = input(&fixture, Stage::Instantiation, Decision::Pass);
    wrong_parent_input.parent_refs = vec![super::support::fixture_ref("wrong-parent")];
    let wrong_parent = build(wrong_parent_input).expect("self-consistent wrong parent receipt");
    assert!(validate_chain(&[receipt.clone(), wrong_parent]).is_err());
    assert!(!replay_receipts_match(&[], &[]));

    let mut overclaim = receipt;
    overclaim.non_claims.retain(|claim| claim != "not-behavioral-correctness");
    overclaim.receipt_ref = crate::preserves_rail::canonical_hash(&value(&overclaim)).expect("tampered receipt hash");
    assert!(validate(&overclaim).is_err());
}

#[test]
fn operator_readback_and_replay_keep_evidence_distinct() {
    // r[verify molten.wasm_component.receipts]
    // r[verify molten.wasm_component.migration]
    let fixture = super::support::Fixture::new(Consumer::Actor);
    let receipt = build(input(&fixture, Stage::Migration, Decision::Pass)).expect("migration receipt");
    let summary = summary(&receipt);
    assert!(summary.contains("wasm component migration pass"));
    assert!(summary.contains("non-normative"));
    assert!(!summary.contains("molten.wasm.abi.v1"));
    assert!(replay_receipts_match(std::slice::from_ref(&receipt), std::slice::from_ref(&receipt)));
}
