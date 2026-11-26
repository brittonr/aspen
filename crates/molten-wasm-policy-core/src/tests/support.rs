//! Reviewed profiles assembled from the core's own constants, plus matching artifact facts.
//!
//! The shell separately proves that the embedded `config/` exports decode to exactly these
//! reviewed profiles; the core tests never read workspace files.

use alloc::string::ToString;

pub(super) fn fixture_ref(label: &str) -> alloc::string::String {
    crate::refs::content_ref(label.as_bytes())
}

fn strings(values: &[&str]) -> alloc::vec::Vec<alloc::string::String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(super) fn component_profile() -> crate::component::model::RuntimeProfile {
    crate::component::model::RuntimeProfile {
        profile_id: crate::component::profile::COMPONENT_PROFILE_ID.into(),
        evidence_scope: crate::component::model::EvidenceScope::Production,
        runtime_strategy: crate::component::profile::COMPONENT_RUNTIME_STRATEGY.into(),
        toolchain: crate::component::model::ToolchainCohort {
            wasmtime: crate::component::profile::COMPONENT_WASMTIME_VERSION.into(),
            wasm_tools: crate::component::profile::COMPONENT_WASM_TOOLS_VERSION.into(),
            wasmparser: crate::component::profile::COMPONENT_WASMPARSER_VERSION.into(),
            wit_bindgen: crate::component::profile::COMPONENT_WIT_BINDGEN_VERSION.into(),
            wasmtime_wasi: crate::component::profile::COMPONENT_WASMTIME_WASI_VERSION.into(),
            wasi_package: crate::component::profile::COMPONENT_WASI_PACKAGE_VERSION.into(),
        },
        wit: crate::component::model::WitCohort {
            package: crate::component::profile::COMPONENT_WIT_PACKAGE.into(),
            world: crate::component::profile::COMPONENT_WIT_WORLD.into(),
            source_ref: crate::component::profile::COMPONENT_WIT_SOURCE_REF.into(),
        },
        features: crate::component::model::FeatureCohort {
            component_model: true,
            multi_value: true,
            bulk_memory: true,
            reference_types: true,
            simd: true,
            relaxed_simd: false,
            threads: false,
            tail_call: false,
            multi_memory: false,
            exceptions: false,
            gc: false,
            memory64: false,
            extended_const: false,
            function_references: false,
            custom_page_sizes: false,
            wide_arithmetic: false,
            component_async: false,
        },
        determinism: crate::component::model::DeterminismProfile {
            fuel_interruption: true,
            nan_canonicalization: true,
            relaxed_simd_deterministic: true,
            memory_growth: crate::component::profile::COMPONENT_FIXED_GROWTH.into(),
            table_growth: crate::component::profile::COMPONENT_FIXED_GROWTH.into(),
            host_inputs: crate::component::profile::COMPONENT_RECORDED_HOST_INPUTS.into(),
        },
        resources: crate::component::model::ResourceLimits {
            fuel: crate::component::profile::COMPONENT_FUEL_LIMIT,
            max_component_bytes: crate::component::profile::COMPONENT_MAX_BYTES,
            max_wit_bytes: crate::component::profile::COMPONENT_MAX_WIT_BYTES,
            max_memory_bytes: crate::component::profile::COMPONENT_MAX_MEMORY_BYTES,
            max_table_elements: crate::component::profile::COMPONENT_MAX_TABLE_ELEMENTS,
            max_instances: crate::component::profile::COMPONENT_MAX_INSTANCES,
            max_memories: crate::component::profile::COMPONENT_MAX_MEMORIES,
            max_tables: crate::component::profile::COMPONENT_MAX_TABLES,
            max_stack_bytes: crate::component::profile::COMPONENT_MAX_STACK_BYTES,
            max_hostcall_bytes: crate::component::profile::COMPONENT_MAX_HOSTCALL_BYTES,
            max_result_bytes: crate::component::profile::COMPONENT_MAX_RESULT_BYTES,
            max_concurrency: crate::component::profile::COMPONENT_MAX_CONCURRENCY,
            max_imports: crate::component::profile::COMPONENT_MAX_IMPORTS,
            max_exports: crate::component::profile::COMPONENT_MAX_EXPORTS,
        },
        slicing: None,
        allowed_imports: alloc::vec::Vec::new(),
        allowed_wasi_interfaces: alloc::vec::Vec::new(),
        non_claims: strings(crate::component::profile::COMPONENT_NON_CLAIMS),
    }
}

/// The closed effect-request cohort: one recorded `effect-request` import, no WASI.
pub(super) fn effect_profile() -> crate::component::model::RuntimeProfile {
    let mut profile = component_profile();
    profile.profile_id = crate::component::profile::HOST_PROFILE_ID.into();
    profile.wit = crate::component::model::WitCohort {
        package: crate::component::profile::HOST_WIT_PACKAGE.into(),
        world: crate::component::profile::HOST_WIT_WORLD.into(),
        source_ref: crate::component::profile::HOST_WIT_SOURCE_REF.into(),
    };
    profile.allowed_imports = alloc::vec![crate::component::profile::HOST_REQUEST_IMPORT.into()];
    profile
}

/// Facts a fixed-growth, single-memory component of `profile` presents.
pub(super) fn facts(profile: &crate::component::model::RuntimeProfile) -> crate::component::admission::ArtifactFacts {
    crate::component::admission::ArtifactFacts {
        artifact_kind: crate::component::model::ArtifactKind::Component,
        declared_profile_id: profile.profile_id.clone(),
        declared_cohort_ref: crate::component::profile::r#ref(profile),
        declared_world: profile.wit.world.clone(),
        imports: profile.allowed_imports.clone(),
        exports: alloc::vec![crate::component::admission::COMPONENT_INVOKE_EXPORT.into()],
        enabled_features: strings(&[
            "bulk-memory",
            "component-model",
            "multi-value",
            "reference-types",
            "simd",
        ]),
        memory: crate::component::admission::GrowthFacts {
            initial: 65_536,
            maximum: Some(65_536),
            strategy: crate::component::model::GrowthStrategy::Fixed,
        },
        table: crate::component::admission::GrowthFacts {
            initial: 0,
            maximum: Some(0),
            strategy: crate::component::model::GrowthStrategy::Fixed,
        },
        instances: 1,
        memories: 1,
        tables: 0,
    }
}

pub(super) fn materialization(
    profile: &crate::component::model::RuntimeProfile,
) -> crate::component::materialization::Admission {
    crate::component::materialization::Admission {
        evidence_scope: crate::component::model::EvidenceScope::Production,
        component_ref: fixture_ref("component"),
        wit_ref: fixture_ref("wit"),
        bundle_ref: Some(fixture_ref("bundle")),
        consumer: crate::component::model::Consumer::Actor,
        profile_ref: crate::component::profile::r#ref(profile),
        mantle_evidence_refs: alloc::vec![fixture_ref("mantle")],
        valence_evidence_refs: alloc::vec![fixture_ref("valence")],
        cairn_evidence_refs: alloc::vec![fixture_ref("cairn")],
        policy_refs: alloc::vec![fixture_ref("materialization-policy")],
        authority_refs: alloc::vec![fixture_ref("materialization-authority")],
        resource_refs: alloc::vec![fixture_ref("materialization-resource")],
    }
}

fn suite(lane: crate::performance::model::BenchmarkLane) -> crate::performance::model::BenchmarkSuite {
    let (suite_id, bundles, workloads, processes, iterations, max_samples) = match lane {
        crate::performance::model::BenchmarkLane::Fast => (
            crate::performance::profile::FAST_SUITE_ID,
            crate::performance::profile::FAST_BUNDLE_REFS,
            &[crate::performance::profile::ACTOR_WORKLOAD_REF][..],
            crate::performance::profile::FAST_PROCESSES,
            crate::performance::profile::FAST_ITERATIONS,
            crate::performance::profile::FAST_MAX_SAMPLES,
        ),
        crate::performance::model::BenchmarkLane::Deep => (
            crate::performance::profile::DEEP_SUITE_ID,
            crate::performance::profile::DEEP_BUNDLE_REFS,
            &[
                crate::performance::profile::ACTOR_WORKLOAD_REF,
                crate::performance::profile::SYSTEM_EXTENSION_WORKLOAD_REF,
            ][..],
            crate::performance::profile::DEEP_PROCESSES,
            crate::performance::profile::DEEP_ITERATIONS,
            crate::performance::profile::DEEP_MAX_SAMPLES,
        ),
    };
    crate::performance::model::BenchmarkSuite {
        lane,
        suite_id: suite_id.into(),
        measurement: crate::performance::profile::SIGHTGLASS_MEASUREMENT.into(),
        pin_to_single_core: true,
        materialization_bundle_refs: strings(bundles),
        workload_refs: strings(workloads),
        host_class_ref: crate::performance::profile::HOST_CLASS_REF.into(),
        resource_envelope_ref: crate::performance::profile::RESOURCE_ENVELOPE_REF.into(),
        engine_cohort_ref: crate::performance::profile::ENGINE_COHORT_REF.into(),
        engine_artifact_ref: crate::performance::profile::ENGINE_ARTIFACT_REF.into(),
        runner_artifact_ref: crate::performance::profile::RUNNER_ARTIFACT_REF.into(),
        phases: crate::performance::model::Phase::ALL.to_vec(),
        sampling: crate::performance::model::SamplingProfile {
            processes,
            iterations_per_process: iterations,
            min_samples_per_phase: processes * iterations,
            max_samples_per_phase: max_samples,
        },
    }
}

pub(super) fn performance_profile() -> crate::performance::model::Profile {
    crate::performance::model::Profile {
        profile_id: crate::performance::profile::PERFORMANCE_PROFILE_ID.into(),
        evidence_role: crate::performance::model::EvidenceRole::RecordedOnly,
        component_profile_id: crate::performance::profile::PERFORMANCE_COMPONENT_PROFILE_ID.into(),
        sightglass: crate::performance::model::SightglassCohort {
            revision: crate::performance::profile::SIGHTGLASS_REVISION.into(),
            runner: crate::performance::profile::SIGHTGLASS_RUNNER.into(),
            raw_schema: crate::performance::profile::SIGHTGLASS_RAW_SCHEMA.into(),
        },
        phases: crate::performance::model::Phase::ALL.to_vec(),
        fast: suite(crate::performance::model::BenchmarkLane::Fast),
        deep: suite(crate::performance::model::BenchmarkLane::Deep),
        comparison: crate::performance::model::ComparisonProfile {
            parts_per_million: crate::performance::profile::PARTS_PER_MILLION,
            basis_points: crate::performance::profile::BASIS_POINTS,
            confidence_basis_points: crate::performance::profile::CONFIDENCE_BASIS_POINTS,
            practical_threshold_ppm: crate::performance::profile::PRACTICAL_THRESHOLD_PPM,
            max_sample_value: crate::performance::profile::MAX_SAMPLE_VALUE,
            max_sightglass_output_bytes: crate::performance::profile::MAX_SIGHTGLASS_OUTPUT_BYTES,
            max_sightglass_runner_bytes: crate::performance::profile::MAX_SIGHTGLASS_RUNNER_BYTES,
            max_sightglass_engine_bytes: crate::performance::profile::MAX_SIGHTGLASS_ENGINE_BYTES,
            max_sightglass_benchmark_bytes: crate::performance::profile::MAX_SIGHTGLASS_BENCHMARK_BYTES,
            max_sightglass_run_seconds: crate::performance::profile::MAX_SIGHTGLASS_RUN_SECONDS,
        },
        optimization_limits: crate::performance::model::OptimizationLimits {
            max_concurrency: crate::performance::profile::MAX_OPTIMIZATION_CONCURRENCY,
            max_queue_depth: crate::performance::profile::MAX_OPTIMIZATION_QUEUE_DEPTH,
            max_pool_memories: crate::performance::profile::MAX_POOL_MEMORIES,
            max_pool_tables: crate::performance::profile::MAX_POOL_TABLES,
            reviewed_profile_ids: strings(crate::performance::profile::REVIEWED_OPTIMIZATION_PROFILE_IDS),
        },
        non_claims: strings(crate::performance::profile::PERFORMANCE_NON_CLAIMS),
    }
}

pub(super) fn optimization() -> crate::performance::model::OptimizationProfile {
    crate::performance::model::OptimizationProfile {
        profile_id: crate::performance::optimization::BASELINE_OPTIMIZATION_PROFILE_ID.into(),
        pooling_allocator: false,
        copy_on_write_heap_images: false,
        instance_pre: false,
        compilation_strategy: crate::performance::model::CompilationStrategy::Cranelift,
        max_concurrency: 1,
        max_queue_depth: 1,
        max_pool_memories: 1,
        max_pool_tables: 1,
        deterministic_conformance_ref: fixture_ref("deterministic-conformance"),
    }
}
