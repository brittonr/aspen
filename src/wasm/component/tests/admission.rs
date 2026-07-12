use super::super::*;
use super::support::ComponentFixture;
use super::support::fixture_ref;

fn admitted_materialization(fixture: &ComponentFixture) -> MaterializationAdmission {
    verify_materialization(&fixture.profile, EvidenceScope::Production, fixture.source())
        .expect("materialization admission")
}

#[test]
fn identical_component_facts_produce_identical_pure_execution_plans() {
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.functional_core]
    // r[verify molten.wasm_component.resources]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    let left = plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &fixture.facts, &[])
        .expect("left plan");
    let right = plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &fixture.facts, &[])
        .expect("right plan");
    assert_eq!(left, right);
    assert!(left.imports.is_empty());
    assert!(left.capabilities.is_empty());
    assert!(!left.mantle_evidence_refs.is_empty());
    assert!(!left.valence_evidence_refs.is_empty());
    assert!(!left.cairn_evidence_refs.is_empty());
    assert!(!left.policy_refs.is_empty());
    assert!(!left.authority_refs.is_empty());
    assert!(!left.resource_refs.is_empty());
}

#[test]
fn admission_rejects_wrong_world_unsupported_feature_and_dynamic_growth() {
    // r[verify molten.wasm_component.abi]
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.fixtures]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);

    let mut wrong_world = fixture.facts.clone();
    wrong_world.declared_world = "other-world".to_string();
    assert!(plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &wrong_world, &[]).is_err());

    let mut unsupported = fixture.facts.clone();
    unsupported.enabled_features.push("threads".to_string());
    unsupported.enabled_features.sort();
    assert!(plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &unsupported, &[]).is_err());

    let mut dynamic = fixture.facts.clone();
    dynamic.memory.strategy = GrowthStrategy::Dynamic;
    dynamic.memory.maximum = None;
    assert!(plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &dynamic, &[]).is_err());
}

#[test]
fn admission_rejects_over_resource_undeclared_wasi_and_unused_authority() {
    // r[verify molten.wasm_component.authority]
    // r[verify molten.wasm_component.resources]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);

    let mut oversized = fixture.facts.clone();
    oversized.memory.initial = fixture.profile.resources.max_memory_bytes + 1;
    oversized.memory.maximum = Some(oversized.memory.initial);
    assert!(plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &oversized, &[]).is_err());

    let mut wasi = fixture.facts.clone();
    wasi.imports = vec!["wasi:filesystem/types@0.2.6".to_string()];
    let grant = ComponentImportGrant {
        import: wasi.imports[0].clone(),
        capability: "filesystem-read".to_string(),
        policy_ref: fixture_ref("import-policy"),
        authority_ref: fixture_ref("import-authority"),
        resource_ref: fixture_ref("import-resource"),
        recorded_effect_ref: fixture_ref("import-effect"),
    };
    assert!(
        plan_component_execution(
            &fixture.profile,
            admitted_materialization(&fixture),
            &wasi,
            std::slice::from_ref(&grant),
        )
        .is_err()
    );

    assert!(
        plan_component_execution(&fixture.profile, admitted_materialization(&fixture), &fixture.facts, &[grant])
            .is_err()
    );
}
