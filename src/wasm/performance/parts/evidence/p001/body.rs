
fn benchmark_run_value(run: &super::model::BenchmarkRun) -> preserves::IOValue {
    let phases = run
        .phases
        .iter()
        .map(|phase| {
            let samples = phase
                .samples
                .iter()
                .map(|sample| {
                    crate::preserves_rail::record("sample", vec![
                        crate::preserves_rail::u64_value(u64::from(sample.process)),
                        crate::preserves_rail::u64_value(u64::from(sample.iteration)),
                        crate::preserves_rail::u64_value(sample.count),
                    ])
                })
                .collect();
            crate::preserves_rail::record("phase", vec![
                crate::preserves_rail::string(phase.phase.as_str()),
                crate::preserves_rail::string(&phase.event),
                crate::preserves_rail::sequence(samples),
            ])
        })
        .collect();
    crate::preserves_rail::record("benchmark-run-v1", vec![
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&run.suite_ref)]),
        crate::preserves_rail::record("run-ref", vec![crate::preserves_rail::string(&run.run_ref)]),
        crate::preserves_rail::record("benchmark-ref", vec![crate::preserves_rail::string(&run.benchmark_ref)]),
        crate::preserves_rail::record("consumer", vec![crate::preserves_rail::string(run.consumer.as_str())]),
        crate::preserves_rail::record("source-component-ref", vec![crate::preserves_rail::string(
            &run.source_component_ref,
        )]),
        crate::preserves_rail::record("component-ref", vec![crate::preserves_rail::string(&run.component_ref)]),
        crate::preserves_rail::record("component-profile-ref", vec![crate::preserves_rail::string(
            &run.component_profile_ref,
        )]),
        crate::preserves_rail::record("performance-profile-ref", vec![crate::preserves_rail::string(
            &run.performance_profile_ref,
        )]),
        crate::preserves_rail::record("engine-cohort-ref", vec![crate::preserves_rail::string(&run.engine_cohort_ref)]),
        crate::preserves_rail::record("engine-artifact-ref", vec![crate::preserves_rail::string(
            &run.engine_artifact_ref,
        )]),
        crate::preserves_rail::record("runner-artifact-ref", vec![crate::preserves_rail::string(
            &run.runner_artifact_ref,
        )]),
        crate::preserves_rail::record("runtime-configuration-ref", vec![crate::preserves_rail::string(
            &run.runtime_configuration_ref,
        )]),
        crate::preserves_rail::record("target", vec![crate::preserves_rail::string(&run.target)]),
        crate::preserves_rail::record("host-class-ref", vec![crate::preserves_rail::string(&run.host_class_ref)]),
        crate::preserves_rail::record("measurement", vec![crate::preserves_rail::string(&run.measurement)]),
        crate::preserves_rail::record("resource-envelope-ref", vec![crate::preserves_rail::string(
            &run.resource_envelope_ref,
        )]),
        crate::preserves_rail::record("recorded-effect-refs", vec![strings(&run.recorded_effect_refs)]),
        crate::preserves_rail::record("phases", vec![crate::preserves_rail::sequence(phases)]),
    ])
}

fn optional_run(run: Option<&super::model::BenchmarkRun>) -> preserves::IOValue {
    run.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |run| crate::preserves_rail::record("some", vec![benchmark_run_value(run)]),
    )
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn optional_ref(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}
