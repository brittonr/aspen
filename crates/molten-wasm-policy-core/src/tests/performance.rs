//! Recorded-only performance profile and optimization policy regressions.

use alloc::string::ToString;

fn component_profile_ref() -> alloc::string::String {
    crate::component::profile::r#ref(&super::support::component_profile())
}

fn record_for(
    optimization: &crate::performance::model::OptimizationProfile,
) -> crate::performance::optimization::ConformanceRecord {
    let output_ref = super::support::fixture_ref("conformance-output");
    let mut record = crate::performance::optimization::ConformanceRecord {
        record_ref: alloc::string::String::new(),
        optimization_configuration_ref: crate::performance::optimization::configuration_ref(optimization),
        component_profile_ref: component_profile_ref(),
        input_ref: super::support::fixture_ref("conformance-input"),
        baseline_output_ref: output_ref.clone(),
        optimized_output_ref: output_ref,
        baseline_execution_receipt_ref: super::support::fixture_ref("baseline-execution"),
        optimized_execution_receipt_ref: super::support::fixture_ref("optimized-execution"),
        baseline_terminal_class: "pass".to_string(),
        optimized_terminal_class: "pass".to_string(),
        recorded_effect_refs: alloc::vec![super::support::fixture_ref("conformance-effect")],
        passed: true,
    };
    record.record_ref = crate::performance::optimization::conformance_record_ref(&record);
    record
}

fn with_passing_conformance(
    mut optimization: crate::performance::model::OptimizationProfile,
) -> (crate::performance::model::OptimizationProfile, crate::performance::optimization::ConformanceRecord) {
    let record = record_for(&optimization);
    optimization.deterministic_conformance_ref = record.record_ref.clone();
    (optimization, record)
}

#[test]
fn fast_and_deep_profiles_pin_sightglass_and_separate_every_phase() {
    // r[verify molten.wasm_performance.suite]
    // r[verify molten.wasm_performance.phases]
    let profile = crate::performance::profile::admit_export(crate::performance::model::ProfileExport {
        schema_id: crate::performance::profile::PERFORMANCE_PROFILE_SCHEMA.into(),
        schema_version: crate::performance::profile::PERFORMANCE_PROFILE_SCHEMA_VERSION,
        source_language: crate::performance::profile::PERFORMANCE_PROFILE_SOURCE_LANGUAGE.into(),
        profile: super::support::performance_profile(),
    })
    .expect("reviewed performance profile");
    crate::performance::profile::validate(&profile).expect("performance profile validates");
    assert_eq!(profile.profile_id, crate::performance::profile::PERFORMANCE_PROFILE_ID);
    assert_eq!(profile.component_profile_id, crate::performance::profile::PERFORMANCE_COMPONENT_PROFILE_ID);
    assert_eq!(profile.sightglass.revision, crate::performance::profile::SIGHTGLASS_REVISION);
    assert_eq!(profile.phases, crate::performance::model::Phase::ALL);
    assert_eq!(profile.fast.lane, crate::performance::model::BenchmarkLane::Fast);
    assert_eq!(profile.deep.lane, crate::performance::model::BenchmarkLane::Deep);
    assert!(profile.fast.sampling.min_samples_per_phase < profile.deep.sampling.min_samples_per_phase);
    assert_ne!(
        crate::performance::profile::suite_ref(&profile.fast),
        crate::performance::profile::suite_ref(&profile.deep)
    );
    assert!(crate::performance::profile::r#ref(&profile).starts_with("blake3:"));
}

#[test]
fn stale_partial_and_overclaiming_profiles_fail_closed() {
    // r[verify molten.wasm_performance.suite]
    // r[verify molten.wasm_performance.evidence]
    let profile = super::support::performance_profile();

    let mut stale = profile.clone();
    stale.sightglass.revision = "unreviewed-main".to_string();
    assert!(crate::performance::profile::validate(&stale).is_err());

    let mut collapsed = profile.clone();
    collapsed.phases.pop();
    assert!(crate::performance::profile::validate(&collapsed).is_err());

    let mut undersampled = profile.clone();
    undersampled.fast.sampling.min_samples_per_phase = 1;
    assert!(crate::performance::profile::validate(&undersampled).is_err());

    let mut loosened = profile.clone();
    loosened.optimization_limits.max_concurrency += 1;
    assert!(crate::performance::profile::validate(&loosened).is_err());

    let mut overclaim = profile;
    overclaim.non_claims = alloc::vec!["proves-release-eligibility".to_string()];
    assert!(crate::performance::profile::validate(&overclaim).is_err());

    let stale_header = crate::performance::model::ProfileExport {
        schema_id: crate::performance::profile::PERFORMANCE_PROFILE_SCHEMA.into(),
        schema_version: crate::performance::profile::PERFORMANCE_PROFILE_SCHEMA_VERSION + 1,
        source_language: crate::performance::profile::PERFORMANCE_PROFILE_SOURCE_LANGUAGE.into(),
        profile: super::support::performance_profile(),
    };
    assert!(crate::performance::profile::admit_export(stale_header).is_err());
}

#[test]
fn named_profiles_bind_conformance_and_capacity() {
    // r[verify molten.wasm_performance.optimizations]
    let profile = super::support::performance_profile();
    let component_profile = super::support::component_profile();
    let baseline = super::support::optimization();
    crate::performance::optimization::validate_profile(&profile, &baseline).expect("baseline optimization profile");

    let mut pooling = baseline.clone();
    pooling.profile_id = crate::performance::optimization::POOLING_OPTIMIZATION_PROFILE_ID.to_string();
    pooling.pooling_allocator = true;
    pooling.max_concurrency = profile.optimization_limits.max_concurrency;
    pooling.max_queue_depth = profile.optimization_limits.max_queue_depth;
    let (pooling, conformance) = with_passing_conformance(pooling);
    crate::performance::optimization::validate_profile(&profile, &pooling).expect("pooling optimization profile");
    crate::performance::optimization::validate_conformance(&pooling, &conformance, &component_profile)
        .expect("pooling deterministic conformance");
    assert_ne!(
        crate::performance::optimization::profile_ref(&baseline),
        crate::performance::optimization::profile_ref(&pooling)
    );

    let mut cow = baseline.clone();
    cow.profile_id = crate::performance::optimization::COW_OPTIMIZATION_PROFILE_ID.to_string();
    cow.copy_on_write_heap_images = true;
    let mut instance_pre = baseline.clone();
    instance_pre.profile_id = crate::performance::optimization::INSTANCE_PRE_OPTIMIZATION_PROFILE_ID.to_string();
    instance_pre.instance_pre = true;
    for optimization in [cow, instance_pre] {
        let (optimization, record) = with_passing_conformance(optimization);
        crate::performance::optimization::validate_profile(&profile, &optimization)
            .expect("named optimization profile");
        crate::performance::optimization::validate_conformance(&optimization, &record, &component_profile)
            .expect("named deterministic conformance");
        assert_ne!(
            crate::performance::optimization::profile_ref(&baseline),
            crate::performance::optimization::profile_ref(&optimization)
        );
    }

    assert_eq!(
        crate::performance::optimization::admit_capacity(&pooling, 0, 0),
        crate::performance::model::CapacityDecision::Start
    );
    assert_eq!(
        crate::performance::optimization::admit_capacity(&pooling, pooling.max_concurrency, 0),
        crate::performance::model::CapacityDecision::Backpressure
    );
    assert_eq!(
        crate::performance::optimization::admit_capacity(&pooling, pooling.max_concurrency, pooling.max_queue_depth),
        crate::performance::model::CapacityDecision::Deny
    );
}

#[test]
fn conformance_binds_the_supplied_component_profile() {
    // r[verify molten.wasm.no_std_wasm_policy_kernel]
    let (pooling, conformance) = with_passing_conformance({
        let mut pooling = super::support::optimization();
        pooling.profile_id = crate::performance::optimization::POOLING_OPTIMIZATION_PROFILE_ID.to_string();
        pooling.pooling_allocator = true;
        pooling
    });
    crate::performance::optimization::validate_conformance(
        &pooling,
        &conformance,
        &super::support::component_profile(),
    )
    .expect("current component profile");
    let mut drifted = super::support::component_profile();
    drifted.resources.fuel += 1;
    let denial = crate::performance::optimization::validate_conformance(&pooling, &conformance, &drifted)
        .expect_err("stale component profile");
    assert_eq!(denial.blockers, alloc::vec![
        "optimization conformance targets a stale component runtime profile".to_string()
    ]);
}

#[test]
fn over_capacity_missing_conformance_and_cross_named_knobs_deny() {
    // r[verify molten.wasm_performance.optimizations]
    // r[verify molten.wasm_performance.validation]
    let profile = super::support::performance_profile();
    let baseline = super::support::optimization();

    let mut over_capacity = baseline.clone();
    over_capacity.max_concurrency = profile.optimization_limits.max_concurrency + 1;
    assert!(crate::performance::optimization::validate_profile(&profile, &over_capacity).is_err());

    let mut wrong_compiler_shape = baseline.clone();
    wrong_compiler_shape.compilation_strategy = crate::performance::model::CompilationStrategy::Winch;
    assert!(crate::performance::optimization::validate_profile(&profile, &wrong_compiler_shape).is_err());

    let mut missing_conformance = baseline.clone();
    missing_conformance.deterministic_conformance_ref = "missing".to_string();
    assert!(crate::performance::optimization::validate_profile(&profile, &missing_conformance).is_err());

    let mut mislabeled = baseline;
    mislabeled.profile_id = crate::performance::optimization::COW_OPTIMIZATION_PROFILE_ID.to_string();
    mislabeled.pooling_allocator = true;
    mislabeled.deterministic_conformance_ref = super::support::fixture_ref("other-conformance");
    assert!(crate::performance::optimization::validate_profile(&profile, &mislabeled).is_err());

    let mut failed_record = record_for(&mislabeled);
    failed_record.optimized_output_ref = super::support::fixture_ref("different-output");
    failed_record.optimized_terminal_class = "trap".to_string();
    failed_record.passed = false;
    failed_record.record_ref = crate::performance::optimization::conformance_record_ref(&failed_record);
    mislabeled.deterministic_conformance_ref = failed_record.record_ref.clone();
    assert!(
        crate::performance::optimization::validate_conformance(
            &mislabeled,
            &failed_record,
            &super::support::component_profile()
        )
        .is_err()
    );
}
