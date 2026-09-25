pub const BLAKE3_REF_PREFIX: &str = "blake3:";
pub const BLAKE3_HEX_LENGTH: usize = 64;
pub const CONTENT_REF_LENGTH: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_LENGTH;
pub const PERFORMANCE_PHASE_COUNT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceDenial {
    pub blockers: Vec<String>,
}

impl PerformanceDenial {
    pub fn new(blocker: impl Into<String>) -> Self {
        Self {
            blockers: vec![blocker.into()],
        }
    }

    pub fn from_blockers(blockers: impl IntoIterator<Item = String>) -> Self {
        let mut blockers = blockers.into_iter().collect::<Vec<_>>();
        blockers.sort();
        blockers.dedup();
        Self { blockers }
    }
}

impl std::fmt::Display for PerformanceDenial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.blockers.join("; "))
    }
}

impl std::error::Error for PerformanceDenial {}

pub type PerformanceResult<T> = std::result::Result<T, PerformanceDenial>;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize
)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkLane {
    Fast,
    Deep,
}

impl BenchmarkLane {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Deep => "deep",
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize
)]
#[serde(rename_all = "kebab-case")]
pub enum PerformancePhase {
    Compilation,
    Instantiation,
    Execution,
}

impl PerformancePhase {
    pub const ALL: [Self; PERFORMANCE_PHASE_COUNT] = [Self::Compilation, Self::Instantiation, Self::Execution];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compilation => "compilation",
            Self::Instantiation => "instantiation",
            Self::Execution => "execution",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "compilation" => Some(Self::Compilation),
            "instantiation" => Some(Self::Instantiation),
            "execution" => Some(Self::Execution),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceEvidenceRole {
    RecordedOnly,
}

impl PerformanceEvidenceRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecordedOnly => "recorded-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SightglassCohort {
    pub revision: String,
    pub runner: String,
    pub raw_schema: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SamplingProfile {
    pub processes: u32,
    pub iterations_per_process: u32,
    pub min_samples_per_phase: u32,
    pub max_samples_per_phase: u32,
}

impl SamplingProfile {
    pub fn expected_samples_per_phase(&self) -> PerformanceResult<u32> {
        self.processes
            .checked_mul(self.iterations_per_process)
            .ok_or_else(|| PerformanceDenial::new("benchmark sampling process and iteration product overflowed"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkSuite {
    pub lane: BenchmarkLane,
    pub suite_id: String,
    pub measurement: String,
    pub pin_to_single_core: bool,
    pub materialization_bundle_refs: Vec<String>,
    pub workload_refs: Vec<String>,
    pub host_class_ref: String,
    pub resource_envelope_ref: String,
    pub engine_cohort_ref: String,
    pub engine_artifact_ref: String,
    pub runner_artifact_ref: String,
    pub phases: Vec<PerformancePhase>,
    pub sampling: SamplingProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComparisonProfile {
    pub parts_per_million: u64,
    pub basis_points: u32,
    pub confidence_basis_points: u32,
    pub practical_threshold_ppm: u64,
    pub max_sample_value: u64,
    pub max_sightglass_output_bytes: u64,
    pub max_sightglass_runner_bytes: u64,
    pub max_sightglass_engine_bytes: u64,
    pub max_sightglass_benchmark_bytes: u64,
    pub max_sightglass_run_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OptimizationLimits {
    pub max_concurrency: u32,
    pub max_queue_depth: u32,
    pub max_pool_memories: u32,
    pub max_pool_tables: u32,
    pub reviewed_profile_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceProfile {
    pub profile_id: String,
    pub evidence_role: PerformanceEvidenceRole,
    pub component_profile_id: String,
    pub sightglass: SightglassCohort,
    pub phases: Vec<PerformancePhase>,
    pub fast: BenchmarkSuite,
    pub deep: BenchmarkSuite,
    pub comparison: ComparisonProfile,
    pub optimization_limits: OptimizationLimits,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceProfileExport {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_language: String,
    pub profile: PerformanceProfile,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize
)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceArtifactKind {
    PortableComponent,
    WizerComponent,
    PrecompiledComponent,
}

impl PerformanceArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PortableComponent => "portable-component",
            Self::WizerComponent => "wizer-component",
            Self::PrecompiledComponent => "precompiled-component",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MaterializationAdmissionSeal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedPerformanceArtifact {
    pub(crate) kind: PerformanceArtifactKind,
    pub(crate) consumer: crate::wasm_component::ComponentConsumer,
    pub(crate) source_component_ref: String,
    pub(crate) artifact_ref: String,
    pub(crate) artifact_length: u64,
    pub(crate) mantle_bundle_ref: String,
    pub(crate) valence_sidecar_refs: Vec<String>,
    pub(crate) build_receipt_refs: Vec<String>,
    pub(crate) build_input_refs: Vec<String>,
    pub(crate) component_profile_ref: String,
    pub(crate) runtime_configuration_ref: String,
    pub(crate) wasmtime_revision: String,
    pub(crate) target: String,
    pub(crate) cpu_features: Vec<String>,
    pub(crate) _admission_seal: MaterializationAdmissionSeal,
}
