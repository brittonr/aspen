use super::super::*;
use super::support::fixture_component_profile_ref;
use super::support::fixture_optimization;
use super::support::fixture_ref;

fn with_passing_conformance(
    mut optimization: OptimizationProfile,
) -> (OptimizationProfile, OptimizationConformanceRecord) {
    let output_ref = fixture_ref("conformance-output");
    let mut record = OptimizationConformanceRecord {
        record_ref: String::new(),
        optimization_configuration_ref: optimization_configuration_ref(&optimization),
        component_profile_ref: fixture_component_profile_ref(),
        input_ref: fixture_ref("conformance-input"),
        baseline_output_ref: output_ref.clone(),
        optimized_output_ref: output_ref,
        baseline_execution_receipt_ref: fixture_ref("baseline-execution"),
        optimized_execution_receipt_ref: fixture_ref("optimized-execution"),
        baseline_terminal_class: "pass".to_string(),
        optimized_terminal_class: "pass".to_string(),
        recorded_effect_refs: vec![fixture_ref("conformance-effect")],
        passed: true,
    };
    record.record_ref = optimization_conformance_record_ref(&record);
    optimization.deterministic_conformance_ref = record.record_ref.clone();
    (optimization, record)
}

#[test]
fn named_optimization_profiles_bind_conformance_and_capacity() {
    // r[verify molten.wasm_performance.optimizations]
    let profile = supported_performance_profile().expect("supported performance profile");
    let baseline = fixture_optimization();
    validate_optimization_profile(&profile, &baseline).expect("baseline optimization profile");

    let mut pooling = baseline.clone();
    pooling.profile_id = POOLING_OPTIMIZATION_PROFILE_ID.to_string();
    pooling.pooling_allocator = true;
    pooling.max_concurrency = profile.optimization_limits.max_concurrency;
    pooling.max_queue_depth = profile.optimization_limits.max_queue_depth;
    let output_ref = fixture_ref("conformance-output");
    let mut conformance = OptimizationConformanceRecord {
        record_ref: String::new(),
        optimization_configuration_ref: optimization_configuration_ref(&pooling),
        component_profile_ref: fixture_component_profile_ref(),
        input_ref: fixture_ref("conformance-input"),
        baseline_output_ref: output_ref.clone(),
        optimized_output_ref: output_ref,
        baseline_execution_receipt_ref: fixture_ref("baseline-execution"),
        optimized_execution_receipt_ref: fixture_ref("optimized-execution"),
        baseline_terminal_class: "pass".to_string(),
        optimized_terminal_class: "pass".to_string(),
        recorded_effect_refs: vec![fixture_ref("conformance-effect")],
        passed: true,
    };
    conformance.record_ref = optimization_conformance_record_ref(&conformance);
    pooling.deterministic_conformance_ref = conformance.record_ref.clone();
    validate_optimization_profile(&profile, &pooling).expect("pooling optimization profile");
    validate_optimization_conformance(&pooling, &conformance).expect("pooling deterministic conformance");
    assert_ne!(optimization_profile_ref(&baseline), optimization_profile_ref(&pooling));

    let mut cow = baseline.clone();
    cow.profile_id = COW_OPTIMIZATION_PROFILE_ID.to_string();
    cow.copy_on_write_heap_images = true;
    let mut instance_pre = baseline.clone();
    instance_pre.profile_id = INSTANCE_PRE_OPTIMIZATION_PROFILE_ID.to_string();
    instance_pre.instance_pre = true;
    for optimization in [cow, instance_pre] {
        let (optimization, record) = with_passing_conformance(optimization);
        validate_optimization_profile(&profile, &optimization).expect("named optimization profile");
        validate_optimization_conformance(&optimization, &record).expect("named deterministic conformance");
        assert_ne!(optimization_profile_ref(&baseline), optimization_profile_ref(&optimization));
    }

    assert_eq!(admit_capacity(&pooling, 0, 0), CapacityDecision::Start);
    assert_eq!(admit_capacity(&pooling, pooling.max_concurrency, 0), CapacityDecision::Backpressure);
    assert_eq!(admit_capacity(&pooling, pooling.max_concurrency, pooling.max_queue_depth), CapacityDecision::Deny);
}

#[test]
fn optimization_over_capacity_missing_conformance_and_cross_named_knobs_deny() {
    // r[verify molten.wasm_performance.optimizations]
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    let baseline = fixture_optimization();

    let mut over_capacity = baseline.clone();
    over_capacity.max_concurrency = profile.optimization_limits.max_concurrency + 1;
    assert!(validate_optimization_profile(&profile, &over_capacity).is_err());

    let mut wrong_compiler_shape = baseline.clone();
    wrong_compiler_shape.compilation_strategy = CompilationStrategy::Winch;
    assert!(validate_optimization_profile(&profile, &wrong_compiler_shape).is_err());

    let mut missing_conformance = baseline.clone();
    missing_conformance.deterministic_conformance_ref = "missing".to_string();
    assert!(validate_optimization_profile(&profile, &missing_conformance).is_err());

    let mut mislabeled = baseline;
    mislabeled.profile_id = COW_OPTIMIZATION_PROFILE_ID.to_string();
    mislabeled.pooling_allocator = true;
    mislabeled.deterministic_conformance_ref = fixture_ref("other-conformance");
    assert!(validate_optimization_profile(&profile, &mislabeled).is_err());

    let mut failed_record = OptimizationConformanceRecord {
        record_ref: String::new(),
        optimization_configuration_ref: optimization_configuration_ref(&mislabeled),
        component_profile_ref: fixture_component_profile_ref(),
        input_ref: fixture_ref("conformance-input"),
        baseline_output_ref: fixture_ref("baseline-output"),
        optimized_output_ref: fixture_ref("different-output"),
        baseline_execution_receipt_ref: fixture_ref("baseline-execution"),
        optimized_execution_receipt_ref: fixture_ref("optimized-execution"),
        baseline_terminal_class: "pass".to_string(),
        optimized_terminal_class: "trap".to_string(),
        recorded_effect_refs: vec![fixture_ref("conformance-effect")],
        passed: false,
    };
    failed_record.record_ref = optimization_conformance_record_ref(&failed_record);
    mislabeled.deterministic_conformance_ref = failed_record.record_ref.clone();
    assert!(validate_optimization_conformance(&mislabeled, &failed_record).is_err());
}
