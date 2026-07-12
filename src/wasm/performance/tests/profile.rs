use super::super::*;

#[test]
fn fast_and_deep_profiles_pin_sightglass_and_separate_every_phase() {
    // r[verify molten.wasm_performance.suite]
    // r[verify molten.wasm_performance.phases]
    let profile = supported_performance_profile().expect("supported performance profile");
    validate_performance_profile(&profile).expect("performance profile validates");
    assert_eq!(profile.profile_id, PERFORMANCE_PROFILE_ID);
    assert_eq!(profile.component_profile_id, PERFORMANCE_COMPONENT_PROFILE_ID);
    assert_eq!(profile.sightglass.revision, SIGHTGLASS_REVISION);
    assert_eq!(profile.phases, PerformancePhase::ALL);
    assert_eq!(profile.fast.lane, BenchmarkLane::Fast);
    assert_eq!(profile.deep.lane, BenchmarkLane::Deep);
    assert!(profile.fast.sampling.min_samples_per_phase < profile.deep.sampling.min_samples_per_phase);
    assert_ne!(performance_suite_ref(&profile.fast), performance_suite_ref(&profile.deep));
    assert!(performance_profile_ref(&profile).starts_with("blake3:"));
}

#[test]
fn stale_partial_and_overclaiming_profiles_fail_closed() {
    // r[verify molten.wasm_performance.suite]
    // r[verify molten.wasm_performance.evidence]
    let profile = supported_performance_profile().expect("supported performance profile");

    let mut stale = profile.clone();
    stale.sightglass.revision = "unreviewed-main".to_string();
    assert!(validate_performance_profile(&stale).is_err());

    let mut collapsed = profile.clone();
    collapsed.phases.pop();
    assert!(validate_performance_profile(&collapsed).is_err());

    let mut undersampled = profile.clone();
    undersampled.fast.sampling.min_samples_per_phase = 1;
    assert!(validate_performance_profile(&undersampled).is_err());

    let mut overclaim = profile;
    overclaim.non_claims = vec!["proves-release-eligibility".to_string()];
    assert!(validate_performance_profile(&overclaim).is_err());
}
