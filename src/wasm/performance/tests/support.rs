use super::super::*;

pub const FIXTURE_TARGET: &str = "x86_64-unknown-linux-gnu";
pub const FIXTURE_ARCHITECTURE: &str = "x86_64";
pub const FIXTURE_CPU_FEATURE: &str = "sse2";
pub const FIXTURE_EVENT: &str = "cycles";
pub const FIXTURE_SAMPLE_COUNT: usize = 3;
pub const BASELINE_SAMPLE_COUNTS: [u64; FIXTURE_SAMPLE_COUNT] = [1_000, 1_010, 990];
pub const IMPROVED_SAMPLE_COUNTS: [u64; FIXTURE_SAMPLE_COUNT] = [800, 810, 790];
pub const MODERATE_IMPROVEMENT_SAMPLE_COUNTS: [u64; FIXTURE_SAMPLE_COUNT] = [900, 910, 890];
pub const REGRESSION_SAMPLE_COUNTS: [u64; FIXTURE_SAMPLE_COUNT] = [1_200, 1_210, 1_190];

pub fn fixture_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub fn fixture_component_bytes() -> Vec<u8> {
    crate::wasm_component::test_identity_component_bytes()
}

pub fn fixture_alternate_component_bytes() -> Vec<u8> {
    crate::wasm_component::test_alternate_component_bytes()
}

pub fn fixture_precompiled_component_bytes() -> Vec<u8> {
    crate::wasm_component::test_precompiled_component_bytes()
}

pub fn fixture_component_profile_ref() -> String {
    let profile = crate::wasm_component::supported_component_profile().expect("component profile");
    crate::wasm_component::component_profile_ref(&profile)
}

pub fn fixture_bundle(
    kind: PerformanceArtifactKind,
    artifact_bytes: &[u8],
    source_component_ref: String,
) -> PerformanceMaterializationBundle {
    let component_profile_ref = fixture_component_profile_ref();
    let artifact_ref = fixture_bytes_ref(artifact_bytes);
    let artifact_length = u64::try_from(artifact_bytes.len()).expect("bounded fixture artifact length");
    let mut bundle = PerformanceMaterializationBundle {
        schema_id: PERFORMANCE_MANTLE_BUNDLE_SCHEMA.to_string(),
        bundle_ref: String::new(),
        kind,
        consumer: crate::wasm_component::ComponentConsumer::Actor,
        source_component_ref,
        artifact_ref,
        artifact_length,
        component_profile_ref,
        runtime_configuration_ref: fixture_ref("runtime-configuration"),
        wasmtime_revision: WASMTIME_COMPONENT_COHORT.to_string(),
        target: FIXTURE_TARGET.to_string(),
        cpu_features: vec![FIXTURE_CPU_FEATURE.to_string()],
        mantle_stage_receipt_refs: vec![
            fixture_ref("mantle-precompile-receipt"),
            fixture_ref("mantle-stage"),
            fixture_ref("wizer-post-receipt"),
            fixture_ref("wizer-pre-receipt"),
        ],
        valence_sidecar_refs: vec![fixture_ref("valence-sidecar")],
        build_input_refs: vec![fixture_ref("build-input")],
        produced_by_mantle: true,
        locally_produced_transform: false,
    };
    bundle.mantle_stage_receipt_refs.sort();
    bundle.bundle_ref = performance_materialization_bundle_ref(&bundle);
    bundle
}

pub fn fixture_materialized(
    profile: &PerformanceProfile,
    kind: PerformanceArtifactKind,
    artifact_bytes: &[u8],
    source_component_ref: String,
) -> (BenchmarkSuite, PerformanceMaterializationBundle, MaterializedPerformanceArtifact) {
    let bundle = fixture_bundle(kind, artifact_bytes, source_component_ref);
    let mut suite = profile.fast.clone();
    suite.materialization_bundle_refs = vec![bundle.bundle_ref.clone()];
    let admitted = verify_performance_materialization(&suite, &bundle, artifact_bytes)
        .expect("fixture performance materialization");
    (suite, bundle, admitted)
}

pub fn fixture_host(suite: &BenchmarkSuite, materialized: &MaterializedPerformanceArtifact) -> BenchmarkHostFacts {
    BenchmarkHostFacts {
        target: materialized.target.clone(),
        host_class_ref: suite.host_class_ref.clone(),
        cpu_features: materialized.cpu_features.clone(),
        measurement: suite.measurement.clone(),
    }
}

pub fn phase_samples(counts: [u64; FIXTURE_SAMPLE_COUNT]) -> Vec<PhaseSamples> {
    PerformancePhase::ALL
        .into_iter()
        .map(|phase| PhaseSamples {
            phase,
            event: FIXTURE_EVENT.to_string(),
            samples: counts
                .iter()
                .enumerate()
                .map(|(iteration, count)| PerformanceSample {
                    process: 0,
                    iteration: u32::try_from(iteration).expect("fixture iteration index"),
                    count: *count,
                })
                .collect(),
        })
        .collect()
}

pub fn fixture_run(
    profile: &PerformanceProfile,
    suite: &BenchmarkSuite,
    materialized: &MaterializedPerformanceArtifact,
    counts: [u64; FIXTURE_SAMPLE_COUNT],
) -> BenchmarkRun {
    build_benchmark_run(BenchmarkRunInput {
        profile,
        suite,
        materialized,
        host: &fixture_host(suite, materialized),
        benchmark_ref: suite.workload_refs[0].clone(),
        recorded_effect_refs: vec![fixture_ref("recorded-host-effect")],
        phases: phase_samples(counts),
    })
    .expect("fixture benchmark run")
}

pub fn fixture_optimization() -> OptimizationProfile {
    OptimizationProfile {
        profile_id: BASELINE_OPTIMIZATION_PROFILE_ID.to_string(),
        pooling_allocator: false,
        copy_on_write_heap_images: false,
        instance_pre: false,
        compilation_strategy: CompilationStrategy::Cranelift,
        max_concurrency: 1,
        max_queue_depth: 1,
        max_pool_memories: 1,
        max_pool_tables: 1,
        deterministic_conformance_ref: fixture_ref("deterministic-conformance"),
    }
}

pub fn fixture_bytes_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}
