use super::super::LifecycleState;
use super::super::ResourceUsage;

// r[impl molten.system_extension.native_host.callback_protocol]
// r[impl molten.system_extension.native_host.execution]
// r[impl molten.system_extension.native_host.value_protocol]
pub const NATIVE_HOST_PROFILE_SCHEMA: &str = "molten.system-extension.native-host-profile.v2";
pub const NATIVE_EXECUTABLE_EVIDENCE_SCHEMA: &str = "molten.system-extension.native-executable-evidence.v2";
pub const NATIVE_CALLBACK_ENVELOPE_SCHEMA: &str = "molten.system-extension.native-callback-envelope.v2";
pub const NATIVE_CALLBACK_OUTCOME_SCHEMA: &str = "molten.system-extension.native-callback-outcome.v2";
pub const NATIVE_INSTANCE_STATE_SCHEMA: &str = "molten.system-extension.native-instance-state.v2";
pub const NATIVE_OPERATION_SCHEMA: &str = "molten.system-extension.native-operation.v2";
pub const NATIVE_INGRESS_SCHEMA: &str = "molten.system-extension.native-ingress.v2";
pub const NATIVE_STATUS_SCHEMA: &str = "molten.system-extension.native-status.v2";
pub const NATIVE_ALPN: &str = "molten/system-extension/native/v2";
pub const NATIVE_FRAMING: &str = "preserves-packed-materialized-values-v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeHostNonClaim {
    Sandboxing,
    Hermeticity,
    ExecutableTrust,
    CallbackCorrectness,
    EffectSuccess,
    ValueMeaning,
    ValueDurability,
    TransportDelivery,
    DistributedAvailability,
    ProductionReadiness,
}

impl NativeHostNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sandboxing => "does-not-prove-sandboxing",
            Self::Hermeticity => "does-not-prove-hermeticity",
            Self::ExecutableTrust => "does-not-prove-executable-trust",
            Self::CallbackCorrectness => "does-not-prove-callback-correctness",
            Self::EffectSuccess => "does-not-prove-effect-success",
            Self::ValueMeaning => "does-not-prove-value-meaning",
            Self::ValueDurability => "does-not-prove-value-durability",
            Self::TransportDelivery => "does-not-prove-transport-delivery",
            Self::DistributedAvailability => "does-not-prove-distributed-availability",
            Self::ProductionReadiness => "does-not-prove-production-readiness",
        }
    }
}

const REQUIRED_NATIVE_HOST_NON_CLAIM_COUNT: usize = 10;

pub const REQUIRED_NATIVE_HOST_NON_CLAIMS: [NativeHostNonClaim; REQUIRED_NATIVE_HOST_NON_CLAIM_COUNT] = [
    NativeHostNonClaim::Sandboxing,
    NativeHostNonClaim::Hermeticity,
    NativeHostNonClaim::ExecutableTrust,
    NativeHostNonClaim::CallbackCorrectness,
    NativeHostNonClaim::EffectSuccess,
    NativeHostNonClaim::ValueMeaning,
    NativeHostNonClaim::ValueDurability,
    NativeHostNonClaim::TransportDelivery,
    NativeHostNonClaim::DistributedAvailability,
    NativeHostNonClaim::ProductionReadiness,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHostProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub execution_profile_ref: String,
    pub transport_profile_ref: String,
    pub alpn: String,
    pub framing: String,
    pub max_callback_input_bytes: u64,
    pub max_callback_output_bytes: u64,
    pub max_diagnostic_bytes: u64,
    pub max_materialized_value_bytes: u64,
    pub max_instances: usize,
    pub max_unresolved_operations: usize,
    pub max_port_bindings: usize,
    pub max_policy_refs: usize,
    pub max_materialized_values: usize,
    pub is_local_live_pilot: bool,
    pub requires_materialized_values: bool,
    pub non_claims: Vec<NativeHostNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedNativeHostProfile {
    pub profile: NativeHostProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutableEvidence {
    pub schema: String,
    pub executable_ref: String,
    pub executable_bytes_ref: String,
    pub artifact_kind_ref: String,
    pub target_ref: String,
    pub dependency_closure_ref: String,
    pub materialization_ref: String,
    pub provenance_ref: String,
    pub source_gate_ref: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub resource_ref: String,
    pub execution_profile_ref: String,
    pub manifest_ref: String,
    pub state_schema_ref: String,
    pub port_binding_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedNativeExecutable {
    pub executable: NativeExecutableEvidence,
    pub profile_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeOperationKind {
    Callback,
    Effect,
    Ingress,
    ValuePublication,
}

impl NativeOperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Callback => "callback",
            Self::Effect => "effect",
            Self::Ingress => "ingress",
            Self::ValuePublication => "value-publication",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeOperationState {
    IntentCommitted,
    Started,
    Terminal,
    Unknown,
    Stale,
}

impl NativeOperationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IntentCommitted => "intent-committed",
            Self::Started => "started",
            Self::Terminal => "terminal",
            Self::Unknown => "unknown",
            Self::Stale => "stale",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOperationRecord {
    pub schema: String,
    pub operation_ref: String,
    pub parent_ref: String,
    pub kind: NativeOperationKind,
    pub generation: u64,
    pub state: NativeOperationState,
    pub terminal_ref: Option<String>,
    pub is_retry_permitted: bool,
}

// r[impl molten.system_extension.native_host.durability]
// r[impl molten.system_extension.native_host.semantic_state]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInstanceRecord {
    pub schema: String,
    pub instance_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub manifest_ref: String,
    pub executable_ref: String,
    pub profile_ref: String,
    pub state_schema_ref: String,
    pub lifecycle: LifecycleState,
    pub usage: ResourceUsage,
    pub callback_sequence: u64,
    pub event_sequence: u64,
    pub state_ref: Option<String>,
    pub checkpoint_ref: Option<String>,
    pub unresolved: Vec<NativeOperationRecord>,
    pub completed_operations: Vec<NativeOperationRecord>,
    pub completed_operation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub is_accepting_ingress: bool,
}

// r[impl molten.system_extension.native_host.value_materialization]
// r[impl molten.system_extension.native_host.value_publication]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCallbackValue {
    pub value_ref: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeIngressEnvelope {
    pub schema: String,
    pub request_ref: String,
    pub endpoint_ref: String,
    pub peer_ref: String,
    pub service_id: String,
    pub manifest_ref: String,
    pub generation: u64,
    pub authority_ref: String,
    pub policy_ref: String,
    pub resource_ref: String,
    pub transport_profile_ref: String,
    pub alpn: String,
    pub framing: String,
    pub payload: NativeCallbackValue,
    pub accounted_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeIngressAdmission {
    pub request_ref: String,
    pub generation: u64,
    pub acknowledgement_ref: String,
}

// r[impl molten.system_extension.native_host.effects]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEffectCompletionInput {
    pub completion_ref: String,
    pub effect_ref: String,
    pub operation_ref: String,
    pub port_binding_ref: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompletionCallbackPlan {
    pub completion_ref: String,
    pub payload_ref: String,
    pub generation: u64,
}
