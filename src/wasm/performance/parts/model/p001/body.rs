
impl MaterializedPerformanceArtifact {
    pub const fn kind(&self) -> PerformanceArtifactKind {
        self.kind
    }

    pub const fn consumer(&self) -> crate::wasm_component::ComponentConsumer {
        self.consumer
    }

    pub fn source_component_ref(&self) -> &str {
        &self.source_component_ref
    }

    pub fn artifact_ref(&self) -> &str {
        &self.artifact_ref
    }

    pub const fn artifact_length(&self) -> u64 {
        self.artifact_length
    }

    pub fn mantle_bundle_ref(&self) -> &str {
        &self.mantle_bundle_ref
    }

    pub fn valence_sidecar_refs(&self) -> &[String] {
        &self.valence_sidecar_refs
    }

    pub fn build_receipt_refs(&self) -> &[String] {
        &self.build_receipt_refs
    }

    pub fn build_input_refs(&self) -> &[String] {
        &self.build_input_refs
    }

    pub fn component_profile_ref(&self) -> &str {
        &self.component_profile_ref
    }

    pub fn runtime_configuration_ref(&self) -> &str {
        &self.runtime_configuration_ref
    }

    pub fn wasmtime_revision(&self) -> &str {
        &self.wasmtime_revision
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn cpu_features(&self) -> &[String] {
        &self.cpu_features
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkHostFacts {
    pub target: String,
    pub host_class_ref: String,
    pub cpu_features: Vec<String>,
    pub measurement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PerformanceSample {
    pub process: u32,
    pub iteration: u32,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseSamples {
    pub phase: PerformancePhase,
    pub event: String,
    pub samples: Vec<PerformanceSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkRun {
    pub suite_ref: String,
    pub run_ref: String,
    pub benchmark_ref: String,
    pub consumer: crate::wasm_component::ComponentConsumer,
    pub source_component_ref: String,
    pub component_ref: String,
    pub component_profile_ref: String,
    pub performance_profile_ref: String,
    pub engine_cohort_ref: String,
    pub engine_artifact_ref: String,
    pub runner_artifact_ref: String,
    pub runtime_configuration_ref: String,
    pub target: String,
    pub host_class_ref: String,
    pub measurement: String,
    pub resource_envelope_ref: String,
    pub recorded_effect_refs: Vec<String>,
    pub phases: Vec<PhaseSamples>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionClass {
    Improvement,
    NoSignificantChange,
    Regression,
}

impl RegressionClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Improvement => "improvement",
            Self::NoSignificantChange => "no-significant-change",
            Self::Regression => "regression",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseComparison {
    pub phase: PerformancePhase,
    pub event: String,
    pub baseline_mean_scaled: u128,
    pub candidate_mean_scaled: u128,
    pub baseline_confidence_half_width_scaled: u128,
    pub candidate_confidence_half_width_scaled: u128,
    pub candidate_ratio_ppm: u64,
    pub ratio_confidence_half_width_ppm: u64,
    pub class: RegressionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkComparison {
    pub baseline_run_ref: String,
    pub candidate_run_ref: String,
    pub suite_ref: String,
    pub phases: Vec<PhaseComparison>,
    pub comparison_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonDecision {
    Comparable(BenchmarkComparison),
    Incompatible { blockers: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationStrategy {
    Cranelift,
    Winch,
}

impl CompilationStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cranelift => "cranelift",
            Self::Winch => "winch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationProfile {
    pub profile_id: String,
    pub pooling_allocator: bool,
    pub copy_on_write_heap_images: bool,
    pub instance_pre: bool,
    pub compilation_strategy: CompilationStrategy,
    pub max_concurrency: u32,
    pub max_queue_depth: u32,
    pub max_pool_memories: u32,
    pub max_pool_tables: u32,
    pub deterministic_conformance_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityDecision {
    Start,
    Backpressure,
    Deny,
}

impl CapacityDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Backpressure => "backpressure",
            Self::Deny => "deny",
        }
    }
}

pub(crate) fn content_ref(bytes: &[u8]) -> String {
    format!("{BLAKE3_REF_PREFIX}{}", blake3::hash(bytes).to_hex())
}

pub(crate) fn valid_content_ref(value: &str) -> bool {
    value.len() == CONTENT_REF_LENGTH
        && value.starts_with(BLAKE3_REF_PREFIX)
        && value[BLAKE3_REF_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn sorted_unique(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

pub(crate) fn valid_ref_collection(values: &[String]) -> bool {
    !values.is_empty() && sorted_unique(values) == values && values.iter().all(|value| valid_content_ref(value))
}
