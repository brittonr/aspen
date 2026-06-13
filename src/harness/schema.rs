use std::borrow::Cow;
use std::collections::BTreeSet;

use preserves::CompoundClass;
use preserves::IOValue;
use preserves::Record;
use preserves::Value;
use preserves::ValueClass;
use preserves::ValueImpl;
use wasmparser::Encoding;
use wasmparser::Parser;
use wasmparser::Payload;
use wasmparser::Validator;

use super::core::AdmissionRequest;
use super::core::CoreEffect;
use super::core::CoreEvent;
use super::core::CoreStep;
use super::core::RuntimeSnapshot;
use super::core::RuntimeValue;
use crate::effects::DeclaredEffect;
use crate::effects::EffectHandleInput;
use crate::effects::EffectHandleRequest;
use crate::effects::EffectManifestInput;
use crate::effects::EffectRequestInput;
use crate::effects::EffectScope;
use crate::effects::HANDLER_PROFILE_LOCAL;
use crate::effects::HandlerBindingInput;
use crate::effects::HandlerProfileInput;
use crate::effects::TRANSFER_LOCAL_ONLY;
use crate::effects::admit_effect_request;
use crate::effects::effect_handle_value;
use crate::effects::effect_manifest_value;
use crate::effects::effect_request_value;
use crate::effects::handler_binding_value;
use crate::effects::handler_profile_value;
use crate::effects::validate_handle_for_request;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA;
use crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA;
use crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA;
use crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA;
use crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA;
use crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA;
use crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA;
use crate::preserves_rail::HARNESS_BUDGET_SCHEMA;
use crate::preserves_rail::HARNESS_BUDGET_USAGE_SCHEMA;
use crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA;
use crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA;
use crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA;
use crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA;
use crate::preserves_rail::HARNESS_EXECUTOR_CONFORMANCE_SCHEMA;
use crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA;
use crate::preserves_rail::HARNESS_FAILURE_SCHEMA;
use crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA;
use crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA;
use crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA;
use crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA;
use crate::preserves_rail::HARNESS_POLICY_SCHEMA;
use crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA;
use crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA;
use crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA;
use crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA;
use crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA;
use crate::preserves_rail::HARNESS_REPORT_SCHEMA;
use crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA;
use crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA;
use crate::preserves_rail::HARNESS_SUITE_SCHEMA;
use crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA;
use crate::preserves_rail::HASH_ALGORITHM;
use crate::preserves_rail::RUNTIME_ACTOR_INPUT_SCHEMA;
use crate::preserves_rail::RUNTIME_ACTOR_OUTPUT_SCHEMA;
use crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA;
use crate::preserves_rail::RUNTIME_ADAPTER_PREFLIGHT_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA;
use crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA;
use crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA;
use crate::preserves_rail::RUNTIME_HOSTCALL_DECISION_SCHEMA;
use crate::preserves_rail::RUNTIME_HOSTCALL_REQUEST_SCHEMA;
use crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA;
use crate::preserves_rail::RUNTIME_REMOTE_PROXY_PREFLIGHT_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA;
use crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA;
use crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA;
use crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;
use crate::runtime::AdmissionAction;
use crate::runtime::AdmissionDecision;
use crate::runtime::AdmissionDenyRule;
use crate::runtime::AdmissionPolicy;
use crate::runtime::CapabilityContext;
use crate::runtime::CapabilityGrant;
use crate::secrets::EncryptedRefInput;
use crate::secrets::PrivateBundleProfileInput;
use crate::secrets::RedactionMarkerInput;
use crate::secrets::encrypted_ref_value;
use crate::secrets::parse_encrypted_ref;
use crate::secrets::parse_private_bundle_profile;
use crate::secrets::parse_redaction_marker;
use crate::secrets::private_bundle_profile_value;
use crate::secrets::redaction_marker_value;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessSuite {
    pub name: String,
    pub seed: u64,
    pub budget: HarnessBudget,
    pub budget_explicit: bool,
    pub actors: Vec<ActorDecl>,
    pub actors_explicit: bool,
    pub capabilities: CapabilityContext,
    pub capabilities_explicit: bool,
    pub policy: AdmissionPolicy,
    pub steps: Vec<CoreStep>,
    pub source_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessReport {
    pub report_ref: String,
    pub status: String,
    pub replay_status: String,
    pub profile: String,
    pub hash_algorithm: String,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub suite_value: IOValue,
    pub policy_gate: Option<PolicyGateEvidence>,
    pub capability_gate: Option<CapabilityGateEvidence>,
    pub budget_gate: Option<BudgetGateEvidence>,
    pub actors: Vec<ActorDecl>,
    pub executor_preflights: Option<ExecutorPreflightsEvidence>,
    pub observations: Vec<HarnessObservation>,
    pub effect_log: Vec<EffectLogEntry>,
    pub budget: BudgetEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessFailure {
    pub failure_ref: String,
    pub phase: String,
    pub kind: String,
    pub message: String,
    pub diagnostics: Vec<IOValue>,
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
pub enum HarnessReproBundleKind {
    Report,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessReproBundle {
    pub bundle_ref: String,
    pub kind: HarnessReproBundleKind,
    pub artifact_ref: String,
    pub report_value: Option<IOValue>,
    pub failure_value: Option<IOValue>,
    pub gate_receipt_ref: Option<String>,
    pub gate_receipt_value: Option<IOValue>,
    pub redaction_policy_ref: Option<String>,
    pub redaction_gate_ref: Option<String>,
    pub export_profile: Option<String>,
    pub export_profile_ref: Option<String>,
    pub export_profile_value: Option<IOValue>,
    pub source_report_ref: Option<String>,
    pub source_suite_ref: Option<String>,
    pub redaction_transform_manifest_ref: Option<String>,
    pub redaction_transform_manifest_value: Option<IOValue>,
    pub redaction_transform_receipt_ref: Option<String>,
    pub redaction_transform_receipt_value: Option<IOValue>,
    pub private_bundle_profile_ref: Option<String>,
    pub private_bundle_profile_value: Option<IOValue>,
    pub loss_classification: Option<String>,
    pub encrypted_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessBudget {
    pub max_steps: u64,
    pub max_effects: u64,
    pub max_events: u64,
    pub max_report_bytes: u64,
}

impl Default for HarnessBudget {
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
    pub limits: HarnessBudget,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectLogEntry {
    pub sequence: u64,
    pub request: IOValue,
    pub response: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyGateEvidence {
    pub value: IOValue,
    pub policy_ref: String,
    pub nickel_source_ref: String,
    pub nickel_export_ref: String,
    pub basalt_preflight_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGateEvidence {
    pub value: IOValue,
    pub capability_ref: String,
    pub authority_preflight_ref: String,
    pub proofset_ref: String,
    pub grant_refs: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetGateEvidence {
    pub value: IOValue,
    pub budget_ref: String,
    pub nickel_source_ref: String,
    pub nickel_export_ref: String,
    pub basalt_preflight_ref: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPreflightsEvidence {
    pub value: IOValue,
    pub preflights: Vec<ExecutorPreflightEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPreflightEvidence {
    pub value: IOValue,
    pub actor_id: String,
    pub kind: ActorKind,
    pub artifact_ref: Option<String>,
    pub sandbox_ref: String,
    pub allowed_hostcalls: Vec<String>,
    pub conformance_refs: Vec<String>,
    pub executor_receipts: Vec<IOValue>,
    pub steel_review: Option<SteelReviewReceipt>,
    pub wasm_inspection: Option<WasmInspectionReceipt>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteelReviewReceipt {
    pub value: IOValue,
    pub source_ref: String,
    pub callable: String,
    pub allowed_hostcalls: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmInspectionReceipt {
    pub value: IOValue,
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
pub struct HarnessObservation {
    pub value: IOValue,
    pub observation_ref: String,
    pub index: u64,
    pub step_ref: String,
    pub before_state_hash: String,
    pub after_state_hash: String,
    pub event_refs: Vec<String>,
    pub events: Vec<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDecisionEvent {
    pub value: IOValue,
    pub request: AdmissionRequest,
    pub authority: Option<AdmissionAuthorityEvidence>,
    pub decision: AdmissionDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionAuthorityEvidence {
    pub capability_ref: String,
    pub authorized: bool,
    pub grant_ref: Option<String>,
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

pub fn parse_suite(value: &IOValue) -> Result<HarnessSuite> {
    let suite = value
        .collect_simple_record("harness-suite-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <harness-suite-v1 ...>"))?;
    let arity = suite.fields_iter().count();
    if !(4..=8).contains(&arity) {
        return Err(MoltenError::invalid_harness(format!(
            "expected <harness-suite-v1 ...> with arity 4 through 8, got {arity}"
        )));
    }
    let schema = required_string(&suite[0], "suite schema")?;
    if schema != HARNESS_SUITE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported suite schema {schema}; expected {HARNESS_SUITE_SCHEMA}"
        )));
    }
    let name = required_string(&suite[1], "suite name")?;
    let seed = required_u64(&suite[2], "suite seed")?;

    let mut cursor = 3;
    let mut budget = HarnessBudget::default();
    let mut has_budget_fixture = false;
    let mut actors = None;
    let mut has_actor_fixture = false;
    let mut capabilities = CapabilityContext::default();
    let mut has_capability_fixture = false;
    let mut policy = AdmissionPolicy::default();
    let mut has_policy_fixture = false;

    while cursor < arity - 1 {
        let field = &suite[cursor];
        if value_has_record_label(field, "budget-v1") {
            if has_budget_fixture {
                return Err(MoltenError::invalid_harness("duplicate suite budget fixture"));
            }
            budget = parse_budget_limits(&value_to_iovalue(field))?;
            has_budget_fixture = true;
            cursor += 1;
            continue;
        }
        if value_has_record_label(field, "actor-registry-v1") {
            if actors.is_some() {
                return Err(MoltenError::invalid_harness("duplicate suite actor registry fixture"));
            }
            actors = Some(parse_actor_registry(&value_to_iovalue(field))?);
            has_actor_fixture = true;
            cursor += 1;
            continue;
        }
        if value_has_record_label(field, "capabilities-v1") {
            if has_capability_fixture {
                return Err(MoltenError::invalid_harness("duplicate suite capability fixture"));
            }
            capabilities = parse_capabilities(&value_to_iovalue(field))?;
            has_capability_fixture = true;
            cursor += 1;
            continue;
        }
        if value_has_record_label(field, "policy-v1") {
            if has_policy_fixture {
                return Err(MoltenError::invalid_harness("duplicate suite policy fixture"));
            }
            policy = parse_policy(&value_to_iovalue(field))?;
            has_policy_fixture = true;
            cursor += 1;
            continue;
        }
        return Err(MoltenError::invalid_harness(
            "unexpected suite field before steps; expected optional budget, actor registry, capabilities, policy, then steps",
        ));
    }

    let step_values = required_sequence(&suite[cursor], "suite steps")?;
    let mut steps = Vec::with_capacity(step_values.len());
    for step in step_values.iter() {
        steps.push(parse_step(&step)?);
    }
    let actors = actors.unwrap_or_else(|| infer_actor_registry(&steps));
    Ok(HarnessSuite {
        name,
        seed,
        budget,
        budget_explicit: has_budget_fixture,
        actors,
        actors_explicit: has_actor_fixture,
        capabilities,
        capabilities_explicit: has_capability_fixture,
        policy,
        steps,
        source_value: value.clone(),
    })
}

pub fn suite_ref(suite: &HarnessSuite) -> Result<String> {
    canonical_hash(&suite.source_value)
}

pub fn step_value(step: &CoreStep) -> IOValue {
    match step {
        CoreStep::Send { from, to, body } => record("send", vec![string(from), string(to), body.as_iovalue().clone()]),
        CoreStep::Observe { actor, pattern } => record("observe", vec![string(actor), pattern.as_iovalue().clone()]),
        CoreStep::Assert { actor, value } => record("assert", vec![string(actor), value.as_iovalue().clone()]),
        CoreStep::Retract { actor, value } => record("retract", vec![string(actor), value.as_iovalue().clone()]),
        CoreStep::Clock { actor } => record("clock", vec![string(actor)]),
        CoreStep::Random { actor, upper } => record("random", vec![string(actor), u64_value(*upper)]),
    }
}

pub fn event_value(event: &CoreEvent) -> IOValue {
    match event {
        CoreEvent::MessageDelivered { from, to, body } => {
            record("message-delivered", vec![string(from), string(to), body.as_iovalue().clone()])
        }
        CoreEvent::ObserveRegistered { actor, pattern } => {
            record("observe-registered", vec![string(actor), pattern.as_iovalue().clone()])
        }
        CoreEvent::AssertionObserved { observer, owner, value } => {
            record("assertion-observed", vec![string(observer), string(owner), value.as_iovalue().clone()])
        }
        CoreEvent::AssertionCommitted { actor, value } => {
            record("assertion-committed", vec![string(actor), value.as_iovalue().clone()])
        }
        CoreEvent::AssertionRetracted { actor, value } => {
            record("assertion-retracted", vec![string(actor), value.as_iovalue().clone()])
        }
        CoreEvent::AssertionRetractionObserved { observer, owner, value } => {
            record("assertion-retraction-observed", vec![string(observer), string(owner), value.as_iovalue().clone()])
        }
        CoreEvent::EffectRequest {
            effect,
            actor,
            sequence,
            upper,
        } => {
            let mut fields = vec![string(effect_name(effect)), string(actor), u64_value(*sequence)];
            if let Some(upper) = upper {
                fields.push(u64_value(*upper));
            }
            record("effect-request", fields)
        }
        CoreEvent::EffectResponse {
            effect,
            actor,
            sequence,
            upper,
            value,
        } => {
            let mut fields = vec![string(effect_name(effect)), string(actor), u64_value(*sequence)];
            if let Some(upper) = upper {
                fields.push(u64_value(*upper));
            }
            fields.push(u64_value(*value));
            record("effect-response", fields)
        }
        CoreEvent::AdmissionDecision { request, decision } => admission_decision_event_value(request, decision),
        CoreEvent::TurnRolledBack { actor, reason } => record("turn-rolled-back", vec![string(actor), string(reason)]),
    }
}

fn admission_decision_event_value(request: &AdmissionRequest, decision: &AdmissionDecision) -> IOValue {
    record("admission-decision-v1", vec![
        string(RUNTIME_ADMISSION_DECISION_SCHEMA),
        admission_request_value(request),
        record("decision", vec![string(decision.status()), string(decision.reason())]),
    ])
}

pub fn admission_decision_event_value_with_authority(
    request: &AdmissionRequest,
    authority: &AdmissionAuthorityEvidence,
    decision: &AdmissionDecision,
) -> IOValue {
    record("admission-decision-v1", vec![
        string(RUNTIME_ADMISSION_DECISION_SCHEMA),
        admission_request_value(request),
        admission_authority_value(authority),
        record("decision", vec![string(decision.status()), string(decision.reason())]),
    ])
}

fn admission_authority_value(authority: &AdmissionAuthorityEvidence) -> IOValue {
    record("authority", vec![
        record("capability-ref", vec![string(&authority.capability_ref)]),
        record("authorized", vec![bool_value(authority.authorized)]),
        optional_string_value(authority.grant_ref.as_deref()),
    ])
}

fn admission_request_value(request: &AdmissionRequest) -> IOValue {
    record("request", vec![
        string(&request.actor),
        string(request.action.as_str()),
        optional_string_value(request.target.as_deref()),
        optional_runtime_value(request.value.as_ref()),
        optional_u64_value(request.upper),
    ])
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_runtime_value(value: Option<&RuntimeValue>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![value.as_iovalue().clone()]))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

pub fn actor_input_value(
    suite: &HarnessSuite,
    step: &CoreStep,
    context: HostcallEvidenceContext<'_>,
) -> Result<IOValue> {
    let actor = step.primary_actor();
    let kind = actor_kind_for_primary_actor(suite, actor)?;
    Ok(record("actor-input-v1", vec![
        string(RUNTIME_ACTOR_INPUT_SCHEMA),
        record("actor", vec![string(actor), string(kind.as_str())]),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("policy-ref", vec![string(context.policy_ref)]),
        record("capability-ref", vec![string(context.capability_ref)]),
        record("budget-ref", vec![string(context.budget_ref)]),
        record("input", vec![step_value(step)]),
        hostcall_checks_value(&["canonical-preserves", "actor-registry-binding", "executor-boundary"]),
    ]))
}

pub fn hostcall_request_value(
    suite: &HarnessSuite,
    step: &CoreStep,
    context: HostcallEvidenceContext<'_>,
    decision: &AdmissionDecision,
) -> Result<IOValue> {
    let request = AdmissionRequest::from_step(step);
    let effect_refs = hostcall_effect_refs(suite, step, context, decision.is_allowed())?;
    let checks = if decision.is_allowed() {
        vec![
            "no-ambient-executor-io",
            "policy-capability-budget-context",
            "handler-binding-available",
            "effect-handle-binding",
            "handle-not-authority",
            "effect-manifest-bound",
            "deny-undeclared-effects",
        ]
    } else {
        vec![
            "no-ambient-executor-io",
            "policy-capability-budget-context",
            "handler-binding-available",
            "effect-handle-binding",
            "handle-not-authority",
        ]
    };
    let mut fields = vec![
        string(RUNTIME_HOSTCALL_REQUEST_SCHEMA),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("operation", vec![string(request.action.as_str())]),
        admission_request_value(&request),
        record("policy-ref", vec![string(context.policy_ref)]),
        record("capability-ref", vec![string(context.capability_ref)]),
        record("budget-ref", vec![string(context.budget_ref)]),
        hostcall_checks_value(&checks),
        record("handler-binding-ref", vec![string(&effect_refs.handler_binding_ref)]),
        record("handle-ref", vec![string(&effect_refs.handle_ref)]),
    ];
    if let Some(effect_manifest_ref) = &effect_refs.effect_manifest_ref {
        fields.push(record("effect-manifest-ref", vec![string(effect_manifest_ref)]));
    }
    if let Some(handler_profile_ref) = &effect_refs.handler_profile_ref {
        fields.push(record("handler-profile-ref", vec![string(handler_profile_ref)]));
    }
    if let Some(effect_request_ref) = &effect_refs.effect_request_ref {
        fields.push(record("effect-request-ref", vec![string(effect_request_ref)]));
    }
    if let Some(effect_binding_receipt_ref) = &effect_refs.effect_binding_receipt_ref {
        fields.push(record("effect-binding-receipt-ref", vec![string(effect_binding_receipt_ref)]));
    }
    Ok(record("hostcall-request-v1", fields))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostcallEffectRefs {
    handler_binding_ref: String,
    handle_ref: String,
    effect_manifest_ref: Option<String>,
    handler_profile_ref: Option<String>,
    effect_request_ref: Option<String>,
    effect_binding_receipt_ref: Option<String>,
}

fn hostcall_effect_refs(
    suite: &HarnessSuite,
    step: &CoreStep,
    context: HostcallEvidenceContext<'_>,
    bind_effect_request: bool,
) -> Result<HostcallEffectRefs> {
    let actor = actor_decl_for_primary_actor(suite, step.primary_actor())?;
    let operation = AdmissionRequest::from_step(step).action.as_str();
    let actor_ref = actor_identity_ref(&actor.id)?;
    let session_ref = hostcall_session_ref(context)?;
    let scope = EffectScope {
        run_ref: context.suite_ref.to_string(),
        session_ref: session_ref.clone(),
        actor_ref: Some(actor_ref.clone()),
        turn_ref: Some(context.step_ref.to_string()),
    };
    let allowed_hostcalls = allowed_hostcalls_for_actor(suite, actor);
    let executor_preflight = executor_preflight_value(actor, &allowed_hostcalls)?;
    let executor_preflight_ref = canonical_hash(&executor_preflight)?;
    let adapter_ref = canonical_hash(&record("hostcall-adapter-surface", vec![
        string(&actor.id),
        string(actor.kind.as_str()),
        string(&executor_preflight_ref),
    ]))?;
    let resource_refs = vec![context.budget_ref.to_string()];
    let evidence_refs = vec![executor_preflight_ref.clone()];
    let handler_binding = handler_binding_value(&HandlerBindingInput {
        profile: "local-hostcall".to_string(),
        scope: scope.clone(),
        adapter_kind: "hostcall".to_string(),
        adapter_ref,
        executor_preflight_ref: Some(executor_preflight_ref.clone()),
        policy_ref: context.policy_ref.to_string(),
        capability_context_ref: context.capability_ref.to_string(),
        authority_context_ref: None,
        resource_refs: resource_refs.clone(),
        operations: allowed_hostcalls.clone(),
        evidence_refs: evidence_refs.clone(),
    })?;
    let handler_binding_ref = canonical_hash(&handler_binding)?;
    let handle = effect_handle_value(&EffectHandleInput {
        kind: "hostcall".to_string(),
        scope,
        handler_binding_ref: handler_binding_ref.clone(),
        operations: vec![operation.to_string()],
        capability_context_ref: context.capability_ref.to_string(),
        authority_context_ref: None,
        resource_refs: resource_refs.clone(),
        not_before: Some(0),
        expires_at: None,
        revocation_refs: Vec::new(),
        transfer: TRANSFER_LOCAL_ONLY.to_string(),
        parent_handle_ref: None,
        evidence_refs,
    })?;
    let handle_ref = canonical_hash(&handle)?;
    let mut effect_manifest_ref = None;
    let mut handler_profile_ref = None;
    let mut effect_request_ref = None;
    let mut effect_binding_receipt_ref = None;
    if bind_effect_request {
        let effect_id = format!("hostcall.{operation}");
        let effect_manifest = effect_manifest_value(&EffectManifestInput {
            artifact_kind: actor.kind.as_str().to_string(),
            artifact_ref: actor_ref.clone(),
            executor_kind: actor.kind.as_str().to_string(),
            declared_effects: allowed_hostcalls
                .as_slice()
                .iter()
                .map(|hostcall| DeclaredEffect {
                    effect_id: format!("hostcall.{hostcall}"),
                    operation: hostcall.clone(),
                    input_schema_ref: context.step_ref.to_string(),
                    output_schema_ref: context.step_ref.to_string(),
                    evidence_refs: vec![executor_preflight_ref.clone()],
                })
                .collect(),
            policy_refs: vec![context.policy_ref.to_string()],
            evidence_refs: vec![executor_preflight_ref.clone()],
        })?;
        effect_manifest_ref = Some(canonical_hash(&effect_manifest)?);
        let handler_profile = handler_profile_value(&HandlerProfileInput {
            profile: HANDLER_PROFILE_LOCAL.to_string(),
            handler_binding_refs: vec![handler_binding_ref.clone()],
            policy_ref: context.policy_ref.to_string(),
            capability_context_ref: context.capability_ref.to_string(),
            resource_refs: resource_refs.clone(),
            evidence_refs: vec![executor_preflight_ref.clone()],
        })?;
        handler_profile_ref = Some(canonical_hash(&handler_profile)?);
        let effect_request = effect_request_value(&EffectRequestInput {
            artifact_ref: actor_ref.clone(),
            effect_id,
            operation: operation.to_string(),
            handler_profile: HANDLER_PROFILE_LOCAL.to_string(),
            input_ref: context.step_ref.to_string(),
            capability_refs: vec![context.capability_ref.to_string()],
            evidence_refs: vec![handler_binding_ref.clone(), handle_ref.clone()],
        })?;
        effect_request_ref = Some(canonical_hash(&effect_request)?);
        let effect_binding = admit_effect_request(&effect_manifest, &handler_profile, &effect_request, &[
            handler_binding_ref.clone(),
            handle_ref.clone(),
        ])?;
        if effect_binding.decision != "pass" {
            return Err(MoltenError::invalid_harness(format!(
                "hostcall effect manifest denied operation {operation}: {:?}",
                effect_binding.diagnostics
            )));
        }
        effect_binding_receipt_ref = Some(effect_binding.receipt_ref.clone());
    }
    let validation = validate_handle_for_request(&handler_binding, &handle, &EffectHandleRequest {
        kind: "hostcall",
        operation,
        run_ref: context.suite_ref,
        session_ref: &session_ref,
        actor_ref: Some(&actor_ref),
        turn_ref: Some(context.step_ref),
        policy_ref: context.policy_ref,
        capability_context_ref: context.capability_ref,
        authority_context_ref: None,
        resource_refs: &resource_refs,
        logical_time: context.sequence,
        remote_use: false,
        revoked_refs: &[],
    })?;
    if validation.handler_binding_ref != handler_binding_ref || validation.handle_ref != handle_ref {
        return Err(MoltenError::invalid_harness("hostcall effect handle validation ref mismatch"));
    }
    Ok(HostcallEffectRefs {
        handler_binding_ref,
        handle_ref,
        effect_manifest_ref,
        handler_profile_ref,
        effect_request_ref,
        effect_binding_receipt_ref,
    })
}

fn actor_identity_ref(actor_id: &str) -> Result<String> {
    canonical_hash(&record("actor-identity-v1", vec![string(actor_id)]))
}

fn hostcall_session_ref(context: HostcallEvidenceContext<'_>) -> Result<String> {
    canonical_hash(&record("hostcall-session-v1", vec![
        string(context.suite_ref),
        string(context.policy_ref),
        string(context.capability_ref),
        string(context.budget_ref),
    ]))
}

pub(crate) fn validate_hostcall_effect_binding_request(hostcall_request: &IOValue, operation: &str) -> Result<()> {
    let request = hostcall_request
        .collect_simple_record("hostcall-request-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("executor hostcall gate requires bound effect request evidence"))?;
    let request_operation = required_record_string(&request[3], "operation", "hostcall request operation")?;
    if request_operation != operation {
        return Err(MoltenError::invalid_harness(format!(
            "executor hostcall gate operation mismatch: got {request_operation}, expected {operation}"
        )));
    }
    for (field, label) in [
        (&request[11], "effect-manifest-ref"),
        (&request[12], "handler-profile-ref"),
        (&request[13], "effect-request-ref"),
        (&request[14], "effect-binding-receipt-ref"),
    ] {
        let content_ref = required_record_string(field, label, label)?;
        validate_content_ref(&content_ref)?;
    }
    Ok(())
}

pub fn hostcall_decision_value(
    context: HostcallEvidenceContext<'_>,
    admission_event: &IOValue,
    authority: &AdmissionAuthorityEvidence,
    decision: &AdmissionDecision,
) -> Result<IOValue> {
    let authority_value = admission_authority_value(authority);
    Ok(record("hostcall-decision-v1", vec![
        string(RUNTIME_HOSTCALL_DECISION_SCHEMA),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("decision", vec![string(decision.status()), string(decision.reason())]),
        record("admission-ref", vec![string(canonical_hash(admission_event)?)]),
        record("authority-ref", vec![string(canonical_hash(&authority_value)?)]),
        record("policy-ref", vec![string(context.policy_ref)]),
        record("capability-ref", vec![string(context.capability_ref)]),
        record("budget-ref", vec![string(context.budget_ref)]),
        hostcall_checks_value(&["admission-binding", "authority-binding", "budget-binding"]),
    ]))
}

pub fn actor_output_value(
    step: &CoreStep,
    context: HostcallEvidenceContext<'_>,
    decision: &AdmissionDecision,
    runtime_events: &[IOValue],
) -> Result<IOValue> {
    let runtime_events_value = sequence(runtime_events.to_vec());
    Ok(record("actor-output-v1", vec![
        string(RUNTIME_ACTOR_OUTPUT_SCHEMA),
        record("actor", vec![string(step.primary_actor())]),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("decision", vec![string(decision.status())]),
        record("events-ref", vec![string(canonical_hash(&runtime_events_value)?)]),
        record("events", vec![u64_value(runtime_events.len() as u64)]),
        hostcall_checks_value(&["staged-output", "deterministic-trace"]),
    ]))
}

pub(crate) struct SteelExecutionReceiptInput<'a> {
    pub actor_id: &'a str,
    pub source_ref: &'a str,
    pub callable: &'a str,
    pub operation: &'a str,
    pub input_ref: &'a str,
    pub output_ref: &'a str,
    pub hostcalls: &'a [String],
    pub resource_limits: SteelResourceReceiptInput,
}

pub(crate) struct SteelResourceReceiptInput {
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
    pub source_bytes: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub hostcall_limit: u64,
    pub hostcall_count: u64,
}

pub(crate) fn steel_execution_receipt_value(input: SteelExecutionReceiptInput<'_>) -> IOValue {
    record("steel-execution-receipt-v1", vec![
        string(RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA),
        record("actor", vec![string(input.actor_id)]),
        record("source-ref", vec![string(input.source_ref)]),
        record("callable", vec![string(input.callable)]),
        record("operation", vec![string(input.operation)]),
        record("input-ref", vec![string(input.input_ref)]),
        record("output-ref", vec![string(input.output_ref)]),
        record("hostcalls", vec![sequence(
            input.hostcalls.iter().map(|hostcall| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("resources", vec![
            record("fuel", vec![
                u64_value(input.resource_limits.fuel_limit),
                u64_value(input.resource_limits.fuel_remaining),
            ]),
            record("source-bytes", vec![u64_value(input.resource_limits.source_bytes)]),
            record("input-bytes", vec![u64_value(input.resource_limits.input_bytes)]),
            record("output-bytes", vec![u64_value(input.resource_limits.output_bytes)]),
            record("hostcalls", vec![
                u64_value(input.resource_limits.hostcall_limit),
                u64_value(input.resource_limits.hostcall_count),
            ]),
        ]),
        hostcall_checks_value(&[
            "steel-vm-executed",
            "reviewed-callable-binding",
            "canonical-preserves-input",
            "canonical-preserves-output",
            "no-ambient-steel-io",
            "hostcall-envelope-binding",
            "effect-manifest-bound",
            "effect-request-admitted",
            "declared-effect-id-required",
            "resource-bounded",
            "fuel-bounded",
            "hostcall-count-bounded",
            "io-bytes-bounded",
        ]),
    ])
}

pub(crate) struct WasmExecutionReceiptInput<'a> {
    pub actor_id: &'a str,
    pub module_ref: &'a str,
    pub export: &'a str,
    pub operation: &'a str,
    pub hostcalls: &'a [String],
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
    pub memory_limit_bytes: u64,
    pub abi: Option<WasmAbiReceiptInput>,
}

pub(crate) struct WasmAbiReceiptInput {
    pub input_ref: String,
    pub output_ref: String,
    pub output_bytes: u64,
}

pub(crate) fn wasm_execution_receipt_value(input: WasmExecutionReceiptInput<'_>) -> IOValue {
    let mut checks = vec![
        "wasmtime-instantiated",
        "no-wasi",
        "fuel-bounded",
        "memory-bounded",
        "hostcall-envelope-binding",
        "effect-manifest-bound",
        "effect-request-admitted",
        "declared-effect-id-required",
    ];
    let mut fields = vec![
        string(RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA),
        record("actor", vec![string(input.actor_id)]),
        record("module-ref", vec![string(input.module_ref)]),
        record("export", vec![string(input.export)]),
        record("operation", vec![string(input.operation)]),
        record("hostcalls", vec![sequence(
            input.hostcalls.iter().map(|hostcall| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("fuel", vec![u64_value(input.fuel_limit), u64_value(input.fuel_remaining)]),
        record("memory-limit", vec![u64_value(input.memory_limit_bytes)]),
    ];
    if let Some(abi) = input.abi {
        checks.extend([
            "preserves-abi-v1",
            "canonical-preserves-input",
            "canonical-preserves-output",
            "guest-memory-bounds",
        ]);
        fields.extend([
            record("abi", vec![string(RUNTIME_WASM_ABI_SCHEMA)]),
            record("input-ref", vec![string(&abi.input_ref)]),
            record("output-ref", vec![string(&abi.output_ref)]),
            record("output-bytes", vec![u64_value(abi.output_bytes)]),
        ]);
    }
    fields.push(hostcall_checks_value(&checks));
    record("wasm-execution-receipt-v1", fields)
}

fn hostcall_checks_value(checks: &[&str]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

fn actor_decl_for_primary_actor<'a>(suite: &'a HarnessSuite, actor: &str) -> Result<&'a ActorDecl> {
    suite
        .actors
        .iter()
        .find(|decl| decl.id == actor)
        .ok_or_else(|| MoltenError::invalid_harness(format!("actor {actor} missing from executor registry")))
}

fn actor_kind_for_primary_actor<'a>(suite: &'a HarnessSuite, actor: &str) -> Result<&'a ActorKind> {
    actor_decl_for_primary_actor(suite, actor).map(|decl| &decl.kind)
}

pub fn snapshot_value(snapshot: &RuntimeSnapshot) -> IOValue {
    record("runtime-state-v1", vec![
        u64_value(snapshot.logical_time),
        u64_value(snapshot.rng_state),
        u64_value(snapshot.effect_sequence),
        tuple_set("messages", &snapshot.messages, |message| {
            record("message", vec![
                string(&message.from),
                string(&message.to),
                message.body.as_iovalue().clone(),
            ])
        }),
        tuple_set("assertions", &snapshot.assertions, |assertion| {
            record("assertion", vec![string(&assertion.actor), assertion.value.as_iovalue().clone()])
        }),
        tuple_set("observers", &snapshot.observers, |observer| {
            record("observer", vec![string(&observer.actor), observer.pattern.as_iovalue().clone()])
        }),
    ])
}

pub fn observation_value(
    index: u64,
    step_ref: String,
    before_state_hash: String,
    after_state_hash: String,
    events: Vec<IOValue>,
) -> Result<IOValue> {
    let mut event_refs = Vec::with_capacity(events.len());
    for event in &events {
        event_refs.push(canonical_hash(event)?);
    }
    let mut event_ref_values: Vec<IOValue> = Vec::with_capacity(event_refs.len());
    for reference in event_refs {
        event_ref_values.push(string(reference));
    }
    Ok(record("turn-observation-v1", vec![
        string(HARNESS_OBSERVATION_SCHEMA),
        u64_value(index),
        string(step_ref),
        string(before_state_hash),
        string(after_state_hash),
        record("event-refs", vec![sequence(event_ref_values)]),
        sequence(events),
    ]))
}

pub struct ReportValueInput<'a> {
    pub suite: &'a HarnessSuite,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub policy_gate: IOValue,
    pub capability_gate: IOValue,
    pub budget_gate: IOValue,
    pub observations: Vec<IOValue>,
    pub effect_log: Vec<EffectLogEntry>,
    pub budget: &'a HarnessBudget,
    pub usage: &'a BudgetUsage,
}

pub fn report_value(input: ReportValueInput<'_>) -> IOValue {
    let executor_preflights = match executor_preflights_value(input.suite) {
        Ok(value) => value,
        Err(error) => record("executor-preflights-invalid-v1", vec![
            string(HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA),
            record("error", vec![string(error.to_string())]),
        ]),
    };
    record("harness-report-v1", vec![
        string(HARNESS_REPORT_SCHEMA),
        string("pass"),
        string("deterministic"),
        string("local-deterministic"),
        string(HASH_ALGORITHM),
        string(input.suite_ref),
        string(input.initial_state_hash),
        string(input.final_state_hash),
        input.suite.source_value.clone(),
        input.policy_gate,
        input.capability_gate,
        input.budget_gate,
        actor_registry_value(&input.suite.actors),
        executor_preflights,
        sequence(input.observations),
        effect_log_value(&input.effect_log),
        budget_value(input.budget, input.usage),
    ])
}

pub fn failure_value(phase: &str, error: &MoltenError, mut diagnostics: Vec<IOValue>) -> IOValue {
    diagnostics.extend(error_diagnostics(error));
    record("harness-failure-v1", vec![
        string(HARNESS_FAILURE_SCHEMA),
        record("phase", vec![string(phase)]),
        record("kind", vec![string(error_kind(error))]),
        record("message", vec![string(error.to_string())]),
        sequence(diagnostics),
    ])
}

pub fn suite_failure_value(phase: &str, error: &MoltenError, suite_value: &IOValue) -> Result<IOValue> {
    Ok(failure_value(phase, error, vec![
        record("suite-ref", vec![string(canonical_hash(suite_value)?)]),
        record("suite", vec![suite_value.clone()]),
    ]))
}

pub fn report_failure_value(phase: &str, error: &MoltenError, report_value: &IOValue) -> Result<IOValue> {
    Ok(failure_value(phase, error, vec![
        record("report-ref", vec![string(canonical_hash(report_value)?)]),
        record("report", vec![report_value.clone()]),
    ]))
}

pub fn parse_failure(failure_value: &IOValue) -> Result<HarnessFailure> {
    let failure = simple_record(failure_value, "harness-failure-v1", 5)?;
    let schema = required_string(&failure[0], "failure schema")?;
    if schema != HARNESS_FAILURE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported failure schema {schema}; expected {HARNESS_FAILURE_SCHEMA}"
        )));
    }
    let phase_value = value_to_iovalue(&failure[1]);
    let phase_record = simple_record(&phase_value, "phase", 1)?;
    let phase = required_string(&phase_record[0], "failure phase")?;
    if !matches!(phase.as_str(), "preflight" | "execute" | "replay" | "validate" | "export" | "verify" | "unpack") {
        return Err(MoltenError::invalid_harness(format!("unsupported failure phase {phase}")));
    }
    let kind_value = value_to_iovalue(&failure[2]);
    let kind_record = simple_record(&kind_value, "kind", 1)?;
    let kind = required_string(&kind_record[0], "failure kind")?;
    if kind.is_empty() {
        return Err(MoltenError::invalid_harness("failure kind must not be empty"));
    }
    let message_value = value_to_iovalue(&failure[3]);
    let message_record = simple_record(&message_value, "message", 1)?;
    let message = required_string(&message_record[0], "failure message")?;
    let diagnostic_values = required_sequence(&failure[4], "failure diagnostics")?;
    let mut diagnostics = Vec::with_capacity(diagnostic_values.len());
    for diagnostic in diagnostic_values.iter() {
        diagnostics.push(value_to_iovalue(&diagnostic));
    }
    Ok(HarnessFailure {
        failure_ref: canonical_hash(failure_value)?,
        phase,
        kind,
        message,
        diagnostics,
    })
}

pub fn failure_summary(failure_value: &IOValue) -> Result<String> {
    let failure = parse_failure(failure_value)?;
    Ok(format!(
        "failure {}\nstatus=fail\nphase={}\nkind={}\nmessage={}\ndiagnostics={}",
        failure.failure_ref,
        failure.phase,
        failure.kind,
        failure.message,
        failure.diagnostics.len()
    ))
}

pub fn parse_report(report_value: &IOValue) -> Result<HarnessReport> {
    let report = report_value
        .collect_simple_record("harness-report-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <harness-report-v1 ...>"))?;
    let arity = report.fields_iter().count();
    if arity != 13 && arity != 14 && arity != 15 && arity != 16 && arity != 17 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <harness-report-v1 ...> with arity 13, 14, 15, 16, or 17, got {arity}"
        )));
    }
    let schema = required_string(&report[0], "report schema")?;
    if schema != HARNESS_REPORT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported report schema {schema}; expected {HARNESS_REPORT_SCHEMA}"
        )));
    }

    let status = required_string(&report[1], "report status")?;
    if status != "pass" {
        return Err(MoltenError::invalid_harness(format!("evidence-bearing report status must be pass, got {status}")));
    }

    let replay_status = required_string(&report[2], "report replay status")?;
    if !matches!(replay_status.as_str(), "deterministic" | "replay" | "record") {
        return Err(MoltenError::invalid_harness(format!("unsupported evidence replay status {replay_status}")));
    }

    let profile = required_string(&report[3], "report profile")?;
    let hash_algorithm = required_string(&report[4], "report hash algorithm")?;
    if hash_algorithm != HASH_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported hash algorithm {hash_algorithm}; expected {HASH_ALGORITHM}"
        )));
    }

    let suite_ref = required_string(&report[5], "report suite ref")?;
    let initial_state_hash = required_hash(&report[6], "report initial state hash")?;
    let final_state_hash = required_hash(&report[7], "report final state hash")?;
    let suite_value = value_to_iovalue(&report[8]);
    let suite = parse_suite(&suite_value)?;
    let actual_suite_ref = canonical_hash(&suite_value)?;
    if suite_ref != actual_suite_ref {
        return Err(MoltenError::invalid_harness(format!(
            "suite ref mismatch: report has {suite_ref}, embedded suite hashes to {actual_suite_ref}"
        )));
    }

    let mut cursor = 9;
    let policy_gate = if cursor < arity && value_has_record_label(&report[cursor], "policy-gate-v1") {
        let parsed = parse_policy_gate(&value_to_iovalue(&report[cursor]))?;
        cursor += 1;
        Some(parsed)
    } else {
        None
    };
    let capability_gate = if cursor < arity && value_has_record_label(&report[cursor], "capability-gate-v1") {
        let parsed = parse_capability_gate(&value_to_iovalue(&report[cursor]))?;
        cursor += 1;
        Some(parsed)
    } else {
        None
    };
    let budget_gate = if cursor < arity && value_has_record_label(&report[cursor], "budget-gate-v1") {
        let parsed = parse_budget_gate(&value_to_iovalue(&report[cursor]))?;
        cursor += 1;
        Some(parsed)
    } else {
        None
    };

    let actors = parse_actor_registry(&value_to_iovalue(&report[cursor]))?;
    cursor += 1;
    if actors != suite.actors {
        return Err(MoltenError::invalid_harness("report actor registry does not match embedded suite actor registry"));
    }

    let executor_preflights = if cursor < arity && value_has_record_label(&report[cursor], "executor-preflights-v1") {
        let parsed = parse_executor_preflights(&value_to_iovalue(&report[cursor]))?;
        cursor += 1;
        Some(parsed)
    } else {
        None
    };

    let observation_values = required_sequence(&report[cursor], "report observations")?;
    cursor += 1;
    let mut observations = Vec::with_capacity(observation_values.len());
    for (position, observation) in observation_values.iter().enumerate() {
        let parsed = parse_observation(&observation)?;
        if parsed.index != position as u64 {
            return Err(MoltenError::invalid_harness(format!(
                "observation index mismatch at position {position}: got {}",
                parsed.index
            )));
        }
        observations.push(parsed);
    }
    let effect_log_value = value_to_iovalue(&report[cursor]);
    cursor += 1;
    let effect_log = parse_effect_log(&effect_log_value)?;
    let budget_value = value_to_iovalue(&report[cursor]);
    let budget = parse_budget(&budget_value)?;
    if budget.limits != suite.budget {
        return Err(MoltenError::invalid_harness("report budget limits do not match embedded suite budget"));
    }

    Ok(HarnessReport {
        report_ref: canonical_hash(report_value)?,
        status,
        replay_status,
        profile,
        hash_algorithm,
        suite_ref,
        initial_state_hash,
        final_state_hash,
        suite_value,
        policy_gate,
        capability_gate,
        budget_gate,
        actors,
        executor_preflights,
        observations,
        effect_log,
        budget,
    })
}

pub fn validate_budget_fixture_evidence(suite: &HarnessSuite) -> Result<()> {
    if suite.budget_explicit {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot satisfy evidence gates",
        ))
    }
}

pub fn budget_gate_value(budget: &HarnessBudget) -> Result<IOValue> {
    let preflight = budget_preflight_material(budget)?;
    Ok(record("budget-gate-v1", vec![
        string(HARNESS_BUDGET_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("budget-ref", vec![string(&preflight.budget_ref)]),
        preflight.nickel_source_value,
        preflight.resource_contract_value,
        preflight.resource_preflight_value,
        budget_gate_checks_value(),
    ]))
}

pub fn parse_budget_gate(value: &IOValue) -> Result<BudgetGateEvidence> {
    let gate = simple_record(value, "budget-gate-v1", 7)?;
    let schema = required_string(&gate[0], "budget gate schema")?;
    if schema != HARNESS_BUDGET_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget gate schema {schema}; expected {HARNESS_BUDGET_GATE_SCHEMA}"
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "budget gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported budget gate decision {decision}")));
    }
    let budget_ref = required_record_hash(&gate[2], "budget-ref", "budget gate budget ref")?;
    let nickel_source = parse_budget_nickel_source_evidence(&gate[3])?;
    let resource_contract = parse_resource_contract_evidence(&gate[4])?;
    let resource_preflight = parse_basalt_resource_preflight_evidence(&gate[5])?;
    if nickel_source.budget_ref != budget_ref {
        return Err(MoltenError::invalid_harness("Nickel resource policy budget ref does not match budget gate ref"));
    }
    if resource_contract.normalized_budget_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "resource contract normalized budget source ref does not match Nickel resource policy evidence",
        ));
    }
    if resource_preflight.budget_ref != budget_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt resource preflight budget ref does not match budget gate ref",
        ));
    }
    if resource_preflight.envelope_ref != resource_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt resource preflight envelope ref does not match resource contract envelope",
        ));
    }
    if resource_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt resource preflight source ref does not match Nickel resource policy evidence",
        ));
    }
    let checks = parse_budget_gate_checks(&gate[6])?;
    require_budget_gate_check(&checks, "budget-schema")?;
    require_budget_gate_check(&checks, "canonical-budget-snapshot")?;
    require_budget_gate_check(&checks, "explicit-budget-fixture")?;
    require_budget_gate_check(&checks, "no-default-resource-policy")?;
    require_budget_gate_check(&checks, "resource-policy-preflight")?;
    require_budget_gate_check(&checks, "nickel-resource-policy")?;
    require_budget_gate_check(&checks, "nickel-resource-export")?;
    require_budget_gate_check(&checks, "basalt-resource-receipt")?;
    require_budget_gate_check(&checks, "budget-usage-binding")?;
    Ok(BudgetGateEvidence {
        value: value.clone(),
        budget_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: resource_preflight.receipt_ref,
        checks,
    })
}

pub fn validate_budget_gate_evidence(suite: &HarnessSuite, budget_gate: Option<&BudgetGateEvidence>) -> Result<()> {
    if !suite.budget_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot satisfy evidence gates",
        ));
    }
    let budget_gate = budget_gate.ok_or_else(|| {
        MoltenError::invalid_harness(
            "missing budget gate evidence; resource policy must pass preflight before side effects",
        )
    })?;
    let expected_ref = canonical_hash(&budget_limits_value(&suite.budget))?;
    if budget_gate.budget_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "budget gate ref mismatch: gate has {}, embedded budget hashes to {expected_ref}",
            budget_gate.budget_ref
        )));
    }
    let expected_gate = budget_gate_value(&suite.budget)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&budget_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "budget gate evidence does not match embedded resource preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct BudgetPreflightMaterial {
    budget_ref: String,
    nickel_source_value: IOValue,
    resource_contract_value: IOValue,
    resource_preflight_value: IOValue,
}

struct BudgetNickelSourceEvidence {
    source_ref: String,
    export_ref: String,
    budget_ref: String,
}

struct ResourceContractEvidence {
    envelope_ref: String,
    normalized_budget_ref: String,
}

struct BasaltResourcePreflightEvidence {
    receipt_ref: String,
    envelope_ref: String,
    budget_ref: String,
    normalized_source_ref: String,
}

const BUDGET_CONTRACT_ID: &str = "molten.harness.resource-budget";
const BUDGET_CONTRACT_VERSION: &str = "v1";

fn budget_preflight_material(budget: &HarnessBudget) -> Result<BudgetPreflightMaterial> {
    let budget_snapshot = budget_limits_value(budget);
    let budget_ref = canonical_hash(&budget_snapshot)?;
    let source = nickel_budget_source(budget, &budget_ref);
    let source_ref = canonical_hash(&string(&source))?;
    let export_json = nickel_export_json(&source)?;
    let export_ref = canonical_hash(&string(&export_json))?;
    let nickel_source_value = budget_nickel_source_value(&source, &source_ref, &export_json, &export_ref, &budget_ref);
    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        BUDGET_CONTRACT_ID,
        BUDGET_CONTRACT_VERSION,
        source_ref.clone(),
        HARNESS_BUDGET_SCHEMA,
        HARNESS_BUDGET_USAGE_SCHEMA,
        HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt resource preflight denied budget contract envelope: {}",
            receipt.reason
        )));
    }
    let resource_contract_value = record("resource-contract", vec![
        string(HARNESS_BUDGET_CONTRACT_SCHEMA),
        envelope_value,
        record("envelope-ref", vec![string(&envelope_ref)]),
    ]);
    let resource_preflight_value = record("basalt-resource-preflight", vec![
        string(HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("backend", vec![string("nickel")]),
        record("contract-id", vec![string(BUDGET_CONTRACT_ID)]),
        record("envelope-ref", vec![string(envelope_ref)]),
        record("budget-ref", vec![string(&budget_ref)]),
        record("normalized-source-ref", vec![string(source_ref)]),
        record("reason", vec![string(receipt.reason)]),
    ]);
    Ok(BudgetPreflightMaterial {
        budget_ref,
        nickel_source_value,
        resource_contract_value,
        resource_preflight_value,
    })
}

fn budget_nickel_source_value(
    source: &str,
    source_ref: &str,
    export_json: &str,
    export_ref: &str,
    budget_ref: &str,
) -> IOValue {
    record("budget-source", vec![
        string(HARNESS_BUDGET_NICKEL_STATIC_SCHEMA),
        record("source", vec![string(source)]),
        record("source-ref", vec![string(source_ref)]),
        record("export-json", vec![string(export_json)]),
        record("export-ref", vec![string(export_ref)]),
        record("budget-ref", vec![string(budget_ref)]),
    ])
}

fn parse_budget_nickel_source_evidence(value: &Value<IOValue>) -> Result<BudgetNickelSourceEvidence> {
    let value = value_to_iovalue(value);
    let source = simple_record(&value, "budget-source", 6)?;
    let schema = required_string(&source[0], "Nickel resource policy schema")?;
    if schema != HARNESS_BUDGET_NICKEL_STATIC_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Nickel resource policy schema {schema}; expected {HARNESS_BUDGET_NICKEL_STATIC_SCHEMA}"
        )));
    }
    let source_text = required_record_string(&source[1], "source", "Nickel resource policy source")?;
    let source_ref = required_record_hash(&source[2], "source-ref", "Nickel resource policy source ref")?;
    let actual_source_ref = canonical_hash(&string(&source_text))?;
    if source_ref != actual_source_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel resource policy source ref mismatch: evidence has {source_ref}, source hashes to {actual_source_ref}"
        )));
    }
    let export_json = required_record_string(&source[3], "export-json", "Nickel resource policy export JSON")?;
    let export_ref = required_record_hash(&source[4], "export-ref", "Nickel resource policy export ref")?;
    let actual_export_ref = canonical_hash(&string(&export_json))?;
    if export_ref != actual_export_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel resource policy export ref mismatch: evidence has {export_ref}, export hashes to {actual_export_ref}"
        )));
    }
    let actual_export = nickel_export_json(&source_text)?;
    if actual_export != export_json {
        return Err(MoltenError::invalid_harness(
            "Nickel resource policy export JSON does not match source normalization",
        ));
    }
    let budget_ref = required_record_hash(&source[5], "budget-ref", "Nickel resource policy budget ref")?;
    Ok(BudgetNickelSourceEvidence {
        source_ref,
        export_ref,
        budget_ref,
    })
}

fn parse_resource_contract_evidence(value: &Value<IOValue>) -> Result<ResourceContractEvidence> {
    let value = value_to_iovalue(value);
    let contract = simple_record(&value, "resource-contract", 3)?;
    let schema = required_string(&contract[0], "resource contract schema")?;
    if schema != HARNESS_BUDGET_CONTRACT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported resource contract schema {schema}; expected {HARNESS_BUDGET_CONTRACT_SCHEMA}"
        )));
    }
    let envelope_value = value_to_iovalue(&contract[1]);
    let envelope = parse_budget_contract_envelope(&envelope_value)?;
    let envelope_ref = required_record_hash(&contract[2], "envelope-ref", "resource contract envelope ref")?;
    let actual_envelope_ref = canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "resource contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt rejected resource contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(ResourceContractEvidence {
        envelope_ref,
        normalized_budget_ref: envelope.normalized_source_hash,
    })
}

fn parse_budget_contract_envelope(value: &IOValue) -> Result<basalt::ContractEnvelope> {
    let envelope = simple_record(value, "contract-envelope", 7)?;
    let backend = required_string(&envelope[0], "budget contract backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!("resource preflight requires Nickel backend, got {backend}")));
    }
    let contract_id = required_string(&envelope[1], "budget contract id")?;
    if contract_id != BUDGET_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract id {contract_id}; expected {BUDGET_CONTRACT_ID}"
        )));
    }
    let contract_version = required_string(&envelope[2], "budget contract version")?;
    if contract_version != BUDGET_CONTRACT_VERSION {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract version {contract_version}; expected {BUDGET_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = required_hash(&envelope[3], "budget contract normalized source ref")?;
    let input_schema = required_string(&envelope[4], "budget contract input schema")?;
    if input_schema != HARNESS_BUDGET_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract input schema {input_schema}; expected {HARNESS_BUDGET_SCHEMA}"
        )));
    }
    let output_schema = required_string(&envelope[5], "budget contract output schema")?;
    if output_schema != HARNESS_BUDGET_USAGE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract output schema {output_schema}; expected {HARNESS_BUDGET_USAGE_SCHEMA}"
        )));
    }
    let receipt_schema_version = required_string(&envelope[6], "budget contract receipt schema")?;
    if receipt_schema_version != HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract receipt schema {receipt_schema_version}; expected {HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA}"
        )));
    }
    Ok(basalt::ContractEnvelope::new(
        backend,
        contract_id,
        contract_version,
        normalized_source_hash,
        input_schema,
        output_schema,
        receipt_schema_version,
    ))
}

fn parse_basalt_resource_preflight_evidence(value: &Value<IOValue>) -> Result<BasaltResourcePreflightEvidence> {
    let value = value_to_iovalue(value);
    let receipt = simple_record(&value, "basalt-resource-preflight", 8)?;
    let schema = required_string(&receipt[0], "Basalt resource preflight schema")?;
    if schema != HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt resource preflight schema {schema}; expected {HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Basalt resource preflight decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt resource preflight decision {decision}")));
    }
    let backend = required_record_string(&receipt[2], "backend", "Basalt resource preflight backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt resource preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_record_string(&receipt[3], "contract-id", "Basalt resource preflight contract id")?;
    if contract_id != BUDGET_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt resource preflight contract id {contract_id}; expected {BUDGET_CONTRACT_ID}"
        )));
    }
    let envelope_ref = required_record_hash(&receipt[4], "envelope-ref", "Basalt resource preflight envelope ref")?;
    let budget_ref = required_record_hash(&receipt[5], "budget-ref", "Basalt resource preflight budget ref")?;
    let normalized_source_ref =
        required_record_hash(&receipt[6], "normalized-source-ref", "Basalt resource preflight source ref")?;
    let reason = required_record_string(&receipt[7], "reason", "Basalt resource preflight reason")?;
    if reason != "accepted" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt resource preflight reason {reason}")));
    }
    Ok(BasaltResourcePreflightEvidence {
        receipt_ref: canonical_hash(&value)?,
        envelope_ref,
        budget_ref,
        normalized_source_ref,
    })
}

fn nickel_budget_source(budget: &HarnessBudget, budget_ref: &str) -> String {
    format!(
        "{{\n  schema_version = {},\n  budget_schema = {},\n  budget_ref = {},\n  limits = {{\n    max_steps = {},\n    max_effects = {},\n    max_events = {},\n    max_report_bytes = {},\n  }},\n}}",
        nickel_string(HARNESS_BUDGET_NICKEL_STATIC_SCHEMA),
        nickel_string(HARNESS_BUDGET_SCHEMA),
        nickel_string(budget_ref),
        budget.max_steps,
        budget.max_effects,
        budget.max_events,
        budget.max_report_bytes,
    )
}

fn budget_gate_checks_value() -> IOValue {
    record("checks", vec![sequence(
        [
            "budget-schema",
            "canonical-budget-snapshot",
            "explicit-budget-fixture",
            "no-default-resource-policy",
            "resource-policy-preflight",
            "nickel-resource-policy",
            "nickel-resource-export",
            "basalt-resource-preflight",
            "basalt-resource-receipt",
            "budget-usage-binding",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_budget_gate_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "budget gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "budget gate check name")?;
        let status = required_string(&check[1], "budget gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("budget gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_budget_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("budget gate missing {expected} check")))
    }
}

pub fn validate_actor_registry_evidence(suite: &HarnessSuite, observations: &[HarnessObservation]) -> Result<()> {
    if !suite.actors_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit actor registry fixture; inferred actors cannot satisfy evidence gates",
        ));
    }
    let actor_ids = suite.actors.iter().map(|actor| actor.id.as_str()).collect::<BTreeSet<_>>();
    for step in &suite.steps {
        for actor in actor_ids_for_step(step) {
            require_declared_actor(&actor_ids, actor, "suite step", None)?;
        }
    }
    for (position, observation) in observations.iter().enumerate() {
        for event in &observation.events {
            for actor in actor_ids_for_event(event)? {
                require_declared_actor(&actor_ids, &actor, "observation event", Some(position))?;
            }
        }
    }
    Ok(())
}

pub fn validate_admission_evidence(
    suite: &HarnessSuite,
    observations: &[HarnessObservation],
    capability_gate: &CapabilityGateEvidence,
) -> Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(MoltenError::invalid_harness(format!(
            "admission evidence observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }

    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        if observation.events.is_empty() {
            return Err(MoltenError::invalid_harness(format!("missing admission decision at observation {position}")));
        }
        if event_boundary(&observation.events[0]) != EventBoundary::PolicyDecision {
            return Err(MoltenError::invalid_harness(format!(
                "missing admission decision at observation {position}; first event is not admission-decision-v1"
            )));
        }
        let mut decision_count = 0usize;
        for event in &observation.events {
            if event_boundary(event) == EventBoundary::PolicyDecision {
                decision_count += 1;
            }
        }
        if decision_count != 1 {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate admission decision at observation {position}: got {decision_count} decisions"
            )));
        }

        let recorded = parse_admission_decision_event(&observation.events[0])?;
        let expected_request = AdmissionRequest::from_step(step);
        if recorded.request != expected_request {
            return Err(MoltenError::invalid_harness(format!("admission request mismatch at observation {position}")));
        }
        let expected_authority = admission_authority_evidence(&suite.capabilities, &expected_request)?;
        let recorded_authority = recorded.authority.as_ref().ok_or_else(|| {
            MoltenError::invalid_harness(format!("missing capability authority evidence at observation {position}"))
        })?;
        if recorded_authority != &expected_authority {
            return Err(MoltenError::invalid_harness(format!(
                "capability authority mismatch at observation {position}"
            )));
        }
        if recorded_authority.capability_ref != capability_gate.capability_ref {
            return Err(MoltenError::invalid_harness(format!(
                "capability authority preflight ref mismatch at observation {position}"
            )));
        }
        let preflight_grant_refs: &[String] = capability_gate.grant_refs.as_slice();
        if let Some(grant_ref) = recorded_authority.grant_ref.as_deref()
            && !preflight_grant_refs.iter().any(|preflight_ref| preflight_ref.as_str() == grant_ref)
        {
            return Err(MoltenError::invalid_harness(format!(
                "capability grant ref at observation {position} is not bound by authority preflight"
            )));
        }
        let expected_decision = suite.policy.decide_with_capabilities(&suite.capabilities, &expected_request);
        if recorded.decision != expected_decision {
            return Err(MoltenError::invalid_harness(format!("admission decision mismatch at observation {position}")));
        }
        if !recorded.decision.is_allowed() {
            validate_denied_observation_events(position, &observation.events[1..])?;
        }
    }
    Ok(())
}

pub fn validate_runtime_predicate_evidence(suite: &HarnessSuite, observations: &[HarnessObservation]) -> Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(MoltenError::invalid_harness(format!(
            "runtime predicate observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }

    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        let admission = observation
            .events
            .first()
            .ok_or_else(|| {
                MoltenError::invalid_harness(format!("missing admission decision at observation {position}"))
            })
            .and_then(parse_admission_decision_event)?;
        let mut runtime_predicates = Vec::with_capacity(observation.events.as_slice().len());
        for event in observation.events.as_slice() {
            if event_boundary(event) == EventBoundary::RuntimePredicate {
                runtime_predicates.push(parse_runtime_predicate_receipt(event)?);
            }
        }
        let expected = expected_runtime_predicates(step, &admission.decision);
        for predicate in &runtime_predicates {
            if !expected.as_slice().iter().any(|expected_predicate| expected_predicate == &predicate.as_str()) {
                return Err(MoltenError::invalid_harness(format!(
                    "unexpected runtime predicate {predicate} at observation {position}"
                )));
            }
        }
        for expected_predicate in expected {
            let count = runtime_predicates
                .as_slice()
                .iter()
                .filter(|predicate| predicate.as_str() == expected_predicate)
                .count();
            if count != 1 {
                return Err(MoltenError::invalid_harness(format!(
                    "runtime predicate {expected_predicate} at observation {position} expected exactly one receipt, got {count}"
                )));
            }
        }
    }
    Ok(())
}

fn expected_runtime_predicates(step: &CoreStep, decision: &AdmissionDecision) -> Vec<&'static str> {
    let mut expected = Vec::with_capacity(2);
    if !decision.is_allowed()
        || matches!(
            step,
            CoreStep::Send { .. } | CoreStep::Observe { .. } | CoreStep::Assert { .. } | CoreStep::Retract { .. }
        )
    {
        expected.push(TURN_COMMIT_ROLLBACK_PREDICATE);
    }
    match step {
        CoreStep::Observe { .. } => expected.push(OBSERVE_DELIVERY_PREDICATE),
        CoreStep::Assert { .. } | CoreStep::Retract { .. } => expected.push(ASSERTION_VISIBILITY_PREDICATE),
        CoreStep::Send { .. } | CoreStep::Clock { .. } | CoreStep::Random { .. } => {}
    }
    expected
}

fn parse_runtime_predicate_receipt(value: &IOValue) -> Result<String> {
    let receipt = value
        .collect_simple_record("runtime-predicate-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <runtime-predicate-receipt-v1 ...>"))?;
    let schema = required_string(&receipt[0], "runtime predicate receipt schema")?;
    if schema != RUNTIME_PREDICATE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate receipt schema {schema}")));
    }
    let predicate = required_string(&receipt[1], "runtime predicate name")?;
    if !matches!(
        predicate.as_str(),
        TURN_COMMIT_ROLLBACK_PREDICATE
            | ASSERTION_VISIBILITY_PREDICATE
            | OBSERVE_DELIVERY_PREDICATE
            | PRESERVES_PATTERN_PREDICATE
            | PROMISE_STATE_PREDICATE
            | PROMISE_PIPELINE_PREDICATE
            | REVOCATION_CLEANUP_PREDICATE
            | ACTORMAP_TRANSACTION_PREDICATE
            | NEAR_FAR_REFS_PREDICATE
            | SNAPSHOT_AUTHORITY_PREDICATE
            | SERVICE_DEPENDENCIES_PREDICATE
    ) {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported runtime predicate receipt predicate {predicate}"
        )));
    }
    let engine = required_string(&receipt[2], "runtime predicate engine")?;
    if engine != RUNTIME_PREDICATE_ENGINE {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate engine {engine}")));
    }
    required_record_hash(&receipt[3], "input-ref", "runtime predicate input ref")?;
    let decision = required_string(&receipt[4], "runtime predicate decision")?;
    if !matches!(decision.as_str(), "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate decision {decision}")));
    }
    let state_refs = sequence_strings(&receipt[5], "runtime predicate state refs")?;
    if state_refs.is_empty() {
        return Err(MoltenError::invalid_harness("runtime predicate receipt missing state refs"));
    }
    for state_ref in &state_refs {
        validate_content_ref(state_ref)?;
    }
    let checks = sequence_strings(&receipt[6], "runtime predicate checks")?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("runtime predicate receipt missing checks"));
    }
    sequence_strings(&receipt[7], "runtime predicate diagnostics")?;
    Ok(predicate)
}

fn sequence_strings(value: &Value<IOValue>, field: &str) -> Result<Vec<String>> {
    let values = required_sequence(value, field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

pub fn validate_hostcall_evidence(
    suite: &HarnessSuite,
    observations: &[HarnessObservation],
    policy_gate: &PolicyGateEvidence,
    capability_gate: &CapabilityGateEvidence,
    budget_gate: &BudgetGateEvidence,
) -> Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(MoltenError::invalid_harness(format!(
            "hostcall evidence observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }

    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        if observation.events.len() < 5 {
            return Err(MoltenError::invalid_harness(format!(
                "missing executor hostcall boundary evidence at observation {position}"
            )));
        }
        let step_ref = canonical_hash(&step_value(step))?;
        if observation.step_ref != step_ref {
            return Err(MoltenError::invalid_harness(format!("hostcall step ref mismatch at observation {position}")));
        }
        let actor_output_index = if observation.events.last().is_some_and(is_turn_journal) {
            observation.events.len() - 2
        } else {
            observation.events.len() - 1
        };
        let Some(actor_output_event) = observation.events.as_slice().get(actor_output_index) else {
            return Err(MoltenError::invalid_harness(format!("missing final actor output at observation {position}")));
        };
        if event_boundary(&observation.events[1]) != EventBoundary::ActorInput
            || event_boundary(&observation.events[2]) != EventBoundary::HostcallRequest
            || event_boundary(&observation.events[3]) != EventBoundary::HostcallDecision
            || event_boundary(actor_output_event) != EventBoundary::ActorOutput
        {
            return Err(MoltenError::invalid_harness(format!(
                "executor hostcall boundary order mismatch at observation {position}"
            )));
        }
        let admission = parse_admission_decision_event(&observation.events[0])?;
        let authority = admission.authority.as_ref().ok_or_else(|| {
            MoltenError::invalid_harness(format!(
                "missing capability authority evidence for hostcall decision at observation {position}"
            ))
        })?;
        let suite_ref = suite_ref(suite)?;
        let hostcall_context = HostcallEvidenceContext {
            sequence: position as u64,
            suite_ref: &suite_ref,
            step_ref: &step_ref,
            policy_ref: &policy_gate.policy_ref,
            capability_ref: &capability_gate.capability_ref,
            budget_ref: &budget_gate.budget_ref,
        };
        let expected_input = actor_input_value(suite, step, hostcall_context)?;
        require_hostcall_event(position, "actor-input", &observation.events[1], &expected_input)?;
        let expected_request = hostcall_request_value(suite, step, hostcall_context, &admission.decision)?;
        require_hostcall_event(position, "hostcall-request", &observation.events[2], &expected_request)?;
        let expected_decision =
            hostcall_decision_value(hostcall_context, &observation.events[0], authority, &admission.decision)?;
        require_hostcall_event(position, "hostcall-decision", &observation.events[3], &expected_decision)?;
        let runtime_events = &observation.events[4..actor_output_index];
        validate_steel_execution_evidence(suite, step, position, &admission.decision, runtime_events)?;
        validate_wasm_execution_evidence(&WasmExecutionEvidenceInput {
            suite,
            step,
            position,
            decision: &admission.decision,
            actor_input: &observation.events[1],
            runtime_events,
        })?;
        let expected_output = actor_output_value(step, hostcall_context, &admission.decision, runtime_events)?;
        require_hostcall_event(position, "actor-output", actor_output_event, &expected_output)?;
    }
    Ok(())
}

struct WasmExecutionEvidenceInput<'a> {
    suite: &'a HarnessSuite,
    step: &'a CoreStep,
    position: usize,
    decision: &'a AdmissionDecision,
    actor_input: &'a IOValue,
    runtime_events: &'a [IOValue],
}

fn validate_steel_execution_evidence(
    suite: &HarnessSuite,
    step: &CoreStep,
    position: usize,
    decision: &AdmissionDecision,
    runtime_events: &[IOValue],
) -> Result<()> {
    let actor = step.primary_actor();
    let decl = actor_decl_for_primary_actor(suite, actor)?;
    if decl.kind != ActorKind::Steel {
        return Ok(());
    }
    if !decision.is_allowed() {
        if runtime_events.iter().any(|event| event_boundary(event) == EventBoundary::SteelExecution) {
            return Err(MoltenError::invalid_harness(format!(
                "denied Steel step at observation {position} must not carry Steel execution evidence"
            )));
        }
        return Ok(());
    }
    let Some(receipt) = runtime_events.first() else {
        return Err(MoltenError::invalid_harness(format!(
            "missing Steel execution evidence at observation {position}"
        )));
    };
    validate_steel_execution_receipt(decl, step, position, receipt)
}

fn validate_steel_execution_receipt(
    actor: &ActorDecl,
    step: &CoreStep,
    position: usize,
    value: &IOValue,
) -> Result<()> {
    let receipt_value = value
        .collect_simple_record("steel-execution-receipt-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <steel-execution-receipt-v1 ...>"))?;
    let arity = receipt_value.fields_iter().count();
    if arity != 9 && arity != 10 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <steel-execution-receipt-v1 ...> with arity 9 or 10, got {arity}"
        )));
    }
    let receipt = &receipt_value;
    let schema = required_string(&receipt[0], "Steel execution receipt schema")?;
    if schema != RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Steel execution receipt schema {schema}; expected {RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA}"
        )));
    }
    let actor_id = required_record_string(&receipt[1], "actor", "Steel execution actor")?;
    if actor_id != actor.id {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution actor mismatch at observation {position}: got {actor_id}, expected {}",
            actor.id
        )));
    }
    let ActorExecutorConfig::Steel(config) = actor.executor.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness(format!("Steel actor {} missing Steel executor config", actor.id))
    })?
    else {
        return Err(MoltenError::invalid_harness(format!("Steel actor {} has non-Steel executor config", actor.id)));
    };
    let source_ref = required_record_hash(&receipt[2], "source-ref", "Steel execution source ref")?;
    if source_ref != steel_source_ref(config)? {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution source ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let callable = required_record_string(&receipt[3], "callable", "Steel execution callable")?;
    if callable != config.callable {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution callable mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let operation = AdmissionRequest::from_step(step).action.as_str().to_string();
    let receipt_operation = required_record_string(&receipt[4], "operation", "Steel execution operation")?;
    if receipt_operation != operation {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution operation mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    required_record_hash(&receipt[5], "input-ref", "Steel execution input ref")?;
    required_record_hash(&receipt[6], "output-ref", "Steel execution output ref")?;
    let hostcalls = required_record_string_sequence(&receipt[7], "hostcalls", "Steel execution hostcalls")?;
    if hostcalls != vec![operation] {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution hostcalls mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let checks_index = if arity == 10 {
        let resources_value = value_to_iovalue(&receipt[8]);
        let resources = simple_record(&resources_value, "resources", 5)?;
        let fuel_value = value_to_iovalue(&resources[0]);
        let fuel = simple_record(&fuel_value, "fuel", 2)?;
        let fuel_limit = required_u64(&fuel[0], "Steel execution fuel limit")?;
        let fuel_remaining = required_u64(&fuel[1], "Steel execution fuel remaining")?;
        if fuel_remaining > fuel_limit {
            return Err(MoltenError::invalid_harness("Steel execution remaining fuel exceeds limit"));
        }
        required_record_u64(&resources[1], "source-bytes", "Steel execution source byte count")?;
        required_record_u64(&resources[2], "input-bytes", "Steel execution input byte count")?;
        required_record_u64(&resources[3], "output-bytes", "Steel execution output byte count")?;
        let hostcalls_value = value_to_iovalue(&resources[4]);
        let hostcalls_record = simple_record(&hostcalls_value, "hostcalls", 2)?;
        let hostcall_limit = required_u64(&hostcalls_record[0], "Steel execution hostcall limit")?;
        let hostcall_count = required_u64(&hostcalls_record[1], "Steel execution hostcall count")?;
        if hostcall_count > hostcall_limit {
            return Err(MoltenError::invalid_harness("Steel execution hostcall count exceeds limit"));
        }
        9
    } else {
        8
    };
    let checks = parse_executor_preflight_checks(&receipt[checks_index])?;
    require_executor_preflight_check(&checks, "steel-vm-executed")?;
    require_executor_preflight_check(&checks, "reviewed-callable-binding")?;
    require_executor_preflight_check(&checks, "canonical-preserves-input")?;
    require_executor_preflight_check(&checks, "canonical-preserves-output")?;
    require_executor_preflight_check(&checks, "no-ambient-steel-io")?;
    require_executor_preflight_check(&checks, "hostcall-envelope-binding")?;
    if arity == 10 {
        require_executor_preflight_check(&checks, "resource-bounded")?;
        require_executor_preflight_check(&checks, "fuel-bounded")?;
        require_executor_preflight_check(&checks, "hostcall-count-bounded")?;
        require_executor_preflight_check(&checks, "io-bytes-bounded")?;
    }
    Ok(())
}

fn validate_wasm_execution_evidence(input: &WasmExecutionEvidenceInput<'_>) -> Result<()> {
    let actor = input.step.primary_actor();
    let decl = actor_decl_for_primary_actor(input.suite, actor)?;
    if decl.kind != ActorKind::Wasm {
        return Ok(());
    }
    if !input.decision.is_allowed() {
        if input.runtime_events.iter().any(|event| event_boundary(event) == EventBoundary::WasmExecution) {
            return Err(MoltenError::invalid_harness(format!(
                "denied Wasm step at observation {} must not carry Wasm execution evidence",
                input.position
            )));
        }
        return Ok(());
    }
    let Some(receipt) = input.runtime_events.first() else {
        return Err(MoltenError::invalid_harness(format!(
            "missing Wasm execution evidence at observation {}",
            input.position
        )));
    };
    validate_wasm_execution_receipt(decl, input.step, input.position, input.actor_input, receipt)
}

fn validate_wasm_execution_receipt(
    actor: &ActorDecl,
    step: &CoreStep,
    position: usize,
    actor_input: &IOValue,
    value: &IOValue,
) -> Result<()> {
    let receipt_value = value
        .collect_simple_record("wasm-execution-receipt-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <wasm-execution-receipt-v1 ...>"))?;
    let arity = receipt_value.fields_iter().count();
    if arity != 9 && arity != 13 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <wasm-execution-receipt-v1 ...> with arity 9 or 13, got {arity}"
        )));
    }
    let receipt = &receipt_value;
    let schema = required_string(&receipt[0], "Wasm execution receipt schema")?;
    if schema != RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm execution receipt schema {schema}; expected {RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA}"
        )));
    }
    let actor_id = required_record_string(&receipt[1], "actor", "Wasm execution actor")?;
    if actor_id != actor.id {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution actor mismatch at observation {position}: got {actor_id}, expected {}",
            actor.id
        )));
    }
    let ActorExecutorConfig::Wasm(config) = actor
        .executor
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness(format!("Wasm actor {} missing Wasm executor config", actor.id)))?
    else {
        return Err(MoltenError::invalid_harness(format!("Wasm actor {} has non-Wasm executor config", actor.id)));
    };
    let module_ref = required_record_hash(&receipt[2], "module-ref", "Wasm execution module ref")?;
    if module_ref != wasm_module_ref(config)? {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution module ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let operation = AdmissionRequest::from_step(step).action.as_str().to_string();
    let expected_export = wasm_executor_export_name(&operation);
    let export = required_record_string(&receipt[3], "export", "Wasm execution export")?;
    if export != expected_export {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution export mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let receipt_operation = required_record_string(&receipt[4], "operation", "Wasm execution operation")?;
    if receipt_operation != operation {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution operation mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let hostcalls = required_record_string_sequence(&receipt[5], "hostcalls", "Wasm execution hostcalls")?;
    if hostcalls != vec![operation] {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution hostcalls mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let fuel_value = value_to_iovalue(&receipt[6]);
    let fuel = simple_record(&fuel_value, "fuel", 2)?;
    let fuel_limit = required_u64(&fuel[0], "Wasm execution fuel limit")?;
    let fuel_remaining = required_u64(&fuel[1], "Wasm execution fuel remaining")?;
    if fuel_remaining > fuel_limit {
        return Err(MoltenError::invalid_harness("Wasm execution remaining fuel exceeds limit"));
    }
    let memory_value = value_to_iovalue(&receipt[7]);
    let memory = simple_record(&memory_value, "memory-limit", 1)?;
    required_u64(&memory[0], "Wasm execution memory limit")?;
    let checks_index = if arity == 13 {
        let abi = required_record_string(&receipt[8], "abi", "Wasm execution ABI schema")?;
        if abi != RUNTIME_WASM_ABI_SCHEMA {
            return Err(MoltenError::invalid_harness(format!(
                "unsupported Wasm execution ABI schema {abi}; expected {RUNTIME_WASM_ABI_SCHEMA}"
            )));
        }
        let input_ref = required_record_hash(&receipt[9], "input-ref", "Wasm execution input ref")?;
        let expected_input_ref = canonical_hash(actor_input)?;
        if input_ref != expected_input_ref {
            return Err(MoltenError::invalid_harness(format!(
                "Wasm execution input ref mismatch for actor {} at observation {position}",
                actor.id
            )));
        }
        required_record_hash(&receipt[10], "output-ref", "Wasm execution output ref")?;
        let output_bytes = required_record_u64(&receipt[11], "output-bytes", "Wasm execution output byte count")?;
        if output_bytes > WASM_ABI_MAX_OUTPUT_BYTES_FOR_VALIDATION {
            return Err(MoltenError::invalid_harness(format!(
                "Wasm execution output byte count exceeds molten.wasm.abi.v1 limit for actor {} at observation {position}",
                actor.id
            )));
        }
        12
    } else {
        8
    };
    let checks = parse_executor_preflight_checks(&receipt[checks_index])?;
    if arity == 13 {
        require_executor_preflight_check(&checks, "preserves-abi-v1")?;
        require_executor_preflight_check(&checks, "canonical-preserves-input")?;
        require_executor_preflight_check(&checks, "canonical-preserves-output")?;
        require_executor_preflight_check(&checks, "guest-memory-bounds")?;
    }
    require_executor_preflight_check(&checks, "wasmtime-instantiated")?;
    require_executor_preflight_check(&checks, "no-wasi")?;
    require_executor_preflight_check(&checks, "fuel-bounded")?;
    require_executor_preflight_check(&checks, "memory-bounded")?;
    require_executor_preflight_check(&checks, "hostcall-envelope-binding")?;
    Ok(())
}

fn require_hostcall_event(position: usize, kind: &str, actual: &IOValue, expected: &IOValue) -> Result<()> {
    if actual == expected {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!(
        "{kind} evidence mismatch at observation {position}: got {}, expected {}",
        canonical_hash(actual)?,
        canonical_hash(expected)?
    )))
}

pub fn parse_admission_decision_event(value: &IOValue) -> Result<AdmissionDecisionEvent> {
    let admission = value
        .collect_simple_record("admission-decision-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <admission-decision-v1 ...>"))?;
    let arity = admission.fields_iter().count();
    if arity != 3 && arity != 4 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <admission-decision-v1 ...> with arity 3 or 4, got {arity}"
        )));
    }
    let schema = required_string(&admission[0], "admission decision schema")?;
    if schema != RUNTIME_ADMISSION_DECISION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported admission decision schema {schema}; expected {RUNTIME_ADMISSION_DECISION_SCHEMA}"
        )));
    }
    let request = parse_admission_request(&admission[1])?;
    let (authority, decision_index) = if arity == 4 {
        (Some(parse_admission_authority(&admission[2])?), 3)
    } else {
        (None, 2)
    };
    let decision = parse_admission_decision(&admission[decision_index])?;
    Ok(AdmissionDecisionEvent {
        value: value.clone(),
        request,
        authority,
        decision,
    })
}

pub fn report_suite_value(report_value: &IOValue) -> Result<IOValue> {
    Ok(parse_report(report_value)?.suite_value)
}

pub fn boundary_coverage_value(report_value: &IOValue) -> Result<IOValue> {
    let report = parse_report(report_value)?;
    let suite = parse_suite(&report.suite_value)?;
    let mut coverage = Vec::new();
    push_boundary_coverage(&mut coverage, "envelope-routes", suite.steps.iter().any(|step| matches!(step, CoreStep::Send { .. })));
    push_boundary_coverage(
        &mut coverage,
        "dataspace-semantics",
        suite.steps.iter().any(|step| {
            matches!(step, CoreStep::Observe { .. } | CoreStep::Assert { .. } | CoreStep::Retract { .. })
        }),
    );
    push_boundary_coverage(&mut coverage, "policy-gates", report.policy_gate.is_some() && report.capability_gate.is_some());
    push_boundary_coverage(&mut coverage, "policy-denials", report_has_denied_admission(&report)?);
    push_boundary_coverage(&mut coverage, "effects", !report.effect_log.is_empty());
    push_boundary_coverage(
        &mut coverage,
        "receipts",
        report.policy_gate.is_some() && report.capability_gate.is_some() && report.budget_gate.is_some(),
    );
    push_boundary_coverage(&mut coverage, "traces", !report.observations.is_empty());
    push_boundary_coverage(&mut coverage, "storage-paths", false);
    push_boundary_coverage(&mut coverage, "resources", report.budget_gate.is_some());
    push_boundary_coverage(
        &mut coverage,
        "replay-branches",
        matches!(report.replay_status.as_str(), "deterministic" | "replay" | "record"),
    );
    push_boundary_coverage(
        &mut coverage,
        "adapters",
        report.actors.iter().any(|actor| !matches!(actor.kind, ActorKind::Native)),
    );
    push_boundary_coverage(&mut coverage, "confidentiality-paths", report_value_contains_label(report_value, "redaction-gate-v1"));

    let unexercised = coverage
        .iter()
        .filter_map(|value| {
            let fields = value.collect_simple_record("boundary", Some(2))?;
            let name = required_string(&fields[0], "coverage boundary name").ok()?;
            let status = required_string(&fields[1], "coverage boundary status").ok()?;
            (status == "unexercised").then_some(string(name))
        })
        .collect::<Vec<_>>();

    Ok(record("harness-boundary-coverage-v1", vec![
        string("molten.harness.boundary-coverage.v1"),
        record("report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        sequence(coverage),
        record("unexercised", vec![sequence(unexercised)]),
    ]))
}

fn push_boundary_coverage(out: &mut Vec<IOValue>, name: &str, exercised: bool) {
    out.push(record("boundary", vec![
        string(name),
        string(if exercised { "exercised" } else { "unexercised" }),
    ]));
}

fn report_has_denied_admission(report: &HarnessReport) -> Result<bool> {
    for observation in &report.observations {
        for event in &observation.events {
            if event.collect_simple_record("admission-decision-v1", None).is_some()
                && !parse_admission_decision_event(event)?.decision.is_allowed()
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn report_value_contains_label(value: &IOValue, label: &str) -> bool {
    to_text(value).is_ok_and(|text| text.contains(label))
}

pub fn repro_bundle_value(report_value: &IOValue) -> Result<IOValue> {
    repro_bundle_value_with_command(report_value, &default_report_bundle_command())
}

pub fn repro_bundle_value_with_command(report_value: &IOValue, command: &[String]) -> Result<IOValue> {
    let report = parse_report(report_value)?;
    Ok(record("harness-repro-bundle-v1", vec![
        string(HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "validate", "report.preserves"][..],
            &["molten", "test", "replay", "report.preserves"][..],
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "gate", "check", "refs.preserves"][..],
        ]),
        report_artifact_refs_value(&report, None, None)?,
        string(&report.report_ref),
        string(&report.suite_ref),
        string(&report.initial_state_hash),
        string(&report.final_state_hash),
        string(&report.replay_status),
        string(&report.profile),
        actor_registry_value(&report.actors),
        effect_log_value(&report.effect_log),
        report.suite_value,
        report_value.clone(),
    ]))
}

pub fn sealed_repro_bundle_value_with_command_and_receipt(
    report_value: &IOValue,
    command: &[String],
    gate_receipt_value: &IOValue,
) -> Result<IOValue> {
    let report = parse_report(report_value)?;
    let gate_receipt_ref = canonical_hash(gate_receipt_value)?;
    let redaction_policy = redaction_policy_value();
    let redaction_policy_ref = canonical_hash(&redaction_policy)?;
    let redaction_gate = redaction_gate_value(report_value, &report)?;
    let redaction_gate_ref = canonical_hash(&redaction_gate)?;
    let seal = repro_seal_value(&report, &gate_receipt_ref);
    let sealed_checks = sealed_repro_checks_value();
    Ok(record("harness-repro-bundle-v1", vec![
        string(HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "validate", "report.preserves"][..],
            &["molten", "test", "replay", "report.preserves"][..],
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "gate", "check", "refs.preserves"][..],
        ]),
        report_artifact_refs_value(
            &report,
            Some(&gate_receipt_ref),
            Some((&redaction_policy_ref, &redaction_gate_ref)),
        )?,
        string(&report.report_ref),
        string(&report.suite_ref),
        string(&report.initial_state_hash),
        string(&report.final_state_hash),
        string(&report.replay_status),
        string(&report.profile),
        actor_registry_value(&report.actors),
        effect_log_value(&report.effect_log),
        report.suite_value,
        report_value.clone(),
        redaction_policy,
        redaction_gate,
        seal,
        gate_receipt_value.clone(),
        sealed_checks,
    ]))
}

pub fn profiled_repro_bundle_value_with_command(
    report_value: &IOValue,
    command: &[String],
    profile: ReproExportProfile,
) -> Result<IOValue> {
    if profile == ReproExportProfile::DenySensitive {
        return Err(MoltenError::invalid_harness(
            "deny-sensitive repro export must use sealed pass bundle construction",
        ));
    }
    let source_report = parse_report(report_value)?;
    let policy_value = redaction_policy_value();
    let policy_ref = canonical_hash(&policy_value)?;
    let transform = redacted_report_for_profile(report_value, &source_report, profile, &policy_ref)?;
    let output_report = parse_report(&transform.report_value)?;
    let export_profile_value = repro_export_profile_value(profile);
    let export_profile_ref = canonical_hash(&export_profile_value)?;
    let mut artifact_refs = report_artifact_refs(&output_report, None, None)?;
    artifact_refs.push(("source-report".to_string(), source_report.report_ref.clone()));
    artifact_refs.push(("source-suite".to_string(), source_report.suite_ref.clone()));
    artifact_refs.push(("redaction-policy".to_string(), policy_ref.clone()));
    artifact_refs.push(("export-profile".to_string(), export_profile_ref));
    artifact_refs.push(("redaction-transform-manifest".to_string(), transform.manifest_ref.clone()));
    artifact_refs.push(("redaction-transform".to_string(), transform.receipt_ref.clone()));
    let private_profile_value = if profile == ReproExportProfile::EncryptedPrivate {
        let value = private_bundle_profile_value(&PrivateBundleProfileInput {
            profile_ref: canonical_hash(&export_profile_value)?,
            encrypted_refs: transform.encrypted_refs.clone(),
            reveal_receipt_refs: Vec::new(),
            transform_receipt_ref: transform.receipt_ref.clone(),
            is_gate_preserving: false,
        })?;
        artifact_refs.push(("private-bundle-profile".to_string(), canonical_hash(&value)?));
        Some(value)
    } else {
        None
    };
    let mut fields = vec![
        string(HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("report")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "show", "report.preserves"][..],
            &["molten", "test", "repro", "unpack", "refs.preserves"][..],
            &["molten", "test", "repro", "verify", "refs.preserves"][..],
        ]),
        artifact_refs_owned_value(&artifact_refs),
        string(&source_report.report_ref),
        string(&source_report.suite_ref),
        string(&output_report.report_ref),
        string(&output_report.suite_ref),
        string(&output_report.initial_state_hash),
        string(&output_report.final_state_hash),
        string(&output_report.replay_status),
        string(&output_report.profile),
        export_profile_value,
        actor_registry_value(&output_report.actors),
        effect_log_value(&output_report.effect_log),
        output_report.suite_value,
        transform.report_value,
        policy_value,
        transform.manifest_value,
        transform.receipt_value,
    ];
    if let Some(private_profile_value) = private_profile_value {
        fields.push(private_profile_value);
    }
    fields.push(profiled_repro_checks_value(profile));
    Ok(record("harness-repro-bundle-v1", fields))
}

struct ProfiledTransformOutput {
    report_value: IOValue,
    manifest_ref: String,
    manifest_value: IOValue,
    receipt_ref: String,
    receipt_value: IOValue,
    encrypted_refs: Vec<String>,
}

struct RedactionTransformState {
    profile: ReproExportProfile,
    policy_ref: String,
    marker_refs: Vec<String>,
    marker_entries: Vec<RedactionManifestEntry>,
    encrypted_refs: Vec<String>,
}

struct RedactionManifestEntry {
    path: String,
    reason: String,
    commitment_ref: String,
    marker_ref: Option<String>,
    encrypted_ref: Option<String>,
}

fn redacted_report_for_profile(
    report_value: &IOValue,
    report: &HarnessReport,
    profile: ReproExportProfile,
    policy_ref: &str,
) -> Result<ProfiledTransformOutput> {
    let mut state = RedactionTransformState {
        profile,
        policy_ref: policy_ref.to_string(),
        marker_refs: Vec::new(),
        marker_entries: Vec::new(),
        encrypted_refs: Vec::new(),
    };
    let redacted_report_value = rebind_report_suite_ref(&transform_sensitive_value(report_value, "/", &mut state)?)?;
    state.marker_refs.sort();
    state.marker_refs.dedup();
    state.encrypted_refs.sort();
    state.encrypted_refs.dedup();
    let redacted_report = parse_report(&redacted_report_value)?;
    validate_profiled_output(&redacted_report_value, profile)?;
    let manifest_value = redaction_transform_manifest_value(
        report,
        &redacted_report,
        profile,
        &state.marker_entries,
        &state.encrypted_refs,
    );
    let manifest_ref = canonical_hash(&manifest_value)?;
    let receipt_value = redaction_transform_receipt_value(&RedactionTransformReceiptInput {
        source_report_ref: &report.report_ref,
        source_suite_ref: &report.suite_ref,
        policy_ref,
        profile,
        manifest_ref: &manifest_ref,
        output_bundle_ref: &redacted_report.report_ref,
        marker_refs: &state.marker_refs,
        encrypted_refs: &state.encrypted_refs,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(ProfiledTransformOutput {
        report_value: redacted_report_value,
        manifest_ref,
        manifest_value,
        receipt_ref,
        receipt_value,
        encrypted_refs: state.encrypted_refs,
    })
}

fn rebind_report_suite_ref(report_value: &IOValue) -> Result<IOValue> {
    let report = simple_record(report_value, "harness-report-v1", 17)
        .or_else(|_| simple_record(report_value, "harness-report-v1", 16))
        .or_else(|_| simple_record(report_value, "harness-report-v1", 15))
        .or_else(|_| simple_record(report_value, "harness-report-v1", 14))
        .or_else(|_| simple_record(report_value, "harness-report-v1", 13))?;
    let suite_value = value_to_iovalue(&report[8]);
    let suite_ref = canonical_hash(&suite_value)?;
    let field_count = report.fields_iter().count();
    let mut fields = Vec::with_capacity(field_count);
    for (index, field) in report.fields_iter().enumerate() {
        if index == 5 {
            fields.push(string(&suite_ref));
        } else {
            fields.push(value_to_iovalue(field));
        }
    }
    Ok(record("harness-report-v1", fields))
}

enum RedactionTraversalFrame {
    Enter {
        value: IOValue,
        path: String,
    },
    ExitRecord {
        original: IOValue,
        label: IOValue,
        field_count: usize,
    },
    ExitSequence {
        original: IOValue,
        item_count: usize,
    },
}

fn ensure_redaction_bound(count: usize, limit: usize, context: &str) -> Result<()> {
    if count > limit {
        return Err(MoltenError::invalid_harness(format!("{context} exceeds redaction transform bound {limit}")));
    }
    Ok(())
}

struct RedactionFrameStack {
    frames: Vec<RedactionTraversalFrame>,
}

impl RedactionFrameStack {
    fn new() -> Self {
        Self {
            frames: Vec::with_capacity(1),
        }
    }

    fn push(&mut self, frame: RedactionTraversalFrame) -> Result<()> {
        ensure_redaction_bound(self.frames.len() + 1, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal stack")?;
        self.frames.push(frame);
        Ok(())
    }

    fn pop(&mut self) -> Option<RedactionTraversalFrame> {
        self.frames.pop()
    }

    fn push_children(&mut self, child_entries: Vec<(IOValue, String)>) -> Result<()> {
        ensure_redaction_bound(
            self.frames.len() + child_entries.len(),
            MAX_REDACTION_TRANSFORM_NODES,
            "redaction traversal stack",
        )?;
        for (child_value, child_path) in child_entries.into_iter().rev() {
            self.push(RedactionTraversalFrame::Enter {
                value: child_value,
                path: child_path,
            })?;
        }
        Ok(())
    }
}

struct RedactionOutputStack {
    values: Vec<IOValue>,
}

impl RedactionOutputStack {
    fn new() -> Self {
        Self {
            values: Vec::with_capacity(1),
        }
    }

    fn push(&mut self, value: IOValue) -> Result<()> {
        ensure_redaction_bound(self.values.len() + 1, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal outputs")?;
        self.values.push(value);
        Ok(())
    }

    fn take(&mut self, count: usize) -> Result<Vec<IOValue>> {
        if self.values.len() < count {
            return Err(MoltenError::invalid_harness("redaction traversal stack underflow"));
        }
        Ok(self.values.split_off(self.values.len() - count))
    }

    fn finish(mut self) -> Result<IOValue> {
        if self.values.len() != 1 {
            return Err(MoltenError::invalid_harness("redaction traversal produced invalid output"));
        }
        self.values
            .pop()
            .ok_or_else(|| MoltenError::invalid_harness("redaction traversal produced no output"))
    }
}

fn bounded_redaction_child_count(value: &IOValue, context: &str) -> Result<usize> {
    let count = value.iter().count();
    ensure_redaction_bound(count, MAX_REDACTION_CONTAINER_ITEMS, context)?;
    Ok(count)
}

fn redaction_child_entries(value: &IOValue, path: &str, context: &str) -> Result<Vec<(IOValue, String)>> {
    let child_count = bounded_redaction_child_count(value, context)?;
    let mut entries = Vec::with_capacity(child_count);
    for (index, child) in value.iter().enumerate() {
        entries.push((value_to_iovalue(&child), format!("{path}/{index}")));
    }
    Ok(entries)
}

fn transform_sensitive_value(value: &IOValue, path: &str, state: &mut RedactionTransformState) -> Result<IOValue> {
    let mut stack = RedactionFrameStack::new();
    let mut outputs = RedactionOutputStack::new();
    stack.push(RedactionTraversalFrame::Enter {
        value: value.clone(),
        path: path.to_string(),
    })?;
    let mut visited_nodes = 0usize;
    while let Some(frame) = stack.pop() {
        visited_nodes += 1;
        ensure_redaction_bound(visited_nodes, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal visited nodes")?;
        match frame {
            RedactionTraversalFrame::Enter { value, path } => {
                if let Some(label) = record_label_string(&value)
                    && is_sensitive_record_label(&label)
                {
                    let redacted = transform_sensitive_record(&value, &label, &path, state)?;
                    outputs.push(redacted)?;
                    continue;
                }
                match value.value_class() {
                    ValueClass::Atomic(_) | ValueClass::Embedded => outputs.push(value)?,
                    ValueClass::Compound(CompoundClass::Record) => {
                        let label = value_to_iovalue(&value.label());
                        let child_entries = redaction_child_entries(&value, &path, "redaction record fields")?;
                        stack.push(RedactionTraversalFrame::ExitRecord {
                            original: value,
                            label,
                            field_count: child_entries.len(),
                        })?;
                        stack.push_children(child_entries)?;
                    }
                    ValueClass::Compound(CompoundClass::Sequence) => {
                        let child_entries = redaction_child_entries(&value, &path, "redaction sequence items")?;
                        stack.push(RedactionTraversalFrame::ExitSequence {
                            original: value,
                            item_count: child_entries.len(),
                        })?;
                        stack.push_children(child_entries)?;
                    }
                    ValueClass::Compound(CompoundClass::Set) | ValueClass::Compound(CompoundClass::Dictionary) => {
                        outputs.push(value)?;
                    }
                }
            }
            RedactionTraversalFrame::ExitRecord {
                original,
                label,
                field_count,
            } => {
                let fields = outputs.take(field_count)?;
                let rebuilt = IOValue::record(label, fields);
                if rebuilt == original {
                    outputs.push(original)?;
                } else {
                    outputs.push(rebuilt)?;
                }
            }
            RedactionTraversalFrame::ExitSequence { original, item_count } => {
                let values = outputs.take(item_count)?;
                let rebuilt = sequence(values);
                if rebuilt == original {
                    outputs.push(original)?;
                } else {
                    outputs.push(rebuilt)?;
                }
            }
        }
    }
    outputs.finish()
}

fn transform_sensitive_record(
    value: &IOValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> Result<IOValue> {
    if label == "encrypted-ref" {
        return Err(MoltenError::invalid_harness(
            "malformed encrypted-ref marker cannot be accepted into a repro export profile",
        ));
    }
    if label == "encrypted-ref-v1" {
        let encrypted = parse_encrypted_ref(value)?;
        if state.profile != ReproExportProfile::EncryptedPrivate {
            return redaction_marker_for_value(value, label, path, state);
        }
        state.encrypted_refs.push(encrypted.encrypted_ref);
        return Ok(value.clone());
    }
    match state.profile {
        ReproExportProfile::DenySensitive => Err(MoltenError::invalid_harness(format!(
            "redaction preflight found sensitive marker {label}; sealed pass repro bundles require explicit redaction before export"
        ))),
        ReproExportProfile::RedactedDiagnostic => redaction_marker_for_value(value, label, path, state),
        ReproExportProfile::EncryptedPrivate => encrypted_ref_for_value(value, label, path, state),
    }
}

fn redaction_marker_for_value(
    value: &IOValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> Result<IOValue> {
    let commitment_ref = canonical_hash(value)?;
    let path_ref = canonical_hash(&string(path))?;
    let receipt_ref = canonical_hash(&record("redaction-marker-seed", vec![
        string(&commitment_ref),
        string(&path_ref),
        string(&state.policy_ref),
    ]))?;
    let marker_value = redaction_marker_value(&RedactionMarkerInput {
        reason: label.to_string(),
        commitment_ref: commitment_ref.clone(),
        schema_ref: canonical_hash(&string(label))?,
        path_ref,
        policy_refs: vec![state.policy_ref.clone()],
        receipt_ref,
    })?;
    let marker = parse_redaction_marker(&marker_value)?;
    state.marker_refs.push(marker.marker_ref.clone());
    state.marker_entries.push(RedactionManifestEntry {
        path: path.to_string(),
        reason: label.to_string(),
        commitment_ref,
        marker_ref: Some(marker.marker_ref),
        encrypted_ref: None,
    });
    Ok(marker_value)
}

fn encrypted_ref_for_value(
    value: &IOValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> Result<IOValue> {
    let commitment_ref = canonical_hash(value)?;
    let ciphertext_ref =
        canonical_hash(&record("encrypted-redaction-ciphertext", vec![string(&commitment_ref), string(path)]))?;
    let encrypted_value = encrypted_ref_value(&EncryptedRefInput {
        ciphertext_ref,
        commitment_ref: commitment_ref.clone(),
        encryption_ref: canonical_hash(&repro_export_profile_value(state.profile))?,
        schema_ref: canonical_hash(&string(label))?,
        policy_refs: vec![state.policy_ref.clone()],
        evidence_refs: vec![canonical_hash(&string(path))?],
    })?;
    let encrypted = parse_encrypted_ref(&encrypted_value)?;
    state.encrypted_refs.push(encrypted.encrypted_ref.clone());
    state.marker_entries.push(RedactionManifestEntry {
        path: path.to_string(),
        reason: label.to_string(),
        commitment_ref,
        marker_ref: None,
        encrypted_ref: Some(encrypted.encrypted_ref),
    });
    Ok(encrypted_value)
}

fn record_label_string(value: &IOValue) -> Option<String> {
    if !value.is_record() {
        return None;
    }
    value.label().as_symbol().map(Cow::into_owned)
}

pub fn failure_repro_bundle_value(failure_value: &IOValue) -> Result<IOValue> {
    failure_repro_bundle_value_with_command(failure_value, &default_failure_bundle_command())
}

pub fn failure_repro_bundle_value_with_command(failure_value: &IOValue, command: &[String]) -> Result<IOValue> {
    let failure = parse_failure(failure_value)?;
    Ok(record("harness-repro-bundle-v1", vec![
        string(HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("failure")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "show", "failure.preserves"][..],
            &["molten", "test", "gate", "check", "failure.preserves"][..],
        ]),
        artifact_refs_value(&[("failure", failure.failure_ref.as_str())]),
        string(failure.failure_ref),
        failure_value.clone(),
    ]))
}

pub fn parse_repro_bundle(value: &IOValue) -> Result<HarnessReproBundle> {
    let bundle = value
        .collect_simple_record("harness-repro-bundle-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <harness-repro-bundle-v1 ...>"))?;
    let arity = bundle.fields_iter().count();
    let schema = required_string(&bundle[0], "repro bundle schema")?;
    if schema != HARNESS_REPRO_BUNDLE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro bundle schema {schema}; expected {HARNESS_REPRO_BUNDLE_SCHEMA}"
        )));
    }

    if arity == 11 {
        return parse_legacy_report_repro_bundle(value, &bundle);
    }
    if arity == 16 {
        return parse_report_repro_bundle(value, &bundle);
    }
    if arity == 19 || arity == 21 {
        return parse_sealed_report_repro_bundle(value, &bundle);
    }
    if arity == 23 || arity == 24 {
        return parse_profiled_report_repro_bundle(value, &bundle);
    }
    if arity == 8 {
        return parse_failure_repro_bundle(value, &bundle);
    }
    Err(MoltenError::invalid_harness(format!(
        "expected <harness-repro-bundle-v1 ...> with arity 8, 11, 16, 19, 21, 23, or 24, got {arity}"
    )))
}

pub fn repro_bundle_report_value(bundle_value: &IOValue) -> Result<IOValue> {
    let bundle = parse_repro_bundle(bundle_value)?;
    match (bundle.kind, bundle.report_value) {
        (HarnessReproBundleKind::Report, Some(report_value)) => Ok(report_value),
        (HarnessReproBundleKind::Failure, _) => Err(MoltenError::invalid_harness(format!(
            "failure repro bundle {} cannot satisfy pass evidence gate",
            bundle.bundle_ref
        ))),
        (HarnessReproBundleKind::Report, None) => {
            Err(MoltenError::invalid_harness("report repro bundle missing report value"))
        }
    }
}

pub fn repro_bundle_summary(bundle_value: &IOValue) -> Result<String> {
    let bundle = parse_repro_bundle(bundle_value)?;
    let gate_receipt = bundle.gate_receipt_ref.as_deref().unwrap_or("none");
    let export_profile = bundle.export_profile.as_deref().unwrap_or("legacy");
    let loss_classification = bundle.loss_classification.as_deref().unwrap_or("unknown");
    Ok(format!(
        "repro bundle {}\nkind={}\nartifact={}\ngate_receipt={}\nprofile={}\nloss_classification={}",
        bundle.bundle_ref,
        match bundle.kind {
            HarnessReproBundleKind::Report => "report",
            HarnessReproBundleKind::Failure => "failure",
        },
        bundle.artifact_ref,
        gate_receipt,
        export_profile,
        loss_classification
    ))
}

pub fn actor_registry_value(actors: &[ActorDecl]) -> IOValue {
    record("actor-registry-v1", vec![
        string(HARNESS_ACTOR_REGISTRY_SCHEMA),
        sequence(actors.iter().map(actor_decl_value).collect()),
    ])
}

fn actor_decl_value(actor: &ActorDecl) -> IOValue {
    let mut fields = vec![string(&actor.id), string(actor.kind.as_str())];
    if let Some(executor) = &actor.executor {
        fields.push(actor_executor_config_value(executor));
    }
    record("actor", fields)
}

fn actor_executor_config_value(config: &ActorExecutorConfig) -> IOValue {
    match config {
        ActorExecutorConfig::Steel(config) => steel_executor_config_value(config),
        ActorExecutorConfig::Wasm(config) => wasm_executor_config_value(config),
        ActorExecutorConfig::Adapter(config) => adapter_executor_config_value(config),
        ActorExecutorConfig::RemoteProxy(config) => remote_proxy_executor_config_value(config),
    }
}

fn steel_executor_config_value(config: &SteelExecutorConfig) -> IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("steel-executor-v1", vec![
        string(RUNTIME_STEEL_EXECUTOR_SCHEMA),
        record("source", vec![string(&config.source)]),
        record("callable", vec![string(&config.callable)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
    ])
}

fn wasm_executor_config_value(config: &WasmExecutorConfig) -> IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("wasm-executor-v1", vec![
        string(RUNTIME_WASM_EXECUTOR_SCHEMA),
        record("module-hex", vec![string(&config.module_hex)]),
        record("wit", vec![string(&config.wit)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
    ])
}

fn adapter_executor_config_value(config: &AdapterExecutorConfig) -> IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("adapter-executor-v1", vec![
        string(RUNTIME_ADAPTER_EXECUTOR_SCHEMA),
        record("manifest", vec![string(&config.manifest)]),
        record("abi", vec![string(&config.abi)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
    ])
}

fn remote_proxy_executor_config_value(config: &RemoteProxyExecutorConfig) -> IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("remote-proxy-executor-v1", vec![
        string(RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA),
        record("peer", vec![string(&config.peer)]),
        record("endpoint", vec![string(&config.endpoint)]),
        record("contract", vec![string(&config.contract)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
    ])
}

pub fn executor_preflights_value(suite: &HarnessSuite) -> Result<IOValue> {
    validate_executor_preflight_inputs(suite)?;
    Ok(record("executor-preflights-v1", vec![
        string(HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA),
        sequence(
            suite
                .actors
                .iter()
                .map(|actor| executor_preflight_value(actor, &allowed_hostcalls_for_actor(suite, actor)))
                .collect::<Result<Vec<_>>>()?,
        ),
    ]))
}

pub fn validate_executor_preflight_inputs(suite: &HarnessSuite) -> Result<()> {
    for actor in &suite.actors {
        match (&actor.kind, &actor.executor) {
            (ActorKind::Native, None) => {}
            (ActorKind::Native, Some(_)) => {
                return Err(MoltenError::invalid_harness(format!(
                    "native actor {} must not declare non-native executor preflight fixture",
                    actor.id
                )));
            }
            (ActorKind::Steel, Some(ActorExecutorConfig::Steel(config))) => {
                validate_steel_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "Steel")?;
            }
            (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(config))) => {
                validate_wasm_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "Wasm")?;
            }
            (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(config))) => {
                validate_adapter_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "adapter")?;
            }
            (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(config))) => {
                validate_remote_proxy_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "remote-proxy")?;
            }
            (ActorKind::Steel, None) => {
                return Err(MoltenError::invalid_harness(format!(
                    "steel actor {} missing reviewed Steel executor preflight fixture",
                    actor.id
                )));
            }
            (ActorKind::Wasm, None) => {
                return Err(MoltenError::invalid_harness(format!(
                    "wasm actor {} missing Wasm executor preflight fixture",
                    actor.id
                )));
            }
            (ActorKind::Adapter, None) | (ActorKind::RemoteProxy, None) => {
                return Err(MoltenError::invalid_harness(format!(
                    "executor kind {} requires executor adapter preflight and remains disabled in local harness",
                    actor.kind.as_str()
                )));
            }
            (ActorKind::Steel, Some(_))
            | (ActorKind::Wasm, Some(_))
            | (ActorKind::Adapter, Some(_))
            | (ActorKind::RemoteProxy, Some(_)) => {
                return Err(MoltenError::invalid_harness(format!(
                    "actor {} kind {} has mismatched executor preflight fixture",
                    actor.id,
                    actor.kind.as_str()
                )));
            }
        }
    }
    Ok(())
}

fn validate_required_hostcalls_allowed(
    suite: &HarnessSuite,
    actor: &ActorDecl,
    allowed_hostcalls: &[String],
    executor_name: &str,
) -> Result<()> {
    let required_hostcalls = hostcalls_required_by_steps(suite, &actor.id);
    for operation in required_hostcalls {
        if !allowed_hostcalls.iter().any(|allowed| allowed.as_str() == operation.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "hostcall operation {operation} is not allowed by {executor_name} executor preflight for actor {}",
                actor.id
            )));
        }
    }
    Ok(())
}

fn executor_preflight_value(actor: &ActorDecl, allowed_hostcalls: &[String]) -> Result<IOValue> {
    let sandbox = executor_sandbox_value(&actor.kind);
    let sandbox_ref = canonical_hash(&sandbox)?;
    let conformance_refs: Vec<std::string::String> = executor_conformance_refs(allowed_hostcalls)?;
    let (artifact_ref, receipts, checks) = match (&actor.kind, &actor.executor) {
        (ActorKind::Native, None) => (None, Vec::new(), executor_preflight_checks(&actor.kind).to_vec()),
        (ActorKind::Steel, Some(ActorExecutorConfig::Steel(config))) => {
            let source_ref = steel_source_ref(config)?;
            let receipt = steel_review_receipt_value(config)?;
            (Some(source_ref), vec![receipt], steel_executor_preflight_checks().to_vec())
        }
        (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(config))) => {
            let module_ref = wasm_module_ref(config)?;
            let receipt = wasm_inspection_receipt_value(config)?;
            (Some(module_ref), vec![receipt], wasm_executor_preflight_checks().to_vec())
        }
        (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(config))) => {
            let manifest_ref = adapter_manifest_ref(config)?;
            let receipt = adapter_preflight_receipt_value(config)?;
            (Some(manifest_ref), vec![receipt], adapter_executor_preflight_checks().to_vec())
        }
        (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(config))) => {
            let endpoint_ref = remote_proxy_endpoint_ref(config)?;
            let receipt = remote_proxy_preflight_receipt_value(config)?;
            (Some(endpoint_ref), vec![receipt], remote_proxy_executor_preflight_checks().to_vec())
        }
        _ => {
            return Err(MoltenError::invalid_harness(format!(
                "unsupported executor preflight fixture for actor {} kind {}",
                actor.id,
                actor.kind.as_str()
            )));
        }
    };
    Ok(record("executor-preflight-v1", vec![
        string(RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA),
        record("actor", vec![string(&actor.id)]),
        record("kind", vec![string(actor.kind.as_str())]),
        record("artifact-ref", vec![optional_string_value(artifact_ref.as_deref())]),
        record("sandbox-ref", vec![string(sandbox_ref)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("conformance-suites", vec![sequence(vec![string(&conformance_refs[0])])]),
        record("executor-receipts", vec![sequence(receipts)]),
        hostcall_checks_value(&checks),
    ]))
}

fn executor_conformance_refs(allowed_hostcalls: &[String]) -> Result<Vec<String>> {
    Ok(vec![canonical_hash(&executor_conformance_suite_value(allowed_hostcalls))?])
}

fn executor_conformance_suite_value(allowed_hostcalls: &[String]) -> IOValue {
    record("executor-conformance-suite-v1", vec![
        string(HARNESS_EXECUTOR_CONFORMANCE_SCHEMA),
        record("boundary", vec![string("molten.runtime.executor-hostcall-boundary.v1")]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("actor-input", vec![string(RUNTIME_ACTOR_INPUT_SCHEMA)]),
        record("hostcall-request", vec![string(RUNTIME_HOSTCALL_REQUEST_SCHEMA)]),
        record("hostcall-decision", vec![string(RUNTIME_HOSTCALL_DECISION_SCHEMA)]),
        record("actor-output", vec![string(RUNTIME_ACTOR_OUTPUT_SCHEMA)]),
        hostcall_checks_value(&[
            "canonical-preserves",
            "hostcall-admission-binding",
            "deterministic-replay",
            "no-ambient-executor-io",
            "cross-kind-compatible",
        ]),
    ])
}

fn executor_sandbox_value(kind: &ActorKind) -> IOValue {
    record("executor-sandbox-v1", vec![
        record("kind", vec![string(kind.as_str())]),
        record("ambient-io", vec![bool_value(false)]),
        record("hostcalls-only", vec![bool_value(true)]),
    ])
}

fn executor_preflight_checks(kind: &ActorKind) -> &'static [&'static str] {
    match kind {
        ActorKind::Native => &[
            "actor-kind-binding",
            "allowed-hostcall-binding",
            "no-ambient-executor-io",
            "native-local-executor",
        ],
        ActorKind::Steel | ActorKind::Wasm | ActorKind::Adapter | ActorKind::RemoteProxy => &[
            "actor-kind-binding",
            "allowed-hostcall-binding",
            "no-ambient-executor-io",
            "requires-executor-adapter",
        ],
    }
}

fn steel_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "steel-source-ref-binding",
        "steel-callable-review",
        "steel-hostcall-contract",
    ]
}

fn wasm_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "wasm-module-ref-binding",
        "wasmparser-inspection",
        "wasm-deny-by-default-wasi",
        "wasm-hostcall-contract",
        "wit-interface-binding",
    ]
}

fn adapter_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "adapter-manifest-binding",
        "adapter-permission-binding",
        "adapter-transcript-replay",
    ]
}

fn remote_proxy_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "remote-peer-binding",
        "remote-contract-binding",
        "remote-transcript-replay",
    ]
}

fn steel_review_receipt_value(config: &SteelExecutorConfig) -> Result<IOValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("steel-review-receipt-v1", vec![
        string(RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("source-ref", vec![string(steel_source_ref(config)?)]),
        record("callable", vec![string(&config.callable)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        hostcall_checks_value(&[
            "source-ref-binding",
            "reviewed-callable",
            "allowed-hostcall-contract",
            "no-ambient-steel-io",
        ]),
    ]))
}

fn adapter_preflight_receipt_value(config: &AdapterExecutorConfig) -> Result<IOValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("adapter-preflight-receipt-v1", vec![
        string(RUNTIME_ADAPTER_PREFLIGHT_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("manifest-ref", vec![string(adapter_manifest_ref(config)?)]),
        record("abi-ref", vec![string(canonical_hash(&string(&config.abi))?)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
        hostcall_checks_value(&[
            "manifest-ref-binding",
            "permission-binding",
            "deterministic-transcript",
            "no-ambient-adapter-io",
        ]),
    ]))
}

fn remote_proxy_preflight_receipt_value(config: &RemoteProxyExecutorConfig) -> Result<IOValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("remote-proxy-preflight-receipt-v1", vec![
        string(RUNTIME_REMOTE_PROXY_PREFLIGHT_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("peer-ref", vec![string(canonical_hash(&string(&config.peer))?)]),
        record("endpoint-ref", vec![string(remote_proxy_endpoint_ref(config)?)]),
        record("contract-ref", vec![string(canonical_hash(&string(&config.contract))?)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
        hostcall_checks_value(&[
            "peer-identity-binding",
            "endpoint-contract-binding",
            "verified-transcript",
            "transport-not-authority",
        ]),
    ]))
}

fn validate_steel_executor_config(actor_id: &str, config: &SteelExecutorConfig) -> Result<()> {
    if config.source.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("Steel executor source for actor {actor_id} is empty")));
    }
    if config.callable.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("Steel executor callable for actor {actor_id} is empty")));
    }
    for token in FORBIDDEN_STEEL_SOURCE_TOKENS {
        if config.source.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "Steel executor source for actor {actor_id} references forbidden ambient IO token {token}; reviewed Steel preflight remains fail-closed"
            )));
        }
    }
    Ok(())
}

const FORBIDDEN_STEEL_SOURCE_TOKENS: &[&str] = &[
    "open-input-file",
    "open-output-file",
    "call-with-input-file",
    "call-with-output-file",
    "delete-file",
    "read-file",
    "write-file",
    "system",
    "process",
    "current-seconds",
    "current-inexact-milliseconds",
    "random",
    "tcp",
    "udp",
    "http",
    "ffi",
];

pub(crate) fn steel_source_ref(config: &SteelExecutorConfig) -> Result<String> {
    canonical_hash(&string(&config.source))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WasmInspection {
    module_kind: String,
    imports: Vec<WasmImportEvidence>,
}

fn wasm_inspection_receipt_value(config: &WasmExecutorConfig) -> Result<IOValue> {
    let inspection = inspect_wasm_module(config)?;
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("wasm-inspection-receipt-v1", vec![
        string(RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("module-ref", vec![string(wasm_module_ref(config)?)]),
        record("module-kind", vec![string(&inspection.module_kind)]),
        record("imports", vec![sequence(inspection.imports.iter().map(wasm_import_value).collect())]),
        record("wit-ref", vec![string(wasm_wit_ref(config)?)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        hostcall_checks_value(&[
            "module-ref-binding",
            "wasmparser-validated",
            "deny-by-default-wasi",
            "allowed-hostcall-contract",
            "wit-interface-binding",
        ]),
    ]))
}

fn validate_wasm_executor_config(actor_id: &str, config: &WasmExecutorConfig) -> Result<()> {
    if config.wit.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("Wasm executor WIT interface for actor {actor_id} is empty")));
    }
    let inspection = inspect_wasm_module(config).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor module for actor {actor_id} failed preflight: {error}"))
    })?;
    validate_wasm_imports(actor_id, &inspection.imports, &config.allowed_hostcalls)
}

fn validate_adapter_executor_config(actor_id: &str, config: &AdapterExecutorConfig) -> Result<()> {
    if config.manifest.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("adapter executor manifest for actor {actor_id} is empty")));
    }
    if config.abi.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("adapter executor ABI for actor {actor_id} is empty")));
    }
    for token in FORBIDDEN_ADAPTER_MANIFEST_TOKENS {
        if config.manifest.contains(token) || config.abi.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "adapter executor manifest for actor {actor_id} references forbidden ambient or stale token {token}"
            )));
        }
    }
    if config.transcript != "deterministic-local" && config.transcript != "verified" {
        return Err(MoltenError::invalid_harness(format!(
            "adapter executor transcript profile for actor {actor_id} must be deterministic-local or verified"
        )));
    }
    Ok(())
}

const FORBIDDEN_ADAPTER_MANIFEST_TOKENS: &[&str] =
    &["ambient-network", "ambient-fs", "process", "socket", "stale-signature"];

fn validate_remote_proxy_executor_config(actor_id: &str, config: &RemoteProxyExecutorConfig) -> Result<()> {
    if config.peer.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("remote-proxy peer for actor {actor_id} is empty")));
    }
    if config.peer == "unknown" || config.peer.contains("revoked") {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy peer for actor {actor_id} cannot satisfy trusted deterministic gate evidence"
        )));
    }
    if config.endpoint.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("remote-proxy endpoint for actor {actor_id} is empty")));
    }
    if !config.endpoint.starts_with("iroh:") {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy endpoint for actor {actor_id} must use an explicit iroh: transport profile"
        )));
    }
    if config.contract.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("remote-proxy contract for actor {actor_id} is empty")));
    }
    if config.contract.contains("stale-signature") {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy contract for actor {actor_id} references stale signature evidence"
        )));
    }
    if config.transcript != "verified" {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy transcript profile for actor {actor_id} must be verified before deterministic gates"
        )));
    }
    Ok(())
}

fn inspect_wasm_module(config: &WasmExecutorConfig) -> Result<WasmInspection> {
    let bytes = wasm_module_bytes(config)?;
    Validator::new()
        .validate_all(&bytes)
        .map_err(|error| MoltenError::invalid_harness(format!("wasmparser validation failed: {error}")))?;
    let mut module_kind = None;
    let mut imports = Vec::new();
    for payload in Parser::new(0).parse_all(&bytes) {
        match payload.map_err(|error| MoltenError::invalid_harness(format!("wasmparser parse failed: {error}")))? {
            Payload::Version { encoding, .. } => {
                module_kind = Some(match encoding {
                    Encoding::Module => "core-module".to_string(),
                    Encoding::Component => "component".to_string(),
                });
            }
            Payload::ImportSection(section) => {
                for import in section {
                    let import = import
                        .map_err(|error| MoltenError::invalid_harness(format!("wasm import parse failed: {error}")))?;
                    push_bounded(
                        &mut imports,
                        WasmImportEvidence {
                            module: import.module.to_string(),
                            name: import.name.to_string(),
                            kind: wasm_type_ref_kind(&import.ty).to_string(),
                        },
                        MAX_WASM_IMPORT_EVIDENCE,
                        "wasm import evidence",
                    )?;
                }
            }
            Payload::ComponentImportSection(section) => {
                for import in section {
                    let import = import.map_err(|error| {
                        MoltenError::invalid_harness(format!("wasm component import parse failed: {error}"))
                    })?;
                    push_bounded(
                        &mut imports,
                        WasmImportEvidence {
                            module: "component".to_string(),
                            name: import.name.0.to_string(),
                            kind: format!("component:{:?}", import.ty.kind()),
                        },
                        MAX_WASM_IMPORT_EVIDENCE,
                        "wasm import evidence",
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(WasmInspection {
        module_kind: module_kind.unwrap_or_else(|| "unknown".to_string()),
        imports,
    })
}

fn validate_wasm_imports(actor_id: &str, imports: &[WasmImportEvidence], allowed_hostcalls: &[String]) -> Result<()> {
    for import in imports {
        if import.module == "molten:hostcall" {
            if import.kind != "func" {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm executor import {}::{} for actor {actor_id} must be a function hostcall",
                    import.module, import.name
                )));
            }
            if !allowed_hostcalls.iter().any(|allowed| allowed == &import.name) {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm executor import {}::{} for actor {actor_id} is not in allowed hostcalls",
                    import.module, import.name
                )));
            }
            continue;
        }
        if import.module == "component" && allowed_hostcalls.iter().any(|allowed| allowed == &import.name) {
            continue;
        }
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor import {}::{} for actor {actor_id} is not an allowed Molten hostcall; WASI and ambient imports remain disabled",
            import.module, import.name
        )));
    }
    Ok(())
}

fn wasm_type_ref_kind(ty: &wasmparser::TypeRef) -> &'static str {
    match ty {
        wasmparser::TypeRef::Func(_) => "func",
        wasmparser::TypeRef::Table(_) => "table",
        wasmparser::TypeRef::Memory(_) => "memory",
        wasmparser::TypeRef::Global(_) => "global",
        wasmparser::TypeRef::Tag(_) => "tag",
    }
}

fn wasm_import_value(import: &WasmImportEvidence) -> IOValue {
    record("import", vec![string(&import.module), string(&import.name), string(&import.kind)])
}

pub(crate) fn wasm_module_ref(config: &WasmExecutorConfig) -> Result<String> {
    canonical_hash(&record("wasm-module-bytes-v1", vec![string(&config.module_hex)]))
}

fn adapter_manifest_ref(config: &AdapterExecutorConfig) -> Result<String> {
    canonical_hash(&record("adapter-manifest-v1", vec![string(&config.manifest), string(&config.abi)]))
}

fn remote_proxy_endpoint_ref(config: &RemoteProxyExecutorConfig) -> Result<String> {
    canonical_hash(&record("remote-proxy-endpoint-v1", vec![
        string(&config.peer),
        string(&config.endpoint),
        string(&config.contract),
    ]))
}

pub(crate) fn wasm_executor_export_name(operation: &str) -> String {
    format!("molten_hostcall_{operation}")
}

fn wasm_wit_ref(config: &WasmExecutorConfig) -> Result<String> {
    canonical_hash(&string(&config.wit))
}

pub(crate) fn wasm_module_bytes(config: &WasmExecutorConfig) -> Result<Vec<u8>> {
    decode_hex_bytes(&config.module_hex, "Wasm executor module hex")
}

fn normalize_hex(input: &str, field: &str) -> Result<String> {
    let mut normalized = String::new();
    for character in input.chars() {
        if character.is_ascii_whitespace() || character == '_' {
            continue;
        }
        if !character.is_ascii_hexdigit() {
            return Err(MoltenError::invalid_harness(format!("{field} contains non-hex character {character:?}")));
        }
        normalized.push(character.to_ascii_lowercase());
    }
    if normalized.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    if !normalized.len().is_multiple_of(2) {
        return Err(MoltenError::invalid_harness(format!("{field} must contain an even number of hex digits")));
    }
    Ok(normalized)
}

fn decode_hex_bytes(input: &str, field: &str) -> Result<Vec<u8>> {
    let normalized = normalize_hex(input, field)?;
    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    for index in (0..normalized.len()).step_by(2) {
        let byte = u8::from_str_radix(&normalized[index..index + 2], 16).map_err(|error| {
            MoltenError::invalid_harness(format!("{field} contains invalid byte at offset {index}: {error}"))
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn allowed_hostcalls_for_actor(suite: &HarnessSuite, actor: &ActorDecl) -> Vec<String> {
    match &actor.executor {
        Some(ActorExecutorConfig::Steel(config)) => config.allowed_hostcalls.clone(),
        Some(ActorExecutorConfig::Wasm(config)) => config.allowed_hostcalls.clone(),
        Some(ActorExecutorConfig::Adapter(config)) => config.allowed_hostcalls.clone(),
        Some(ActorExecutorConfig::RemoteProxy(config)) => config.allowed_hostcalls.clone(),
        None => hostcalls_required_by_steps(suite, &actor.id),
    }
}

fn hostcalls_required_by_steps(suite: &HarnessSuite, actor_id: &str) -> Vec<String> {
    let mut hostcalls = BTreeSet::new();
    for step in &suite.steps {
        if step.primary_actor() == actor_id {
            hostcalls.insert(AdmissionRequest::from_step(step).action.as_str().to_string());
        }
    }
    hostcalls.into_iter().collect()
}

pub fn parse_executor_preflights(value: &IOValue) -> Result<ExecutorPreflightsEvidence> {
    let preflights = simple_record(value, "executor-preflights-v1", 2)?;
    let schema = required_string(&preflights[0], "executor preflights schema")?;
    if schema != HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported executor preflights schema {schema}; expected {HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA}"
        )));
    }
    let preflight_values = required_sequence(&preflights[1], "executor preflight entries")?;
    let mut entries = Vec::with_capacity(preflight_values.len());
    for preflight in preflight_values.iter() {
        entries.push(parse_executor_preflight(&value_to_iovalue(&preflight))?);
    }
    Ok(ExecutorPreflightsEvidence {
        value: value.clone(),
        preflights: entries,
    })
}

fn parse_executor_preflight(value: &IOValue) -> Result<ExecutorPreflightEvidence> {
    let preflight = value
        .collect_simple_record("executor-preflight-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <executor-preflight-v1 ...>"))?;
    let arity = preflight.fields_iter().count();
    if arity != 8 && arity != 9 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <executor-preflight-v1 ...> with arity 8 or 9, got {arity}"
        )));
    }
    let schema = required_string(&preflight[0], "executor preflight schema")?;
    if schema != RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported executor preflight schema {schema}; expected {RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA}"
        )));
    }
    let actor_id = required_record_string(&preflight[1], "actor", "executor preflight actor")?;
    let kind = parse_actor_kind(&required_record_string(&preflight[2], "kind", "executor preflight kind")?)?;
    let artifact_ref = optional_executor_hash(&preflight[3], "artifact-ref", "executor artifact ref")?;
    let sandbox_ref = required_record_hash(&preflight[4], "sandbox-ref", "executor sandbox ref")?;
    let allowed_hostcalls =
        required_record_string_sequence(&preflight[5], "allowed-hostcalls", "executor allowed hostcalls")?;
    let conformance_refs =
        required_record_hash_sequence(&preflight[6], "conformance-suites", "executor conformance suites")?;
    let (executor_receipts, checks_index) = if arity == 9 {
        let receipts = required_record_iovalue_sequence(&preflight[7], "executor-receipts", "executor receipts")?;
        (receipts, 8)
    } else {
        (Vec::new(), 7)
    };
    let steel_review = parse_optional_steel_review_receipt(&executor_receipts)?;
    let wasm_inspection = parse_optional_wasm_inspection_receipt(&executor_receipts)?;
    let checks = parse_executor_preflight_checks(&preflight[checks_index])?;
    require_executor_preflight_check(&checks, "actor-kind-binding")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-binding")?;
    require_executor_preflight_check(&checks, "no-ambient-executor-io")?;
    Ok(ExecutorPreflightEvidence {
        value: value.clone(),
        actor_id,
        kind,
        artifact_ref,
        sandbox_ref,
        allowed_hostcalls,
        conformance_refs,
        executor_receipts,
        steel_review,
        wasm_inspection,
        checks,
    })
}

pub fn validate_executor_preflight_evidence(
    suite: &HarnessSuite,
    observations: &[HarnessObservation],
    preflights: Option<&ExecutorPreflightsEvidence>,
) -> Result<()> {
    let preflights = preflights.ok_or_else(|| MoltenError::invalid_harness("missing executor preflight evidence"))?;
    let expected = executor_preflights_value(suite)?;
    if preflights.value != expected {
        return Err(MoltenError::invalid_harness(format!(
            "executor preflight evidence mismatch: got {}, expected {}",
            canonical_hash(&preflights.value)?,
            canonical_hash(&expected)?
        )));
    }
    let mut by_actor = std::collections::BTreeMap::new();
    for preflight in &preflights.preflights {
        if by_actor.insert(preflight.actor_id.as_str(), preflight).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate executor preflight for actor {}",
                preflight.actor_id
            )));
        }
    }
    for actor in &suite.actors {
        let Some(preflight) = by_actor.get(actor.id.as_str()) else {
            return Err(MoltenError::invalid_harness(format!("missing executor preflight for actor {}", actor.id)));
        };
        if preflight.kind != actor.kind {
            return Err(MoltenError::invalid_harness(format!("executor kind binding mismatch for actor {}", actor.id)));
        }
        validate_actor_executor_preflight(actor, preflight)?;
    }
    for (position, observation) in observations.iter().enumerate() {
        for event in &observation.events {
            if let Some(request) = event.collect_simple_record("hostcall-request-v1", None) {
                let arity = request.fields_iter().count();
                if arity != 9 && arity != 11 && arity != 15 {
                    return Err(MoltenError::invalid_harness(format!(
                        "hostcall request at observation {position} must have arity 9, 11, or 15, got {arity}"
                    )));
                }
                let admission_request = parse_admission_request(&request[4])?;
                let preflight: &&ExecutorPreflightEvidence =
                    by_actor.get(admission_request.actor.as_str()).ok_or_else(|| {
                        MoltenError::invalid_harness(format!(
                            "hostcall request at observation {position} has no executor preflight for actor {}",
                            admission_request.actor
                        ))
                    })?;
                let preflight = *preflight;
                let allowed_hostcalls: &[String] = preflight.allowed_hostcalls.as_slice();
                let operation = admission_request.action.as_str();
                if !allowed_hostcalls.iter().any(|allowed| allowed.as_str() == operation) {
                    return Err(MoltenError::invalid_harness(format!(
                        "hostcall operation {operation} at observation {position} is not allowed by executor preflight for actor {}",
                        admission_request.actor
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_actor_executor_preflight(actor: &ActorDecl, preflight: &ExecutorPreflightEvidence) -> Result<()> {
    let expected_conformance_refs = executor_conformance_refs(&preflight.allowed_hostcalls)?;
    if preflight.conformance_refs != expected_conformance_refs {
        return Err(MoltenError::invalid_harness(format!(
            "executor conformance suite refs mismatch for actor {}",
            actor.id
        )));
    }
    match (&actor.kind, &actor.executor) {
        (ActorKind::Native, None) => {
            if preflight.artifact_ref.is_some() || !preflight.executor_receipts.is_empty() {
                return Err(MoltenError::invalid_harness(format!(
                    "native executor preflight for actor {} must not carry artifact or review receipts",
                    actor.id
                )));
            }
            require_executor_preflight_check(&preflight.checks, "native-local-executor")
        }
        (ActorKind::Steel, Some(ActorExecutorConfig::Steel(config))) => {
            require_executor_preflight_check(&preflight.checks, "steel-source-ref-binding")?;
            require_executor_preflight_check(&preflight.checks, "steel-callable-review")?;
            require_executor_preflight_check(&preflight.checks, "steel-hostcall-contract")?;
            let source_ref = steel_source_ref(config)?;
            if preflight.artifact_ref.as_deref() != Some(source_ref.as_str()) {
                return Err(MoltenError::invalid_harness(format!(
                    "Steel executor preflight source ref mismatch for actor {}",
                    actor.id
                )));
            }
            let review = preflight.steel_review.as_ref().ok_or_else(|| {
                MoltenError::invalid_harness(format!(
                    "Steel executor preflight missing review receipt for actor {}",
                    actor.id
                ))
            })?;
            if review.source_ref != source_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "Steel review receipt source ref mismatch for actor {}",
                    actor.id
                )));
            }
            if review.callable != config.callable {
                return Err(MoltenError::invalid_harness(format!(
                    "Steel review receipt callable mismatch for actor {}",
                    actor.id
                )));
            }
            if review.allowed_hostcalls != config.allowed_hostcalls
                || preflight.allowed_hostcalls != config.allowed_hostcalls
            {
                return Err(MoltenError::invalid_harness(format!(
                    "Steel review receipt allowed hostcalls mismatch for actor {}",
                    actor.id
                )));
            }
            Ok(())
        }
        (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(config))) => {
            require_executor_preflight_check(&preflight.checks, "wasm-module-ref-binding")?;
            require_executor_preflight_check(&preflight.checks, "wasmparser-inspection")?;
            require_executor_preflight_check(&preflight.checks, "wasm-deny-by-default-wasi")?;
            require_executor_preflight_check(&preflight.checks, "wasm-hostcall-contract")?;
            require_executor_preflight_check(&preflight.checks, "wit-interface-binding")?;
            let module_ref = wasm_module_ref(config)?;
            if preflight.artifact_ref.as_deref() != Some(module_ref.as_str()) {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm executor preflight module ref mismatch for actor {}",
                    actor.id
                )));
            }
            let inspection = preflight.wasm_inspection.as_ref().ok_or_else(|| {
                MoltenError::invalid_harness(format!(
                    "Wasm executor preflight missing inspection receipt for actor {}",
                    actor.id
                ))
            })?;
            if inspection.module_ref != module_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm inspection receipt module ref mismatch for actor {}",
                    actor.id
                )));
            }
            if inspection.wit_ref != wasm_wit_ref(config)? {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm inspection receipt WIT ref mismatch for actor {}",
                    actor.id
                )));
            }
            if inspection.allowed_hostcalls != config.allowed_hostcalls
                || preflight.allowed_hostcalls != config.allowed_hostcalls
            {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm inspection receipt allowed hostcalls mismatch for actor {}",
                    actor.id
                )));
            }
            validate_wasm_imports(&actor.id, &inspection.imports, &config.allowed_hostcalls)
        }
        (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(config))) => {
            require_executor_preflight_check(&preflight.checks, "adapter-manifest-binding")?;
            require_executor_preflight_check(&preflight.checks, "adapter-permission-binding")?;
            require_executor_preflight_check(&preflight.checks, "adapter-transcript-replay")?;
            let manifest_ref = adapter_manifest_ref(config)?;
            if preflight.artifact_ref.as_deref() != Some(manifest_ref.as_str()) {
                return Err(MoltenError::invalid_harness(format!(
                    "adapter executor preflight manifest ref mismatch for actor {}",
                    actor.id
                )));
            }
            if preflight.allowed_hostcalls != config.allowed_hostcalls {
                return Err(MoltenError::invalid_harness(format!(
                    "adapter executor preflight allowed hostcalls mismatch for actor {}",
                    actor.id
                )));
            }
            Ok(())
        }
        (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(config))) => {
            require_executor_preflight_check(&preflight.checks, "remote-peer-binding")?;
            require_executor_preflight_check(&preflight.checks, "remote-contract-binding")?;
            require_executor_preflight_check(&preflight.checks, "remote-transcript-replay")?;
            let endpoint_ref = remote_proxy_endpoint_ref(config)?;
            if preflight.artifact_ref.as_deref() != Some(endpoint_ref.as_str()) {
                return Err(MoltenError::invalid_harness(format!(
                    "remote-proxy executor preflight endpoint ref mismatch for actor {}",
                    actor.id
                )));
            }
            if preflight.allowed_hostcalls != config.allowed_hostcalls {
                return Err(MoltenError::invalid_harness(format!(
                    "remote-proxy executor preflight allowed hostcalls mismatch for actor {}",
                    actor.id
                )));
            }
            Ok(())
        }
        (ActorKind::Steel, None) => Err(MoltenError::invalid_harness(format!(
            "steel actor {} missing reviewed Steel executor preflight fixture",
            actor.id
        ))),
        (ActorKind::Wasm, None) => Err(MoltenError::invalid_harness(format!(
            "wasm actor {} missing Wasm executor preflight fixture",
            actor.id
        ))),
        (ActorKind::Adapter | ActorKind::RemoteProxy, _) => Err(MoltenError::invalid_harness(format!(
            "executor kind {} requires executor adapter preflight and remains disabled in local harness",
            actor.kind.as_str()
        ))),
        (ActorKind::Steel, Some(_)) | (ActorKind::Wasm, Some(_)) => Err(MoltenError::invalid_harness(format!(
            "actor {} kind {} has mismatched executor preflight fixture",
            actor.id,
            actor.kind.as_str()
        ))),
        (ActorKind::Native, Some(_)) => Err(MoltenError::invalid_harness(format!(
            "native actor {} must not declare non-native executor preflight fixture",
            actor.id
        ))),
    }
}

fn parse_optional_steel_review_receipt(receipts: &[IOValue]) -> Result<Option<SteelReviewReceipt>> {
    let mut parsed = None;
    for receipt in receipts {
        if receipt.collect_simple_record("steel-review-receipt-v1", None).is_some() {
            if parsed.is_some() {
                return Err(MoltenError::invalid_harness("duplicate Steel review receipt in executor preflight"));
            }
            parsed = Some(parse_steel_review_receipt(receipt)?);
        }
    }
    Ok(parsed)
}

fn parse_steel_review_receipt(value: &IOValue) -> Result<SteelReviewReceipt> {
    let receipt = simple_record(value, "steel-review-receipt-v1", 6)?;
    let schema = required_string(&receipt[0], "Steel review receipt schema")?;
    if schema != RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Steel review receipt schema {schema}; expected {RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Steel review receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Steel review receipt decision {decision}")));
    }
    let source_ref = required_record_hash(&receipt[2], "source-ref", "Steel review receipt source ref")?;
    let callable = required_record_string(&receipt[3], "callable", "Steel review receipt callable")?;
    let allowed_hostcalls =
        required_record_string_sequence(&receipt[4], "allowed-hostcalls", "Steel review receipt allowed hostcalls")?;
    let checks = parse_executor_preflight_checks(&receipt[5])?;
    require_executor_preflight_check(&checks, "source-ref-binding")?;
    require_executor_preflight_check(&checks, "reviewed-callable")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-contract")?;
    require_executor_preflight_check(&checks, "no-ambient-steel-io")?;
    Ok(SteelReviewReceipt {
        value: value.clone(),
        source_ref,
        callable,
        allowed_hostcalls,
        checks,
    })
}

fn parse_optional_wasm_inspection_receipt(receipts: &[IOValue]) -> Result<Option<WasmInspectionReceipt>> {
    let mut parsed = None;
    for receipt in receipts {
        if receipt.collect_simple_record("wasm-inspection-receipt-v1", None).is_some() {
            if parsed.is_some() {
                return Err(MoltenError::invalid_harness("duplicate Wasm inspection receipt in executor preflight"));
            }
            parsed = Some(parse_wasm_inspection_receipt(receipt)?);
        }
    }
    Ok(parsed)
}

fn parse_wasm_inspection_receipt(value: &IOValue) -> Result<WasmInspectionReceipt> {
    let receipt = simple_record(value, "wasm-inspection-receipt-v1", 8)?;
    let schema = required_string(&receipt[0], "Wasm inspection receipt schema")?;
    if schema != RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm inspection receipt schema {schema}; expected {RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Wasm inspection receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Wasm inspection receipt decision {decision}")));
    }
    let module_ref = required_record_hash(&receipt[2], "module-ref", "Wasm inspection receipt module ref")?;
    let module_kind = required_record_string(&receipt[3], "module-kind", "Wasm inspection receipt module kind")?;
    if !matches!(module_kind.as_str(), "core-module" | "component") {
        return Err(MoltenError::invalid_harness(format!("unsupported Wasm inspection module kind {module_kind}")));
    }
    let import_values = required_record_sequence(&receipt[4], "imports", "Wasm inspection imports")?;
    let mut imports = Vec::with_capacity(import_values.len());
    for import_value in import_values {
        imports.push(parse_wasm_import(&value_to_iovalue(&import_value))?);
    }
    let wit_ref = required_record_hash(&receipt[5], "wit-ref", "Wasm inspection WIT ref")?;
    let allowed_hostcalls =
        required_record_string_sequence(&receipt[6], "allowed-hostcalls", "Wasm inspection allowed hostcalls")?;
    let checks = parse_executor_preflight_checks(&receipt[7])?;
    require_executor_preflight_check(&checks, "module-ref-binding")?;
    require_executor_preflight_check(&checks, "wasmparser-validated")?;
    require_executor_preflight_check(&checks, "deny-by-default-wasi")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-contract")?;
    require_executor_preflight_check(&checks, "wit-interface-binding")?;
    Ok(WasmInspectionReceipt {
        value: value.clone(),
        module_ref,
        module_kind,
        imports,
        wit_ref,
        allowed_hostcalls,
        checks,
    })
}

fn parse_wasm_import(value: &IOValue) -> Result<WasmImportEvidence> {
    let import = simple_record(value, "import", 3)?;
    Ok(WasmImportEvidence {
        module: required_string(&import[0], "Wasm import module")?,
        name: required_string(&import[1], "Wasm import name")?,
        kind: required_string(&import[2], "Wasm import kind")?,
    })
}

fn optional_executor_hash(value: &Value<IOValue>, label: &str, field: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let parsed = optional_request_string(&record[0], field)?;
    if let Some(hash) = parsed.as_deref() {
        required_hash(&string(hash), field)?;
    }
    Ok(parsed)
}

fn parse_executor_preflight_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "executor preflight checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "executor preflight check name")?;
        let status = required_string(&check[1], "executor preflight check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("executor preflight check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_executor_preflight_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("executor preflight missing {expected} check")))
    }
}

fn validate_denied_observation_events(position: usize, events: &[IOValue]) -> Result<()> {
    let mut has_rollback_event = false;
    for event in events {
        match event_boundary(event) {
            EventBoundary::EffectRequest | EventBoundary::EffectResponse => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied effect emitted effect request/response at observation {position}"
                )));
            }
            EventBoundary::SteelExecution => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied turn emitted Steel execution evidence at observation {position}"
                )));
            }
            EventBoundary::WasmExecution => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied turn emitted Wasm execution evidence at observation {position}"
                )));
            }
            EventBoundary::PolicyDecision => {
                return Err(MoltenError::invalid_harness(format!(
                    "duplicate admission decision at observation {position}"
                )));
            }
            EventBoundary::ActorInput
            | EventBoundary::HostcallRequest
            | EventBoundary::HostcallDecision
            | EventBoundary::RuntimePredicate
            | EventBoundary::ActorOutput => {}
            EventBoundary::Trace if is_turn_rolled_back(event) => {
                has_rollback_event = true;
            }
            EventBoundary::Trace if is_turn_journal(event) => {}
            EventBoundary::Trace => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied turn committed action or non-rollback trace at observation {position}"
                )));
            }
        }
    }
    if !has_rollback_event {
        return Err(MoltenError::invalid_harness(format!(
            "denied turn missing rollback evidence at observation {position}"
        )));
    }
    Ok(())
}

fn is_turn_rolled_back(value: &IOValue) -> bool {
    value.collect_simple_record("turn-rolled-back", Some(2)).is_some()
}

fn is_turn_journal(value: &IOValue) -> bool {
    value.collect_simple_record("turn-journal-v1", None).is_some()
}

fn parse_admission_request(value: &Value<IOValue>) -> Result<AdmissionRequest> {
    let request_value = value_to_iovalue(value);
    let request = simple_record(&request_value, "request", 5)?;
    Ok(AdmissionRequest {
        actor: required_string(&request[0], "admission request actor")?,
        action: parse_admission_action(&required_string(&request[1], "admission request action")?)?,
        target: optional_request_string(&request[2], "admission request target")?,
        value: optional_request_runtime_value(&request[3], "admission request value")?,
        upper: optional_request_u64(&request[4], "admission request upper")?,
    })
}

fn parse_admission_authority(value: &Value<IOValue>) -> Result<AdmissionAuthorityEvidence> {
    let authority_value = value_to_iovalue(value);
    let authority = simple_record(&authority_value, "authority", 3)?;
    Ok(AdmissionAuthorityEvidence {
        capability_ref: required_record_hash(&authority[0], "capability-ref", "admission authority capability ref")?,
        authorized: required_record_bool(&authority[1], "authorized", "admission authority authorized")?,
        grant_ref: optional_request_string(&authority[2], "admission authority grant ref")?,
    })
}

fn parse_admission_decision(value: &Value<IOValue>) -> Result<AdmissionDecision> {
    let decision_value = value_to_iovalue(value);
    let decision = simple_record(&decision_value, "decision", 2)?;
    let status = required_string(&decision[0], "admission decision status")?;
    let reason = required_string(&decision[1], "admission decision reason")?;
    match status.as_str() {
        "allow" => Ok(AdmissionDecision::Allow { reason }),
        "deny" => Ok(AdmissionDecision::Deny { reason }),
        other => Err(MoltenError::invalid_harness(format!("unknown admission decision status {other}"))),
    }
}

pub fn policy_value(policy: &AdmissionPolicy) -> IOValue {
    record("policy-v1", vec![
        string(HARNESS_POLICY_SCHEMA),
        sequence(policy.deny_rules().iter().map(deny_rule_value).collect()),
    ])
}

pub fn policy_gate_value(policy: &AdmissionPolicy) -> Result<IOValue> {
    let preflight = policy_preflight_material(policy)?;
    Ok(record("policy-gate-v1", vec![
        string(HARNESS_POLICY_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("policy-ref", vec![string(&preflight.policy_ref)]),
        preflight.nickel_source_value,
        preflight.nickel_contract_value,
        preflight.basalt_preflight_value,
        record("steel-predicates", vec![sequence(Vec::new())]),
        policy_gate_checks_value(),
    ]))
}

pub fn parse_policy_gate(value: &IOValue) -> Result<PolicyGateEvidence> {
    let gate = simple_record(value, "policy-gate-v1", 8)?;
    let schema = required_string(&gate[0], "policy gate schema")?;
    if schema != HARNESS_POLICY_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy gate schema {schema}; expected {HARNESS_POLICY_GATE_SCHEMA}"
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "policy gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported policy gate decision {decision}")));
    }
    let policy_ref = required_record_hash(&gate[2], "policy-ref", "policy gate policy ref")?;
    let nickel_source = parse_nickel_source_evidence(&gate[3])?;
    let nickel_contract = parse_nickel_contract_evidence(&gate[4])?;
    let basalt_preflight = parse_basalt_policy_preflight_evidence(&gate[5])?;
    if nickel_source.policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("Nickel source policy ref does not match policy gate ref"));
    }
    if nickel_contract.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Nickel contract normalized source ref does not match Nickel source evidence",
        ));
    }
    if basalt_preflight.policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("Basalt policy preflight policy ref does not match policy gate ref"));
    }
    if basalt_preflight.envelope_ref != nickel_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt policy preflight envelope ref does not match Nickel contract envelope",
        ));
    }
    if basalt_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt policy preflight source ref does not match Nickel source evidence",
        ));
    }
    let steel_predicates = required_record_sequence(&gate[6], "steel-predicates", "policy gate Steel predicates")?;
    if !steel_predicates.is_empty() {
        return Err(MoltenError::invalid_harness(
            "Steel predicates require reviewed callable receipts and are disabled in local harness policy gates",
        ));
    }
    let checks = parse_policy_gate_checks(&gate[7])?;
    require_policy_gate_check(&checks, "policy-schema")?;
    require_policy_gate_check(&checks, "canonical-policy-snapshot")?;
    require_policy_gate_check(&checks, "nickel-static-boundary")?;
    require_policy_gate_check(&checks, "nickel-policy-source")?;
    require_policy_gate_check(&checks, "nickel-export-normalization")?;
    require_policy_gate_check(&checks, "basalt-preflight")?;
    require_policy_gate_check(&checks, "basalt-receipt-binding")?;
    require_policy_gate_check(&checks, "steel-predicate-review")?;
    Ok(PolicyGateEvidence {
        value: value.clone(),
        policy_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: basalt_preflight.receipt_ref,
        checks,
    })
}

pub fn validate_policy_gate_evidence(suite: &HarnessSuite, policy_gate: Option<&PolicyGateEvidence>) -> Result<()> {
    let policy_gate = policy_gate.ok_or_else(|| {
        MoltenError::invalid_harness("missing policy gate evidence; policy must pass preflight before side effects")
    })?;
    let expected_ref = canonical_hash(&policy_value(&suite.policy))?;
    if policy_gate.policy_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "policy gate ref mismatch: gate has {}, embedded policy hashes to {expected_ref}",
            policy_gate.policy_ref
        )));
    }
    let expected_gate = policy_gate_value(&suite.policy)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&policy_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "policy gate evidence does not match embedded suite policy preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct PolicyPreflightMaterial {
    policy_ref: String,
    nickel_source_value: IOValue,
    nickel_contract_value: IOValue,
    basalt_preflight_value: IOValue,
}

struct NickelSourceEvidence {
    source_ref: String,
    export_ref: String,
    policy_ref: String,
}

struct NickelContractEvidence {
    envelope_ref: String,
    normalized_source_ref: String,
}

struct BasaltPolicyPreflightEvidence {
    receipt_ref: String,
    envelope_ref: String,
    policy_ref: String,
    normalized_source_ref: String,
}

const POLICY_CONTRACT_ID: &str = "molten.harness.admission-policy";
const POLICY_CONTRACT_VERSION: &str = "v1";
const POLICY_INPUT_SCHEMA: &str = "molten.runtime.admission-request.v1";

fn policy_preflight_material(policy: &AdmissionPolicy) -> Result<PolicyPreflightMaterial> {
    let policy_snapshot = policy_value(policy);
    let policy_ref = canonical_hash(&policy_snapshot)?;
    let source = nickel_policy_source(policy, &policy_ref)?;
    let source_ref = canonical_hash(&string(&source))?;
    let export_json = nickel_export_json(&source)?;
    let export_ref = canonical_hash(&string(&export_json))?;
    let nickel_source_value = nickel_source_value(&source, &source_ref, &export_json, &export_ref, &policy_ref);

    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        POLICY_CONTRACT_ID,
        POLICY_CONTRACT_VERSION,
        source_ref.clone(),
        POLICY_INPUT_SCHEMA,
        RUNTIME_ADMISSION_DECISION_SCHEMA,
        HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt policy preflight denied Nickel contract envelope: {}",
            receipt.reason
        )));
    }
    let nickel_contract_value = record("nickel-contract", vec![
        string(HARNESS_POLICY_CONTRACT_SCHEMA),
        envelope_value,
        record("envelope-ref", vec![string(&envelope_ref)]),
    ]);
    let basalt_preflight_value = record("basalt-preflight", vec![
        string(HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("backend", vec![string("nickel")]),
        record("contract-id", vec![string(POLICY_CONTRACT_ID)]),
        record("envelope-ref", vec![string(envelope_ref)]),
        record("policy-ref", vec![string(&policy_ref)]),
        record("normalized-source-ref", vec![string(source_ref)]),
        record("reason", vec![string(receipt.reason)]),
    ]);
    Ok(PolicyPreflightMaterial {
        policy_ref,
        nickel_source_value,
        nickel_contract_value,
        basalt_preflight_value,
    })
}

fn nickel_source_value(
    source: &str,
    source_ref: &str,
    export_json: &str,
    export_ref: &str,
    policy_ref: &str,
) -> IOValue {
    record("nickel-source", vec![
        string(HARNESS_POLICY_NICKEL_STATIC_SCHEMA),
        record("source", vec![string(source)]),
        record("source-ref", vec![string(source_ref)]),
        record("export-json", vec![string(export_json)]),
        record("export-ref", vec![string(export_ref)]),
        record("policy-ref", vec![string(policy_ref)]),
    ])
}

fn contract_envelope_value(envelope: &basalt::ContractEnvelope) -> IOValue {
    record("contract-envelope", vec![
        string(&envelope.backend),
        string(&envelope.contract_id),
        string(&envelope.contract_version),
        string(&envelope.normalized_source_hash),
        string(&envelope.input_schema),
        string(&envelope.output_schema),
        string(&envelope.receipt_schema_version),
    ])
}

fn parse_nickel_source_evidence(value: &Value<IOValue>) -> Result<NickelSourceEvidence> {
    let value = value_to_iovalue(value);
    let source = simple_record(&value, "nickel-source", 6)?;
    let schema = required_string(&source[0], "Nickel source schema")?;
    if schema != HARNESS_POLICY_NICKEL_STATIC_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Nickel source schema {schema}; expected {HARNESS_POLICY_NICKEL_STATIC_SCHEMA}"
        )));
    }
    let source_text = required_record_string(&source[1], "source", "Nickel policy source")?;
    let source_ref = required_record_hash(&source[2], "source-ref", "Nickel policy source ref")?;
    let actual_source_ref = canonical_hash(&string(&source_text))?;
    if source_ref != actual_source_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel policy source ref mismatch: evidence has {source_ref}, source hashes to {actual_source_ref}"
        )));
    }
    let export_json = required_record_string(&source[3], "export-json", "Nickel policy export JSON")?;
    let export_ref = required_record_hash(&source[4], "export-ref", "Nickel policy export ref")?;
    let actual_export_ref = canonical_hash(&string(&export_json))?;
    if export_ref != actual_export_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel policy export ref mismatch: evidence has {export_ref}, export hashes to {actual_export_ref}"
        )));
    }
    let actual_export = nickel_export_json(&source_text)?;
    if actual_export != export_json {
        return Err(MoltenError::invalid_harness("Nickel policy export JSON does not match source normalization"));
    }
    let policy_ref = required_record_hash(&source[5], "policy-ref", "Nickel policy source policy ref")?;
    Ok(NickelSourceEvidence {
        source_ref,
        export_ref,
        policy_ref,
    })
}

fn parse_nickel_contract_evidence(value: &Value<IOValue>) -> Result<NickelContractEvidence> {
    let value = value_to_iovalue(value);
    let contract = simple_record(&value, "nickel-contract", 3)?;
    let schema = required_string(&contract[0], "Nickel contract schema")?;
    if schema != HARNESS_POLICY_CONTRACT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Nickel contract schema {schema}; expected {HARNESS_POLICY_CONTRACT_SCHEMA}"
        )));
    }
    let envelope_value = value_to_iovalue(&contract[1]);
    let envelope = parse_contract_envelope(&envelope_value)?;
    let envelope_ref = required_record_hash(&contract[2], "envelope-ref", "Nickel contract envelope ref")?;
    let actual_envelope_ref = canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt rejected Nickel contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(NickelContractEvidence {
        envelope_ref,
        normalized_source_ref: envelope.normalized_source_hash,
    })
}

fn parse_contract_envelope(value: &IOValue) -> Result<basalt::ContractEnvelope> {
    let envelope = simple_record(value, "contract-envelope", 7)?;
    let backend = required_string(&envelope[0], "policy contract backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!("policy preflight requires Nickel backend, got {backend}")));
    }
    let contract_id = required_string(&envelope[1], "policy contract id")?;
    if contract_id != POLICY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract id {contract_id}; expected {POLICY_CONTRACT_ID}"
        )));
    }
    let contract_version = required_string(&envelope[2], "policy contract version")?;
    if contract_version != POLICY_CONTRACT_VERSION {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract version {contract_version}; expected {POLICY_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = required_hash(&envelope[3], "policy contract normalized source ref")?;
    let input_schema = required_string(&envelope[4], "policy contract input schema")?;
    if input_schema != POLICY_INPUT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract input schema {input_schema}; expected {POLICY_INPUT_SCHEMA}"
        )));
    }
    let output_schema = required_string(&envelope[5], "policy contract output schema")?;
    if output_schema != RUNTIME_ADMISSION_DECISION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract output schema {output_schema}; expected {RUNTIME_ADMISSION_DECISION_SCHEMA}"
        )));
    }
    let receipt_schema_version = required_string(&envelope[6], "policy contract receipt schema")?;
    if receipt_schema_version != HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract receipt schema {receipt_schema_version}; expected {HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA}"
        )));
    }
    Ok(basalt::ContractEnvelope::new(
        backend,
        contract_id,
        contract_version,
        normalized_source_hash,
        input_schema,
        output_schema,
        receipt_schema_version,
    ))
}

fn parse_basalt_policy_preflight_evidence(value: &Value<IOValue>) -> Result<BasaltPolicyPreflightEvidence> {
    let value = value_to_iovalue(value);
    let receipt = simple_record(&value, "basalt-preflight", 8)?;
    let schema = required_string(&receipt[0], "Basalt policy preflight schema")?;
    if schema != HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt policy preflight schema {schema}; expected {HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Basalt policy preflight decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt policy preflight decision {decision}")));
    }
    let backend = required_record_string(&receipt[2], "backend", "Basalt policy preflight backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt policy preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_record_string(&receipt[3], "contract-id", "Basalt policy preflight contract id")?;
    if contract_id != POLICY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt policy preflight contract id {contract_id}; expected {POLICY_CONTRACT_ID}"
        )));
    }
    let envelope_ref = required_record_hash(&receipt[4], "envelope-ref", "Basalt policy preflight envelope ref")?;
    let policy_ref = required_record_hash(&receipt[5], "policy-ref", "Basalt policy preflight policy ref")?;
    let normalized_source_ref =
        required_record_hash(&receipt[6], "normalized-source-ref", "Basalt policy preflight source ref")?;
    let reason = required_record_string(&receipt[7], "reason", "Basalt policy preflight reason")?;
    if reason != "accepted" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt policy preflight reason {reason}")));
    }
    Ok(BasaltPolicyPreflightEvidence {
        receipt_ref: canonical_hash(&value)?,
        envelope_ref,
        policy_ref,
        normalized_source_ref,
    })
}

fn nickel_policy_source(policy: &AdmissionPolicy, policy_ref: &str) -> Result<String> {
    let mut source = String::from("{\n");
    source.push_str(&format!("  schema_version = {},\n", nickel_string(HARNESS_POLICY_NICKEL_STATIC_SCHEMA)));
    source.push_str(&format!("  policy_schema = {},\n", nickel_string(HARNESS_POLICY_SCHEMA)));
    source.push_str(&format!("  policy_ref = {},\n", nickel_string(policy_ref)));
    source.push_str("  deny_rules = [\n");
    for rule in policy.deny_rules() {
        source.push_str("    {\n");
        source.push_str(&format!("      actor = {},\n", nickel_optional_string(rule.actor.as_deref())));
        source.push_str(&format!(
            "      action = {},\n",
            nickel_optional_string(rule.action.as_ref().map(AdmissionAction::as_str))
        ));
        source.push_str(&format!("      target = {},\n", nickel_optional_string(rule.target.as_deref())));
        source.push_str(&format!("      value = {},\n", nickel_optional_runtime_value(rule.value.as_ref())?));
        source.push_str(&format!("      reason = {},\n", nickel_string(&rule.reason)));
        source.push_str("    },\n");
    }
    source.push_str("  ],\n}");
    Ok(source)
}

fn nickel_optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_string(), nickel_string)
}

fn nickel_optional_runtime_value(value: Option<&RuntimeValue>) -> Result<String> {
    match value {
        Some(value) => {
            let text = to_text(value.as_iovalue())?;
            let value_ref = canonical_hash(value.as_iovalue())?;
            Ok(format!("{{ preserves = {}, ref = {} }}", nickel_string(&text), nickel_string(&value_ref)))
        }
        None => Ok("null".to_string()),
    }
}

fn nickel_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => escaped.push_str(&format!("\\u{{{:x}}}", character as u32)),
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn nickel_export_json(source: &str) -> Result<String> {
    let mut context = nickel_lang::Context::new();
    let expression = context.eval_deep_for_export(source).map_err(nickel_error)?;
    context.expr_to_json(&expression).map_err(nickel_error)
}

fn nickel_error(error: nickel_lang::Error) -> MoltenError {
    let mut message = Vec::new();
    if error.format(&mut message, nickel_lang::ErrorFormat::Text).is_ok() {
        MoltenError::invalid_harness(format!(
            "Nickel static policy normalization failed: {}",
            String::from_utf8_lossy(&message).trim()
        ))
    } else {
        MoltenError::invalid_harness(format!("Nickel static policy normalization failed: {error:?}"))
    }
}

pub fn capabilities_value(capabilities: &CapabilityContext) -> IOValue {
    record("capabilities-v1", vec![
        string(HARNESS_CAPABILITIES_SCHEMA),
        sequence(capabilities.grants().iter().map(capability_grant_value).collect()),
    ])
}

pub fn capability_gate_value(capabilities: &CapabilityContext) -> Result<IOValue> {
    let preflight = capability_preflight_material(capabilities)?;
    Ok(record("capability-gate-v1", vec![
        string(HARNESS_CAPABILITY_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("capability-ref", vec![string(&preflight.capability_ref)]),
        preflight.authority_contract_value,
        preflight.authority_preflight_value,
        preflight.proofset_value,
        capability_gate_checks_value(),
    ]))
}

pub fn parse_capability_gate(value: &IOValue) -> Result<CapabilityGateEvidence> {
    let gate = simple_record(value, "capability-gate-v1", 7)?;
    let schema = required_string(&gate[0], "capability gate schema")?;
    if schema != HARNESS_CAPABILITY_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability gate schema {schema}; expected {HARNESS_CAPABILITY_GATE_SCHEMA}"
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "capability gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported capability gate decision {decision}")));
    }
    let capability_ref = required_record_hash(&gate[2], "capability-ref", "capability gate capability ref")?;
    let authority_contract = parse_authority_contract_evidence(&gate[3])?;
    let authority_preflight = parse_basalt_authority_preflight_evidence(&gate[4])?;
    let proofset = parse_ucan_proofset_evidence(&gate[5])?;
    if authority_contract.normalized_capability_ref != capability_ref {
        return Err(MoltenError::invalid_harness(
            "authority contract normalized capability ref does not match capability gate ref",
        ));
    }
    if authority_preflight.capability_ref != capability_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt authority preflight capability ref does not match capability gate ref",
        ));
    }
    if authority_preflight.envelope_ref != authority_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt authority preflight envelope ref does not match authority contract envelope",
        ));
    }
    if authority_preflight.proofset_ref != proofset.proofset_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt authority preflight proofset ref does not match UCAN proofset evidence",
        ));
    }
    let checks = parse_capability_gate_checks(&gate[6])?;
    require_capability_gate_check(&checks, "capability-schema")?;
    require_capability_gate_check(&checks, "canonical-capability-context")?;
    require_capability_gate_check(&checks, "deny-by-default")?;
    require_capability_gate_check(&checks, "explicit-capability-fixture")?;
    require_capability_gate_check(&checks, "no-implicit-authority")?;
    require_capability_gate_check(&checks, "basalt-authority-preflight")?;
    require_capability_gate_check(&checks, "basalt-authority-receipt")?;
    require_capability_gate_check(&checks, "capability-proofset-binding")?;
    require_capability_gate_check(&checks, "grant-ref-binding")?;
    Ok(CapabilityGateEvidence {
        value: value.clone(),
        capability_ref,
        authority_preflight_ref: authority_preflight.receipt_ref,
        proofset_ref: proofset.proofset_ref,
        grant_refs: authority_preflight.grant_refs,
        checks,
    })
}

pub fn validate_capability_gate_evidence(
    suite: &HarnessSuite,
    capability_gate: Option<&CapabilityGateEvidence>,
) -> Result<()> {
    if !suite.capabilities_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit capability fixture; implicit authority cannot satisfy evidence gates",
        ));
    }
    let capability_gate = capability_gate.ok_or_else(|| {
        MoltenError::invalid_harness(
            "missing capability gate evidence; authority context must pass preflight before side effects",
        )
    })?;
    let expected_ref = canonical_hash(&capabilities_value(&suite.capabilities))?;
    require_capability_gate_check(&capability_gate.checks, "explicit-capability-fixture")?;
    require_capability_gate_check(&capability_gate.checks, "no-implicit-authority")?;
    if capability_gate.capability_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "capability gate ref mismatch: gate has {}, embedded capabilities hash to {expected_ref}",
            capability_gate.capability_ref
        )));
    }
    let expected_grant_refs = capability_grant_refs(&suite.capabilities)?;
    if capability_gate.grant_refs != expected_grant_refs {
        return Err(MoltenError::invalid_harness("capability gate grant refs do not match embedded capabilities"));
    }
    let expected_gate = capability_gate_value(&suite.capabilities)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&capability_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "capability gate evidence does not match embedded authority preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct CapabilityPreflightMaterial {
    capability_ref: String,
    authority_contract_value: IOValue,
    authority_preflight_value: IOValue,
    proofset_value: IOValue,
}

struct AuthorityContractEvidence {
    envelope_ref: String,
    normalized_capability_ref: String,
}

struct BasaltAuthorityPreflightEvidence {
    receipt_ref: String,
    envelope_ref: String,
    capability_ref: String,
    proofset_ref: String,
    grant_refs: Vec<String>,
}

struct UcanProofsetEvidence {
    proofset_ref: String,
}

const CAPABILITY_CONTRACT_ID: &str = "molten.harness.capability-context";
const CAPABILITY_CONTRACT_VERSION: &str = "v1";
const CAPABILITY_INPUT_SCHEMA: &str = "molten.runtime.admission-request.v1";

fn capability_preflight_material(capabilities: &CapabilityContext) -> Result<CapabilityPreflightMaterial> {
    let capability_snapshot = capabilities_value(capabilities);
    let capability_ref = canonical_hash(&capability_snapshot)?;
    let grant_refs: Vec<String> = capability_grant_refs(capabilities)?;
    let proofset_value = ucan_proofset_value();
    let proofset_ref = canonical_hash(&proofset_value)?;
    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        CAPABILITY_CONTRACT_ID,
        CAPABILITY_CONTRACT_VERSION,
        capability_ref.clone(),
        CAPABILITY_INPUT_SCHEMA,
        RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA,
        HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt authority preflight denied capability contract envelope: {}",
            receipt.reason
        )));
    }
    let authority_contract_value = record("authority-contract", vec![
        string(HARNESS_CAPABILITY_CONTRACT_SCHEMA),
        envelope_value,
        record("envelope-ref", vec![string(&envelope_ref)]),
    ]);
    let mut grant_ref_values = Vec::with_capacity(grant_refs.len());
    for grant_ref in &grant_refs {
        grant_ref_values.push(string(grant_ref.as_str()));
    }
    let authority_preflight_value = record("basalt-authority-preflight", vec![
        string(HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("backend", vec![string("nickel")]),
        record("contract-id", vec![string(CAPABILITY_CONTRACT_ID)]),
        record("envelope-ref", vec![string(envelope_ref)]),
        record("capability-ref", vec![string(&capability_ref)]),
        record("proofset-ref", vec![string(proofset_ref)]),
        record("grant-refs", vec![sequence(grant_ref_values)]),
        record("reason", vec![string(receipt.reason)]),
    ]);
    Ok(CapabilityPreflightMaterial {
        capability_ref,
        authority_contract_value,
        authority_preflight_value,
        proofset_value,
    })
}

fn capability_grant_refs(capabilities: &CapabilityContext) -> Result<Vec<String>> {
    capabilities.grants().iter().map(|grant| canonical_hash(&capability_grant_value(grant))).collect()
}

fn ucan_proofset_value() -> IOValue {
    record("ucan-proofset-v1", vec![string(HARNESS_UCAN_PROOFSET_SCHEMA), sequence(Vec::new())])
}

fn parse_authority_contract_evidence(value: &Value<IOValue>) -> Result<AuthorityContractEvidence> {
    let value = value_to_iovalue(value);
    let contract = simple_record(&value, "authority-contract", 3)?;
    let schema = required_string(&contract[0], "authority contract schema")?;
    if schema != HARNESS_CAPABILITY_CONTRACT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported authority contract schema {schema}; expected {HARNESS_CAPABILITY_CONTRACT_SCHEMA}"
        )));
    }
    let envelope_value = value_to_iovalue(&contract[1]);
    let envelope = parse_capability_contract_envelope(&envelope_value)?;
    let envelope_ref = required_record_hash(&contract[2], "envelope-ref", "authority contract envelope ref")?;
    let actual_envelope_ref = canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "authority contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt rejected authority contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(AuthorityContractEvidence {
        envelope_ref,
        normalized_capability_ref: envelope.normalized_source_hash,
    })
}

fn parse_capability_contract_envelope(value: &IOValue) -> Result<basalt::ContractEnvelope> {
    let envelope = simple_record(value, "contract-envelope", 7)?;
    let backend = required_string(&envelope[0], "capability contract backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "capability authority preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_string(&envelope[1], "capability contract id")?;
    if contract_id != CAPABILITY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract id {contract_id}; expected {CAPABILITY_CONTRACT_ID}"
        )));
    }
    let contract_version = required_string(&envelope[2], "capability contract version")?;
    if contract_version != CAPABILITY_CONTRACT_VERSION {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract version {contract_version}; expected {CAPABILITY_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = required_hash(&envelope[3], "capability contract normalized context ref")?;
    let input_schema = required_string(&envelope[4], "capability contract input schema")?;
    if input_schema != CAPABILITY_INPUT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract input schema {input_schema}; expected {CAPABILITY_INPUT_SCHEMA}"
        )));
    }
    let output_schema = required_string(&envelope[5], "capability contract output schema")?;
    if output_schema != RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract output schema {output_schema}; expected {RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA}"
        )));
    }
    let receipt_schema_version = required_string(&envelope[6], "capability contract receipt schema")?;
    if receipt_schema_version != HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract receipt schema {receipt_schema_version}; expected {HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA}"
        )));
    }
    Ok(basalt::ContractEnvelope::new(
        backend,
        contract_id,
        contract_version,
        normalized_source_hash,
        input_schema,
        output_schema,
        receipt_schema_version,
    ))
}

fn parse_basalt_authority_preflight_evidence(value: &Value<IOValue>) -> Result<BasaltAuthorityPreflightEvidence> {
    let value = value_to_iovalue(value);
    let receipt = simple_record(&value, "basalt-authority-preflight", 9)?;
    let schema = required_string(&receipt[0], "Basalt authority preflight schema")?;
    if schema != HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt authority preflight schema {schema}; expected {HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Basalt authority preflight decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt authority preflight decision {decision}"
        )));
    }
    let backend = required_record_string(&receipt[2], "backend", "Basalt authority preflight backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt authority preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_record_string(&receipt[3], "contract-id", "Basalt authority preflight contract id")?;
    if contract_id != CAPABILITY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt authority preflight contract id {contract_id}; expected {CAPABILITY_CONTRACT_ID}"
        )));
    }
    let envelope_ref = required_record_hash(&receipt[4], "envelope-ref", "Basalt authority preflight envelope ref")?;
    let capability_ref =
        required_record_hash(&receipt[5], "capability-ref", "Basalt authority preflight capability ref")?;
    let proofset_ref = required_record_hash(&receipt[6], "proofset-ref", "Basalt authority preflight proofset ref")?;
    let grant_refs = required_record_hash_sequence(&receipt[7], "grant-refs", "Basalt authority preflight grant refs")?;
    let reason = required_record_string(&receipt[8], "reason", "Basalt authority preflight reason")?;
    if reason != "accepted" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt authority preflight reason {reason}")));
    }
    Ok(BasaltAuthorityPreflightEvidence {
        receipt_ref: canonical_hash(&value)?,
        envelope_ref,
        capability_ref,
        proofset_ref,
        grant_refs,
    })
}

fn parse_ucan_proofset_evidence(value: &Value<IOValue>) -> Result<UcanProofsetEvidence> {
    let value = value_to_iovalue(value);
    let proofset = simple_record(&value, "ucan-proofset-v1", 2)?;
    let schema = required_string(&proofset[0], "UCAN proofset schema")?;
    if schema != HARNESS_UCAN_PROOFSET_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported UCAN proofset schema {schema}; expected {HARNESS_UCAN_PROOFSET_SCHEMA}"
        )));
    }
    let proofs = required_sequence(&proofset[1], "UCAN proofset refs")?;
    if !proofs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "UCAN proof refs require Basalt/UCAN proof validation and are disabled in local harness capability gates",
        ));
    }
    Ok(UcanProofsetEvidence {
        proofset_ref: canonical_hash(&value)?,
    })
}

pub fn admission_authority_evidence(
    capabilities: &CapabilityContext,
    request: &AdmissionRequest,
) -> Result<AdmissionAuthorityEvidence> {
    let authorization = capabilities.authorize(request);
    let grant_ref = authorization
        .grant
        .as_ref()
        .map(|grant| canonical_hash(&capability_grant_value(grant)))
        .transpose()?;
    Ok(AdmissionAuthorityEvidence {
        capability_ref: canonical_hash(&capabilities_value(capabilities))?,
        authorized: authorization.authorized,
        grant_ref,
    })
}

pub fn parse_capabilities(value: &IOValue) -> Result<CapabilityContext> {
    let capabilities = simple_record(value, "capabilities-v1", 2)?;
    let schema = required_string(&capabilities[0], "capabilities schema")?;
    if schema != HARNESS_CAPABILITIES_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capabilities schema {schema}; expected {HARNESS_CAPABILITIES_SCHEMA}"
        )));
    }
    let grant_values = required_sequence(&capabilities[1], "capability grants")?;
    let mut grants = Vec::with_capacity(grant_values.len());
    for grant in grant_values.iter() {
        let grant_value = value_to_iovalue(&grant);
        let grant = simple_record(&grant_value, "grant", 4)?;
        grants.push(CapabilityGrant {
            actor: optional_string(&grant[0], "capability grant actor")?,
            action: optional_action(&grant[1], "capability grant action")?,
            target: optional_string(&grant[2], "capability grant target")?,
            value: optional_runtime_match_value(&grant[3])?,
        });
    }
    Ok(CapabilityContext::from_grants(grants))
}

fn capability_grant_value(grant: &CapabilityGrant) -> IOValue {
    record("grant", vec![
        optional_policy_string(grant.actor.as_deref()),
        optional_policy_action(grant.action.as_ref()),
        optional_policy_string(grant.target.as_deref()),
        optional_policy_runtime_value(grant.value.as_ref()),
    ])
}

fn capability_gate_checks_value() -> IOValue {
    record("checks", vec![sequence(
        [
            "capability-schema",
            "canonical-capability-context",
            "deny-by-default",
            "explicit-capability-fixture",
            "no-implicit-authority",
            "basalt-authority-preflight",
            "basalt-authority-receipt",
            "capability-proofset-binding",
            "grant-ref-binding",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_capability_gate_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "capability gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "capability gate check name")?;
        let status = required_string(&check[1], "capability gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("capability gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_capability_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("capability gate missing {expected} check")))
    }
}

fn deny_rule_value(rule: &AdmissionDenyRule) -> IOValue {
    record("deny", vec![
        optional_policy_string(rule.actor.as_deref()),
        optional_policy_action(rule.action.as_ref()),
        optional_policy_string(rule.target.as_deref()),
        optional_policy_runtime_value(rule.value.as_ref()),
        string(&rule.reason),
    ])
}

fn optional_policy_string(value: Option<&str>) -> IOValue {
    value.map_or_else(|| bool_value(false), string)
}

fn optional_policy_action(value: Option<&AdmissionAction>) -> IOValue {
    value.map_or_else(|| bool_value(false), |action| string(action.as_str()))
}

fn optional_policy_runtime_value(value: Option<&RuntimeValue>) -> IOValue {
    value.map_or_else(|| bool_value(false), |value| value.as_iovalue().clone())
}

fn policy_gate_checks_value() -> IOValue {
    record("checks", vec![sequence(
        [
            "policy-schema",
            "canonical-policy-snapshot",
            "nickel-static-boundary",
            "nickel-policy-source",
            "nickel-export-normalization",
            "basalt-preflight",
            "basalt-receipt-binding",
            "steel-predicate-review",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_policy_gate_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "policy gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "policy gate check name")?;
        let status = required_string(&check[1], "policy gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("policy gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_policy_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("policy gate missing {expected} check")))
    }
}

pub fn parse_policy(value: &IOValue) -> Result<AdmissionPolicy> {
    let policy = simple_record(value, "policy-v1", 2)?;
    let schema = required_string(&policy[0], "policy schema")?;
    if schema != HARNESS_POLICY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy schema {schema}; expected {HARNESS_POLICY_SCHEMA}"
        )));
    }
    let rule_values = required_sequence(&policy[1], "policy deny rules")?;
    let mut rules = Vec::with_capacity(rule_values.len());
    for rule in rule_values.iter() {
        let rule_value = value_to_iovalue(&rule);
        if rule_value.collect_simple_record("steel-predicate", None).is_some()
            || rule_value.collect_simple_record("dynamic-predicate", None).is_some()
        {
            return Err(MoltenError::invalid_harness(
                "Steel predicates require reviewed callable receipts and are disabled in local harness policy fixtures",
            ));
        }
        let rule = simple_record(&rule_value, "deny", 5)?;
        let actor = optional_string(&rule[0], "policy deny actor")?;
        let action = optional_action(&rule[1], "policy deny action")?;
        let target = optional_string(&rule[2], "policy deny target")?;
        let value = optional_runtime_match_value(&rule[3])?;
        let reason = required_string(&rule[4], "policy deny reason")?;
        if reason.is_empty() {
            return Err(MoltenError::invalid_harness("policy deny reason must not be empty"));
        }
        rules.push(AdmissionDenyRule {
            actor,
            action,
            target,
            value,
            reason,
        });
    }
    Ok(AdmissionPolicy::from_deny_rules(rules))
}

pub fn parse_actor_registry(value: &IOValue) -> Result<Vec<ActorDecl>> {
    let registry = simple_record(value, "actor-registry-v1", 2)?;
    let schema = required_string(&registry[0], "actor registry schema")?;
    if schema != HARNESS_ACTOR_REGISTRY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported actor registry schema {schema}; expected {HARNESS_ACTOR_REGISTRY_SCHEMA}"
        )));
    }
    let actor_values = required_sequence(&registry[1], "actor registry entries")?;
    let mut seen = BTreeSet::new();
    let mut actors = Vec::with_capacity(actor_values.len());
    for actor in actor_values.iter() {
        let actor_value = value_to_iovalue(&actor);
        let actor = actor_value
            .collect_simple_record("actor", None)
            .ok_or_else(|| MoltenError::invalid_harness("expected <actor ...> in actor registry"))?;
        let arity = actor.fields_iter().count();
        if arity != 2 && arity != 3 {
            return Err(MoltenError::invalid_harness(format!(
                "actor registry entry arity must be 2 or 3, got {arity}"
            )));
        }
        let id = required_string(&actor[0], "actor id")?;
        if !seen.insert(id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate actor id {id}")));
        }
        let kind = parse_actor_kind(&required_string(&actor[1], "actor kind")?)?;
        let executor = if arity == 3 {
            Some(parse_actor_executor_config(&value_to_iovalue(&actor[2]), &kind, &id)?)
        } else {
            None
        };
        actors.push(ActorDecl { id, kind, executor });
    }
    Ok(actors)
}

fn parse_actor_executor_config(value: &IOValue, kind: &ActorKind, actor_id: &str) -> Result<ActorExecutorConfig> {
    if value.collect_simple_record("steel-executor-v1", None).is_some() {
        if kind != &ActorKind::Steel {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use Steel executor config",
                kind.as_str()
            )));
        }
        return parse_steel_executor_config(value).map(ActorExecutorConfig::Steel);
    }
    if value.collect_simple_record("wasm-executor-v1", None).is_some() {
        if kind != &ActorKind::Wasm {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use Wasm executor config",
                kind.as_str()
            )));
        }
        return parse_wasm_executor_config(value).map(ActorExecutorConfig::Wasm);
    }
    if value.collect_simple_record("adapter-executor-v1", None).is_some() {
        if kind != &ActorKind::Adapter {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use adapter executor config",
                kind.as_str()
            )));
        }
        return parse_adapter_executor_config(value).map(ActorExecutorConfig::Adapter);
    }
    if value.collect_simple_record("remote-proxy-executor-v1", None).is_some() {
        if kind != &ActorKind::RemoteProxy {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use remote-proxy executor config",
                kind.as_str()
            )));
        }
        return parse_remote_proxy_executor_config(value).map(ActorExecutorConfig::RemoteProxy);
    }
    Err(MoltenError::invalid_harness(format!(
        "unsupported executor config for actor {actor_id}; expected <steel-executor-v1 ...>, <wasm-executor-v1 ...>, <adapter-executor-v1 ...>, or <remote-proxy-executor-v1 ...>"
    )))
}

fn parse_steel_executor_config(value: &IOValue) -> Result<SteelExecutorConfig> {
    let config = simple_record(value, "steel-executor-v1", 4)?;
    let schema = required_string(&config[0], "Steel executor schema")?;
    if schema != RUNTIME_STEEL_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Steel executor schema {schema}; expected {RUNTIME_STEEL_EXECUTOR_SCHEMA}"
        )));
    }
    let source = required_record_string(&config[1], "source", "Steel executor source")?;
    let callable = required_record_string(&config[2], "callable", "Steel executor callable")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[3],
        "allowed-hostcalls",
        "Steel executor allowed hostcalls",
    )?)?;
    Ok(SteelExecutorConfig {
        source,
        callable,
        allowed_hostcalls,
    })
}

fn parse_wasm_executor_config(value: &IOValue) -> Result<WasmExecutorConfig> {
    let config = simple_record(value, "wasm-executor-v1", 4)?;
    let schema = required_string(&config[0], "Wasm executor schema")?;
    if schema != RUNTIME_WASM_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm executor schema {schema}; expected {RUNTIME_WASM_EXECUTOR_SCHEMA}"
        )));
    }
    let module_hex = normalize_hex(
        &required_record_string(&config[1], "module-hex", "Wasm executor module hex")?,
        "Wasm executor module hex",
    )?;
    let wit = required_record_string(&config[2], "wit", "Wasm executor WIT interface")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[3],
        "allowed-hostcalls",
        "Wasm executor allowed hostcalls",
    )?)?;
    Ok(WasmExecutorConfig {
        module_hex,
        wit,
        allowed_hostcalls,
    })
}

fn parse_adapter_executor_config(value: &IOValue) -> Result<AdapterExecutorConfig> {
    let config = simple_record(value, "adapter-executor-v1", 5)?;
    let schema = required_string(&config[0], "adapter executor schema")?;
    if schema != RUNTIME_ADAPTER_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported adapter executor schema {schema}; expected {RUNTIME_ADAPTER_EXECUTOR_SCHEMA}"
        )));
    }
    let manifest = required_record_string(&config[1], "manifest", "adapter manifest")?;
    let abi = required_record_string(&config[2], "abi", "adapter ABI")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[3],
        "allowed-hostcalls",
        "adapter allowed hostcalls",
    )?)?;
    let transcript = required_record_string(&config[4], "transcript", "adapter transcript")?;
    Ok(AdapterExecutorConfig {
        manifest,
        abi,
        allowed_hostcalls,
        transcript,
    })
}

fn parse_remote_proxy_executor_config(value: &IOValue) -> Result<RemoteProxyExecutorConfig> {
    let config = simple_record(value, "remote-proxy-executor-v1", 6)?;
    let schema = required_string(&config[0], "remote-proxy executor schema")?;
    if schema != RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported remote-proxy executor schema {schema}; expected {RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA}"
        )));
    }
    let peer = required_record_string(&config[1], "peer", "remote-proxy peer")?;
    let endpoint = required_record_string(&config[2], "endpoint", "remote-proxy endpoint")?;
    let contract = required_record_string(&config[3], "contract", "remote-proxy contract")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[4],
        "allowed-hostcalls",
        "remote-proxy allowed hostcalls",
    )?)?;
    let transcript = required_record_string(&config[5], "transcript", "remote-proxy transcript")?;
    Ok(RemoteProxyExecutorConfig {
        peer,
        endpoint,
        contract,
        allowed_hostcalls,
        transcript,
    })
}

fn normalize_allowed_hostcalls(values: Vec<String>) -> Result<Vec<String>> {
    let mut seen = BTreeSet::new();
    for value in values {
        parse_admission_action(&value)?;
        if !seen.insert(value.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate allowed hostcall {value}")));
        }
    }
    Ok(seen.into_iter().collect())
}

pub fn actor_ids_for_step(step: &CoreStep) -> Vec<&str> {
    step.actor_ids()
}

fn actor_ids_for_event(event: &IOValue) -> Result<Vec<String>> {
    if let Some(message) = event.collect_simple_record("message-delivered", Some(3)) {
        return Ok(vec![
            required_string(&message[0], "message sender")?,
            required_string(&message[1], "message recipient")?,
        ]);
    }
    if let Some(observe) = event.collect_simple_record("observe-registered", Some(2)) {
        return Ok(vec![required_string(&observe[0], "observer actor")?]);
    }
    if let Some(observed) = event.collect_simple_record("assertion-observed", Some(3)) {
        return Ok(vec![
            required_string(&observed[0], "assertion observer")?,
            required_string(&observed[1], "assertion owner")?,
        ]);
    }
    if let Some(assertion) = event.collect_simple_record("assertion-committed", Some(2)) {
        return Ok(vec![required_string(&assertion[0], "assertion actor")?]);
    }
    if let Some(retraction) = event.collect_simple_record("assertion-retracted", Some(2)) {
        return Ok(vec![required_string(&retraction[0], "retraction actor")?]);
    }
    if let Some(observed) = event.collect_simple_record("assertion-retraction-observed", Some(3)) {
        return Ok(vec![
            required_string(&observed[0], "assertion retraction observer")?,
            required_string(&observed[1], "assertion retraction owner")?,
        ]);
    }
    if let Some(request) = event.collect_simple_record("effect-request", None) {
        let arity = request.fields_iter().count();
        if arity != 3 && arity != 4 {
            return Err(MoltenError::invalid_harness(format!("effect-request arity must be 3 or 4, got {arity}")));
        }
        return Ok(vec![required_string(&request[1], "effect request actor")?]);
    }
    if let Some(response) = event.collect_simple_record("effect-response", None) {
        let arity = response.fields_iter().count();
        if arity != 4 && arity != 5 {
            return Err(MoltenError::invalid_harness(format!("effect-response arity must be 4 or 5, got {arity}")));
        }
        return Ok(vec![required_string(&response[1], "effect response actor")?]);
    }
    if let Some(rollback) = event.collect_simple_record("turn-rolled-back", Some(2)) {
        return Ok(vec![required_string(&rollback[0], "rollback actor")?]);
    }
    if event.collect_simple_record("admission-decision-v1", None).is_some() {
        let decision = parse_admission_decision_event(event)?;
        let mut actors = vec![decision.request.actor];
        if matches!(&decision.request.action, AdmissionAction::Send)
            && let Some(target) = decision.request.target
        {
            actors.push(target);
        }
        return Ok(actors);
    }
    if let Some(input) = event.collect_simple_record("actor-input-v1", Some(9)) {
        let actor_value = value_to_iovalue(&input[1]);
        let actor = simple_record(&actor_value, "actor", 2)?;
        return Ok(vec![required_string(&actor[0], "actor input actor")?]);
    }
    if let Some(request) = event.collect_simple_record("hostcall-request-v1", None) {
        let arity = request.fields_iter().count();
        if arity != 9 && arity != 11 && arity != 15 {
            return Err(MoltenError::invalid_harness(format!(
                "hostcall-request arity must be 9, 11, or 15, got {arity}"
            )));
        }
        let parsed_request = parse_admission_request(&request[4])?;
        let mut actors = vec![parsed_request.actor];
        if matches!(&parsed_request.action, AdmissionAction::Send)
            && let Some(target) = parsed_request.target
        {
            actors.push(target);
        }
        return Ok(actors);
    }
    if let Some(output) = event.collect_simple_record("actor-output-v1", Some(8)) {
        return Ok(vec![required_record_string(&output[1], "actor", "actor output actor")?]);
    }
    if let Some(receipt) = event.collect_simple_record("steel-execution-receipt-v1", None) {
        return Ok(vec![required_record_string(&receipt[1], "actor", "Steel execution actor")?]);
    }
    if let Some(receipt) = event.collect_simple_record("wasm-execution-receipt-v1", None) {
        return Ok(vec![required_record_string(&receipt[1], "actor", "Wasm execution actor")?]);
    }
    Ok(Vec::new())
}

fn require_declared_actor(
    actor_ids: &BTreeSet<&str>,
    actor: &str,
    context: &str,
    observation: Option<usize>,
) -> Result<()> {
    if actor_ids.contains(actor) {
        return Ok(());
    }
    let location = observation.map_or_else(String::new, |position| format!(" at observation {position}"));
    Err(MoltenError::invalid_harness(format!(
        "actor {actor} in {context}{location} is not declared in explicit actor registry"
    )))
}

fn infer_actor_registry(steps: &[CoreStep]) -> Vec<ActorDecl> {
    let mut ids = BTreeSet::new();
    for step in steps {
        for actor in actor_ids_for_step(step) {
            ids.insert(actor.to_owned());
        }
    }
    ids.into_iter()
        .map(|id| ActorDecl {
            id,
            kind: ActorKind::Native,
            executor: None,
        })
        .collect()
}

fn parse_admission_action(action: &str) -> Result<AdmissionAction> {
    match action {
        "send" => Ok(AdmissionAction::Send),
        "observe" => Ok(AdmissionAction::Observe),
        "assert" => Ok(AdmissionAction::Assert),
        "retract" => Ok(AdmissionAction::Retract),
        "clock" => Ok(AdmissionAction::Clock),
        "random" => Ok(AdmissionAction::Random),
        other => Err(MoltenError::invalid_harness(format!("unknown admission action {other}"))),
    }
}

fn parse_actor_kind(kind: &str) -> Result<ActorKind> {
    match kind {
        "native" => Ok(ActorKind::Native),
        "steel" => Ok(ActorKind::Steel),
        "wasm" => Ok(ActorKind::Wasm),
        "adapter" => Ok(ActorKind::Adapter),
        "remote-proxy" => Ok(ActorKind::RemoteProxy),
        other => Err(MoltenError::invalid_harness(format!("unknown actor kind {other}"))),
    }
}

fn parse_legacy_report_repro_bundle(
    bundle_value: &IOValue,
    bundle: &Record<Value<IOValue>>,
) -> Result<HarnessReproBundle> {
    let report_ref = required_string(&bundle[1], "repro bundle report ref")?;
    let suite_ref = required_string(&bundle[2], "repro bundle suite ref")?;
    let initial_state_hash = required_hash(&bundle[3], "repro bundle initial state hash")?;
    let final_state_hash = required_hash(&bundle[4], "repro bundle final state hash")?;
    let replay_status = required_string(&bundle[5], "repro bundle replay status")?;
    let profile = required_string(&bundle[6], "repro bundle profile")?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[7]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[8]))?;
    let suite_value = value_to_iovalue(&bundle[9]);
    let report_value = value_to_iovalue(&bundle[10]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(HarnessReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: HarnessReproBundleKind::Report,
        artifact_ref: report_ref,
        report_value: Some(report_value),
        failure_value: None,
        gate_receipt_ref: None,
        gate_receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: None,
        encrypted_refs: Vec::new(),
    })
}

fn parse_report_repro_bundle(bundle_value: &IOValue, bundle: &Record<Value<IOValue>>) -> Result<HarnessReproBundle> {
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(MoltenError::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let report_ref = required_string(&bundle[6], "repro bundle report ref")?;
    let suite_ref = required_string(&bundle[7], "repro bundle suite ref")?;
    require_artifact_ref(&artifact_refs, "report", &report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &suite_ref)?;
    let initial_state_hash = required_hash(&bundle[8], "repro bundle initial state hash")?;
    let final_state_hash = required_hash(&bundle[9], "repro bundle final state hash")?;
    let replay_status = required_string(&bundle[10], "repro bundle replay status")?;
    let profile = required_string(&bundle[11], "repro bundle profile")?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[12]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[13]))?;
    let suite_value = value_to_iovalue(&bundle[14]);
    let report_value = value_to_iovalue(&bundle[15]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(HarnessReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: HarnessReproBundleKind::Report,
        artifact_ref: report_ref,
        report_value: Some(report_value),
        failure_value: None,
        gate_receipt_ref: None,
        gate_receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: None,
        encrypted_refs: Vec::new(),
    })
}

fn parse_sealed_report_repro_bundle(
    bundle_value: &IOValue,
    bundle: &Record<Value<IOValue>>,
) -> Result<HarnessReproBundle> {
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(MoltenError::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let report_ref = required_string(&bundle[6], "repro bundle report ref")?;
    let suite_ref = required_string(&bundle[7], "repro bundle suite ref")?;
    require_artifact_ref(&artifact_refs, "report", &report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &suite_ref)?;
    let initial_state_hash = required_hash(&bundle[8], "repro bundle initial state hash")?;
    let final_state_hash = required_hash(&bundle[9], "repro bundle final state hash")?;
    let replay_status = required_string(&bundle[10], "repro bundle replay status")?;
    let profile = required_string(&bundle[11], "repro bundle profile")?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[12]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[13]))?;
    let suite_value = value_to_iovalue(&bundle[14]);
    let report_value = value_to_iovalue(&bundle[15]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    require_report_artifact_refs(&artifact_refs, &report)?;
    let arity = bundle.fields_iter().count();
    let (redaction_policy_ref, redaction_gate_ref, seal_index, receipt_index, checks_index) = if arity == 21 {
        let redaction_policy_value = value_to_iovalue(&bundle[16]);
        let redaction_gate_value = value_to_iovalue(&bundle[17]);
        let (redaction_policy_ref, redaction_gate_ref) =
            validate_redaction_evidence(&report_value, &report, &redaction_policy_value, &redaction_gate_value)?;
        require_artifact_ref(&artifact_refs, "redaction-policy", &redaction_policy_ref)?;
        require_artifact_ref(&artifact_refs, "redaction-gate", &redaction_gate_ref)?;
        (Some(redaction_policy_ref), Some(redaction_gate_ref), 18, 19, 20)
    } else {
        (None, None, 16, 17, 18)
    };
    let seal = parse_repro_seal(&bundle[seal_index], &report_ref, &suite_ref, &profile, &replay_status)?;
    require_artifact_ref(&artifact_refs, "gate-receipt", &seal.gate_receipt_ref)?;
    let gate_receipt_value = value_to_iovalue(&bundle[receipt_index]);
    let actual_gate_receipt_ref = canonical_hash(&gate_receipt_value)?;
    if actual_gate_receipt_ref != seal.gate_receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle gate receipt ref mismatch: seal has {}, embedded receipt hashes to {actual_gate_receipt_ref}",
            seal.gate_receipt_ref
        )));
    }
    let seal_checks = parse_seal_checks(&bundle[checks_index])?;
    require_seal_check(&seal_checks, "sealed-report")?;
    require_seal_check(&seal_checks, "embedded-gate-receipt")?;
    require_seal_check(&seal_checks, "report-ref-binding")?;
    require_seal_check(&seal_checks, "suite-ref-binding")?;
    require_seal_check(&seal_checks, "actor-registry-binding")?;
    require_seal_check(&seal_checks, "effect-log-binding")?;
    require_seal_check(&seal_checks, "policy-gate-ref-binding")?;
    require_seal_check(&seal_checks, "capability-gate-ref-binding")?;
    require_seal_check(&seal_checks, "budget-gate-ref-binding")?;
    if arity == 21 {
        require_seal_check(&seal_checks, "redaction-preflight")?;
        require_seal_check(&seal_checks, "redaction-gate-ref-binding")?;
        require_seal_check(&seal_checks, "no-sensitive-markers")?;
    }
    require_seal_check(&seal_checks, "replay-metadata-binding")?;
    Ok(HarnessReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: HarnessReproBundleKind::Report,
        artifact_ref: report_ref,
        report_value: Some(report_value),
        failure_value: None,
        gate_receipt_ref: Some(seal.gate_receipt_ref),
        gate_receipt_value: Some(gate_receipt_value),
        redaction_policy_ref,
        redaction_gate_ref,
        export_profile: Some(ReproExportProfile::DenySensitive.as_str().to_string()),
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: Some(ReproExportProfile::DenySensitive.loss_classification().to_string()),
        encrypted_refs: Vec::new(),
    })
}

fn parse_profiled_report_repro_bundle(
    bundle_value: &IOValue,
    bundle: &Record<Value<IOValue>>,
) -> Result<HarnessReproBundle> {
    let arity = bundle.fields_iter().count();
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(MoltenError::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let source_report_ref = required_hash(&bundle[6], "profiled repro source report ref")?;
    let source_suite_ref = required_hash(&bundle[7], "profiled repro source suite ref")?;
    let output_report_ref = required_hash(&bundle[8], "profiled repro output report ref")?;
    let output_suite_ref = required_hash(&bundle[9], "profiled repro output suite ref")?;
    require_artifact_ref(&artifact_refs, "source-report", &source_report_ref)?;
    require_artifact_ref(&artifact_refs, "source-suite", &source_suite_ref)?;
    require_artifact_ref(&artifact_refs, "report", &output_report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &output_suite_ref)?;
    let initial_state_hash = required_hash(&bundle[10], "profiled repro initial state hash")?;
    let final_state_hash = required_hash(&bundle[11], "profiled repro final state hash")?;
    let replay_status = required_string(&bundle[12], "profiled repro replay status")?;
    let run_profile = required_string(&bundle[13], "profiled repro harness profile")?;
    let export_profile_value = value_to_iovalue(&bundle[14]);
    let export_profile = parse_repro_export_profile(&export_profile_value)?;
    require_artifact_ref(&artifact_refs, "export-profile", &export_profile.profile_ref)?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[15]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[16]))?;
    let suite_value = value_to_iovalue(&bundle[17]);
    let report_value = value_to_iovalue(&bundle[18]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &output_report_ref,
        suite_ref: &output_suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &run_profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    require_report_artifact_refs(&artifact_refs, &report)?;
    let policy_value = value_to_iovalue(&bundle[19]);
    parse_redaction_policy(&policy_value)?;
    let policy_ref = canonical_hash(&policy_value)?;
    require_artifact_ref(&artifact_refs, "redaction-policy", &policy_ref)?;
    let manifest_value = value_to_iovalue(&bundle[20]);
    let manifest_ref = canonical_hash(&manifest_value)?;
    require_artifact_ref(&artifact_refs, "redaction-transform-manifest", &manifest_ref)?;
    let transform_receipt_value = value_to_iovalue(&bundle[21]);
    let transform_receipt = parse_redaction_transform_receipt(&transform_receipt_value)?;
    require_artifact_ref(&artifact_refs, "redaction-transform", &transform_receipt.receipt_ref)?;
    if transform_receipt.source_report_ref != source_report_ref
        || transform_receipt.source_suite_ref != source_suite_ref
        || transform_receipt.policy_ref != policy_ref
        || transform_receipt.profile != export_profile.profile
        || transform_receipt.manifest_ref != manifest_ref
        || transform_receipt.output_bundle_ref != output_report_ref
        || transform_receipt.loss_classification != export_profile.loss_classification
        || export_profile.is_gate_preserving
        || export_profile.requires_reveal != export_profile.profile.requires_reveal()
    {
        return Err(MoltenError::invalid_harness(
            "redaction transform receipt binding does not match profiled repro bundle",
        ));
    }
    validate_redaction_transform_manifest(
        &manifest_value,
        &source_report_ref,
        &source_suite_ref,
        &report,
        export_profile.profile,
    )?;
    let output_encrypted_refs = validate_profiled_output(&report_value, export_profile.profile)?;
    if output_encrypted_refs != transform_receipt.encrypted_refs {
        return Err(MoltenError::invalid_harness(
            "redaction transform encrypted-ref inventory does not match output bundle",
        ));
    }
    let output_marker_refs = collect_redaction_marker_refs(&report_value)?;
    if output_marker_refs != transform_receipt.marker_refs {
        return Err(MoltenError::invalid_harness(
            "redaction transform marker manifest does not cover output bundle markers",
        ));
    }
    let (private_bundle_profile_ref, private_bundle_profile_value, checks_index) = if arity == 24 {
        let private_value = value_to_iovalue(&bundle[22]);
        let private = parse_private_bundle_profile(&private_value)?;
        require_artifact_ref(&artifact_refs, "private-bundle-profile", &canonical_hash(&private_value)?)?;
        if export_profile.profile != ReproExportProfile::EncryptedPrivate {
            return Err(MoltenError::invalid_harness(
                "private bundle profile is only valid for encrypted-private repro exports",
            ));
        }
        if private.transform_receipt_ref != transform_receipt.receipt_ref
            || private.encrypted_refs != transform_receipt.encrypted_refs
            || private.is_gate_preserving
        {
            return Err(MoltenError::invalid_harness(
                "private bundle profile does not bind encrypted refs and diagnostic-only transform receipt",
            ));
        }
        (Some(canonical_hash(&private_value)?), Some(private_value), 23)
    } else {
        if export_profile.profile == ReproExportProfile::EncryptedPrivate {
            return Err(MoltenError::invalid_harness(
                "encrypted-private repro bundle missing private bundle profile evidence",
            ));
        }
        (None, None, 22)
    };
    let checks = parse_seal_checks(&bundle[checks_index])?;
    require_seal_check(&checks, "profile-schema")?;
    require_seal_check(&checks, "redaction-transform-receipt")?;
    require_seal_check(&checks, "transform-manifest-bound")?;
    require_seal_check(&checks, "source-report-ref-binding")?;
    require_seal_check(&checks, "output-report-ref-binding")?;
    require_seal_check(&checks, "no-forbidden-cleartext")?;
    match export_profile.profile {
        ReproExportProfile::DenySensitive => require_seal_check(&checks, "gate-preserving")?,
        ReproExportProfile::RedactedDiagnostic => require_seal_check(&checks, "diagnostic-only")?,
        ReproExportProfile::EncryptedPrivate => {
            require_seal_check(&checks, "requires-reveal")?;
            require_seal_check(&checks, "encrypted-ref-validation")?;
        }
    }
    Ok(HarnessReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: HarnessReproBundleKind::Report,
        artifact_ref: output_report_ref,
        report_value: Some(report_value),
        failure_value: None,
        gate_receipt_ref: None,
        gate_receipt_value: None,
        redaction_policy_ref: Some(policy_ref),
        redaction_gate_ref: None,
        export_profile: Some(export_profile.profile.as_str().to_string()),
        export_profile_ref: Some(export_profile.profile_ref),
        export_profile_value: Some(export_profile_value),
        source_report_ref: Some(source_report_ref),
        source_suite_ref: Some(source_suite_ref),
        redaction_transform_manifest_ref: Some(manifest_ref),
        redaction_transform_manifest_value: Some(manifest_value),
        redaction_transform_receipt_ref: Some(transform_receipt.receipt_ref.clone()),
        redaction_transform_receipt_value: Some(transform_receipt.value.clone()),
        private_bundle_profile_ref,
        private_bundle_profile_value,
        loss_classification: Some(transform_receipt.loss_classification.clone()),
        encrypted_refs: transform_receipt.encrypted_refs.clone(),
    })
}

fn validate_redaction_transform_manifest(
    value: &IOValue,
    source_report_ref: &str,
    source_suite_ref: &str,
    report: &HarnessReport,
    profile: ReproExportProfile,
) -> Result<()> {
    let manifest = simple_record(value, "redaction-transform-manifest-v1", 9)?;
    let schema = required_string(&manifest[0], "redaction transform manifest schema")?;
    if schema != HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction transform manifest schema {schema}; expected {HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA}"
        )));
    }
    let manifest_source_report = required_record_hash(&manifest[1], "source-report", "manifest source report")?;
    let manifest_source_suite = required_record_hash(&manifest[2], "source-suite", "manifest source suite")?;
    let manifest_output_report = required_record_hash(&manifest[3], "output-report", "manifest output report")?;
    let manifest_output_suite = required_record_hash(&manifest[4], "output-suite", "manifest output suite")?;
    let manifest_profile = required_record_string(&manifest[5], "profile", "manifest profile")?;
    if manifest_source_report != source_report_ref
        || manifest_source_suite != source_suite_ref
        || manifest_output_report != report.report_ref
        || manifest_output_suite != report.suite_ref
        || manifest_profile != profile.as_str()
    {
        return Err(MoltenError::invalid_harness("redaction transform manifest binding mismatch"));
    }
    validate_sequence_record(&manifest[6], "markers", "redaction transform manifest markers")?;
    let manifest_encrypted_refs =
        required_record_hash_sequence(&manifest[7], "encrypted-refs", "manifest encrypted refs")?;
    if profile == ReproExportProfile::EncryptedPrivate && manifest_encrypted_refs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "encrypted-private repro bundle transform manifest missing encrypted refs",
        ));
    }
    let checks = parse_redaction_gate_checks(&manifest[8])?;
    require_redaction_check(&checks, "source-report-bound")?;
    require_redaction_check(&checks, "output-report-bound")?;
    require_redaction_check(&checks, "deterministic-traversal-order")?;
    require_redaction_check(&checks, "marker-coverage-manifest")?;
    require_redaction_check(&checks, "encrypted-ref-inventory")?;
    Ok(())
}

fn collect_redaction_marker_refs(value: &IOValue) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(8);
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.collect_simple_record("redaction-marker-v1", None).is_some() {
            ensure_redaction_bound(refs.len() + 1, MAX_REDACTION_MARKER_REFS, "redaction marker refs")?;
            refs.push(parse_redaction_marker(&current)?.marker_ref);
            continue;
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record)
            | ValueClass::Compound(CompoundClass::Sequence)
            | ValueClass::Compound(CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(value_to_iovalue(&child));
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(value_to_iovalue(&key));
                    stack.push(value_to_iovalue(&value));
                }
            }
        }
    }
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn parse_failure_repro_bundle(bundle_value: &IOValue, bundle: &Record<Value<IOValue>>) -> Result<HarnessReproBundle> {
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "failure" {
        return Err(MoltenError::invalid_harness(format!("expected failure repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let failure_ref = required_string(&bundle[6], "repro bundle failure ref")?;
    require_artifact_ref(&artifact_refs, "failure", &failure_ref)?;
    let failure_value = value_to_iovalue(&bundle[7]);
    let failure = parse_failure(&failure_value)?;
    if failure.failure_ref != failure_ref {
        return Err(MoltenError::invalid_harness(format!(
            "failure repro bundle ref mismatch: bundle has {failure_ref}, embedded failure hashes to {}",
            failure.failure_ref
        )));
    }
    Ok(HarnessReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: HarnessReproBundleKind::Failure,
        artifact_ref: failure_ref,
        report_value: None,
        failure_value: Some(failure_value),
        gate_receipt_ref: None,
        gate_receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: Some("diagnostic-only".to_string()),
        encrypted_refs: Vec::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReproSeal {
    gate_receipt_ref: String,
}

fn parse_repro_seal(
    value: &Value<IOValue>,
    report_ref: &str,
    suite_ref: &str,
    profile: &str,
    replay_status: &str,
) -> Result<ReproSeal> {
    let value = value_to_iovalue(value);
    let seal = simple_record(&value, "repro-seal", 7)?;
    let schema = required_string(&seal[0], "repro seal schema")?;
    if schema != HARNESS_REPRO_SEAL_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro seal schema {schema}; expected {HARNESS_REPRO_SEAL_SCHEMA}"
        )));
    }
    let decision = required_record_string(&seal[1], "decision", "repro seal decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported repro seal decision {decision}")));
    }
    let gate_receipt_ref = required_record_hash(&seal[2], "gate-receipt-ref", "repro seal gate receipt ref")?;
    let sealed_report_ref = required_record_hash(&seal[3], "report-ref", "repro seal report ref")?;
    if sealed_report_ref != report_ref {
        return Err(MoltenError::invalid_harness("repro seal report ref does not match bundle report ref"));
    }
    let sealed_suite_ref = required_record_hash(&seal[4], "suite-ref", "repro seal suite ref")?;
    if sealed_suite_ref != suite_ref {
        return Err(MoltenError::invalid_harness("repro seal suite ref does not match bundle suite ref"));
    }
    let sealed_profile = required_record_string(&seal[5], "profile", "repro seal profile")?;
    if sealed_profile != profile {
        return Err(MoltenError::invalid_harness("repro seal profile does not match bundle profile"));
    }
    let sealed_replay_status = required_record_string(&seal[6], "replay-status", "repro seal replay status")?;
    if sealed_replay_status != replay_status {
        return Err(MoltenError::invalid_harness("repro seal replay status does not match bundle replay status"));
    }
    Ok(ReproSeal { gate_receipt_ref })
}

fn parse_seal_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "seal-checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "repro seal checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "repro seal check name")?;
        let status = required_string(&check[1], "repro seal check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("repro seal check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_seal_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("repro seal missing {expected} check")))
    }
}

struct ReproReportMatchInput<'a> {
    report: &'a HarnessReport,
    report_ref: &'a str,
    suite_ref: &'a str,
    initial_state_hash: &'a str,
    final_state_hash: &'a str,
    replay_status: &'a str,
    profile: &'a str,
    actors: &'a [ActorDecl],
    effect_log: &'a [EffectLogEntry],
    suite_value: &'a IOValue,
}

fn require_report_artifact_refs(refs: &[(String, String)], report: &HarnessReport) -> Result<()> {
    for (kind, artifact_ref) in report_artifact_refs(report, None, None)? {
        require_artifact_ref(refs, &kind, &artifact_ref)?;
    }
    Ok(())
}

fn require_repro_report_matches(input: &ReproReportMatchInput<'_>) -> Result<()> {
    if input.report.report_ref != input.report_ref {
        return Err(MoltenError::invalid_harness(format!(
            "repro bundle report ref mismatch: bundle has {}, embedded report hashes to {}",
            input.report_ref, input.report.report_ref
        )));
    }
    if input.report.suite_ref != input.suite_ref {
        return Err(MoltenError::invalid_harness("repro bundle suite ref does not match embedded report"));
    }
    if input.report.initial_state_hash != input.initial_state_hash
        || input.report.final_state_hash != input.final_state_hash
    {
        return Err(MoltenError::invalid_harness("repro bundle state refs do not match embedded report"));
    }
    if input.report.replay_status != input.replay_status || input.report.profile != input.profile {
        return Err(MoltenError::invalid_harness(
            "repro bundle replay/profile metadata does not match embedded report",
        ));
    }
    if input.report.actors != input.actors {
        return Err(MoltenError::invalid_harness("repro bundle actor registry does not match embedded report"));
    }
    if input.report.effect_log != input.effect_log {
        return Err(MoltenError::invalid_harness("repro bundle effect log does not match embedded report"));
    }
    if &input.report.suite_value != input.suite_value {
        return Err(MoltenError::invalid_harness("repro bundle suite value does not match embedded report"));
    }
    Ok(())
}

fn tool_value() -> IOValue {
    record("tool", vec![string("molten"), string(env!("CARGO_PKG_VERSION"))])
}

fn command_value(command: &[String]) -> IOValue {
    record("command", vec![sequence(command.iter().map(string).collect())])
}

fn replay_instructions_value(instructions: &[&[&str]]) -> IOValue {
    record("replay-instructions", vec![sequence(
        instructions
            .iter()
            .map(|instruction| sequence(instruction.iter().map(|part| string(*part)).collect()))
            .collect(),
    )])
}

fn artifact_refs_value(refs: &[(&str, &str)]) -> IOValue {
    record("artifact-refs", vec![sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| record("artifact-ref", vec![string(*kind), string(*artifact_ref)]))
            .collect(),
    )])
}

fn artifact_refs_owned_value(refs: &[(String, String)]) -> IOValue {
    record("artifact-refs", vec![sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| record("artifact-ref", vec![string(kind), string(artifact_ref)]))
            .collect(),
    )])
}

fn report_artifact_refs_value(
    report: &HarnessReport,
    gate_receipt_ref: Option<&str>,
    redaction_refs: Option<(&str, &str)>,
) -> Result<IOValue> {
    Ok(artifact_refs_owned_value(&report_artifact_refs(report, gate_receipt_ref, redaction_refs)?))
}

fn report_artifact_refs(
    report: &HarnessReport,
    gate_receipt_ref: Option<&str>,
    redaction_refs: Option<(&str, &str)>,
) -> Result<Vec<(String, String)>> {
    let policy_gate = report
        .policy_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing policy gate evidence"))?;
    let capability_gate = report
        .capability_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing capability gate evidence"))?;
    let budget_gate = report
        .budget_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing budget gate evidence"))?;
    let executor_preflights = report
        .executor_preflights
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing executor preflight evidence"))?;
    let mut refs = vec![
        ("report".to_string(), report.report_ref.clone()),
        ("suite".to_string(), report.suite_ref.clone()),
        ("initial-state".to_string(), report.initial_state_hash.clone()),
        ("final-state".to_string(), report.final_state_hash.clone()),
        ("actor-registry".to_string(), canonical_hash(&actor_registry_value(&report.actors))?),
        ("executor-preflights".to_string(), canonical_hash(&executor_preflights.value)?),
        ("effect-log".to_string(), canonical_hash(&effect_log_value(&report.effect_log))?),
        ("policy".to_string(), policy_gate.policy_ref.clone()),
        ("policy-gate".to_string(), canonical_hash(&policy_gate.value)?),
        ("policy-nickel-source".to_string(), policy_gate.nickel_source_ref.clone()),
        ("policy-nickel-export".to_string(), policy_gate.nickel_export_ref.clone()),
        ("policy-basalt-preflight".to_string(), policy_gate.basalt_preflight_ref.clone()),
        ("budget".to_string(), budget_gate.budget_ref.clone()),
        ("budget-gate".to_string(), canonical_hash(&budget_gate.value)?),
        ("budget-nickel-source".to_string(), budget_gate.nickel_source_ref.clone()),
        ("budget-nickel-export".to_string(), budget_gate.nickel_export_ref.clone()),
        ("budget-basalt-preflight".to_string(), budget_gate.basalt_preflight_ref.clone()),
        ("capabilities".to_string(), capability_gate.capability_ref.clone()),
        ("capability-gate".to_string(), canonical_hash(&capability_gate.value)?),
        ("capability-authority-preflight".to_string(), capability_gate.authority_preflight_ref.clone()),
        ("ucan-proofset".to_string(), capability_gate.proofset_ref.clone()),
    ];
    if let Some(gate_receipt_ref) = gate_receipt_ref {
        refs.push(("gate-receipt".to_string(), gate_receipt_ref.to_string()));
    }
    if let Some((redaction_policy_ref, redaction_gate_ref)) = redaction_refs {
        refs.push(("redaction-policy".to_string(), redaction_policy_ref.to_string()));
        refs.push(("redaction-gate".to_string(), redaction_gate_ref.to_string()));
    }
    Ok(refs)
}

const FORBIDDEN_REDACTION_MARKERS: &[&str] = &[
    "secret",
    "confidential",
    "credential",
    "private",
    "encrypted-ref",
    "encrypted-ref-v1",
    "secret-ref-v1",
];

struct ReproExportProfileEvidence {
    profile: ReproExportProfile,
    profile_ref: String,
    loss_classification: String,
    is_gate_preserving: bool,
    requires_reveal: bool,
}

struct RedactionTransformReceiptInput<'a> {
    source_report_ref: &'a str,
    source_suite_ref: &'a str,
    policy_ref: &'a str,
    profile: ReproExportProfile,
    manifest_ref: &'a str,
    output_bundle_ref: &'a str,
    marker_refs: &'a [String],
    encrypted_refs: &'a [String],
}

struct RedactionTransformReceiptEvidence {
    receipt_ref: String,
    source_report_ref: String,
    source_suite_ref: String,
    policy_ref: String,
    profile: ReproExportProfile,
    manifest_ref: String,
    output_bundle_ref: String,
    loss_classification: String,
    marker_refs: Vec<String>,
    encrypted_refs: Vec<String>,
    value: IOValue,
}

fn redaction_policy_value() -> IOValue {
    record("redaction-policy-v1", vec![
        string(HARNESS_REDACTION_POLICY_SCHEMA),
        record("mode", vec![string("deny-sensitive-markers")]),
        record("forbidden-markers", vec![sequence(
            FORBIDDEN_REDACTION_MARKERS.iter().map(|marker| string(*marker)).collect(),
        )]),
    ])
}

fn repro_export_profile_value(profile: ReproExportProfile) -> IOValue {
    record("repro-export-profile-v1", vec![
        string(HARNESS_REDACTION_PROFILE_SCHEMA),
        record("name", vec![string(profile.as_str())]),
        record("loss-classification", vec![string(profile.loss_classification())]),
        record("gate-preserving", vec![bool_value(profile.is_gate_preserving())]),
        record("requires-reveal", vec![bool_value(profile.requires_reveal())]),
        checks_value_for_names(&[
            "explicit-export-profile",
            "loss-classification-bound",
            "gate-preserving-bound",
            "reveal-requirement-bound",
        ]),
    ])
}

fn parse_repro_export_profile(value: &IOValue) -> Result<ReproExportProfileEvidence> {
    let profile_value = simple_record(value, "repro-export-profile-v1", 6)?;
    let schema = required_string(&profile_value[0], "repro export profile schema")?;
    if schema != HARNESS_REDACTION_PROFILE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro export profile schema {schema}; expected {HARNESS_REDACTION_PROFILE_SCHEMA}"
        )));
    }
    let name = required_record_string(&profile_value[1], "name", "repro export profile name")?;
    let profile = ReproExportProfile::parse(&name)?;
    let loss_classification =
        required_record_string(&profile_value[2], "loss-classification", "repro export loss classification")?;
    if loss_classification != profile.loss_classification() {
        return Err(MoltenError::invalid_harness("repro export profile loss classification is not canonical"));
    }
    let is_gate_preserving =
        required_record_bool(&profile_value[3], "gate-preserving", "repro export gate preserving flag")?;
    if is_gate_preserving != profile.is_gate_preserving() {
        return Err(MoltenError::invalid_harness("repro export gate-preserving flag is not canonical"));
    }
    let is_requires_reveal = required_record_bool(&profile_value[4], "requires-reveal", "repro export reveal flag")?;
    if is_requires_reveal != profile.requires_reveal() {
        return Err(MoltenError::invalid_harness("repro export reveal flag is not canonical"));
    }
    let checks = parse_redaction_gate_checks(&profile_value[5])?;
    require_redaction_check(&checks, "explicit-export-profile")?;
    require_redaction_check(&checks, "loss-classification-bound")?;
    require_redaction_check(&checks, "gate-preserving-bound")?;
    require_redaction_check(&checks, "reveal-requirement-bound")?;
    Ok(ReproExportProfileEvidence {
        profile,
        profile_ref: canonical_hash(value)?,
        loss_classification,
        is_gate_preserving,
        requires_reveal: is_requires_reveal,
    })
}

fn redaction_transform_manifest_value(
    source_report: &HarnessReport,
    output_report: &HarnessReport,
    profile: ReproExportProfile,
    entries: &[RedactionManifestEntry],
    encrypted_refs: &[String],
) -> IOValue {
    record("redaction-transform-manifest-v1", vec![
        string(HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA),
        record("source-report", vec![string(&source_report.report_ref)]),
        record("source-suite", vec![string(&source_report.suite_ref)]),
        record("output-report", vec![string(&output_report.report_ref)]),
        record("output-suite", vec![string(&output_report.suite_ref)]),
        record("profile", vec![string(profile.as_str())]),
        record("markers", vec![sequence(
            entries
                .iter()
                .map(|entry| {
                    record("redaction", vec![
                        string(&entry.path),
                        string(&entry.reason),
                        string(&entry.commitment_ref),
                        optional_ref_value(entry.marker_ref.as_deref()),
                        optional_ref_value(entry.encrypted_ref.as_deref()),
                    ])
                })
                .collect(),
        )]),
        record("encrypted-refs", vec![refs_sequence(encrypted_refs)]),
        checks_value_for_names(&[
            "source-report-bound",
            "output-report-bound",
            "deterministic-traversal-order",
            "marker-coverage-manifest",
            "encrypted-ref-inventory",
        ]),
    ])
}

fn redaction_transform_receipt_value(input: &RedactionTransformReceiptInput<'_>) -> Result<IOValue> {
    Ok(record("redaction-transform-receipt-v1", vec![
        string(HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("source-report", vec![string(input.source_report_ref)]),
        record("source-suite", vec![string(input.source_suite_ref)]),
        record("policy", vec![string(input.policy_ref)]),
        record("profile", vec![string(input.profile.as_str())]),
        record("transform-manifest", vec![string(input.manifest_ref)]),
        record("output-bundle", vec![string(input.output_bundle_ref)]),
        record("loss-classification", vec![string(input.profile.loss_classification())]),
        record("markers", vec![refs_sequence(input.marker_refs)]),
        record("encrypted-refs", vec![refs_sequence(input.encrypted_refs)]),
        checks_value_for_names(&redaction_transform_check_names(input.profile)),
    ]))
}

fn parse_redaction_transform_receipt(value: &IOValue) -> Result<RedactionTransformReceiptEvidence> {
    let receipt = simple_record(value, "redaction-transform-receipt-v1", 12)?;
    let schema = required_string(&receipt[0], "redaction transform schema")?;
    if schema != HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction transform schema {schema}; expected {HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "redaction transform decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported redaction transform decision {decision}")));
    }
    let source_report_ref = required_record_hash(&receipt[2], "source-report", "redaction source report")?;
    let source_suite_ref = required_record_hash(&receipt[3], "source-suite", "redaction source suite")?;
    let policy_ref = required_record_hash(&receipt[4], "policy", "redaction policy")?;
    let profile_name = required_record_string(&receipt[5], "profile", "redaction profile")?;
    let profile = ReproExportProfile::parse(&profile_name)?;
    let manifest_ref = required_record_hash(&receipt[6], "transform-manifest", "redaction transform manifest")?;
    let output_bundle_ref = required_record_hash(&receipt[7], "output-bundle", "redaction output bundle")?;
    let loss_classification =
        required_record_string(&receipt[8], "loss-classification", "redaction loss classification")?;
    if loss_classification != profile.loss_classification() {
        return Err(MoltenError::invalid_harness("redaction transform loss classification is not canonical"));
    }
    let marker_refs = required_record_hash_sequence(&receipt[9], "markers", "redaction marker refs")?;
    let encrypted_refs = required_record_hash_sequence(&receipt[10], "encrypted-refs", "redaction encrypted refs")?;
    let checks = parse_redaction_gate_checks(&receipt[11])?;
    for check in redaction_transform_check_names(profile) {
        require_redaction_check(&checks, check)?;
    }
    Ok(RedactionTransformReceiptEvidence {
        receipt_ref: canonical_hash(value)?,
        source_report_ref,
        source_suite_ref,
        policy_ref,
        profile,
        manifest_ref,
        output_bundle_ref,
        loss_classification,
        marker_refs,
        encrypted_refs,
        value: value.clone(),
    })
}

fn redaction_gate_value(report_value: &IOValue, report: &HarnessReport) -> Result<IOValue> {
    if let Some(marker) = first_sensitive_marker(report_value) {
        return Err(MoltenError::invalid_harness(format!(
            "redaction preflight found sensitive marker {marker}; sealed pass repro bundles require explicit redaction before export"
        )));
    }
    let policy = redaction_policy_value();
    let policy_ref = canonical_hash(&policy)?;
    Ok(record("redaction-gate-v1", vec![
        string(HARNESS_REDACTION_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("policy-ref", vec![string(policy_ref)]),
        record("report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        record("scan-root-ref", vec![string(canonical_hash(report_value)?)]),
        redaction_gate_checks_value(),
    ]))
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    match reference {
        Some(reference) => record("some", vec![string(reference)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value_for_names(names: &[&str]) -> IOValue {
    record("checks", vec![sequence(
        names.iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

fn redaction_transform_check_names(profile: ReproExportProfile) -> Vec<&'static str> {
    let mut checks = vec![
        "source-report-ref-bound",
        "source-suite-ref-bound",
        "policy-ref-bound",
        "profile-ref-bound",
        "transform-manifest-bound",
        "output-bundle-ref-bound",
        "marker-coverage",
        "deterministic-traversal-order",
        "forbidden-cleartext-absent",
    ];
    match profile {
        ReproExportProfile::DenySensitive => checks.push("gate-preserving"),
        ReproExportProfile::RedactedDiagnostic => checks.push("diagnostic-only"),
        ReproExportProfile::EncryptedPrivate => {
            checks.push("requires-reveal");
            checks.push("encrypted-ref-validation");
        }
    }
    checks
}

fn profiled_repro_checks_value(profile: ReproExportProfile) -> IOValue {
    let mut checks = vec![
        "profile-schema",
        "redaction-transform-receipt",
        "transform-manifest-bound",
        "source-report-ref-binding",
        "output-report-ref-binding",
        "no-forbidden-cleartext",
    ];
    match profile {
        ReproExportProfile::DenySensitive => checks.push("gate-preserving"),
        ReproExportProfile::RedactedDiagnostic => checks.push("diagnostic-only"),
        ReproExportProfile::EncryptedPrivate => {
            checks.push("requires-reveal");
            checks.push("encrypted-ref-validation");
        }
    }
    record("seal-checks", vec![sequence(
        checks.as_slice().iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

fn redaction_gate_checks_value() -> IOValue {
    record("checks", vec![sequence(
        [
            "redaction-policy",
            "canonical-report-scan",
            "no-secret-markers",
            "no-confidential-markers",
            "no-credential-markers",
            "no-private-markers",
            "no-unvalidated-encrypted-refs",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn validate_redaction_evidence(
    report_value: &IOValue,
    report: &HarnessReport,
    policy_value: &IOValue,
    gate_value: &IOValue,
) -> Result<(String, String)> {
    let expected_policy = redaction_policy_value();
    let expected_policy_ref = canonical_hash(&expected_policy)?;
    let actual_policy_ref = canonical_hash(policy_value)?;
    if actual_policy_ref != expected_policy_ref || policy_value != &expected_policy {
        return Err(MoltenError::invalid_harness(format!(
            "redaction policy evidence mismatch: policy hashes to {actual_policy_ref}, expected {expected_policy_ref}"
        )));
    }
    parse_redaction_policy(policy_value)?;
    let expected_gate = redaction_gate_value(report_value, report)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(gate_value)?;
    if actual_gate_ref != expected_gate_ref || gate_value != &expected_gate {
        return Err(MoltenError::invalid_harness(format!(
            "redaction gate evidence mismatch: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    parse_redaction_gate(gate_value, report, &expected_policy_ref, report_value)?;
    Ok((actual_policy_ref, actual_gate_ref))
}

fn parse_redaction_policy(value: &IOValue) -> Result<()> {
    let policy = simple_record(value, "redaction-policy-v1", 3)?;
    let schema = required_string(&policy[0], "redaction policy schema")?;
    if schema != HARNESS_REDACTION_POLICY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction policy schema {schema}; expected {HARNESS_REDACTION_POLICY_SCHEMA}"
        )));
    }
    let mode = required_record_string(&policy[1], "mode", "redaction policy mode")?;
    if mode != "deny-sensitive-markers" {
        return Err(MoltenError::invalid_harness(format!("unsupported redaction policy mode {mode}")));
    }
    let markers = required_record_sequence(&policy[2], "forbidden-markers", "redaction forbidden markers")?;
    let actual = markers
        .iter()
        .map(|marker| required_string(&marker, "redaction marker"))
        .collect::<Result<Vec<_>>>()?;
    if actual != FORBIDDEN_REDACTION_MARKERS {
        return Err(MoltenError::invalid_harness("redaction policy forbidden marker set is not canonical"));
    }
    Ok(())
}

fn parse_redaction_gate(
    value: &IOValue,
    report: &HarnessReport,
    policy_ref: &str,
    report_value: &IOValue,
) -> Result<()> {
    let gate = simple_record(value, "redaction-gate-v1", 7)?;
    let schema = required_string(&gate[0], "redaction gate schema")?;
    if schema != HARNESS_REDACTION_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported redaction gate schema {schema}; expected {HARNESS_REDACTION_GATE_SCHEMA}"
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "redaction gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported redaction gate decision {decision}")));
    }
    let actual_policy_ref = required_record_hash(&gate[2], "policy-ref", "redaction policy ref")?;
    if actual_policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("redaction gate policy ref does not match policy evidence"));
    }
    let report_ref = required_record_hash(&gate[3], "report-ref", "redaction gate report ref")?;
    if report_ref != report.report_ref {
        return Err(MoltenError::invalid_harness("redaction gate report ref does not match embedded report"));
    }
    let suite_ref = required_record_hash(&gate[4], "suite-ref", "redaction gate suite ref")?;
    if suite_ref != report.suite_ref {
        return Err(MoltenError::invalid_harness("redaction gate suite ref does not match embedded report"));
    }
    let scan_root_ref = required_record_hash(&gate[5], "scan-root-ref", "redaction gate scan root ref")?;
    let actual_scan_root_ref = canonical_hash(report_value)?;
    if scan_root_ref != actual_scan_root_ref {
        return Err(MoltenError::invalid_harness("redaction gate scan root ref does not match embedded report"));
    }
    let checks = parse_redaction_gate_checks(&gate[6])?;
    require_redaction_check(&checks, "redaction-policy")?;
    require_redaction_check(&checks, "canonical-report-scan")?;
    require_redaction_check(&checks, "no-secret-markers")?;
    require_redaction_check(&checks, "no-confidential-markers")?;
    require_redaction_check(&checks, "no-credential-markers")?;
    require_redaction_check(&checks, "no-private-markers")?;
    require_redaction_check(&checks, "no-unvalidated-encrypted-refs")?;
    Ok(())
}

fn parse_redaction_gate_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "redaction gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "redaction gate check name")?;
        let status = required_string(&check[1], "redaction gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("redaction gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_redaction_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("redaction gate missing {expected} check")))
    }
}

fn is_sensitive_record_label(label: &str) -> bool {
    FORBIDDEN_REDACTION_MARKERS.iter().any(|marker| marker == &label)
}

fn validate_profiled_output(value: &IOValue, profile: ReproExportProfile) -> Result<Vec<String>> {
    let mut encrypted_refs = Vec::with_capacity(8);
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.is_record() {
            if let Some(label) = current.label().as_symbol() {
                let label = label.as_ref();
                if matches!(label, "secret" | "confidential" | "credential" | "private" | "secret-ref-v1") {
                    return Err(MoltenError::invalid_harness(format!(
                        "redaction transform missed sensitive marker {label}"
                    )));
                }
                if label == "encrypted-ref" {
                    return Err(MoltenError::invalid_harness(
                        "malformed encrypted-ref marker in redacted repro bundle",
                    ));
                }
                if label == "encrypted-ref-v1" {
                    if profile != ReproExportProfile::EncryptedPrivate {
                        return Err(MoltenError::invalid_harness(
                            "encrypted refs are allowed only in encrypted-private repro bundles",
                        ));
                    }
                    let encrypted = parse_encrypted_ref(&current)?;
                    ensure_redaction_bound(
                        encrypted_refs.len() + 1,
                        MAX_REDACTION_ENCRYPTED_REFS,
                        "redaction encrypted refs",
                    )?;
                    encrypted_refs.push(encrypted.encrypted_ref);
                    continue;
                }
            }
            stack.push(value_to_iovalue(&current.label()));
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record)
            | ValueClass::Compound(CompoundClass::Sequence)
            | ValueClass::Compound(CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(value_to_iovalue(&child));
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(value_to_iovalue(&key));
                    stack.push(value_to_iovalue(&value));
                }
            }
        }
    }
    encrypted_refs.sort();
    encrypted_refs.dedup();
    Ok(encrypted_refs)
}

fn first_sensitive_marker(value: &IOValue) -> Option<String> {
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.is_record() {
            if let Some(label) = current.label().as_symbol()
                && FORBIDDEN_REDACTION_MARKERS.iter().any(|marker| marker == &label.as_ref())
            {
                return Some(label.into_owned());
            }
            stack.push(value_to_iovalue(&current.label()));
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record)
            | ValueClass::Compound(CompoundClass::Sequence)
            | ValueClass::Compound(CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(value_to_iovalue(&child));
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(value_to_iovalue(&key));
                    stack.push(value_to_iovalue(&value));
                }
            }
        }
    }
    None
}

fn repro_seal_value(report: &HarnessReport, gate_receipt_ref: &str) -> IOValue {
    record("repro-seal", vec![
        string(HARNESS_REPRO_SEAL_SCHEMA),
        record("decision", vec![string("pass")]),
        record("gate-receipt-ref", vec![string(gate_receipt_ref)]),
        record("report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        record("profile", vec![string(&report.profile)]),
        record("replay-status", vec![string(&report.replay_status)]),
    ])
}

fn sealed_repro_checks_value() -> IOValue {
    record("seal-checks", vec![sequence(
        [
            "sealed-report",
            "embedded-gate-receipt",
            "report-ref-binding",
            "suite-ref-binding",
            "actor-registry-binding",
            "effect-log-binding",
            "policy-gate-ref-binding",
            "capability-gate-ref-binding",
            "budget-gate-ref-binding",
            "redaction-preflight",
            "redaction-gate-ref-binding",
            "no-sensitive-markers",
            "replay-metadata-binding",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn default_report_bundle_command() -> Vec<String> {
    [
        "molten",
        "test",
        "repro",
        "export",
        "report.preserves",
        "--out",
        "repro",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

fn default_failure_bundle_command() -> Vec<String> {
    [
        "molten",
        "test",
        "repro",
        "export",
        "failure.preserves",
        "--out",
        "repro",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

fn required_record_string(value: &Value<IOValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

fn required_record_hash(value: &Value<IOValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_hash(&record[0], field)
}

fn required_record_bool(value: &Value<IOValue>, label: &str, field: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_bool(&record[0], field)
}

fn required_record_u64(value: &Value<IOValue>, label: &str, field: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], field)
}

fn required_record_sequence(value: &Value<IOValue>, label: &str, field: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let owned = value_to_iovalue(&record[0]);
    Ok(owned
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))?
        .into_owned())
}

fn required_record_hash_sequence(value: &Value<IOValue>, label: &str, field: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    values.iter().map(|value| required_hash(&value, field)).collect()
}

fn required_record_string_sequence(value: &Value<IOValue>, label: &str, field: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

fn required_record_iovalue_sequence(value: &Value<IOValue>, label: &str, field: &str) -> Result<Vec<IOValue>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    Ok(values.iter().map(|value| value_to_iovalue(&value)).collect())
}

fn validate_tool_record(value: &Value<IOValue>) -> Result<()> {
    let value = value_to_iovalue(value);
    let tool = simple_record(&value, "tool", 2)?;
    let name = required_string(&tool[0], "repro bundle tool name")?;
    if name != "molten" {
        return Err(MoltenError::invalid_harness(format!("unsupported repro bundle tool {name}")));
    }
    let version = required_string(&tool[1], "repro bundle tool version")?;
    if version.is_empty() {
        return Err(MoltenError::invalid_harness("repro bundle tool version must not be empty"));
    }
    Ok(())
}

fn validate_sequence_record(value: &Value<IOValue>, label: &str, field: &str) -> Result<()> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_sequence(&record[0], field)?;
    Ok(())
}

fn parse_artifact_refs(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let artifact_refs = simple_record(&value, "artifact-refs", 1)?;
    let ref_values = required_sequence(&artifact_refs[0], "repro bundle artifact refs")?;
    let mut refs = Vec::with_capacity(ref_values.len());
    for ref_value in ref_values.iter() {
        let ref_value = value_to_iovalue(&ref_value);
        let artifact_ref = simple_record(&ref_value, "artifact-ref", 2)?;
        refs.push((
            required_string(&artifact_ref[0], "artifact ref kind")?,
            required_string(&artifact_ref[1], "artifact ref value")?,
        ));
    }
    Ok(refs)
}

fn require_artifact_ref(refs: &[(String, String)], kind: &str, expected: &str) -> Result<()> {
    if refs.iter().any(|(actual_kind, actual_ref)| actual_kind == kind && actual_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("repro bundle artifact refs missing {kind} ref {expected}")))
    }
}

pub fn effect_log_value(entries: &[EffectLogEntry]) -> IOValue {
    record("effect-log-v1", vec![
        string(HARNESS_EFFECT_LOG_SCHEMA),
        sequence(
            entries
                .iter()
                .map(|entry| {
                    record("effect-entry", vec![
                        u64_value(entry.sequence),
                        entry.request.clone(),
                        entry.response.clone(),
                    ])
                })
                .collect(),
        ),
    ])
}

pub fn budget_limits_value(budget: &HarnessBudget) -> IOValue {
    record("budget-v1", vec![string(HARNESS_BUDGET_SCHEMA), limits_value(budget)])
}

pub fn budget_value(budget: &HarnessBudget, usage: &BudgetUsage) -> IOValue {
    record("budget-v1", vec![
        string(HARNESS_BUDGET_SCHEMA),
        limits_value(budget),
        record("usage", vec![
            u64_value(usage.steps),
            u64_value(usage.effects),
            u64_value(usage.events),
            u64_value(usage.report_bytes),
        ]),
    ])
}

pub fn parse_budget(value: &IOValue) -> Result<BudgetEvidence> {
    let budget = simple_record(value, "budget-v1", 3)?;
    let limits = parse_budget_schema_and_limits(&budget)?;
    let usage_value = value_to_iovalue(&budget[2]);
    let usage = simple_record(&usage_value, "usage", 4)?;
    let usage = BudgetUsage {
        steps: required_u64(&usage[0], "budget used steps")?,
        effects: required_u64(&usage[1], "budget used effects")?,
        events: required_u64(&usage[2], "budget used events")?,
        report_bytes: required_u64(&usage[3], "budget used report bytes")?,
    };
    Ok(BudgetEvidence { limits, usage })
}

pub fn parse_budget_limits(value: &IOValue) -> Result<HarnessBudget> {
    let budget = simple_record(value, "budget-v1", 2)?;
    parse_budget_schema_and_limits(&budget)
}

fn limits_value(budget: &HarnessBudget) -> IOValue {
    record("limits", vec![
        u64_value(budget.max_steps),
        u64_value(budget.max_effects),
        u64_value(budget.max_events),
        u64_value(budget.max_report_bytes),
    ])
}

fn parse_budget_schema_and_limits(budget: &Record<Value<IOValue>>) -> Result<HarnessBudget> {
    let schema = required_string(&budget[0], "budget schema")?;
    if schema != HARNESS_BUDGET_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget schema {schema}; expected {HARNESS_BUDGET_SCHEMA}"
        )));
    }
    let limits_value = value_to_iovalue(&budget[1]);
    let limits = simple_record(&limits_value, "limits", 4)?;
    Ok(HarnessBudget {
        max_steps: required_u64(&limits[0], "budget max steps")?,
        max_effects: required_u64(&limits[1], "budget max effects")?,
        max_events: required_u64(&limits[2], "budget max events")?,
        max_report_bytes: required_u64(&limits[3], "budget max report bytes")?,
    })
}

pub fn parse_effect_log(value: &IOValue) -> Result<Vec<EffectLogEntry>> {
    let effect_log = simple_record(value, "effect-log-v1", 2)?;
    let schema = required_string(&effect_log[0], "effect log schema")?;
    if schema != HARNESS_EFFECT_LOG_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported effect log schema {schema}; expected {HARNESS_EFFECT_LOG_SCHEMA}"
        )));
    }
    let entry_values = required_sequence(&effect_log[1], "effect log entries")?;
    let mut entries = Vec::with_capacity(entry_values.len());
    for (position, entry) in entry_values.iter().enumerate() {
        let entry_value = value_to_iovalue(&entry);
        let entry_record = simple_record(&entry_value, "effect-entry", 3)?;
        let sequence = required_u64(&entry_record[0], "effect entry sequence")?;
        if sequence != position as u64 {
            return Err(MoltenError::invalid_harness(format!(
                "effect log sequence mismatch at position {position}: got {sequence}"
            )));
        }
        let request = value_to_iovalue(&entry_record[1]);
        let response = value_to_iovalue(&entry_record[2]);
        let request_sequence = effect_request_sequence(&request)?;
        let response_sequence = effect_response_sequence_and_value(&response)?.0;
        if sequence != request_sequence || sequence != response_sequence {
            return Err(MoltenError::invalid_harness(format!(
                "effect entry {sequence} request/response sequence mismatch"
            )));
        }
        entries.push(EffectLogEntry {
            sequence,
            request,
            response,
        });
    }
    Ok(entries)
}

pub fn effect_log_from_observations(observations: &[HarnessObservation]) -> Result<Vec<EffectLogEntry>> {
    let mut entries = Vec::new();
    let mut pending_request: Option<(u64, IOValue)> = None;
    for observation in observations {
        for event in &observation.events {
            match event_boundary(event) {
                EventBoundary::EffectRequest => {
                    let sequence = effect_request_sequence(event)?;
                    if pending_request.is_some() {
                        return Err(MoltenError::invalid_harness("nested effect request without response"));
                    }
                    pending_request = Some((sequence, event.clone()));
                }
                EventBoundary::EffectResponse => {
                    let (sequence, _value) = effect_response_sequence_and_value(event)?;
                    let Some((request_sequence, request)) = pending_request.take() else {
                        return Err(MoltenError::invalid_harness("effect response without request"));
                    };
                    if sequence != request_sequence {
                        return Err(MoltenError::invalid_harness(format!(
                            "effect response sequence {sequence} does not match request sequence {request_sequence}"
                        )));
                    }
                    push_bounded(
                        &mut entries,
                        EffectLogEntry {
                            sequence,
                            request,
                            response: event.clone(),
                        },
                        MAX_HARNESS_EFFECT_LOG_ENTRIES,
                        "harness effect log entries",
                    )?;
                }
                EventBoundary::PolicyDecision
                | EventBoundary::ActorInput
                | EventBoundary::HostcallRequest
                | EventBoundary::HostcallDecision
                | EventBoundary::ActorOutput
                | EventBoundary::SteelExecution
                | EventBoundary::WasmExecution
                | EventBoundary::RuntimePredicate
                | EventBoundary::Trace => {}
            }
        }
    }
    if pending_request.is_some() {
        return Err(MoltenError::invalid_harness("effect request without response"));
    }
    Ok(entries)
}

pub(crate) fn append_effect_entries_from_events(
    events: &[IOValue],
    entries: &mut impl crate::bounded::VecSink<EffectLogEntry>,
) -> Result<()> {
    let mut pending_request: Option<(u64, IOValue)> = None;
    for event in events {
        match event_boundary(event) {
            EventBoundary::EffectRequest => {
                let sequence = effect_request_sequence(event)?;
                if pending_request.is_some() {
                    return Err(MoltenError::invalid_harness("nested effect request without response"));
                }
                pending_request = Some((sequence, event.clone()));
            }
            EventBoundary::EffectResponse => {
                let (sequence, _value) = effect_response_sequence_and_value(event)?;
                let Some((request_sequence, request)) = pending_request.take() else {
                    return Err(MoltenError::invalid_harness("effect response without request"));
                };
                if sequence != request_sequence {
                    return Err(MoltenError::invalid_harness(format!(
                        "effect response sequence {sequence} does not match request sequence {request_sequence}"
                    )));
                }
                push_bounded(
                    &mut *entries,
                    EffectLogEntry {
                        sequence,
                        request,
                        response: event.clone(),
                    },
                    MAX_HARNESS_EFFECT_LOG_ENTRIES,
                    "harness effect log entries",
                )?;
            }
            EventBoundary::PolicyDecision
            | EventBoundary::ActorInput
            | EventBoundary::HostcallRequest
            | EventBoundary::HostcallDecision
            | EventBoundary::ActorOutput
            | EventBoundary::SteelExecution
            | EventBoundary::WasmExecution
            | EventBoundary::RuntimePredicate
            | EventBoundary::Trace => {}
        }
    }
    if pending_request.is_some() {
        return Err(MoltenError::invalid_harness("effect request without response"));
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

pub fn effect_response_sequence_and_value(value: &IOValue) -> Result<(u64, u64)> {
    let response = value
        .collect_simple_record("effect-response", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected effect-response record"))?;
    let arity = response.fields_iter().count();
    if arity != 4 && arity != 5 {
        return Err(MoltenError::invalid_harness(format!("effect-response arity must be 4 or 5, got {arity}")));
    }
    let sequence = required_u64(&response[2], "effect response sequence")?;
    let value_index = arity - 1;
    let value = required_u64(&response[value_index], "effect response value")?;
    Ok((sequence, value))
}

pub fn effect_request_sequence(value: &IOValue) -> Result<u64> {
    let request = value
        .collect_simple_record("effect-request", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected effect-request record"))?;
    let arity = request.fields_iter().count();
    if arity != 3 && arity != 4 {
        return Err(MoltenError::invalid_harness(format!("effect-request arity must be 3 or 4, got {arity}")));
    }
    required_u64(&request[2], "effect request sequence")
}

pub fn event_boundary(value: &IOValue) -> EventBoundary {
    if value.collect_simple_record("effect-request", None).is_some() {
        return EventBoundary::EffectRequest;
    }
    if value.collect_simple_record("effect-response", None).is_some() {
        return EventBoundary::EffectResponse;
    }
    if value.collect_simple_record("admission-decision-v1", None).is_some() {
        return EventBoundary::PolicyDecision;
    }
    if value.collect_simple_record("actor-input-v1", None).is_some() {
        return EventBoundary::ActorInput;
    }
    if value.collect_simple_record("hostcall-request-v1", None).is_some() {
        return EventBoundary::HostcallRequest;
    }
    if value.collect_simple_record("hostcall-decision-v1", None).is_some() {
        return EventBoundary::HostcallDecision;
    }
    if value.collect_simple_record("actor-output-v1", None).is_some() {
        return EventBoundary::ActorOutput;
    }
    if value.collect_simple_record("steel-execution-receipt-v1", None).is_some() {
        return EventBoundary::SteelExecution;
    }
    if value.collect_simple_record("wasm-execution-receipt-v1", None).is_some() {
        return EventBoundary::WasmExecution;
    }
    if value.collect_simple_record("runtime-predicate-receipt-v1", None).is_some() {
        return EventBoundary::RuntimePredicate;
    }
    EventBoundary::Trace
}

fn parse_observation(value: &Value<IOValue>) -> Result<HarnessObservation> {
    let value = value_to_iovalue(value);
    let observation = value
        .collect_simple_record("turn-observation-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <turn-observation-v1 ...>"))?;
    let arity = observation.len();
    if arity != 6 && arity != 7 {
        return Err(MoltenError::invalid_harness(format!(
            "turn observation arity {arity} is unsupported; expected 6 or 7"
        )));
    }
    let schema = required_string(&observation[0], "observation schema")?;
    if schema != HARNESS_OBSERVATION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported observation schema {schema}; expected {HARNESS_OBSERVATION_SCHEMA}"
        )));
    }
    let events_index = if arity == 7 { 6 } else { 5 };
    let event_values = required_sequence(&observation[events_index], "observation events")?;
    let mut events = Vec::with_capacity(event_values.len());
    for event in event_values.iter() {
        events.push(value_to_iovalue(&event));
    }
    let mut computed_event_refs = Vec::with_capacity(events.len());
    for event in &events {
        computed_event_refs.push(canonical_hash(event)?);
    }
    let event_refs = if arity == 7 {
        required_record_hash_sequence(&observation[5], "event-refs", "observation event ref")?
    } else {
        computed_event_refs
    };
    let observation_ref = canonical_hash(&value)?;
    let index = required_u64(&observation[1], "observation index")?;
    let step_ref = required_hash(&observation[2], "observation step ref")?;
    let before_state_hash = required_hash(&observation[3], "observation before state hash")?;
    let after_state_hash = required_hash(&observation[4], "observation after state hash")?;
    Ok(HarnessObservation {
        value,
        observation_ref,
        index,
        step_ref,
        before_state_hash,
        after_state_hash,
        event_refs,
        events,
    })
}

fn parse_step(value: &Value<IOValue>) -> Result<CoreStep> {
    if let Some(record) = value.collect_simple_record("send", Some(3)) {
        return Ok(CoreStep::Send {
            from: required_string(&record[0], "send from")?,
            to: required_string(&record[1], "send to")?,
            body: required_runtime_value(&record[2], "send body")?,
        });
    }
    if let Some(record) = value.collect_simple_record("observe", Some(2)) {
        return Ok(CoreStep::Observe {
            actor: required_string(&record[0], "observe actor")?,
            pattern: required_runtime_value(&record[1], "observe pattern")?,
        });
    }
    if let Some(record) = value.collect_simple_record("assert", Some(2)) {
        return Ok(CoreStep::Assert {
            actor: required_string(&record[0], "assert actor")?,
            value: required_runtime_value(&record[1], "assert value")?,
        });
    }
    if let Some(record) = value.collect_simple_record("retract", Some(2)) {
        return Ok(CoreStep::Retract {
            actor: required_string(&record[0], "retract actor")?,
            value: required_runtime_value(&record[1], "retract value")?,
        });
    }
    if let Some(record) = value.collect_simple_record("clock", Some(1)) {
        return Ok(CoreStep::Clock {
            actor: required_string(&record[0], "clock actor")?,
        });
    }
    if let Some(record) = value.collect_simple_record("random", Some(2)) {
        return Ok(CoreStep::Random {
            actor: required_string(&record[0], "random actor")?,
            upper: required_u64(&record[1], "random upper bound")?,
        });
    }
    Err(MoltenError::invalid_harness("unknown harness step record"))
}

fn tuple_set<T, F>(label: &'static str, values: &BTreeSet<T>, mut render: F) -> IOValue
where F: FnMut(&T) -> IOValue {
    record(label, vec![sequence(values.iter().map(&mut render).collect())])
}

fn effect_name(effect: &CoreEffect) -> &'static str {
    match effect {
        CoreEffect::Clock => "clock",
        CoreEffect::Random => "random",
    }
}

fn error_kind(error: &MoltenError) -> String {
    match error {
        MoltenError::Io(_) => "io".to_string(),
        MoltenError::Preserves(_) => "preserves".to_string(),
        MoltenError::InvalidHarness(_) => "invalid-harness".to_string(),
        MoltenError::HarnessDivergence(divergence) => divergence.kind.clone(),
    }
}

fn error_diagnostics(error: &MoltenError) -> Vec<IOValue> {
    match error {
        MoltenError::HarnessDivergence(divergence) => {
            let mut diagnostics = Vec::new();
            if let Some(step) = divergence.step {
                diagnostics.push(record("step", vec![u64_value(step)]));
            }
            diagnostics.push(record("expected", vec![string(&divergence.expected)]));
            diagnostics.push(record("actual", vec![string(&divergence.actual)]));
            diagnostics.push(record("detail", vec![string(&divergence.detail)]));
            diagnostics
        }
        MoltenError::Io(_) | MoltenError::Preserves(_) | MoltenError::InvalidHarness(_) => Vec::new(),
    }
}

fn simple_record<'a>(value: &'a IOValue, label: &str, arity: usize) -> Result<Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn value_has_record_label(value: &Value<IOValue>, label: &str) -> bool {
    value.collect_simple_record(label, None).is_some()
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_bool(value: &Value<IOValue>, field: &str) -> Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected boolean for {field}")))
}

fn optional_string(value: &Value<IOValue>, field: &str) -> Result<Option<String>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        required_string(value, field).map(Some)
    }
}

fn optional_request_string(value: &Value<IOValue>, field: &str) -> Result<Option<String>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], field).map(Some);
    }
    // Compatibility with early reports that encoded present optional strings directly.
    required_string(value, field).map(Some)
}

fn optional_request_runtime_value(value: &Value<IOValue>, _field: &str) -> Result<Option<RuntimeValue>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return RuntimeValue::new(value_to_iovalue(&some[0])).map(Some);
    }
    // Compatibility with early reports that encoded present optional values directly.
    RuntimeValue::new(value_to_iovalue(value)).map(Some)
}

fn optional_request_u64(value: &Value<IOValue>, field: &str) -> Result<Option<u64>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&some[0], field).map(Some);
    }
    // Compatibility with early reports that encoded present optional integers directly.
    required_u64(value, field).map(Some)
}

fn optional_action(value: &Value<IOValue>, field: &str) -> Result<Option<AdmissionAction>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        parse_admission_action(&required_string(value, field)?).map(Some)
    }
}

fn optional_runtime_match_value(value: &Value<IOValue>) -> Result<Option<RuntimeValue>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        RuntimeValue::new(value_to_iovalue(value)).map(Some)
    }
}

fn required_hash(value: &Value<IOValue>, field: &str) -> Result<String> {
    let hash = required_string(value, field)?;
    validate_content_ref(&hash).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {hash}: {error}"))
    })?;
    Ok(hash)
}

fn required_runtime_value(value: &Value<IOValue>, _field: &str) -> Result<RuntimeValue> {
    RuntimeValue::new(value_to_iovalue(value))
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::parse_suite;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    #[test]
    fn suite_schema_roundtrip_preserves_canonical_hash() {
        let suite = parse_text(r#"<harness-suite-v1 "molten.harness.suite.v1" "roundtrip" 1 [<clock "actor">]>"#)
            .expect("parse suite");
        let parsed = parse_suite(&suite).expect("parse suite schema");
        let rendered = to_text(&parsed.source_value).expect("render suite");
        let reparsed = parse_text(&rendered).expect("reparse rendered suite");
        assert_eq!(canonical_hash(&suite).unwrap(), canonical_hash(&reparsed).unwrap());
    }
}
