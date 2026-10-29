use super::*;

#[test]
fn incomplete_stale_and_circular_evidence_denies_before_compilation() {
    // r[verify molten.wasm_component_adoption.parity]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let mut incomplete = super::super::support::Fixture::new(consumer);
        incomplete.envelope.valence_sidecar_refs.clear();
        let mut authority = super::super::support::Fixture::new(consumer);
        authority.envelope.authority_refs.clear();
        let mut stale = super::super::support::Fixture::new(consumer);
        stale.bundle.expected_profile_id = "molten.wasm.component.v0".to_string();
        stale.bundle.bundle_ref = mantle_bundle_ref(&stale.bundle);
        stale.envelope.bundle_ref = stale.bundle.bundle_ref.clone();
        let mut circular = super::super::support::Fixture::new(consumer);
        circular.bundle.embedded_admission_refs = vec![super::super::support::fixture_ref("circular-admission")];
        circular.bundle.bundle_ref = mantle_bundle_ref(&circular.bundle);
        circular.envelope.bundle_ref = circular.bundle.bundle_ref.clone();
        for fixture in [incomplete, authority, stale, circular] {
            let outcome = run(&fixture);
            assert!(!outcome.is_pass());
            assert_eq!(stages(&outcome), vec![Stage::Denial]);
        }
    }
}

#[test]
fn production_loose_bytes_never_gain_a_pass_or_materialization_receipt() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.nonclaims]
    let fixture = super::super::support::Fixture::new(Consumer::Actor);
    let input = super::super::support::input_value();
    let mut request = fixture.request(&input);
    request.source = ArtifactSource::TestOnlyLoose {
        component_bytes: &fixture.component_bytes,
        wit_bytes: &fixture.wit_bytes,
    };
    let outcome = super::super::support::execute(&request);
    assert!(!outcome.is_pass());
    assert_eq!(stages(&outcome), vec![Stage::Denial]);
    assert_eq!(outcome.receipts[0].input.bundle_ref, None);
}

#[test]
fn hidden_ambient_import_denies_even_when_supplied_facts_claim_none() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.hostcalls]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let mut fixture = super::super::support::Fixture::new(consumer);
        fixture.replace_component_bytes(fixtures::undeclared_import());
        super::super::super::admission::inspection::verify_artifact_facts(&fixture.component_bytes, &fixture.facts)
            .expect("resource facts match actual imported component");
        let outcome = run(&fixture);
        assert!(!outcome.is_pass());
        assert_eq!(outcome.output, None);
        assert_eq!(stages(&outcome), vec![Stage::Denial]);
        assert_eq!(outcome.receipts[0].input.trap_class.as_deref(), Some("component-admission-denial"));
    }
}

#[test]
fn successful_receipts_cannot_drop_correctness_nonclaims() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.nonclaims]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let fixture = super::super::support::Fixture::new(consumer);
        let outcome = run(&fixture);
        assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
        let mut records = outcome.receipts.clone();
        let execution = records.last_mut().expect("actual execution receipt");
        execution.non_claims.retain(|claim| claim != "not-behavioral-correctness");
        execution.receipt_ref =
            crate::preserves_rail::canonical_hash(&value(execution)).expect("self-consistent overclaim identity");
        assert!(validate(execution).is_err());
        assert!(validate_chain(&records).is_err());
        assert!(!replay_receipts_match(&outcome.receipts, &records));
    }
}
