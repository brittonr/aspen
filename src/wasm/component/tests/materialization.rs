use super::super::*;
use super::support::ComponentFixture;
use super::support::fixture_ref;

const OVER_BOUND_EVIDENCE_REF_COUNT: usize = 129;

#[test]
fn complete_mantle_bundle_remeasures_actor_and_system_extension_bytes() {
    // r[verify molten.wasm_component.materialization]
    // r[verify molten.wasm_component.functional_core]
    for consumer in [ComponentConsumer::Actor, ComponentConsumer::SystemExtension] {
        let fixture = ComponentFixture::new(consumer);
        let admitted = verify_materialization(&fixture.profile, EvidenceScope::Production, fixture.source())
            .expect("materialization admitted");
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
fn materialization_rejects_tampered_stale_incomplete_and_circular_bundles() {
    // r[verify molten.wasm_component.materialization]
    // r[verify molten.wasm_component.fixtures]
    let mut tampered = ComponentFixture::new(ComponentConsumer::Actor);
    tampered.bundle.component.content_ref = fixture_ref("other-component");
    tampered.bundle.bundle_ref = mantle_bundle_ref(&tampered.bundle);
    assert!(verify_materialization(&tampered.profile, EvidenceScope::Production, tampered.source()).is_err());

    let mut tampered_wit = ComponentFixture::new(ComponentConsumer::Actor);
    tampered_wit.wit_bytes = b"tampered WIT".to_vec();
    assert!(verify_materialization(&tampered_wit.profile, EvidenceScope::Production, tampered_wit.source()).is_err());

    let mut stale = ComponentFixture::new(ComponentConsumer::Actor);
    stale.bundle.expected_profile_id = "molten.wasm.component.v0".to_string();
    stale.bundle.bundle_ref = mantle_bundle_ref(&stale.bundle);
    assert!(verify_materialization(&stale.profile, EvidenceScope::Production, stale.source()).is_err());

    let mut incomplete = ComponentFixture::new(ComponentConsumer::Actor);
    incomplete.envelope.valence_sidecar_refs.clear();
    assert!(verify_materialization(&incomplete.profile, EvidenceScope::Production, incomplete.source()).is_err());

    let mut over_bound = ComponentFixture::new(ComponentConsumer::Actor);
    over_bound.bundle.stage_receipt_refs = vec![fixture_ref("over-bound-stage"); OVER_BOUND_EVIDENCE_REF_COUNT];
    over_bound.bundle.bundle_ref = mantle_bundle_ref(&over_bound.bundle);
    over_bound.envelope.bundle_ref = over_bound.bundle.bundle_ref.clone();
    assert!(verify_materialization(&over_bound.profile, EvidenceScope::Production, over_bound.source()).is_err());

    let mut circular = ComponentFixture::new(ComponentConsumer::Actor);
    circular.bundle.embedded_admission_refs = vec![fixture_ref("circular-admission")];
    circular.bundle.bundle_ref = mantle_bundle_ref(&circular.bundle);
    circular.envelope.bundle_ref = circular.bundle.bundle_ref.clone();
    assert!(verify_materialization(&circular.profile, EvidenceScope::Production, circular.source()).is_err());
}

#[test]
fn loose_component_bytes_are_test_only_and_never_production_evidence() {
    // r[verify molten.wasm_component.materialization]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    let loose = ComponentArtifactSource::TestOnlyLoose {
        component_bytes: &fixture.component_bytes,
        wit_bytes: &fixture.wit_bytes,
    };
    assert!(verify_materialization(&fixture.profile, EvidenceScope::Production, loose).is_err());
    let admitted = verify_materialization(&fixture.profile, EvidenceScope::TestOnly, loose)
        .expect("test-only loose fixture admitted");
    assert_eq!(admitted.evidence_scope, EvidenceScope::TestOnly);
    assert!(admitted.bundle_ref.is_none());
    assert!(admitted.mantle_evidence_refs.is_empty());
    assert!(admitted.valence_evidence_refs.is_empty());
    assert!(admitted.cairn_evidence_refs.is_empty());
    assert!(admitted.policy_refs.is_empty());
    assert!(admitted.authority_refs.is_empty());
    assert!(admitted.resource_refs.is_empty());
}
