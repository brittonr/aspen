use serde::Deserialize;
use serde::Serialize;

pub const BLAKE3_REF_PREFIX: &str = "blake3:";
pub const BLAKE3_HEX_LENGTH: usize = 64;
pub const CONTENT_REF_LENGTH: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_LENGTH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentDenialClass {
    FuelExhausted,
    InvalidPreservesPayload,
    ComponentCompilationDenied,
    ComponentInstantiationDenied,
    GuestDenial,
    ComponentTrap,
    ResourceDenial,
    AuthorityDenial,
    ProfileDenial,
    MaterializationDenial,
    ComponentAdmissionDenial,
}

impl ComponentDenialClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FuelExhausted => "fuel-exhausted",
            Self::InvalidPreservesPayload => "invalid-preserves-payload",
            Self::ComponentCompilationDenied => "component-compilation-denied",
            Self::ComponentInstantiationDenied => "component-instantiation-denied",
            Self::GuestDenial => "guest-denial",
            Self::ComponentTrap => "component-trap",
            Self::ResourceDenial => "resource-denial",
            Self::AuthorityDenial => "authority-denial",
            Self::ProfileDenial => "profile-denial",
            Self::MaterializationDenial => "materialization-denial",
            Self::ComponentAdmissionDenial => "component-admission-denial",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fuel-exhausted" => Some(Self::FuelExhausted),
            "invalid-preserves-payload" => Some(Self::InvalidPreservesPayload),
            "component-compilation-denied" => Some(Self::ComponentCompilationDenied),
            "component-instantiation-denied" => Some(Self::ComponentInstantiationDenied),
            "guest-denial" => Some(Self::GuestDenial),
            "component-trap" => Some(Self::ComponentTrap),
            "resource-denial" => Some(Self::ResourceDenial),
            "authority-denial" => Some(Self::AuthorityDenial),
            "profile-denial" => Some(Self::ProfileDenial),
            "materialization-denial" => Some(Self::MaterializationDenial),
            "component-admission-denial" => Some(Self::ComponentAdmissionDenial),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentDenial {
    pub blockers: Vec<String>,
    class: ComponentDenialClass,
}

impl ComponentDenial {
    pub fn new(blocker: impl Into<String>) -> Self {
        Self::classified(ComponentDenialClass::ComponentAdmissionDenial, blocker)
    }

    pub fn classified(class: ComponentDenialClass, blocker: impl Into<String>) -> Self {
        Self {
            blockers: vec![blocker.into()],
            class,
        }
    }

    pub fn from_blockers(blockers: impl IntoIterator<Item = String>) -> Self {
        Self::from_classified_blockers(ComponentDenialClass::ComponentAdmissionDenial, blockers)
    }

    pub fn from_classified_blockers(class: ComponentDenialClass, blockers: impl IntoIterator<Item = String>) -> Self {
        let mut blockers = blockers.into_iter().collect::<Vec<_>>();
        blockers.sort();
        blockers.dedup();
        Self { blockers, class }
    }

    pub const fn canonical_class(&self) -> &'static str {
        self.class.as_str()
    }
}

impl std::fmt::Display for ComponentDenial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.blockers.join("; "))
    }
}

impl std::error::Error for ComponentDenial {}

pub type ComponentResult<T> = std::result::Result<T, ComponentDenial>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WasmArtifactKind {
    CoreModule,
    Component,
}

impl WasmArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CoreModule => "core-module",
            Self::Component => "component",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequestedExecutionProfile {
    LegacyCoreV1,
    ComponentV1,
}

impl RequestedExecutionProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyCoreV1 => "molten.wasm.abi.v1",
            Self::ComponentV1 => "molten.wasm.component.v1",
        }
    }

    pub const fn required_kind(self) -> WasmArtifactKind {
        match self {
            Self::LegacyCoreV1 => WasmArtifactKind::CoreModule,
            Self::ComponentV1 => WasmArtifactKind::Component,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentConsumer {
    Actor,
    SystemExtension,
}

impl ComponentConsumer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::SystemExtension => "system-extension",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentToolchainCohort {
    pub wasmtime: String,
    pub wasm_tools: String,
    pub wasmparser: String,
    pub wit_bindgen: String,
    pub wasmtime_wasi: String,
    pub wasi_package: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentWitCohort {
    pub package: String,
    pub world: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentFeatureCohort {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeterminismProfile {
    pub fuel_interruption: bool,
    pub nan_canonicalization: bool,
    pub relaxed_simd_deterministic: bool,
    pub memory_growth: String,
    pub table_growth: String,
    pub host_inputs: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentResourceLimits {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentRuntimeProfile {
    pub profile_id: String,
    pub evidence_scope: EvidenceScope,
    pub runtime_strategy: String,
    pub toolchain: ComponentToolchainCohort,
    pub wit: ComponentWitCohort,
    pub features: ComponentFeatureCohort,
    pub determinism: ComponentDeterminismProfile,
    pub resources: ComponentResourceLimits,
    pub allowed_imports: Vec<String>,
    pub allowed_wasi_interfaces: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentProfileExport {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_language: String,
    pub profile: ComponentRuntimeProfile,
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

pub(crate) fn valid_ref_collection(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| valid_content_ref(value)) && sorted_unique(values) == values
}

pub(crate) fn sorted_unique(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}
