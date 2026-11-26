#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectLogEntry {
    pub sequence: u64,
    pub request: preserves::IOValue,
    pub response: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyGateEvidence {
    pub value: preserves::IOValue,
    pub policy_ref: String,
    pub nickel_source_ref: String,
    pub nickel_export_ref: String,
    pub basalt_preflight_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGateEvidence {
    pub value: preserves::IOValue,
    pub capability_ref: String,
    pub authority_preflight_ref: String,
    pub proofset_ref: String,
    pub grant_refs: Vec<String>,
    pub ucan_verification_receipt_refs: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetGateEvidence {
    pub value: preserves::IOValue,
    pub budget_ref: String,
    pub nickel_source_ref: String,
    pub nickel_export_ref: String,
    pub basalt_preflight_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPreflightsEvidence {
    pub value: preserves::IOValue,
    pub preflights: Vec<ExecutorPreflightEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPreflightEvidence {
    pub value: preserves::IOValue,
    pub actor_id: String,
    pub kind: super::model::ActorKind,
    pub artifact_ref: Option<String>,
    pub sandbox_ref: String,
    pub allowed_hostcalls: Vec<String>,
    pub conformance_refs: Vec<String>,
    pub executor_receipts: Vec<preserves::IOValue>,
    pub steel_review: Option<SteelReviewReceipt>,
    pub wasm_inspection: Option<WasmInspectionReceipt>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteelReviewReceipt {
    pub value: preserves::IOValue,
    pub source_ref: String,
    pub callable: String,
    pub allowed_hostcalls: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmInspectionReceipt {
    pub value: preserves::IOValue,
    pub module_ref: String,
    pub module_kind: String,
    pub imports: Vec<WasmImportEvidence>,
    pub wit_ref: String,
    pub allowed_hostcalls: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmImportEvidence {
    pub module: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub value: preserves::IOValue,
    pub observation_ref: String,
    pub index: u64,
    pub step_ref: String,
    pub before_state_hash: String,
    pub after_state_hash: String,
    pub event_refs: Vec<String>,
    pub events: Vec<preserves::IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDecisionEvent {
    pub value: preserves::IOValue,
    pub request: crate::harness::nucleus::AdmissionRequest,
    pub authority: Option<AdmissionAuthorityEvidence>,
    pub decision: crate::runtime::AdmissionDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionAuthorityEvidence {
    pub source: String,
    pub capability_ref: String,
    pub authorized: bool,
    pub grant_ref: Option<String>,
    pub request_ref: String,
    pub proofset_ref: String,
    pub ucan_verification_receipt_refs: Vec<String>,
    pub derived_grant_refs: Vec<String>,
    pub basalt_enforcement_receipt_ref: String,
    pub basalt_enforcement_receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventBoundary {
    EffectRequest,
    EffectResponse,
    PolicyDecision,
    ActorInput,
    HostcallRequest,
    HostcallDecision,
    ActorOutput,
    SteelExecution,
    WasmExecution,
    RuntimePredicate,
    Trace,
}

#[derive(Debug, Clone, Copy)]
pub struct HostcallEvidenceContext<'a> {
    pub sequence: u64,
    pub suite_ref: &'a str,
    pub step_ref: &'a str,
    pub policy_ref: &'a str,
    pub capability_ref: &'a str,
    pub budget_ref: &'a str,
}
