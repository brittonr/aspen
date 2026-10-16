use super::super::*;

#[test]
fn export_binds_exact_toolchain_wit_and_determinism() {
    // r[verify molten.wasm_component.profile]
    // r[verify molten.wasm_component.abi]
    let profile = supported().expect("supported profile");
    validate_runtime(&profile).expect("profile validates");
    assert_eq!(profile.profile_id, COMPONENT_PROFILE_ID);
    assert_eq!(profile.wit.package, COMPONENT_WIT_PACKAGE);
    assert_eq!(profile.wit.world, COMPONENT_WIT_WORLD);
    assert_eq!(profile.wit.source_ref, COMPONENT_WIT_SOURCE_REF);
    let wit = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/wit/molten-component-runtime/runtime.wit"));
    assert_eq!(super::super::model::content_ref(wit), COMPONENT_WIT_SOURCE_REF);
    assert!(super::super::model::valid_content_ref(&r#ref(&profile)));
}

#[test]
fn rejects_stale_partial_or_authority_bearing_cohort() {
    // r[verify molten.wasm_component.profile]
    // r[verify molten.wasm_component.authority]
    let profile = supported().expect("supported profile");
    let mut stale = profile.clone();
    stale.toolchain.wasmtime = "44.0.0".to_string();
    assert!(validate_runtime(&stale).is_err());

    let mut partial = profile.clone();
    partial.determinism.fuel_interruption = false;
    assert!(validate_runtime(&partial).is_err());

    let mut unsupported = profile.clone();
    unsupported.features.tail_call = true;
    assert!(validate_runtime(&unsupported).is_err());

    let mut ambient = profile;
    ambient.allowed_imports = vec!["wasi:filesystem/types@0.2.6".to_string()];
    ambient.allowed_wasi_interfaces = ambient.allowed_imports.clone();
    assert!(validate_runtime(&ambient).is_err());
}

#[test]
fn artifact_classifier_keeps_core_and_profiles_distinct_without_fallback() {
    // r[verify molten.wasm_component.migration]
    let core = wat::parse_str("(module)").expect("core module");
    let component = super::support::identity_bytes();
    assert_eq!(classify_artifact(&core).expect("classify core"), ArtifactKind::CoreModule);
    assert_eq!(classify_artifact(&component).expect("classify component"), ArtifactKind::Component);
    assert!(classify_for_profile(RequestedExecutionProfile::ComponentV1, &core).is_err());
    assert!(classify_for_profile(RequestedExecutionProfile::LegacyCoreV1, &component).is_err());
    assert!(classify_artifact(super::support::fixture_ref("not-wasm").as_bytes()).is_err());
}
