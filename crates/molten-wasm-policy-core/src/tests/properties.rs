//! Bounded Hegel properties of the reviewed cohort and capacity admission.

// r[verify molten.wasm.no_std_wasm_policy_kernel]
#[hegel::test(test_cases = 32)]
fn hegel_any_resource_limit_drift_denies_the_runtime_profile(tc: hegel::TestCase) {
    let mut profile = super::support::component_profile();
    let reviewed = profile.resources.fuel;
    let fuel = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(2 * reviewed));
    let max_imports = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(256));
    profile.resources.fuel = fuel;
    profile.resources.max_imports = max_imports;
    let is_reviewed = fuel == reviewed && max_imports == crate::component::profile::COMPONENT_MAX_IMPORTS;
    assert_eq!(crate::component::profile::validate_runtime(&profile).is_ok(), is_reviewed);
}

#[hegel::test(test_cases = 64)]
fn hegel_capacity_starts_then_backpressures_then_denies(tc: hegel::TestCase) {
    let mut optimization = super::support::optimization();
    optimization.max_concurrency = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(16));
    optimization.max_queue_depth = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(64));
    let running = tc.draw(hegel::generators::integers::<u32>().min_value(0).max_value(32));
    let queued = tc.draw(hegel::generators::integers::<u32>().min_value(0).max_value(128));
    let decision = crate::performance::optimization::admit_capacity(&optimization, running, queued);
    let expected = if running < optimization.max_concurrency {
        crate::performance::model::CapacityDecision::Start
    } else if queued < optimization.max_queue_depth {
        crate::performance::model::CapacityDecision::Backpressure
    } else {
        crate::performance::model::CapacityDecision::Deny
    };
    assert_eq!(decision, expected);
}
