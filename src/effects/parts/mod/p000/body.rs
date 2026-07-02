type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const EFFECT_BINDING_RECEIPT_SCHEMA: &str = crate::preserves_rail::EFFECT_BINDING_RECEIPT_SCHEMA;
const EFFECT_COMPOUND_HANDLER_SCHEMA: &str = crate::preserves_rail::EFFECT_COMPOUND_HANDLER_SCHEMA;
const EFFECT_DYNAMIC_OPERATION_SCHEMA: &str = crate::preserves_rail::EFFECT_DYNAMIC_OPERATION_SCHEMA;
const EFFECT_HANDLE_CLEANUP_SCHEMA: &str = crate::preserves_rail::EFFECT_HANDLE_CLEANUP_SCHEMA;
const EFFECT_HANDLE_SCHEMA: &str = crate::preserves_rail::EFFECT_HANDLE_SCHEMA;
const EFFECT_HANDLER_BINDING_SCHEMA: &str = crate::preserves_rail::EFFECT_HANDLER_BINDING_SCHEMA;
const EFFECT_HANDLER_PROFILE_SCHEMA: &str = crate::preserves_rail::EFFECT_HANDLER_PROFILE_SCHEMA;
const EFFECT_MANIFEST_SCHEMA: &str = crate::preserves_rail::EFFECT_MANIFEST_SCHEMA;
const EFFECT_REQUEST_SCHEMA: &str = crate::preserves_rail::EFFECT_REQUEST_SCHEMA;
const EFFECT_RESPONSE_SCHEMA: &str = crate::preserves_rail::EFFECT_RESPONSE_SCHEMA;

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

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const TRANSFER_LOCAL_ONLY: &str = "local-only";
pub const TRANSFER_ATTENUATED_DELEGATION: &str = "attenuated-delegation";
pub const TRANSFER_REMOTE_PROXY: &str = "remote-proxy";

pub const ADAPTER_KIND_DATASPACE: &str = "dataspace";
pub const ADAPTER_KIND_STORAGE: &str = "storage";
pub const ADAPTER_KIND_BLOB: &str = "blob";
pub const ADAPTER_KIND_NETWORK: &str = "network";
pub const ADAPTER_KIND_REMOTE_SYNC: &str = "remote-sync";
pub const ADAPTER_KIND_REPLAY_RECORD: &str = "replay-record";
pub const ADAPTER_KIND_HOSTCALL: &str = "hostcall";

pub const HANDLER_PROFILE_PRODUCTION: &str = "production";
pub const HANDLER_PROFILE_LOCAL: &str = "local";
pub const HANDLER_PROFILE_MOCK: &str = "mock";
pub const HANDLER_PROFILE_CHAOS: &str = "chaos";
pub const HANDLER_PROFILE_PROFILING: &str = "profiling";
pub const HANDLER_PROFILE_DRY_RUN: &str = "dry-run";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredEffect {
    pub effect_id: String,
    pub operation: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectManifestInput {
    pub artifact_kind: String,
    pub artifact_ref: String,
    pub executor_kind: String,
    pub declared_effects: Vec<DeclaredEffect>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectManifest {
    pub manifest_ref: String,
    pub artifact_kind: String,
    pub artifact_ref: String,
    pub executor_kind: String,
    pub declared_effects: Vec<DeclaredEffect>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerProfileInput {
    pub profile: String,
    pub handler_binding_refs: Vec<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerProfile {
    pub profile_ref: String,
    pub profile: String,
    pub handler_binding_refs: Vec<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRequestInput {
    pub artifact_ref: String,
    pub effect_id: String,
    pub operation: String,
    pub handler_profile: String,
    pub input_ref: String,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRequest {
    pub request_ref: String,
    pub artifact_ref: String,
    pub effect_id: String,
    pub operation: String,
    pub handler_profile: String,
    pub input_ref: String,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectResponseInput {
    pub request_ref: String,
    pub decision: String,
    pub output_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectResponse {
    pub response_ref: String,
    pub request_ref: String,
    pub decision: String,
    pub output_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectBindingReceiptInput {
    pub decision: String,
    pub manifest_ref: String,
    pub handler_profile_ref: String,
    pub request_ref: String,
    pub effect_id: String,
    pub operation: String,
    pub handler_profile: String,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectBindingReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub handler_profile_ref: String,
    pub request_ref: String,
    pub effect_id: String,
    pub operation: String,
    pub handler_profile: String,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectScope {
    pub run_ref: String,
    pub session_ref: String,
    pub actor_ref: Option<String>,
    pub turn_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerBindingInput {
    pub profile: String,
    pub scope: EffectScope,
    pub adapter_kind: String,
    pub adapter_ref: String,
    pub executor_preflight_ref: Option<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub operations: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerBinding {
    pub binding_ref: String,
    pub profile: String,
    pub scope: EffectScope,
    pub adapter_kind: String,
    pub adapter_ref: String,
    pub executor_preflight_ref: Option<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub operations: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleInput {
    pub kind: String,
    pub scope: EffectScope,
    pub handler_binding_ref: String,
    pub operations: Vec<String>,
    pub capability_context_ref: String,
    pub context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: Vec<String>,
    pub transfer: String,
    pub parent_handle_ref: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandle {
    pub handle_ref: String,
    pub kind: String,
    pub scope: EffectScope,
    pub handler_binding_ref: String,
    pub operations: Vec<String>,
    pub capability_context_ref: String,
    pub context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: Vec<String>,
    pub transfer: String,
    pub parent_handle_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleRequest<'a> {
    pub kind: &'a str,
    pub operation: &'a str,
    pub run_ref: &'a str,
    pub session_ref: &'a str,
    pub actor_ref: Option<&'a str>,
    pub turn_ref: Option<&'a str>,
    pub policy_ref: &'a str,
    pub capability_context_ref: &'a str,
    pub context_ref: Option<&'a str>,
    pub resource_refs: &'a [String],
    pub logical_time: u64,
    pub remote_use: bool,
    pub revoked_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleValidation {
    pub handler_binding_ref: String,
    pub handle_ref: String,
    pub checks: Vec<String>,
}
