use super::super::*;

const OVER_BOUND_EVIDENCE_REF_COUNT: usize = 129;

#[test]
fn complete_mantle_bundle_remeasures_actor_and_system_extension_bytes() {
    // r[verify molten.wasm_component.materialization]
    // r[verify molten.wasm_component.functional_core]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let fixture = super::support::Fixture::new(consumer);
        let admitted =
            verify(&fixture.profile, EvidenceScope::Production, fixture.source()).expect("materialization admitted");
        assert_eq!(admitted.consumer, consumer);
        assert_eq!(admitted.component_ref, fixture.bundle.component.content_ref);
        assert_eq!(admitted.wit_ref, fixture.profile.wit.source_ref);
        assert_eq!(admitted.bundle_ref.as_deref(), Some(fixture.bundle.bundle_ref.as_str()));
        assert_eq!(admitted.evidence_scope, EvidenceScope::Production);
        assert!(!admitted.mantle_evidence_refs.is_empty());
        assert!(!admitted.valence_evidence_refs.is_empty());
        assert!(!admitted.cairn_evidence_refs.is_empty());
    }
}

#[test]
fn rejects_tampered_stale_incomplete_and_circular_bundles() {
    // r[verify molten.wasm_component.materialization]
    // r[verify molten.wasm_component.fixtures]
    let mut tampered = super::support::Fixture::new(Consumer::Actor);
    tampered.bundle.component.content_ref = super::support::fixture_ref("other-component");
    tampered.bundle.bundle_ref = mantle_bundle_ref(&tampered.bundle);
    assert!(verify(&tampered.profile, EvidenceScope::Production, tampered.source()).is_err());

    let mut tampered_wit = super::support::Fixture::new(Consumer::Actor);
    tampered_wit.wit_bytes = b"tampered WIT".to_vec();
    assert!(verify(&tampered_wit.profile, EvidenceScope::Production, tampered_wit.source()).is_err());

    let mut stale = super::support::Fixture::new(Consumer::Actor);
    stale.bundle.expected_profile_id = "molten.wasm.component.v0".to_string();
    stale.bundle.bundle_ref = mantle_bundle_ref(&stale.bundle);
    assert!(verify(&stale.profile, EvidenceScope::Production, stale.source()).is_err());

    let mut incomplete = super::support::Fixture::new(Consumer::Actor);
    incomplete.envelope.valence_sidecar_refs.clear();
    assert!(verify(&incomplete.profile, EvidenceScope::Production, incomplete.source()).is_err());

    let mut over_bound = super::support::Fixture::new(Consumer::Actor);
    over_bound.bundle.stage_receipt_refs =
        vec![super::support::fixture_ref("over-bound-stage"); OVER_BOUND_EVIDENCE_REF_COUNT];
    over_bound.bundle.bundle_ref = mantle_bundle_ref(&over_bound.bundle);
    over_bound.envelope.bundle_ref = over_bound.bundle.bundle_ref.clone();
    assert!(verify(&over_bound.profile, EvidenceScope::Production, over_bound.source()).is_err());

    let mut circular = super::support::Fixture::new(Consumer::Actor);
    circular.bundle.embedded_admission_refs = vec![super::support::fixture_ref("circular-admission")];
    circular.bundle.bundle_ref = mantle_bundle_ref(&circular.bundle);
    circular.envelope.bundle_ref = circular.bundle.bundle_ref.clone();
    assert!(verify(&circular.profile, EvidenceScope::Production, circular.source()).is_err());
}

#[test]
fn loose_bytes_are_test_only_and_never_production_evidence() {
    // r[verify molten.wasm_component.materialization]
    let fixture = super::support::Fixture::new(Consumer::Actor);
    let loose = ArtifactSource::TestOnlyLoose {
        component_bytes: &fixture.component_bytes,
        wit_bytes: &fixture.wit_bytes,
    };
    assert!(verify(&fixture.profile, EvidenceScope::Production, loose).is_err());
    let admitted = verify(&fixture.profile, EvidenceScope::TestOnly, loose).expect("test-only loose fixture admitted");
    assert_eq!(admitted.evidence_scope, EvidenceScope::TestOnly);
    assert!(admitted.bundle_ref.is_none());
    assert!(admitted.mantle_evidence_refs.is_empty());
    assert!(admitted.valence_evidence_refs.is_empty());
    assert!(admitted.cairn_evidence_refs.is_empty());
    assert!(admitted.policy_refs.is_empty());
    assert!(admitted.authority_refs.is_empty());
    assert!(admitted.resource_refs.is_empty());
}
