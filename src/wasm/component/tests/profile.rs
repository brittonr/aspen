use super::super::*;

// r[verify molten.wasm_component_adoption.patched_engine]
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
fn embedded_config_exports_decode_into_admitted_cohorts() {
    // r[verify molten.wasm.no_std_wasm_policy_kernel]
    let time_sliced = time_sliced().expect("embedded time-sliced profile");
    assert_eq!(time_sliced.profile_id, TIME_SLICED_PROFILE_ID);
    assert!(time_sliced.slicing.is_some());
    let hosted = hosted().expect("embedded effect profile");
    assert_eq!(hosted.profile_id, HOST_PROFILE_ID);
    assert_eq!(hosted.allowed_imports, vec![HOST_REQUEST_IMPORT.to_string()]);
    let malformed = super::super::profile::decode("{\"schema_id\":", "component profile export is invalid")
        .expect_err("malformed export");
    assert_eq!(malformed.canonical_class(), DenialClass::ProfileDenial.as_str());
    let stale = super::super::profile::decode(
        &include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/wasm-component-runtime/generated/profile.json"))
            .replace("49.0.5", "44.0.0"),
        "component profile export is invalid",
    )
    .expect_err("stale embedded toolchain");
    assert!(stale.blockers.iter().any(|blocker| blocker.contains("wasmtime")));
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
