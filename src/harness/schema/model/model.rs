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
    pub steps: Vec<crate::harness::nucleus::CoreStep>,
    pub source_value: preserves::IOValue,
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
    pub suite_value: preserves::IOValue,
    pub policy_gate: Option<super::observed::PolicyGateEvidence>,
    pub capability_gate: Option<super::observed::CapabilityGateEvidence>,
    pub budget_gate: Option<super::observed::BudgetGateEvidence>,
    pub actors: Vec<ActorDecl>,
    pub executor_preflights: Option<super::observed::ExecutorPreflightsEvidence>,
    pub observations: Vec<super::observed::Observation>,
    pub effect_log: Vec<super::observed::EffectLogEntry>,
    pub budget: BudgetEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub failure_ref: String,
    pub phase: String,
    pub kind: String,
    pub message: String,
    pub diagnostics: Vec<preserves::IOValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReproExportProfile {
    DenySensitive,
    RedactedDiagnostic,
    EncryptedPrivate,
}

impl ReproExportProfile {
    pub fn parse(name: &str) -> crate::error::Result<Self> {
        match name {
            "deny-sensitive" => Ok(Self::DenySensitive),
            "redacted-diagnostic" => Ok(Self::RedactedDiagnostic),
            "encrypted-private" => Ok(Self::EncryptedPrivate),
            _ => Err(crate::error::Failure::invalid_harness(format!(
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
    pub report_value: Option<preserves::IOValue>,
    pub failure_value: Option<preserves::IOValue>,
    pub gate_receipt_ref: Option<String>,
    pub receipt_value: Option<preserves::IOValue>,
    pub redaction_policy_ref: Option<String>,
    pub redaction_gate_ref: Option<String>,
    pub export_profile: Option<String>,
    pub export_profile_ref: Option<String>,
    pub export_profile_value: Option<preserves::IOValue>,
    pub source_report_ref: Option<String>,
    pub source_suite_ref: Option<String>,
    pub redaction_transform_manifest_ref: Option<String>,
    pub redaction_transform_manifest_value: Option<preserves::IOValue>,
    pub redaction_transform_receipt_ref: Option<String>,
    pub redaction_transform_receipt_value: Option<preserves::IOValue>,
    pub private_bundle_profile_ref: Option<String>,
    pub private_bundle_profile_value: Option<preserves::IOValue>,
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
