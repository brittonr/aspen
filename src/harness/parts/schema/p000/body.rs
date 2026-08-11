use preserves::ValueImpl;

type CompoundClass = preserves::CompoundClass;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;
type ValueClass = preserves::ValueClass;

const WASM_ABI_MAX_OUTPUT_BYTES_FOR_VALIDATION: u64 = 8 * 1024;
const MAX_WASM_IMPORT_EVIDENCE: usize = 1024;
const MAX_HARNESS_EFFECT_LOG_ENTRIES: usize = 100_000;
const MAX_REDACTION_TRANSFORM_NODES: usize = 100_000;
const MAX_REDACTION_CONTAINER_ITEMS: usize = 100_000;
const MAX_REDACTION_MARKER_REFS: usize = 100_000;
const MAX_REDACTION_ENCRYPTED_REFS: usize = 100_000;
const RUNTIME_PREDICATE_ENGINE: &str = "trellis-bounded-local";
const TURN_COMMIT_ROLLBACK_PREDICATE: &str = "molten.trellis-runtime.turn-commit-rollback.v1";
const ASSERTION_VISIBILITY_PREDICATE: &str = "molten.trellis-runtime.assertion-visibility.v1";
const OBSERVE_DELIVERY_PREDICATE: &str = "molten.trellis-runtime.observe-delivery.v1";
const PRESERVES_PATTERN_PREDICATE: &str = "molten.trellis-runtime.preserves-pattern.v1";
const PROMISE_STATE_PREDICATE: &str = "molten.trellis-runtime.promise-state.v1";
const PROMISE_PIPELINE_PREDICATE: &str = "molten.trellis-runtime.promise-pipeline.v1";
const REVOCATION_CLEANUP_PREDICATE: &str = "molten.trellis-runtime.revocation-cleanup.v1";
const ACTORMAP_TRANSACTION_PREDICATE: &str = "molten.trellis-runtime.actormap-transaction.v1";
const NEAR_FAR_REFS_PREDICATE: &str = "molten.trellis-runtime.near-far-refs.v1";
const SNAPSHOT_AUTHORITY_PREDICATE: &str = "molten.trellis-runtime.snapshot-authority.v1";
const SERVICE_DEPENDENCIES_PREDICATE: &str = "molten.trellis-runtime.service-dependencies.v1";

const _: () = assert!(MAX_WASM_IMPORT_EVIDENCE <= 16_384);
const _: () = assert!(MAX_HARNESS_EFFECT_LOG_ENTRIES <= 1_000_000);
const _: () = assert!(MAX_REDACTION_TRANSFORM_NODES <= 1_000_000);
const _: () = assert!(MAX_REDACTION_CONTAINER_ITEMS <= 1_000_000);
const _: () = assert!(MAX_REDACTION_MARKER_REFS <= 1_000_000);
const _: () = assert!(MAX_REDACTION_ENCRYPTED_REFS <= 1_000_000);

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

#[cfg(test)]
fn parse_text(source: &str) -> Result<IoValue> {
    crate::preserves_rail::parse_text(source)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    pub name: String,
    pub seed: u64,
    pub budget: Budget,
    pub budget_explicit: bool,
    pub actors: Vec<ActorDecl>,
    pub actors_explicit: bool,
    pub capabilities: crate::runtime::CapabilityContext,
    pub capabilities_explicit: bool,
    pub policy: crate::runtime::AdmissionPolicy,
    pub steps: Vec<super::core::CoreStep>,
    pub source_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub report_ref: String,
    pub status: String,
    pub replay_status: String,
    pub profile: String,
    pub hash_algorithm: String,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub suite_value: IoValue,
    pub policy_gate: Option<PolicyGateEvidence>,
    pub capability_gate: Option<CapabilityGateEvidence>,
    pub budget_gate: Option<BudgetGateEvidence>,
    pub actors: Vec<ActorDecl>,
    pub executor_preflights: Option<ExecutorPreflightsEvidence>,
    pub observations: Vec<Observation>,
    pub effect_log: Vec<EffectLogEntry>,
    pub budget: BudgetEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub failure_ref: String,
    pub phase: String,
    pub kind: String,
    pub message: String,
    pub diagnostics: Vec<IoValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReproExportProfile {
    DenySensitive,
    RedactedDiagnostic,
    EncryptedPrivate,
}

impl ReproExportProfile {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "deny-sensitive" => Ok(Self::DenySensitive),
            "redacted-diagnostic" => Ok(Self::RedactedDiagnostic),
            "encrypted-private" => Ok(Self::EncryptedPrivate),
            _ => Err(MoltenError::invalid_harness(format!(
                "unsupported repro export profile {name}; expected deny-sensitive, redacted-diagnostic, or encrypted-private"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DenySensitive => "deny-sensitive",
            Self::RedactedDiagnostic => "redacted-diagnostic",
            Self::EncryptedPrivate => "encrypted-private",
        }
    }

    pub fn loss_classification(self) -> &'static str {
        match self {
            Self::DenySensitive => "gate-preserving",
            Self::RedactedDiagnostic => "diagnostic-only",
            Self::EncryptedPrivate => "requires-reveal",
        }
    }

    pub fn is_gate_preserving(self) -> bool {
        matches!(self, Self::DenySensitive)
    }

    pub fn requires_reveal(self) -> bool {
        matches!(self, Self::EncryptedPrivate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReproBundleKind {
    Report,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproBundle {
    pub bundle_ref: String,
    pub kind: ReproBundleKind,
    pub artifact_ref: String,
    pub report_value: Option<IoValue>,
    pub failure_value: Option<IoValue>,
    pub gate_receipt_ref: Option<String>,
    pub receipt_value: Option<IoValue>,
    pub redaction_policy_ref: Option<String>,
    pub redaction_gate_ref: Option<String>,
    pub export_profile: Option<String>,
    pub export_profile_ref: Option<String>,
    pub export_profile_value: Option<IoValue>,
    pub source_report_ref: Option<String>,
    pub source_suite_ref: Option<String>,
    pub redaction_transform_manifest_ref: Option<String>,
    pub redaction_transform_manifest_value: Option<IoValue>,
    pub redaction_transform_receipt_ref: Option<String>,
    pub redaction_transform_receipt_value: Option<IoValue>,
    pub private_bundle_profile_ref: Option<String>,
    pub private_bundle_profile_value: Option<IoValue>,
    pub loss_classification: Option<String>,
    pub encrypted_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub max_steps: u64,
    pub max_effects: u64,
    pub max_events: u64,
    pub max_report_bytes: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_steps: 64,
            max_effects: 16,
            max_events: 256,
            max_report_bytes: 65_536,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetUsage {
    pub steps: u64,
    pub effects: u64,
    pub events: u64,
    pub report_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetEvidence {
    pub limits: Budget,
    pub usage: BudgetUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorDecl {
    pub id: String,
    pub kind: ActorKind,
    pub executor: Option<ActorExecutorConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorExecutorConfig {
    Steel(SteelExecutorConfig),
    Wasm(WasmExecutorConfig),
    Adapter(AdapterExecutorConfig),
    RemoteProxy(RemoteProxyExecutorConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteelExecutorConfig {
    pub source: String,
    pub callable: String,
    pub allowed_hostcalls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExecutorConfig {
    pub module_hex: String,
    pub wit: String,
    pub allowed_hostcalls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterExecutorConfig {
    pub manifest: String,
    pub abi: String,
    pub allowed_hostcalls: Vec<String>,
    pub transcript: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProxyExecutorConfig {
    pub peer: String,
    pub endpoint: String,
    pub contract: String,
    pub allowed_hostcalls: Vec<String>,
    pub transcript: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorKind {
    Native,
    Steel,
    Wasm,
    Adapter,
    RemoteProxy,
}

impl ActorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActorKind::Native => "native",
            ActorKind::Steel => "steel",
            ActorKind::Wasm => "wasm",
            ActorKind::Adapter => "adapter",
            ActorKind::RemoteProxy => "remote-proxy",
        }
    }
}
