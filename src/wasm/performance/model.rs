//! Performance evidence model. Profile and optimization policy types are the pure
//! `molten_wasm_policy_core::performance::model` vocabulary; benchmark runs, comparisons, and the
//! sealed materialized artifact stay in the shell that produces them.

pub use molten_wasm_policy_core::performance::model::BenchmarkLane;
pub use molten_wasm_policy_core::performance::model::BenchmarkSuite;
pub use molten_wasm_policy_core::performance::model::CapacityDecision;
pub use molten_wasm_policy_core::performance::model::CompilationStrategy;
pub use molten_wasm_policy_core::performance::model::Denial;
pub use molten_wasm_policy_core::performance::model::EvidenceRole;
pub use molten_wasm_policy_core::performance::model::OptimizationProfile;
pub use molten_wasm_policy_core::performance::model::PERFORMANCE_PHASE_COUNT;
pub use molten_wasm_policy_core::performance::model::Phase;
pub use molten_wasm_policy_core::performance::model::Profile;
pub use molten_wasm_policy_core::performance::model::ProfileExport;
pub use molten_wasm_policy_core::performance::model::Result;
pub use molten_wasm_policy_core::performance::model::content_ref;
pub use molten_wasm_policy_core::performance::model::sorted_unique;
pub use molten_wasm_policy_core::performance::model::valid_content_ref;
pub use molten_wasm_policy_core::performance::model::valid_ref_collection;

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
pub enum ArtifactKind {
    PortableComponent,
    WizerComponent,
    PrecompiledComponent,
}

impl ArtifactKind {
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
pub struct MaterializedArtifact {
    pub(crate) kind: ArtifactKind,
    pub(crate) consumer: crate::wasm_component::Consumer,
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

impl MaterializedArtifact {
    pub const fn kind(&self) -> ArtifactKind {
        self.kind
    }

    pub const fn consumer(&self) -> crate::wasm_component::Consumer {
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
pub struct Sample {
    pub process: u32,
    pub iteration: u32,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseSamples {
    pub phase: Phase,
    pub event: String,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkRun {
    pub suite_ref: String,
    pub run_ref: String,
    pub benchmark_ref: String,
    pub consumer: crate::wasm_component::Consumer,
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
    pub phase: Phase,
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
