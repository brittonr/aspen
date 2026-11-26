//! Content identities of the reviewed performance profile and its benchmark suites.

pub fn suite_ref(suite: &super::model::BenchmarkSuite) -> alloc::string::String {
    let mut lines = alloc::vec![
        alloc::format!("lane:{}", suite.lane.as_str()),
        alloc::format!("suite-id:{}", suite.suite_id),
        alloc::format!("measurement:{}", suite.measurement),
        alloc::format!("pin:{}", suite.pin_to_single_core),
        alloc::format!("host-class-ref:{}", suite.host_class_ref),
        alloc::format!("resource-envelope-ref:{}", suite.resource_envelope_ref),
        alloc::format!("engine-cohort-ref:{}", suite.engine_cohort_ref),
        alloc::format!("engine-artifact-ref:{}", suite.engine_artifact_ref),
        alloc::format!("runner-artifact-ref:{}", suite.runner_artifact_ref),
        alloc::format!("processes:{}", suite.sampling.processes),
        alloc::format!("iterations:{}", suite.sampling.iterations_per_process),
        alloc::format!("minimum-samples:{}", suite.sampling.min_samples_per_phase),
        alloc::format!("maximum-samples:{}", suite.sampling.max_samples_per_phase),
    ];
    lines.extend(
        super::model::sorted_unique(&suite.materialization_bundle_refs)
            .into_iter()
            .map(|value| alloc::format!("bundle-ref:{value}")),
    );
    lines.extend(
        super::model::sorted_unique(&suite.workload_refs)
            .into_iter()
            .map(|value| alloc::format!("workload-ref:{value}")),
    );
    lines.extend(suite.phases.iter().map(|phase| alloc::format!("phase:{}", phase.as_str())));
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn r#ref(profile: &super::model::Profile) -> alloc::string::String {
    let mut lines = alloc::vec![
        alloc::format!("profile-id:{}", profile.profile_id),
        alloc::format!("evidence-role:{}", profile.evidence_role.as_str()),
        alloc::format!("component-profile-id:{}", profile.component_profile_id),
        alloc::format!("sightglass-revision:{}", profile.sightglass.revision),
        alloc::format!("sightglass-runner:{}", profile.sightglass.runner),
        alloc::format!("sightglass-schema:{}", profile.sightglass.raw_schema),
        alloc::format!("fast-suite-ref:{}", suite_ref(&profile.fast)),
        alloc::format!("deep-suite-ref:{}", suite_ref(&profile.deep)),
        alloc::format!("parts-per-million:{}", profile.comparison.parts_per_million),
        alloc::format!("basis-points:{}", profile.comparison.basis_points),
        alloc::format!("confidence-basis-points:{}", profile.comparison.confidence_basis_points),
        alloc::format!("practical-threshold-ppm:{}", profile.comparison.practical_threshold_ppm),
        alloc::format!("max-sample-value:{}", profile.comparison.max_sample_value),
        alloc::format!("max-sightglass-output-bytes:{}", profile.comparison.max_sightglass_output_bytes),
        alloc::format!("max-sightglass-runner-bytes:{}", profile.comparison.max_sightglass_runner_bytes),
        alloc::format!("max-sightglass-engine-bytes:{}", profile.comparison.max_sightglass_engine_bytes),
        alloc::format!("max-sightglass-benchmark-bytes:{}", profile.comparison.max_sightglass_benchmark_bytes),
        alloc::format!("max-sightglass-run-seconds:{}", profile.comparison.max_sightglass_run_seconds),
        alloc::format!("max-concurrency:{}", profile.optimization_limits.max_concurrency),
        alloc::format!("max-queue-depth:{}", profile.optimization_limits.max_queue_depth),
        alloc::format!("max-pool-memories:{}", profile.optimization_limits.max_pool_memories),
        alloc::format!("max-pool-tables:{}", profile.optimization_limits.max_pool_tables),
    ];
    lines.extend(profile.phases.iter().map(|phase| alloc::format!("phase:{}", phase.as_str())));
    lines.extend(
        profile
            .optimization_limits
            .reviewed_profile_ids
            .iter()
            .map(|profile_id| alloc::format!("optimization-profile-id:{profile_id}")),
    );
    lines.extend(profile.non_claims.iter().map(|claim| alloc::format!("non-claim:{claim}")));
    super::model::content_ref(lines.join("\n").as_bytes())
}
