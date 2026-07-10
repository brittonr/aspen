type IoValue = preserves::IOValue;

use crate::bounded::PushLimited;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type ControlRegistryRuntime = crate::raft_control_plane::ControlRegistryRuntime;
type RaftReadReceipt = crate::raft_control_plane::RaftReadReceipt;

const COORDINATION_APPLY_REPORT_SCHEMA: &str = crate::preserves_rail::COORDINATION_APPLY_REPORT_SCHEMA;
const COORDINATION_FENCING_TOKEN_SCHEMA: &str = crate::preserves_rail::COORDINATION_FENCING_TOKEN_SCHEMA;
const COORDINATION_RECEIPT_SCHEMA: &str = crate::preserves_rail::COORDINATION_RECEIPT_SCHEMA;
const COORDINATION_REQUEST_SCHEMA: &str = crate::preserves_rail::COORDINATION_REQUEST_SCHEMA;
const COORDINATION_SERVICE_MANIFEST_SCHEMA: &str = crate::preserves_rail::COORDINATION_SERVICE_MANIFEST_SCHEMA;
const COORDINATION_STATE_SNAPSHOT_SCHEMA: &str = crate::preserves_rail::COORDINATION_STATE_SNAPSHOT_SCHEMA;
const COORDINATION_STATUS_ASSERTION_SCHEMA: &str = crate::preserves_rail::COORDINATION_STATUS_ASSERTION_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
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

pub const SERVICE_LOCK: &str = "lock";
pub const SERVICE_QUEUE: &str = "queue";
pub const SERVICE_SEMAPHORE: &str = "semaphore";
pub const SERVICE_RATE_LIMIT: &str = "rate-limit";
pub const SERVICE_ELECTION: &str = "election";
pub const SERVICE_BARRIER: &str = "barrier";
pub const SERVICE_REGISTRY: &str = "registry";

pub const OP_ACQUIRE: &str = "acquire";
pub const OP_RELEASE: &str = "release";
pub const OP_ENQUEUE: &str = "enqueue";
pub const OP_DEQUEUE: &str = "dequeue";
pub const OP_ELECT: &str = "elect";
pub const OP_ARRIVE: &str = "arrive";
pub const OP_REGISTER: &str = "register";
pub const OP_UNREGISTER: &str = "unregister";
pub const OP_READ: &str = "read";
pub const READ_CONSISTENCY_LINEARIZABLE: &str = crate::raft_control_plane::READ_CONSISTENCY_LINEARIZABLE;
pub const READ_CONSISTENCY_LOCAL_STALE: &str = crate::raft_control_plane::READ_CONSISTENCY_LOCAL_STALE;

const TRANSITION_KIND_ADVANCE: &str = "advance";
const TRANSITION_KIND_DENY_PRESERVE: &str = "deny-preserve";
const TRANSITION_KIND_DUPLICATE_REPLAY: &str = "duplicate-replay";
const TRANSITION_KIND_CONFLICTING_DUPLICATE: &str = "conflicting-duplicate-deny";
const TRANSITION_KIND_READ_OBSERVE: &str = "read-observe";

const SHELL_INTENT_COMMIT: &str = "control-plane-commit";
const SHELL_INTENT_ASSERT_STATUS: &str = "dataspace-status-assertion";
const SHELL_INTENT_EMIT_RECEIPT: &str = "emit-coordination-receipt";
const SHELL_INTENT_REPLAY_OUTPUT: &str = "return-prior-output";

const COORDINATION_RECEIPT_FIELD_COUNT: usize = 13;
const COORDINATION_TRANSITION_FIELD_COUNT: usize = 8;

const COORDINATION_NAMESPACE_PREFIX: &str = "coordination";
pub const DEFAULT_COORDINATION_SERVICE_ID: &str = "coordination:local";
pub const DEFAULT_COORDINATION_QUEUE_CAPACITY: u64 = 4;
pub const DEFAULT_COORDINATION_SEMAPHORE_CAPACITY: u64 = 2;
pub const DEFAULT_COORDINATION_RATE_LIMIT: u64 = 2;
pub const DEFAULT_COORDINATION_BARRIER_PARTIES: u64 = 2;

const MAX_COORDINATION_REFS: usize = 4096;
const MAX_COORDINATION_ITEMS: usize = 4096;
const MAX_COORDINATION_DIAGNOSTICS: usize = 256;
const MAX_COORDINATION_CHECKS: usize = 96;
const MAX_COORDINATION_SERVICES: usize = 16;
const MAX_COORDINATION_KEY_LEN: usize = 256;
const _: () = assert!(MAX_COORDINATION_REFS <= 100_000);
const _: () = assert!(MAX_COORDINATION_ITEMS <= 100_000);
const _: () = assert!(MAX_COORDINATION_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_COORDINATION_CHECKS <= 10_000);
const _: () = assert!(MAX_COORDINATION_SERVICES <= 64);
const _: () = assert!(MAX_COORDINATION_KEY_LEN <= 4096);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationServiceManifestInput {
    pub service_id: String,
    pub services: Vec<String>,
    pub control_group_ref: String,
    pub queue_capacity: u64,
    pub semaphore_capacity: u64,
    pub rate_limit: u64,
    pub barrier_parties: u64,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationServiceManifest {
    pub manifest_ref: String,
    pub service_id: String,
    pub services: Vec<String>,
    pub control_group_ref: String,
    pub queue_capacity: u64,
    pub semaphore_capacity: u64,
    pub rate_limit: u64,
    pub barrier_parties: u64,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationRequestInput {
    pub service: String,
    pub operation: String,
    pub key: String,
    pub client_session: String,
    pub operation_id_ref: String,
    pub read_consistency_mode: String,
    pub payload: Option<IoValue>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationRequest {
    pub request_ref: String,
    pub service: String,
    pub operation: String,
    pub key: String,
    pub client_session: String,
    pub operation_id_ref: String,
    pub read_consistency_mode: String,
    pub payload: Option<IoValue>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencingToken {
    pub token_ref: String,
    pub key: String,
    pub owner: String,
    pub token: u64,
    pub lease_epoch: u64,
    pub commit_receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationStatusAssertion {
    pub assertion_ref: String,
    pub service: String,
    pub key: String,
    pub read_consistency_mode: String,
    pub state_ref: String,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationStateSnapshot {
    pub state_ref: String,
    pub retention_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub service: String,
    pub operation: String,
    pub read_consistency_mode: String,
    pub request_ref: String,
    pub raft_receipt_ref: Option<String>,
    pub token_ref: Option<String>,
    pub state_ref: String,
    pub transition_kind: String,
    pub before_state_ref: String,
    pub after_state_ref: Option<String>,
    pub preserved_state_ref: Option<String>,
    pub output_refs: Vec<String>,
    pub control_plane_intent_ref: Option<String>,
    pub prior_receipt_ref: Option<String>,
    pub dataspace_assertion_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationApplyResult {
    pub receipt: CoordinationReceipt,
    pub request: CoordinationRequest,
    pub token: Option<FencingToken>,
    pub state_snapshot: CoordinationStateSnapshot,
    pub assertions: Vec<CoordinationStatusAssertion>,
    pub raft_commit_ref: Option<String>,
    pub raft_read_receipt: Option<RaftReadReceipt>,
    pub evidence_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationFixtureRun {
    pub decision: String,
    pub manifest_ref: String,
    pub final_state_ref: String,
    pub receipt_refs: Vec<String>,
    pub assertion_refs: Vec<String>,
    pub evidence_values: Vec<IoValue>,
    pub report_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationApplyReport {
    pub report_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub final_state_ref: String,
    pub receipt_refs: Vec<String>,
    pub assertion_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationRuntime {
    pub manifest: CoordinationServiceManifest,
    pub raft: ControlRegistryRuntime,
    pub state: CoordinationState,
    pub applied_operations: OrderedMap<String, CoordinationApplyResult>,
    pub receipts: Vec<CoordinationReceipt>,
    pub assertions: Vec<CoordinationStatusAssertion>,
    pub tokens: Vec<FencingToken>,
    pub next_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoordinationState {
    pub locks: OrderedMap<String, LockState>,
    pub queues: OrderedMap<String, Vec<String>>,
    pub semaphores: OrderedMap<String, OrderedSet<String>>,
    pub rates: OrderedMap<String, u64>,
    pub elections: OrderedMap<String, ElectionState>,
    pub barriers: OrderedMap<String, BarrierState>,
    pub registry: OrderedMap<String, RegistryEntry>,
    pub next_fencing_token: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockState {
    pub owner: String,
    pub token: u64,
    pub token_ref: String,
    pub lease_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionState {
    pub leader: String,
    pub token: u64,
    pub token_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarrierState {
    pub participants: OrderedSet<String>,
    pub required: u64,
    pub is_released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub endpoint_ref: String,
    pub evidence_ref: String,
}

// r[impl molten.coordination_state_machine_proof.primitive_transition_cores]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveTransitionResult {
    pub kind: String,
    pub decision: String,
    pub before_state: CoordinationState,
    pub after_state: CoordinationState,
    pub token: Option<FencingToken>,
    pub status_fact: IoValue,
    pub output_facts: Vec<IoValue>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(&'static str, &'static str)>,
    pub shell_intents: Vec<String>,
}

impl PrimitiveTransitionResult {
    fn state_for_receipt(&self) -> &CoordinationState {
        if self.decision == "pass" {
            &self.after_state
        } else {
            &self.before_state
        }
    }
}

#[derive(Debug, Clone)]
struct PreparedMutation {
    state: CoordinationState,
    token: Option<FencingToken>,
    status_fact: IoValue,
    checks: Vec<(&'static str, &'static str)>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptTransitionInput<'a> {
    pub kind: &'a str,
    pub before_state_ref: &'a str,
    pub after_state_ref: Option<&'a str>,
    pub preserved_state_ref: Option<&'a str>,
    pub output_refs: &'a [String],
    pub control_plane_intent_ref: Option<&'a str>,
    pub prior_receipt_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueInput<'a> {
    pub decision: &'a str,
    pub service: &'a str,
    pub operation: &'a str,
    pub read_consistency_mode: &'a str,
    pub request_ref: &'a str,
    pub raft_receipt_ref: Option<&'a str>,
    pub token_ref: Option<&'a str>,
    pub state_ref: &'a str,
    pub transition: ReceiptTransitionInput<'a>,
    pub dataspace_assertion_refs: &'a [String],
    pub diagnostics: &'a [String],
    pub checks: &'a [(&'a str, &'a str)],
}
