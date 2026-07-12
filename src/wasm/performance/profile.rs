use super::model::BenchmarkLane;
use super::model::BenchmarkSuite;
use super::model::PerformanceDenial;
use super::model::PerformanceEvidenceRole;
use super::model::PerformancePhase;
use super::model::PerformanceProfile;
use super::model::PerformanceProfileExport;
use super::model::PerformanceResult;
use super::model::content_ref;
use super::model::sorted_unique;
use super::model::valid_content_ref;

pub const PERFORMANCE_PROFILE_SCHEMA: &str = "molten.wasm-component-performance-profile.v1";
pub const PERFORMANCE_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const PERFORMANCE_PROFILE_SOURCE_LANGUAGE: &str = "nickel";
pub const PERFORMANCE_PROFILE_ID: &str = "molten.wasm.performance.v1";
pub const PERFORMANCE_COMPONENT_PROFILE_ID: &str = "molten.wasm.component.v1";
pub const SIGHTGLASS_REVISION: &str = "c18bbe75803a6a610f7ff3b15549c927c6e02667";
pub const SIGHTGLASS_RUNNER: &str = "sightglass-cli";
pub const SIGHTGLASS_RAW_SCHEMA: &str = "sightglass-data.measurement.v1";
pub const SIGHTGLASS_MEASUREMENT: &str = "cycles";
pub const WASMTIME_COMPONENT_COHORT: &str = "45.0.0";
pub const FAST_SUITE_ID: &str = "molten-wasm-component-fast-v1";
pub const DEEP_SUITE_ID: &str = "molten-wasm-component-deep-v1";
pub const PARTS_PER_MILLION: u64 = 1_000_000;
pub const BASIS_POINTS: u32 = 10_000;
pub const CONFIDENCE_BASIS_POINTS: u32 = 9_500;
pub const PRACTICAL_THRESHOLD_PPM: u64 = 10_000;
pub const MAX_SAMPLE_VALUE: u64 = 1_000_000_000;
pub const MAX_SIGHTGLASS_OUTPUT_BYTES: u64 = 4_194_304;
pub const MAX_SIGHTGLASS_RUNNER_BYTES: u64 = 33_554_432;
pub const MAX_SIGHTGLASS_ENGINE_BYTES: u64 = 67_108_864;
pub const MAX_SIGHTGLASS_BENCHMARK_BYTES: u64 = 16_777_216;
pub const MAX_SIGHTGLASS_RUN_SECONDS: u64 = 3_600;
pub const FAST_PROCESSES: u32 = 1;
pub const FAST_ITERATIONS: u32 = 3;
pub const FAST_MAX_SAMPLES: u32 = 3;
pub const DEEP_PROCESSES: u32 = 10;
pub const DEEP_ITERATIONS: u32 = 10;
pub const DEEP_MAX_SAMPLES: u32 = 128;
pub const MAX_OPTIMIZATION_CONCURRENCY: u32 = 16;
pub const MAX_OPTIMIZATION_QUEUE_DEPTH: u32 = 64;
pub const MAX_POOL_MEMORIES: u32 = 32;
pub const MAX_POOL_TABLES: u32 = 32;
pub const REVIEWED_OPTIMIZATION_PROFILE_IDS: &[&str] = &[
    "molten.wasm.optimization.baseline.v1",
    "molten.wasm.optimization.pooling.v1",
    "molten.wasm.optimization.cow.v1",
    "molten.wasm.optimization.instance-pre.v1",
];

pub const FAST_BUNDLE_REFS: &[&str] = &["blake3:8984e68c835ce8bd0b3af729092b98cc2c47d3a7a8f2eada28f2d23970e26884"];
pub const DEEP_BUNDLE_REFS: &[&str] = &[
    "blake3:6ea7c323251905bff8a51935567a0ce75fad233ac498530f202307c35ea9b925",
    "blake3:7393db08a46ff6733394426fa42c38c3a4be2ca11eaf8746631d0cae2fdc8a5b",
];
pub const ACTOR_WORKLOAD_REF: &str = "blake3:31e5f8b8a4579f62dde10521790a1a36905cb1900480fabb0e190f61b3f95a3b";
pub const SYSTEM_EXTENSION_WORKLOAD_REF: &str =
    "blake3:c41e25ce415096156dde58eb0057e60a3b84a6d4f1912506accc4eab1da39e26";
pub const HOST_CLASS_REF: &str = "blake3:09959b9b17ddf92cef356c835fc8bfb6020f642d7f5018b9b758f0f8e9bfa62c";
pub const RESOURCE_ENVELOPE_REF: &str = "blake3:291658cf3a3c5653c1f109b66453cef93e3f1b9e8a9bcb20d55014d2ad93ff1b";
pub const ENGINE_COHORT_REF: &str = "blake3:5295b3f4ef5ec7f4ebabff7a2e8aaa8ef585a4dda6c40226b67988c5474c09d1";
pub const ENGINE_ARTIFACT_REF: &str = "blake3:5295b3f4ef5ec7f4ebabff7a2e8aaa8ef585a4dda6c40226b67988c5474c09d1";
pub const RUNNER_ARTIFACT_REF: &str = "blake3:b06044cd1cc2067897ae836504de53e63e56b4171ec09180e08f9c4208927462";

pub const PERFORMANCE_NON_CLAIMS: &[&str] = &[
    "not-behavioral-correctness",
    "not-determinism-beyond-conformance",
    "not-authority",
    "not-security",
    "not-cross-machine-performance",
    "not-cross-runtime-ranking",
    "not-release-eligibility",
    "not-semantic-equivalence",
];

const PROFILE_EXPORT_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/wasm-component-performance/generated/profile.json"));

pub fn supported_performance_profile() -> PerformanceResult<PerformanceProfile> {
    let exported: PerformanceProfileExport = serde_json::from_str(PROFILE_EXPORT_JSON).map_err(|error| {
        PerformanceDenial::new(format!("Wasm component performance profile export is invalid: {error}"))
    })?;
    validate_profile_export(&exported)?;
    Ok(exported.profile)
}

pub fn validate_performance_profile(profile: &PerformanceProfile) -> PerformanceResult<()> {
    let mut blockers = Vec::new();
    require_equal(&mut blockers, "profile id", &profile.profile_id, PERFORMANCE_PROFILE_ID);
    if profile.evidence_role != PerformanceEvidenceRole::RecordedOnly {
        blockers.push("performance profile evidence role must remain recorded-only".to_string());
    }
    require_equal(
        &mut blockers,
        "component profile id",
        &profile.component_profile_id,
        PERFORMANCE_COMPONENT_PROFILE_ID,
    );
    require_equal(&mut blockers, "Sightglass revision", &profile.sightglass.revision, SIGHTGLASS_REVISION);
    require_equal(&mut blockers, "Sightglass runner", &profile.sightglass.runner, SIGHTGLASS_RUNNER);
    require_equal(&mut blockers, "Sightglass raw schema", &profile.sightglass.raw_schema, SIGHTGLASS_RAW_SCHEMA);
    if profile.phases != PerformancePhase::ALL {
        blockers.push(
            "performance profile must retain separate compilation, instantiation, and execution phases".to_string(),
        );
    }
    validate_suite(
        &profile.fast,
        SuiteExpectation {
            lane: BenchmarkLane::Fast,
            suite_id: FAST_SUITE_ID,
            bundle_refs: FAST_BUNDLE_REFS,
            workload_refs: &[ACTOR_WORKLOAD_REF],
            processes: FAST_PROCESSES,
            iterations: FAST_ITERATIONS,
            max_samples: FAST_MAX_SAMPLES,
        },
        &mut blockers,
    );
    validate_suite(
        &profile.deep,
        SuiteExpectation {
            lane: BenchmarkLane::Deep,
            suite_id: DEEP_SUITE_ID,
            bundle_refs: DEEP_BUNDLE_REFS,
            workload_refs: &[ACTOR_WORKLOAD_REF, SYSTEM_EXTENSION_WORKLOAD_REF],
            processes: DEEP_PROCESSES,
            iterations: DEEP_ITERATIONS,
            max_samples: DEEP_MAX_SAMPLES,
        },
        &mut blockers,
    );
    let comparison = &profile.comparison;
    if comparison.parts_per_million != PARTS_PER_MILLION
        || comparison.basis_points != BASIS_POINTS
        || comparison.confidence_basis_points != CONFIDENCE_BASIS_POINTS
        || comparison.practical_threshold_ppm != PRACTICAL_THRESHOLD_PPM
        || comparison.max_sample_value != MAX_SAMPLE_VALUE
        || comparison.max_sightglass_output_bytes != MAX_SIGHTGLASS_OUTPUT_BYTES
        || comparison.max_sightglass_runner_bytes != MAX_SIGHTGLASS_RUNNER_BYTES
        || comparison.max_sightglass_engine_bytes != MAX_SIGHTGLASS_ENGINE_BYTES
        || comparison.max_sightglass_benchmark_bytes != MAX_SIGHTGLASS_BENCHMARK_BYTES
        || comparison.max_sightglass_run_seconds != MAX_SIGHTGLASS_RUN_SECONDS
    {
        blockers.push("performance comparison constants differ from the reviewed deterministic profile".to_string());
    }
    let optimization = &profile.optimization_limits;
    if optimization.max_concurrency != MAX_OPTIMIZATION_CONCURRENCY
        || optimization.max_queue_depth != MAX_OPTIMIZATION_QUEUE_DEPTH
        || optimization.max_pool_memories != MAX_POOL_MEMORIES
        || optimization.max_pool_tables != MAX_POOL_TABLES
        || optimization.reviewed_profile_ids
            != REVIEWED_OPTIMIZATION_PROFILE_IDS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>()
    {
        blockers.push("performance optimization limits differ from the reviewed hard caps".to_string());
    }
    let expected_non_claims = PERFORMANCE_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    if profile.non_claims != expected_non_claims {
        blockers.push("performance profile changes the required recorded-only non-claims".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

pub fn performance_suite_ref(suite: &BenchmarkSuite) -> String {
    let mut lines = vec![
        format!("lane:{}", suite.lane.as_str()),
        format!("suite-id:{}", suite.suite_id),
        format!("measurement:{}", suite.measurement),
        format!("pin:{}", suite.pin_to_single_core),
        format!("host-class-ref:{}", suite.host_class_ref),
        format!("resource-envelope-ref:{}", suite.resource_envelope_ref),
        format!("engine-cohort-ref:{}", suite.engine_cohort_ref),
        format!("engine-artifact-ref:{}", suite.engine_artifact_ref),
        format!("runner-artifact-ref:{}", suite.runner_artifact_ref),
        format!("processes:{}", suite.sampling.processes),
        format!("iterations:{}", suite.sampling.iterations_per_process),
        format!("minimum-samples:{}", suite.sampling.min_samples_per_phase),
        format!("maximum-samples:{}", suite.sampling.max_samples_per_phase),
    ];
    lines.extend(
        sorted_unique(&suite.materialization_bundle_refs)
            .into_iter()
            .map(|value| format!("bundle-ref:{value}")),
    );
    lines.extend(sorted_unique(&suite.workload_refs).into_iter().map(|value| format!("workload-ref:{value}")));
    lines.extend(suite.phases.iter().map(|phase| format!("phase:{}", phase.as_str())));
    content_ref(lines.join("\n").as_bytes())
}

pub fn performance_profile_ref(profile: &PerformanceProfile) -> String {
    let mut lines = vec![
        format!("profile-id:{}", profile.profile_id),
        format!("evidence-role:{}", profile.evidence_role.as_str()),
        format!("component-profile-id:{}", profile.component_profile_id),
        format!("sightglass-revision:{}", profile.sightglass.revision),
        format!("sightglass-runner:{}", profile.sightglass.runner),
        format!("sightglass-schema:{}", profile.sightglass.raw_schema),
        format!("fast-suite-ref:{}", performance_suite_ref(&profile.fast)),
        format!("deep-suite-ref:{}", performance_suite_ref(&profile.deep)),
        format!("parts-per-million:{}", profile.comparison.parts_per_million),
        format!("basis-points:{}", profile.comparison.basis_points),
        format!("confidence-basis-points:{}", profile.comparison.confidence_basis_points),
        format!("practical-threshold-ppm:{}", profile.comparison.practical_threshold_ppm),
        format!("max-sample-value:{}", profile.comparison.max_sample_value),
        format!("max-sightglass-output-bytes:{}", profile.comparison.max_sightglass_output_bytes),
        format!("max-sightglass-runner-bytes:{}", profile.comparison.max_sightglass_runner_bytes),
        format!("max-sightglass-engine-bytes:{}", profile.comparison.max_sightglass_engine_bytes),
        format!("max-sightglass-benchmark-bytes:{}", profile.comparison.max_sightglass_benchmark_bytes),
        format!("max-sightglass-run-seconds:{}", profile.comparison.max_sightglass_run_seconds),
        format!("max-concurrency:{}", profile.optimization_limits.max_concurrency),
        format!("max-queue-depth:{}", profile.optimization_limits.max_queue_depth),
        format!("max-pool-memories:{}", profile.optimization_limits.max_pool_memories),
        format!("max-pool-tables:{}", profile.optimization_limits.max_pool_tables),
    ];
    lines.extend(profile.phases.iter().map(|phase| format!("phase:{}", phase.as_str())));
    lines.extend(
        profile
            .optimization_limits
            .reviewed_profile_ids
            .iter()
            .map(|profile_id| format!("optimization-profile-id:{profile_id}")),
    );
    lines.extend(profile.non_claims.iter().map(|claim| format!("non-claim:{claim}")));
    content_ref(lines.join("\n").as_bytes())
}

fn validate_profile_export(exported: &PerformanceProfileExport) -> PerformanceResult<()> {
    let mut blockers = Vec::new();
    require_equal(&mut blockers, "schema id", &exported.schema_id, PERFORMANCE_PROFILE_SCHEMA);
    if exported.schema_version != PERFORMANCE_PROFILE_SCHEMA_VERSION {
        blockers.push("performance profile schema version is unsupported".to_string());
    }
    require_equal(&mut blockers, "source language", &exported.source_language, PERFORMANCE_PROFILE_SOURCE_LANGUAGE);
    if !blockers.is_empty() {
        return Err(PerformanceDenial::from_blockers(blockers));
    }
    validate_performance_profile(&exported.profile)
}

struct SuiteExpectation<'a> {
    lane: BenchmarkLane,
    suite_id: &'a str,
    bundle_refs: &'a [&'a str],
    workload_refs: &'a [&'a str],
    processes: u32,
    iterations: u32,
    max_samples: u32,
}

fn validate_suite(suite: &BenchmarkSuite, expected: SuiteExpectation<'_>, blockers: &mut Vec<String>) {
    if suite.lane != expected.lane || suite.suite_id != expected.suite_id {
        blockers.push(format!("{} performance suite identity is stale", expected.lane.as_str()));
    }
    if suite.measurement != SIGHTGLASS_MEASUREMENT || !suite.pin_to_single_core {
        blockers.push(format!("{} performance suite measurement posture is unsupported", expected.lane.as_str()));
    }
    let expected_bundles = expected.bundle_refs.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    let expected_workloads = expected.workload_refs.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    if suite.materialization_bundle_refs != expected_bundles
        || suite.workload_refs != expected_workloads
        || suite.materialization_bundle_refs.iter().any(|value| !valid_content_ref(value))
        || suite.workload_refs.iter().any(|value| !valid_content_ref(value))
    {
        blockers
            .push(format!("{} performance suite fixture identities are stale or malformed", expected.lane.as_str()));
    }
    if suite.host_class_ref != HOST_CLASS_REF
        || suite.resource_envelope_ref != RESOURCE_ENVELOPE_REF
        || suite.engine_cohort_ref != ENGINE_COHORT_REF
        || suite.engine_artifact_ref != ENGINE_ARTIFACT_REF
        || suite.runner_artifact_ref != RUNNER_ARTIFACT_REF
    {
        blockers.push(format!("{} performance suite environment identity is stale", expected.lane.as_str()));
    }
    if suite.phases != PerformancePhase::ALL {
        blockers.push(format!("{} performance suite collapses or changes required phases", expected.lane.as_str()));
    }
    let expected_samples = suite.sampling.expected_samples_per_phase();
    if suite.sampling.processes != expected.processes
        || suite.sampling.iterations_per_process != expected.iterations
        || expected_samples.as_ref().ok() != Some(&suite.sampling.min_samples_per_phase)
        || suite.sampling.max_samples_per_phase != expected.max_samples
        || suite.sampling.max_samples_per_phase < suite.sampling.min_samples_per_phase
    {
        blockers.push(format!("{} performance suite sampling plan is unsupported", expected.lane.as_str()));
    }
}

fn require_equal(blockers: &mut Vec<String>, label: &str, actual: &str, expected: &str) {
    if actual != expected {
        blockers.push(format!("performance profile {label} must equal {expected}"));
    }
}
