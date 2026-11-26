use super::super::*;

#[test]
fn embedded_config_export_decodes_into_the_reviewed_suites() {
    // r[verify molten.wasm_performance.suite]
    // r[verify molten.wasm.no_std_wasm_policy_kernel]
    let profile = supported().expect("supported performance profile");
    validate(&profile).expect("performance profile validates");
    assert_eq!(profile.profile_id, PERFORMANCE_PROFILE_ID);
    assert_eq!(profile.component_profile_id, PERFORMANCE_COMPONENT_PROFILE_ID);
    assert_eq!(profile.sightglass.revision, SIGHTGLASS_REVISION);
    assert_eq!(profile.phases, Phase::ALL);
    assert!(r#ref(&profile).starts_with("blake3:"));
}
