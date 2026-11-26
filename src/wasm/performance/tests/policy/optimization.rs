use super::super::*;

fn conformance(optimization: &OptimizationProfile, component_profile_ref: String) -> ConformanceRecord {
    let output_ref = super::support::fixture_ref("conformance-output");
    let mut record = ConformanceRecord {
        record_ref: String::new(),
        optimization_configuration_ref: configuration_ref(optimization),
        component_profile_ref,
        input_ref: super::support::fixture_ref("conformance-input"),
        baseline_output_ref: output_ref.clone(),
        optimized_output_ref: output_ref,
        baseline_execution_receipt_ref: super::support::fixture_ref("baseline-execution"),
        optimized_execution_receipt_ref: super::support::fixture_ref("optimized-execution"),
        baseline_terminal_class: "pass".to_string(),
        optimized_terminal_class: "pass".to_string(),
        recorded_effect_refs: vec![super::support::fixture_ref("conformance-effect")],
        passed: true,
    };
    record.record_ref = conformance_record_ref(&record);
    record
}

#[test]
fn shell_conformance_binds_the_embedded_supported_component_profile() {
    // r[verify molten.wasm_performance.optimizations]
    let profile = supported().expect("supported performance profile");
    let mut pooling = super::support::fixture_optimization();
    pooling.profile_id = POOLING_OPTIMIZATION_PROFILE_ID.to_string();
    pooling.pooling_allocator = true;
    let record = conformance(&pooling, super::support::fixture_component_profile_ref());
    pooling.deterministic_conformance_ref = record.record_ref.clone();
    validate_profile(&profile, &pooling).expect("pooling optimization profile");
    validate_conformance(&pooling, &record).expect("conformance against the embedded component profile");

    let stale = conformance(&pooling, super::support::fixture_ref("retired-component-profile"));
    pooling.deterministic_conformance_ref = stale.record_ref.clone();
    assert!(validate_conformance(&pooling, &stale).is_err());
}
