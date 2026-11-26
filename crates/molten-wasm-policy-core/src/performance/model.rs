//! Typed Wasm performance profile and optimization policy vocabulary.

pub use crate::refs::BLAKE3_HEX_LENGTH;
pub use crate::refs::BLAKE3_REF_PREFIX;
pub use crate::refs::CONTENT_REF_LENGTH;
pub use crate::refs::content_ref;
pub use crate::refs::sorted_unique;
pub use crate::refs::valid_content_ref;
pub use crate::refs::valid_ref_collection;

pub const PERFORMANCE_PHASE_COUNT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denial {
    pub blockers: alloc::vec::Vec<alloc::string::String>,
}

impl Denial {
    pub fn new(blocker: impl Into<alloc::string::String>) -> Self {
        Self {
            blockers: alloc::vec![blocker.into()],
        }
    }

    pub fn from_blockers(blockers: impl IntoIterator<Item = alloc::string::String>) -> Self {
        let mut blockers = blockers.into_iter().collect::<alloc::vec::Vec<_>>();
        blockers.sort();
        blockers.dedup();
        Self { blockers }
    }
}

impl core::fmt::Display for Denial {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}", self.blockers.join("; "))
    }
}

impl core::error::Error for Denial {}

pub type Result<T, E = Denial> = core::result::Result<T, E>;

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
pub enum Phase {
    Compilation,
    Instantiation,
    Execution,
}

impl Phase {
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
pub enum EvidenceRole {
    RecordedOnly,
}

impl EvidenceRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecordedOnly => "recorded-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SightglassCohort {
    pub revision: alloc::string::String,
    pub runner: alloc::string::String,
    pub raw_schema: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SamplingProfile {
    pub processes: u32,
    pub iterations_per_process: u32,
    pub min_samples_per_phase: u32,
    pub max_samples_per_phase: u32,
}

impl SamplingProfile {
    pub fn expected_samples_per_phase(&self) -> Result<u32> {
        self.processes
            .checked_mul(self.iterations_per_process)
            .ok_or_else(|| Denial::new("benchmark sampling process and iteration product overflowed"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkSuite {
    pub lane: BenchmarkLane,
    pub suite_id: alloc::string::String,
    pub measurement: alloc::string::String,
    pub pin_to_single_core: bool,
    pub materialization_bundle_refs: alloc::vec::Vec<alloc::string::String>,
    pub workload_refs: alloc::vec::Vec<alloc::string::String>,
    pub host_class_ref: alloc::string::String,
    pub resource_envelope_ref: alloc::string::String,
    pub engine_cohort_ref: alloc::string::String,
    pub engine_artifact_ref: alloc::string::String,
    pub runner_artifact_ref: alloc::string::String,
    pub phases: alloc::vec::Vec<Phase>,
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
    pub reviewed_profile_ids: alloc::vec::Vec<alloc::string::String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub profile_id: alloc::string::String,
    pub evidence_role: EvidenceRole,
    pub component_profile_id: alloc::string::String,
    pub sightglass: SightglassCohort,
    pub phases: alloc::vec::Vec<Phase>,
    pub fast: BenchmarkSuite,
    pub deep: BenchmarkSuite,
    pub comparison: ComparisonProfile,
    pub optimization_limits: OptimizationLimits,
    pub non_claims: alloc::vec::Vec<alloc::string::String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProfileExport {
    pub schema_id: alloc::string::String,
    pub schema_version: u32,
    pub source_language: alloc::string::String,
    pub profile: Profile,
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
    pub profile_id: alloc::string::String,
    pub pooling_allocator: bool,
    pub copy_on_write_heap_images: bool,
    pub instance_pre: bool,
    pub compilation_strategy: CompilationStrategy,
    pub max_concurrency: u32,
    pub max_queue_depth: u32,
    pub max_pool_memories: u32,
    pub max_pool_tables: u32,
    pub deterministic_conformance_ref: alloc::string::String,
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
