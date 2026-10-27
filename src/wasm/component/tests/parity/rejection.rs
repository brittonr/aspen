use super::*;

#[test]
fn incomplete_stale_and_circular_evidence_denies_before_shared_compilation() {
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
            let (local, _) = compare_synchronous(&fixture);
            assert!(!local.is_pass());
            assert_eq!(local.receipts.iter().map(|record| record.input.stage).collect::<Vec<_>>(), vec![Stage::Denial]);
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
    let local = execute(&request);
    let shared = runtime(&fixture).execute(&request, DEADLINE_TICKS);
    compare(&local, &shared);
    assert!(!shared.is_pass());
    assert_eq!(shared.receipts.iter().map(|record| record.input.stage).collect::<Vec<_>>(), vec![Stage::Denial]);
    assert_eq!(shared.receipts[0].input.bundle_ref, None);
}
