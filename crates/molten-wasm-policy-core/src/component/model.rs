//! Typed Wasm component runtime profile and the classified admission denial.

pub use super::verdict::Denial;
pub use super::verdict::DenialClass;
pub use super::verdict::Result;
pub use crate::refs::BLAKE3_HEX_LENGTH;
pub use crate::refs::BLAKE3_REF_PREFIX;
pub use crate::refs::CONTENT_REF_LENGTH;
pub use crate::refs::content_ref;
pub use crate::refs::sorted_unique;
pub use crate::refs::valid_content_ref;
pub use crate::refs::valid_ref_collection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceScope {
    Production,
    TestOnly,
}

impl EvidenceScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::TestOnly => "test-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    CoreModule,
    Component,
}

impl ArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CoreModule => "core-module",
            Self::Component => "component",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequestedExecutionProfile {
    LegacyCoreV1,
    ComponentV1,
    TimeSlicedComponentV1,
}

impl RequestedExecutionProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyCoreV1 => "molten.wasm.abi.v1",
            Self::ComponentV1 => "molten.wasm.component.v1",
            Self::TimeSlicedComponentV1 => "molten.wasm.component.time-sliced.v1",
        }
    }

    pub const fn required_kind(self) -> ArtifactKind {
        match self {
            Self::LegacyCoreV1 => ArtifactKind::CoreModule,
            Self::ComponentV1 | Self::TimeSlicedComponentV1 => ArtifactKind::Component,
        }
    }

    /// Whether the request asks for the time-sliced execution profile.
    pub const fn requests_time_slicing(self) -> bool {
        matches!(self, Self::TimeSlicedComponentV1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Consumer {
    Actor,
    SystemExtension,
}

impl Consumer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::SystemExtension => "system-extension",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrowthStrategy {
    Fixed,
    Dynamic,
}

impl GrowthStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Dynamic => "dynamic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainCohort {
    pub wasmtime: alloc::string::String,
    pub wasm_tools: alloc::string::String,
    pub wasmparser: alloc::string::String,
    pub wit_bindgen: alloc::string::String,
    pub wasmtime_wasi: alloc::string::String,
    pub wasi_package: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitCohort {
    pub package: alloc::string::String,
    pub world: alloc::string::String,
    pub source_ref: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureCohort {
    pub component_model: bool,
    pub multi_value: bool,
    pub bulk_memory: bool,
    pub reference_types: bool,
    pub simd: bool,
    pub relaxed_simd: bool,
    pub threads: bool,
    pub tail_call: bool,
    pub multi_memory: bool,
    pub exceptions: bool,
    pub gc: bool,
    pub memory64: bool,
    pub extended_const: bool,
    pub function_references: bool,
    pub custom_page_sizes: bool,
    pub wide_arithmetic: bool,
    pub component_async: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeterminismProfile {
    pub fuel_interruption: bool,
    pub nan_canonicalization: bool,
    pub relaxed_simd_deterministic: bool,
    pub memory_growth: alloc::string::String,
    pub table_growth: alloc::string::String,
    pub host_inputs: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub fuel: u64,
    pub max_component_bytes: u64,
    pub max_wit_bytes: u64,
    pub max_memory_bytes: u64,
    pub max_table_elements: u64,
    pub max_instances: u64,
    pub max_memories: u64,
    pub max_tables: u64,
    pub max_stack_bytes: u64,
    pub max_hostcall_bytes: u64,
    pub max_result_bytes: u64,
    pub max_concurrency: u64,
    pub max_imports: u64,
    pub max_exports: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProfile {
    pub profile_id: alloc::string::String,
    pub evidence_scope: EvidenceScope,
    pub runtime_strategy: alloc::string::String,
    pub toolchain: ToolchainCohort,
    pub wit: WitCohort,
    pub features: FeatureCohort,
    pub determinism: DeterminismProfile,
    pub resources: ResourceLimits,
    pub slicing: Option<SlicingProfile>,
    pub allowed_imports: alloc::vec::Vec<alloc::string::String>,
    pub allowed_wasi_interfaces: alloc::vec::Vec<alloc::string::String>,
    pub non_claims: alloc::vec::Vec<alloc::string::String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileExport {
    pub schema_id: alloc::string::String,
    pub schema_version: u32,
    pub source_language: alloc::string::String,
    pub profile: RuntimeProfile,
}

/// The time-sliced execution binding: the scheduling quantum and the total
/// execution budget, bound independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlicingProfile {
    pub quantum_fuel: u64,
    pub total_budget_fuel: u64,
}
