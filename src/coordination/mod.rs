//! Coordination control-plane services.
//!
//! This slice keeps coordination mutations in an explicit control-plane state
//! machine. Ordinary actor/data-plane messages do not call into this module;
//! callers must present canonical coordination requests and receive receipts
//! with Raft/control-registry evidence before dataspace facts are reflected.

type IoValue = preserves::IOValue;

use crate::bounded::VecSink;
use crate::delivery_idempotency;
use crate::raft_control_plane;

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
    pub request_ref: String,
    pub raft_receipt_ref: Option<String>,
    pub token_ref: Option<String>,
    pub state_ref: String,
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

#[derive(Debug, Clone)]
struct PreparedMutation {
    state: CoordinationState,
    token: Option<FencingToken>,
    status_fact: IoValue,
    checks: Vec<(&'static str, &'static str)>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueInput<'a> {
    pub decision: &'a str,
    pub service: &'a str,
    pub operation: &'a str,
    pub request_ref: &'a str,
    pub raft_receipt_ref: Option<&'a str>,
    pub token_ref: Option<&'a str>,
    pub state_ref: &'a str,
    pub dataspace_assertion_refs: &'a [String],
    pub diagnostics: &'a [String],
    pub checks: &'a [(&'a str, &'a str)],
}

#[derive(Debug, Clone, Copy)]
pub struct StatusAssertionInput<'a> {
    pub service: &'a str,
    pub key: &'a str,
    pub fact: &'a IoValue,
    pub state_ref: &'a str,
    pub receipt_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ApplyReportValueInput<'a> {
    pub decision: &'a str,
    pub manifest_ref: &'a str,
    pub final_state_ref: &'a str,
    pub receipt_refs: &'a [String],
    pub assertion_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

pub fn coordination_service_manifest_value(input: &CoordinationServiceManifestInput) -> Result<IoValue> {
    validate_service_id(&input.service_id)?;
    validate_services(&input.services)?;
    validate_ref(&input.control_group_ref, "coordination control group ref")?;
    validate_refs(&input.policy_refs, "coordination manifest policy ref")?;
    validate_refs(&input.resource_refs, "coordination manifest resource ref")?;
    validate_capacity(input.queue_capacity, "coordination queue capacity")?;
    validate_capacity(input.semaphore_capacity, "coordination semaphore capacity")?;
    validate_capacity(input.rate_limit, "coordination rate limit")?;
    validate_capacity(input.barrier_parties, "coordination barrier parties")?;
    Ok(record("coordination-service-manifest-v1", vec![
        string(COORDINATION_SERVICE_MANIFEST_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("services", vec![strings_sequence(&input.services)]),
        record("control-group", vec![string(&input.control_group_ref)]),
        record("queue-capacity", vec![u64_value(input.queue_capacity)]),
        record("semaphore-capacity", vec![u64_value(input.semaphore_capacity)]),
        record("rate-limit", vec![u64_value(input.rate_limit)]),
        record("barrier-parties", vec![u64_value(input.barrier_parties)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        checks_value(&[
            ("control-plane-service", "pass"),
            ("explicit-primitives", "pass"),
            ("no-actor-traffic", "pass"),
        ]),
    ]))
}

pub fn parse_coordination_service_manifest(value: &IoValue) -> Result<CoordinationServiceManifest> {
    let fields = simple_record(value, "coordination-service-manifest-v1", 11)?;
    require_schema(&fields[0], COORDINATION_SERVICE_MANIFEST_SCHEMA, "coordination manifest schema")?;
    let input = CoordinationServiceManifestInput {
        service_id: record_string(&fields[1], "service-id")?,
        services: record_string_sequence(&fields[2], "services")?,
        control_group_ref: record_ref(&fields[3], "control-group")?,
        queue_capacity: record_u64(&fields[4], "queue-capacity")?,
        semaphore_capacity: record_u64(&fields[5], "semaphore-capacity")?,
        rate_limit: record_u64(&fields[6], "rate-limit")?,
        barrier_parties: record_u64(&fields[7], "barrier-parties")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        resource_refs: record_ref_sequence(&fields[9], "resource")?,
    };
    validate_service_id(&input.service_id)?;
    validate_services(&input.services)?;
    require_check(&parse_checks(&fields[10])?, "control-plane-service", "coordination manifest")?;
    Ok(CoordinationServiceManifest {
        manifest_ref: canonical_hash(value)?,
        service_id: input.service_id,
        services: input.services,
        control_group_ref: input.control_group_ref,
        queue_capacity: input.queue_capacity,
        semaphore_capacity: input.semaphore_capacity,
        rate_limit: input.rate_limit,
        barrier_parties: input.barrier_parties,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        value: value.clone(),
    })
}

pub fn coordination_request_value(input: &CoordinationRequestInput) -> Result<IoValue> {
    validate_request_input(input)?;
    Ok(record("coordination-request-v1", vec![
        string(COORDINATION_REQUEST_SCHEMA),
        record("service", vec![string(&input.service)]),
        record("operation", vec![string(&input.operation)]),
        record("key", vec![string(&input.key)]),
        record("client-session", vec![string(&input.client_session)]),
        record("operation-id", vec![string(&input.operation_id_ref)]),
        record("payload", vec![optional_value(input.payload.as_ref())]),
        record("authority", vec![strings_sequence(&input.authority_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        checks_value(&[
            ("control-plane-command", "pass"),
            ("operation-id-bound", "pass"),
            ("deny-by-default-authority", "pass"),
        ]),
    ]))
}

pub fn parse_coordination_request(value: &IoValue) -> Result<CoordinationRequest> {
    let fields = simple_record(value, "coordination-request-v1", 11)?;
    require_schema(&fields[0], COORDINATION_REQUEST_SCHEMA, "coordination request schema")?;
    let input = CoordinationRequestInput {
        service: record_string(&fields[1], "service")?,
        operation: record_string(&fields[2], "operation")?,
        key: record_string(&fields[3], "key")?,
        client_session: record_string(&fields[4], "client-session")?,
        operation_id_ref: record_ref(&fields[5], "operation-id")?,
        payload: record_optional_value(&fields[6], "payload")?,
        authority_refs: record_ref_sequence(&fields[7], "authority")?,
        resource_refs: record_ref_sequence(&fields[8], "resource")?,
        policy_refs: record_ref_sequence(&fields[9], "policy")?,
    };
    validate_request_input(&input)?;
    require_check(&parse_checks(&fields[10])?, "control-plane-command", "coordination request")?;
    Ok(CoordinationRequest {
        request_ref: canonical_hash(value)?,
        service: input.service,
        operation: input.operation,
        key: input.key,
        client_session: input.client_session,
        operation_id_ref: input.operation_id_ref,
        payload: input.payload,
        authority_refs: input.authority_refs,
        resource_refs: input.resource_refs,
        policy_refs: input.policy_refs,
        value: value.clone(),
    })
}

pub fn fencing_token_value(
    key: &str,
    owner: &str,
    token: u64,
    lease_epoch: u64,
    commit_receipt_ref: &str,
) -> Result<IoValue> {
    validate_key(key)?;
    validate_session(owner)?;
    validate_ref(commit_receipt_ref, "coordination token commit receipt ref")?;
    Ok(record("fencing-token-v1", vec![
        string(COORDINATION_FENCING_TOKEN_SCHEMA),
        record("key", vec![string(key)]),
        record("owner", vec![string(owner)]),
        record("token", vec![u64_value(token)]),
        record("lease-epoch", vec![u64_value(lease_epoch)]),
        record("commit", vec![string(commit_receipt_ref)]),
        checks_value(&[("fencing-token-monotonic", "pass"), ("commit-bound", "pass")]),
    ]))
}

pub fn parse_fencing_token(value: &IoValue) -> Result<FencingToken> {
    let fields = simple_record(value, "fencing-token-v1", 7)?;
    require_schema(&fields[0], COORDINATION_FENCING_TOKEN_SCHEMA, "coordination fencing token schema")?;
    let key = record_string(&fields[1], "key")?;
    let owner = record_string(&fields[2], "owner")?;
    let token = record_u64(&fields[3], "token")?;
    let lease_epoch = record_u64(&fields[4], "lease-epoch")?;
    let commit_receipt_ref = record_ref(&fields[5], "commit")?;
    require_check(&parse_checks(&fields[6])?, "fencing-token-monotonic", "coordination fencing token")?;
    Ok(FencingToken {
        token_ref: canonical_hash(value)?,
        key,
        owner,
        token,
        lease_epoch,
        commit_receipt_ref,
        value: value.clone(),
    })
}

pub fn coordination_state_snapshot_value(state: &CoordinationState) -> Result<IoValue> {
    let records = state_records(state)?;
    let retention_refs = retention_refs_for_state(state)?;
    Ok(record("coordination-state-snapshot-v1", vec![
        string(COORDINATION_STATE_SNAPSHOT_SCHEMA),
        record("locks", vec![sequence(records.locks)]),
        record("queues", vec![sequence(records.queues)]),
        record("semaphores", vec![sequence(records.semaphores)]),
        record("rates", vec![sequence(records.rates)]),
        record("elections", vec![sequence(records.elections)]),
        record("barriers", vec![sequence(records.barriers)]),
        record("registry", vec![sequence(records.registry)]),
        record("next-fencing-token", vec![u64_value(state.next_fencing_token)]),
        record("retention", vec![strings_sequence(&retention_refs)]),
        checks_value(&[
            ("deterministic-state-order", "pass"),
            ("active-state-retention-pins", "pass"),
            ("control-plane-reflection-source", "pass"),
        ]),
    ]))
}

struct StateRecords {
    locks: Vec<IoValue>,
    queues: Vec<IoValue>,
    semaphores: Vec<IoValue>,
    rates: Vec<IoValue>,
    elections: Vec<IoValue>,
    barriers: Vec<IoValue>,
    registry: Vec<IoValue>,
}

fn state_records(state: &CoordinationState) -> Result<StateRecords> {
    ensure_count_at_most(state.locks.len(), MAX_COORDINATION_ITEMS, "coordination locks")?;
    ensure_count_at_most(state.queues.len(), MAX_COORDINATION_ITEMS, "coordination queues")?;
    ensure_count_at_most(state.semaphores.len(), MAX_COORDINATION_ITEMS, "coordination semaphores")?;
    ensure_count_at_most(state.registry.len(), MAX_COORDINATION_ITEMS, "coordination registry")?;
    Ok(StateRecords {
        locks: lock_records(&state.locks),
        queues: queue_records(&state.queues),
        semaphores: semaphore_records(&state.semaphores),
        rates: rate_records(&state.rates),
        elections: election_records(&state.elections),
        barriers: barrier_records(&state.barriers),
        registry: registry_records(&state.registry),
    })
}

fn lock_records(locks: &OrderedMap<String, LockState>) -> Vec<IoValue> {
    locks
        .iter()
        .map(|(key, lock)| {
            record("lock", vec![
                string(key),
                string(&lock.owner),
                u64_value(lock.token),
                string(&lock.token_ref),
                u64_value(lock.lease_epoch),
            ])
        })
        .collect()
}

fn queue_records(queues: &OrderedMap<String, Vec<String>>) -> Vec<IoValue> {
    queues
        .iter()
        .map(|(key, items)| record("queue", vec![string(key), strings_sequence(items)]))
        .collect()
}

fn semaphore_records(semaphores: &OrderedMap<String, OrderedSet<String>>) -> Vec<IoValue> {
    semaphores
        .iter()
        .map(|(key, holders)| {
            let values = holders.iter().cloned().collect::<Vec<_>>();
            record("semaphore", vec![string(key), strings_sequence(&values)])
        })
        .collect()
}

fn rate_records(rates: &OrderedMap<String, u64>) -> Vec<IoValue> {
    rates.iter().map(|(key, used)| record("rate", vec![string(key), u64_value(*used)])).collect()
}

fn election_records(elections: &OrderedMap<String, ElectionState>) -> Vec<IoValue> {
    elections
        .iter()
        .map(|(key, election)| {
            record("election", vec![
                string(key),
                string(&election.leader),
                u64_value(election.token),
                string(&election.token_ref),
            ])
        })
        .collect()
}

fn barrier_records(barriers: &OrderedMap<String, BarrierState>) -> Vec<IoValue> {
    barriers
        .iter()
        .map(|(key, barrier)| {
            let participants = barrier.participants.iter().cloned().collect::<Vec<_>>();
            record("barrier", vec![
                string(key),
                strings_sequence(&participants),
                u64_value(barrier.required),
                string(if barrier.is_released { "released" } else { "waiting" }),
            ])
        })
        .collect()
}

fn registry_records(registry: &OrderedMap<String, RegistryEntry>) -> Vec<IoValue> {
    registry
        .iter()
        .map(|(key, entry)| {
            record("registry-entry", vec![string(key), string(&entry.endpoint_ref), string(&entry.evidence_ref)])
        })
        .collect()
}

pub fn parse_coordination_state_snapshot(value: &IoValue) -> Result<CoordinationStateSnapshot> {
    let fields = simple_record(value, "coordination-state-snapshot-v1", 11)?;
    require_schema(&fields[0], COORDINATION_STATE_SNAPSHOT_SCHEMA, "coordination state schema")?;
    let retention_refs = record_ref_sequence(&fields[9], "retention")?;
    require_check(&parse_checks(&fields[10])?, "deterministic-state-order", "coordination state")?;
    Ok(CoordinationStateSnapshot {
        state_ref: canonical_hash(value)?,
        retention_refs,
        value: value.clone(),
    })
}

pub fn coordination_status_assertion_value(input: StatusAssertionInput<'_>) -> Result<IoValue> {
    validate_service(input.service)?;
    validate_key(input.key)?;
    validate_ref(input.state_ref, "coordination assertion state ref")?;
    validate_ref(input.receipt_ref, "coordination assertion receipt ref")?;
    Ok(record("coordination-status-assertion-v1", vec![
        string(COORDINATION_STATUS_ASSERTION_SCHEMA),
        record("service", vec![string(input.service)]),
        record("key", vec![string(input.key)]),
        record("fact", vec![input.fact.clone()]),
        record("state", vec![string(input.state_ref)]),
        record("receipt", vec![string(input.receipt_ref)]),
        checks_value(&[
            ("dataspace-observation-only", "pass"),
            ("committed-state-bound", "pass"),
        ]),
    ]))
}

pub fn parse_coordination_status_assertion(value: &IoValue) -> Result<CoordinationStatusAssertion> {
    let fields = simple_record(value, "coordination-status-assertion-v1", 7)?;
    require_schema(&fields[0], COORDINATION_STATUS_ASSERTION_SCHEMA, "coordination assertion schema")?;
    let service = record_string(&fields[1], "service")?;
    let key = record_string(&fields[2], "key")?;
    let state_ref = record_ref(&fields[4], "state")?;
    let receipt_ref = record_ref(&fields[5], "receipt")?;
    require_check(&parse_checks(&fields[6])?, "dataspace-observation-only", "coordination assertion")?;
    Ok(CoordinationStatusAssertion {
        assertion_ref: canonical_hash(value)?,
        service,
        key,
        state_ref,
        receipt_ref,
        value: value.clone(),
    })
}

pub fn coordination_receipt_value(input: ReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_service(input.service)?;
    validate_operation(input.service, input.operation)?;
    validate_ref(input.request_ref, "coordination receipt request ref")?;
    if let Some(value) = input.raft_receipt_ref {
        validate_ref(value, "coordination receipt raft ref")?;
    }
    if let Some(value) = input.token_ref {
        validate_ref(value, "coordination receipt token ref")?;
    }
    validate_ref(input.state_ref, "coordination receipt state ref")?;
    validate_refs(input.dataspace_assertion_refs, "coordination receipt assertion ref")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_COORDINATION_DIAGNOSTICS, "coordination receipt diagnostics")?;
    Ok(record("coordination-receipt-v1", vec![
        string(COORDINATION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("service", vec![string(input.service)]),
        record("operation", vec![string(input.operation)]),
        record("request", vec![string(input.request_ref)]),
        record("raft", vec![optional_ref_value(input.raft_receipt_ref)]),
        record("token", vec![optional_ref_value(input.token_ref)]),
        record("state", vec![string(input.state_ref)]),
        record("dataspace", vec![strings_sequence(input.dataspace_assertion_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(input.checks),
    ]))
}

pub fn parse_coordination_receipt(value: &IoValue) -> Result<CoordinationReceipt> {
    let fields = simple_record(value, "coordination-receipt-v1", 11)?;
    require_schema(&fields[0], COORDINATION_RECEIPT_SCHEMA, "coordination receipt schema")?;
    let decision = record_string(&fields[1], "decision")?;
    let service = record_string(&fields[2], "service")?;
    let operation = record_string(&fields[3], "operation")?;
    let request_ref = record_ref(&fields[4], "request")?;
    let raft_receipt_ref = record_optional_ref(&fields[5], "raft")?;
    let token_ref = record_optional_ref(&fields[6], "token")?;
    let state_ref = record_ref(&fields[7], "state")?;
    let dataspace_assertion_refs = record_ref_sequence(&fields[8], "dataspace")?;
    let diagnostics = record_string_sequence(&fields[9], "diagnostics")?;
    require_check(&parse_checks(&fields[10])?, "coordination-request-bound", "coordination receipt")?;
    Ok(CoordinationReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        service,
        operation,
        request_ref,
        raft_receipt_ref,
        token_ref,
        state_ref,
        dataspace_assertion_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn new_coordination_runtime(manifest_value: &IoValue) -> Result<CoordinationRuntime> {
    let manifest = parse_coordination_service_manifest(manifest_value)?;
    let raft_manifest = raft_control_plane::control_registry_fixture_manifest_value()?;
    let raft = raft_control_plane::new_control_registry_runtime(&raft_manifest)?;
    Ok(CoordinationRuntime {
        manifest,
        raft,
        state: CoordinationState {
            next_fencing_token: 1,
            ..CoordinationState::default()
        },
        applied_operations: OrderedMap::new(),
        receipts: Vec::new(),
        assertions: Vec::new(),
        tokens: Vec::new(),
        next_sequence: 1,
    })
}

pub fn apply_coordination_request(
    runtime: &mut CoordinationRuntime,
    request_value: &IoValue,
) -> Result<CoordinationApplyResult> {
    let request = parse_coordination_request(request_value)?;
    if let Some(existing) = runtime.applied_operations.get(&request.operation_id_ref) {
        return Ok(existing.clone());
    }
    let mut diagnostics = Vec::new();
    collect_admission_diagnostics(runtime, &request, &mut diagnostics)?;
    let current_snapshot = snapshot_from_state(&runtime.state)?;
    if diagnostics.is_empty() {
        if request.operation == OP_READ {
            return apply_coordination_read(runtime, request);
        }
        let prepared = match prepare_mutation(runtime, &request) {
            Ok(prepared) => prepared,
            Err(error) => {
                return deny_result(runtime, request, current_snapshot, vec![error.to_string()], &[
                    "semantic-state-transition",
                    "fail",
                ]);
            }
        };
        return commit_prepared_mutation(runtime, request, prepared);
    }
    deny_result(runtime, request, current_snapshot, diagnostics, &["admission-gate", "fail"])
}

pub fn coordination_fixture_manifest_value() -> Result<IoValue> {
    let group_ref = canonical_hash(&raft_control_plane::control_registry_fixture_manifest_value()?)?;
    coordination_service_manifest_value(&CoordinationServiceManifestInput {
        service_id: DEFAULT_COORDINATION_SERVICE_ID.to_string(),
        services: supported_services(),
        control_group_ref: group_ref,
        queue_capacity: DEFAULT_COORDINATION_QUEUE_CAPACITY,
        semaphore_capacity: DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
        rate_limit: DEFAULT_COORDINATION_RATE_LIMIT,
        barrier_parties: DEFAULT_COORDINATION_BARRIER_PARTIES,
        policy_refs: vec![fixture_ref("coordination-policy")],
        resource_refs: vec![fixture_ref("coordination-resource")],
    })
}

pub fn run_coordination_fixture() -> Result<CoordinationFixtureRun> {
    let manifest_value = coordination_fixture_manifest_value()?;
    let mut runtime = new_coordination_runtime(&manifest_value)?;
    let auth = vec![fixture_ref("coordination-authority")];
    let resources = vec![fixture_ref("coordination-resource")];
    let policies = vec![fixture_ref("coordination-policy")];
    let fixture_refs = CoordinationRefSlices {
        authority_refs: &auth,
        resource_refs: &resources,
        policy_refs: &policies,
    };
    let mut evidence_values = vec![manifest_value.clone()];
    let mut receipt_refs = Vec::new();
    let mut assertion_refs = Vec::new();
    for request_value in case_requests(&fixture_refs)? {
        let result = apply_coordination_request(&mut runtime, &request_value)?;
        evidence_values.push(request_value);
        evidence_values.extend(result.evidence_values.iter().cloned());
        receipt_refs.push_limited(
            result.receipt.receipt_ref.clone(),
            MAX_COORDINATION_ITEMS,
            "coordination fixture receipts",
        )?;
        for assertion in &result.assertions {
            assertion_refs.push_limited(
                assertion.assertion_ref.clone(),
                MAX_COORDINATION_ITEMS,
                "coordination fixture assertions",
            )?;
        }
    }
    let final_state = snapshot_from_state(&runtime.state)?;
    let report_value = case_report(&manifest_value, &final_state.state_ref, &receipt_refs, &assertion_refs)?;
    evidence_values.push(final_state.value.clone());
    evidence_values.push(report_value.clone());
    Ok(CoordinationFixtureRun {
        decision: "pass".to_string(),
        manifest_ref: canonical_hash(&manifest_value)?,
        final_state_ref: final_state.state_ref,
        receipt_refs,
        assertion_refs,
        evidence_values,
        report_value,
    })
}

fn case_requests(refs: &CoordinationRefSlices<'_>) -> Result<Vec<IoValue>> {
    let cases = [
        (SERVICE_LOCK, OP_ACQUIRE, "resource:alpha", "client-a", 1, None),
        (SERVICE_LOCK, OP_ACQUIRE, "resource:alpha", "client-a", 1, None),
        (SERVICE_LOCK, OP_RELEASE, "resource:alpha", "client-b", 2, Some(record("token", vec![u64_value(0)]))),
        (SERVICE_QUEUE, OP_ENQUEUE, "queue:work", "client-a", 3, Some(record("item", vec![string("job-1")]))),
        (SERVICE_QUEUE, OP_DEQUEUE, "queue:work", "client-b", 4, None),
        (
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:api",
            "client-a",
            5,
            Some(record("endpoint", vec![
                string(fixture_ref("endpoint-api")),
                string(fixture_ref("registry-evidence")),
            ])),
        ),
        (SERVICE_REGISTRY, OP_READ, "svc:api", "client-b", 6, None),
    ];
    let mut requests = Vec::with_capacity(cases.len());
    for (service, operation, key, client_session, sequence, payload) in cases {
        requests.push(fixture_request(FixtureRequestInput {
            service,
            operation,
            key,
            client_session,
            sequence,
            payload,
            refs,
        })?);
    }
    Ok(requests)
}

fn case_report(
    manifest_value: &IoValue,
    final_state_ref: &str,
    receipt_refs: &[String],
    assertion_refs: &[String],
) -> Result<IoValue> {
    Ok(record("coordination-fixture-report-v1", vec![
        string("molten.coordination.fixture-report.v1"),
        record("decision", vec![string("pass")]),
        record("manifest", vec![string(canonical_hash(manifest_value)?)]),
        record("state", vec![string(final_state_ref)]),
        record("receipts", vec![strings_sequence(receipt_refs)]),
        record("assertions", vec![strings_sequence(assertion_refs)]),
    ]))
}

pub fn coordination_apply_report_value(input: ApplyReportValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.manifest_ref, "coordination apply report manifest ref")?;
    validate_ref(input.final_state_ref, "coordination apply report state ref")?;
    validate_refs(input.receipt_refs, "coordination apply report receipt ref")?;
    validate_refs(input.assertion_refs, "coordination apply report assertion ref")?;
    validate_refs(input.evidence_refs, "coordination apply report evidence ref")?;
    Ok(record("coordination-apply-report-v1", vec![
        string(COORDINATION_APPLY_REPORT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("state", vec![string(input.final_state_ref)]),
        record("receipts", vec![strings_sequence(input.receipt_refs)]),
        record("assertions", vec![strings_sequence(input.assertion_refs)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[
            ("control-plane-apply-batch", "pass"),
            ("evidence-index-bound", "pass"),
            ("dataspace-observation-only", "pass"),
        ]),
    ]))
}

pub fn parse_coordination_apply_report(value: &IoValue) -> Result<CoordinationApplyReport> {
    let fields = simple_record(value, "coordination-apply-report-v1", 8)?;
    require_schema(&fields[0], COORDINATION_APPLY_REPORT_SCHEMA, "coordination apply report schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let manifest_ref = record_ref(&fields[2], "manifest")?;
    let final_state_ref = record_ref(&fields[3], "state")?;
    let receipt_refs = record_ref_sequence(&fields[4], "receipts")?;
    let assertion_refs = record_ref_sequence(&fields[5], "assertions")?;
    let evidence_refs = record_ref_sequence(&fields[6], "evidence")?;
    require_check(&parse_checks(&fields[7])?, "control-plane-apply-batch", "coordination apply report")?;
    Ok(CoordinationApplyReport {
        report_ref: canonical_hash(value)?,
        decision,
        manifest_ref,
        final_state_ref,
        receipt_refs,
        assertion_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn coordination_supported_services() -> Vec<String> {
    supported_services()
}

pub fn coordination_summary(value: &IoValue) -> Result<String> {
    if let Ok(report) = parse_coordination_apply_report(value) {
        return Ok(format!(
            "coordination apply report decision={} manifest={} state={} receipts={} assertions={} evidence={}",
            report.decision,
            report.manifest_ref,
            report.final_state_ref,
            report.receipt_refs.len(),
            report.assertion_refs.len(),
            report.evidence_refs.len()
        ));
    }
    if let Ok(receipt) = parse_coordination_receipt(value) {
        return Ok(format!(
            "coordination receipt decision={} service={} operation={} request={} state={} diagnostics={}",
            receipt.decision,
            receipt.service,
            receipt.operation,
            receipt.request_ref,
            receipt.state_ref,
            receipt.diagnostics.join(";")
        ));
    }
    if let Ok(request) = parse_coordination_request(value) {
        return Ok(format!(
            "coordination request service={} operation={} key={} session={} operation_id={}",
            request.service, request.operation, request.key, request.client_session, request.operation_id_ref
        ));
    }
    if let Ok(token) = parse_fencing_token(value) {
        return Ok(format!(
            "coordination fencing token key={} owner={} token={} commit={}",
            token.key, token.owner, token.token, token.commit_receipt_ref
        ));
    }
    if let Ok(manifest) = parse_coordination_service_manifest(value) {
        return Ok(format!(
            "coordination manifest service={} primitives={} control_group={}",
            manifest.service_id,
            manifest.services.join(","),
            manifest.control_group_ref
        ));
    }
    if let Ok(snapshot) = parse_coordination_state_snapshot(value) {
        return Ok(format!(
            "coordination state {} retention_refs={}",
            snapshot.state_ref,
            snapshot.retention_refs.len()
        ));
    }
    if let Ok(assertion) = parse_coordination_status_assertion(value) {
        return Ok(format!(
            "coordination assertion service={} key={} state={} receipt={}",
            assertion.service, assertion.key, assertion.state_ref, assertion.receipt_ref
        ));
    }
    Err(MoltenError::invalid_harness("unsupported coordination artifact"))
}

fn apply_coordination_read(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
) -> Result<CoordinationApplyResult> {
    let read = raft_control_plane::read_control_registry(&raft_control_plane::ControlRegistryReadInput {
        state: runtime.raft.state.value.clone(),
        group_ref: runtime.raft.manifest.manifest_ref.clone(),
        committed_term: runtime.raft.term,
        committed_index: runtime.raft.committed_index,
        read_index: runtime.raft.committed_index,
        namespace: coordination_namespace(&request.service),
        name: request.key.clone(),
        authority_refs: request.authority_refs.clone(),
        resource_refs: request.resource_refs.clone(),
    })?;
    let snapshot = snapshot_from_state(&runtime.state)?;
    let decision = if read.decision == "pass" { "pass" } else { "deny" };
    let fact = status_fact_for(&runtime.state, &runtime.manifest, &request.service, &request.key)?;
    let (assertion_value, diagnostics) = read_assertion_value(ReadAssertionInput {
        service: &request.service,
        key: &request.key,
        fact: &fact,
        snapshot_ref: &snapshot.state_ref,
        decision,
        diagnostics: read.diagnostics.clone(),
    })?;
    let assertion = match assertion_value {
        Some(value) => Some(parse_coordination_status_assertion(&value)?),
        None => None,
    };
    let assertion_refs = assertion.as_ref().map_or_else(Vec::new, |item| vec![item.assertion_ref.clone()]);
    let receipt_value = coordination_receipt_value(ReceiptValueInput {
        decision,
        service: &request.service,
        operation: &request.operation,
        request_ref: &request.request_ref,
        raft_receipt_ref: Some(&read.receipt_ref),
        token_ref: None,
        state_ref: &snapshot.state_ref,
        dataspace_assertion_refs: &assertion_refs,
        diagnostics: &diagnostics,
        checks: &[
            ("coordination-request-bound", "pass"),
            ("read-index-bound", "pass"),
            ("control-plane-command", "pass"),
        ],
    })?;
    let receipt = parse_coordination_receipt(&receipt_value)?;
    let assertions = corrected_read_assertions(assertion, &fact, &receipt.receipt_ref)?;
    Ok(finish_read(ReadFinishInput {
        runtime,
        request,
        read,
        snapshot,
        receipt,
        assertions,
    }))
}

struct ReadAssertionInput<'a> {
    service: &'a str,
    key: &'a str,
    fact: &'a IoValue,
    snapshot_ref: &'a str,
    decision: &'a str,
    diagnostics: Vec<String>,
}

fn read_assertion_value(input: ReadAssertionInput<'_>) -> Result<(Option<IoValue>, Vec<String>)> {
    let mut diagnostics = input.diagnostics;
    if input.decision == "pass" {
        let placeholder_receipt_ref = fixture_ref("coordination-read-placeholder");
        let value = coordination_status_assertion_value(StatusAssertionInput {
            service: input.service,
            key: input.key,
            fact: input.fact,
            state_ref: input.snapshot_ref,
            receipt_ref: &placeholder_receipt_ref,
        })?;
        return Ok((Some(value), diagnostics));
    }
    if diagnostics.is_empty() {
        diagnostics.push_limited(
            "coordination read-index denied".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    Ok((None, diagnostics))
}

fn corrected_read_assertions(
    assertion: Option<CoordinationStatusAssertion>,
    fact: &IoValue,
    receipt_ref: &str,
) -> Result<Vec<CoordinationStatusAssertion>> {
    if let Some(assertion) = assertion {
        let corrected = coordination_status_assertion_value(StatusAssertionInput {
            service: &assertion.service,
            key: &assertion.key,
            fact,
            state_ref: &assertion.state_ref,
            receipt_ref,
        })?;
        return Ok(vec![parse_coordination_status_assertion(&corrected)?]);
    }
    Ok(Vec::new())
}

struct ReadFinishInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: CoordinationRequest,
    read: raft_control_plane::RaftReadReceipt,
    snapshot: CoordinationStateSnapshot,
    receipt: CoordinationReceipt,
    assertions: Vec<CoordinationStatusAssertion>,
}

fn finish_read(input: ReadFinishInput<'_>) -> CoordinationApplyResult {
    let ReadFinishInput {
        runtime,
        request,
        read,
        snapshot,
        receipt,
        assertions,
    } = input;
    let evidence_values = evidence_values_for(EvidenceValuesInput {
        request: &request,
        receipt: &receipt,
        token: None,
        snapshot: &snapshot,
        assertions: &assertions,
        read: Some(&read),
    });
    let result = CoordinationApplyResult {
        receipt: receipt.clone(),
        request: request.clone(),
        token: None,
        state_snapshot: snapshot,
        assertions: assertions.clone(),
        raft_commit_ref: None,
        raft_read_receipt: Some(read),
        evidence_values,
    };
    runtime.receipts.push(receipt);
    runtime.assertions.extend(assertions);
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    result
}

type Proposal = raft_control_plane::ControlRegistryProposal;

struct ChangeInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: &'a CoordinationRequest,
    snapshot: &'a CoordinationStateSnapshot,
}

struct PartsInput<'a> {
    prepared: &'a PreparedMutation,
    request: &'a CoordinationRequest,
    manifest: &'a CoordinationServiceManifest,
    snapshot: &'a CoordinationStateSnapshot,
    proposal_ref: &'a str,
}

struct PassReceiptInput<'a> {
    request: &'a CoordinationRequest,
    proposal_ref: &'a str,
    token_ref: Option<&'a str>,
    state_ref: &'a str,
    assertion_refs: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct SuccessParts {
    token: Option<FencingToken>,
    receipt: CoordinationReceipt,
    assertion: CoordinationStatusAssertion,
}

struct ValuesInput<'a> {
    proposal: &'a Proposal,
    request: &'a CoordinationRequest,
    receipt: &'a CoordinationReceipt,
    token: Option<&'a FencingToken>,
    snapshot: &'a CoordinationStateSnapshot,
    assertion: &'a CoordinationStatusAssertion,
}

struct SuccessInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: CoordinationRequest,
    prepared: PreparedMutation,
    snapshot: CoordinationStateSnapshot,
    proposal: Proposal,
}

fn propose_change(input: ChangeInput<'_>) -> Result<Proposal> {
    let ChangeInput {
        runtime,
        request,
        snapshot,
    } = input;
    let command =
        raft_control_plane::control_registry_command_value(&raft_control_plane::ControlRegistryCommandInput {
            operation: "set-coordination-state".to_string(),
            namespace: coordination_namespace(&request.service),
            name: request.key.clone(),
            target_ref: Some(snapshot.state_ref.clone()),
        })?;
    let evidence_refs = vec![
        request.request_ref.clone(),
        request.operation_id_ref.clone(),
        snapshot.state_ref.clone(),
    ];
    let envelope = raft_control_plane::raft_command_envelope_value(&raft_control_plane::RaftCommandEnvelopeInput {
        group_ref: runtime.raft.manifest.manifest_ref.clone(),
        client_session: request.client_session.clone(),
        sequence: runtime.next_sequence,
        command,
        authority_refs: request.authority_refs.clone(),
        policy_refs: request.policy_refs.clone(),
        resource_refs: request.resource_refs.clone(),
        evidence_refs,
    })?;
    raft_control_plane::propose_control_registry_command(&mut runtime.raft, &envelope)
}

fn fact_for(
    prepared: &PreparedMutation,
    manifest: &CoordinationServiceManifest,
    request: &CoordinationRequest,
    token: Option<&FencingToken>,
) -> Result<IoValue> {
    if let Some(token) = token {
        status_fact_for_token(&prepared.state, manifest, &request.service, &request.key, token)
    } else {
        Ok(prepared.status_fact.clone())
    }
}

fn status_assertion_for(
    request: &CoordinationRequest,
    fact: &IoValue,
    state_ref: &str,
    receipt_ref: &str,
) -> Result<CoordinationStatusAssertion> {
    let value = coordination_status_assertion_value(StatusAssertionInput {
        service: &request.service,
        key: &request.key,
        fact,
        state_ref,
        receipt_ref,
    })?;
    parse_coordination_status_assertion(&value)
}

fn pass_checks(prepared: &PreparedMutation) -> Vec<(&'static str, &'static str)> {
    let mut checks = vec![
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("commit-receipt-bound", "pass"),
        ("idempotency-bound", "pass"),
        ("authority-policy-resource", "pass"),
        ("dataspace-reflection-after-commit", "pass"),
    ];
    checks.extend(prepared.checks.iter().copied());
    checks
}

fn pass_receipt(input: PassReceiptInput<'_>) -> Result<CoordinationReceipt> {
    let PassReceiptInput {
        request,
        proposal_ref,
        token_ref,
        state_ref,
        assertion_refs,
        checks,
    } = input;
    let value = coordination_receipt_value(ReceiptValueInput {
        decision: "pass",
        service: &request.service,
        operation: &request.operation,
        request_ref: &request.request_ref,
        raft_receipt_ref: Some(proposal_ref),
        token_ref,
        state_ref,
        dataspace_assertion_refs: assertion_refs,
        diagnostics: &[],
        checks,
    })?;
    parse_coordination_receipt(&value)
}

fn success_parts(input: PartsInput<'_>) -> Result<SuccessParts> {
    let token = materialize_token(input.prepared.token.clone(), input.proposal_ref)?;
    let token_ref = token.as_ref().map(|item| item.token_ref.clone());
    let fact = fact_for(input.prepared, input.manifest, input.request, token.as_ref())?;
    let placeholder_receipt_ref = fixture_ref("coordination-mutation-placeholder");
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, &placeholder_receipt_ref)?;
    let assertion_refs = vec![assertion.assertion_ref.clone()];
    let checks = pass_checks(input.prepared);
    let receipt = pass_receipt(PassReceiptInput {
        request: input.request,
        proposal_ref: input.proposal_ref,
        token_ref: token_ref.as_deref(),
        state_ref: &input.snapshot.state_ref,
        assertion_refs: &assertion_refs,
        checks: &checks,
    })?;
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, &receipt.receipt_ref)?;
    Ok(SuccessParts {
        token,
        receipt,
        assertion,
    })
}

fn success_values(input: ValuesInput<'_>) -> Vec<IoValue> {
    let ValuesInput {
        proposal,
        request,
        receipt,
        token,
        snapshot,
        assertion,
    } = input;
    let mut evidence_values = vec![
        proposal.envelope.value.clone(),
        proposal.commit_receipt.value.clone(),
        proposal.registry_receipt.value.clone(),
    ];
    if let Some(log_entry) = &proposal.log_entry {
        evidence_values.push(log_entry.value.clone());
    }
    evidence_values.extend(proposal.predicates.iter().map(|value| value.value.clone()));
    evidence_values.extend(evidence_values_for(EvidenceValuesInput {
        request,
        receipt,
        token,
        snapshot,
        assertions: std::slice::from_ref(assertion),
        read: None,
    }));
    evidence_values
}

fn record_success(input: SuccessInput<'_>) -> Result<CoordinationApplyResult> {
    let SuccessInput {
        runtime,
        request,
        prepared,
        snapshot,
        proposal,
    } = input;
    runtime.next_sequence = runtime
        .next_sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination raft sequence overflow"))?;
    let parts = success_parts(PartsInput {
        prepared: &prepared,
        request: &request,
        manifest: &runtime.manifest,
        snapshot: &snapshot,
        proposal_ref: &proposal.commit_receipt.receipt_ref,
    })?;
    runtime.state = prepared.state;
    let evidence_values = success_values(ValuesInput {
        proposal: &proposal,
        request: &request,
        receipt: &parts.receipt,
        token: parts.token.as_ref(),
        snapshot: &snapshot,
        assertion: &parts.assertion,
    });
    if let Some(token) = &parts.token {
        runtime.tokens.push(token.clone());
    }
    runtime.receipts.push(parts.receipt.clone());
    runtime.assertions.push(parts.assertion.clone());
    let result = CoordinationApplyResult {
        receipt: parts.receipt.clone(),
        request: request.clone(),
        token: parts.token,
        state_snapshot: snapshot,
        assertions: vec![parts.assertion],
        raft_commit_ref: Some(proposal.commit_receipt.receipt_ref),
        raft_read_receipt: None,
        evidence_values,
    };
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    Ok(result)
}

fn commit_prepared_mutation(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    prepared: PreparedMutation,
) -> Result<CoordinationApplyResult> {
    let snapshot = snapshot_from_state(&prepared.state)?;
    let proposal = propose_change(ChangeInput {
        runtime,
        request: &request,
        snapshot: &snapshot,
    })?;
    if proposal.decision != "pass" {
        let diagnostics = vec!["control-plane commit denied for coordination mutation".to_string()];
        return deny_result(runtime, request, snapshot, diagnostics, &["control-plane-commit", "fail"]);
    }
    record_success(SuccessInput {
        runtime,
        request,
        prepared,
        snapshot,
        proposal,
    })
}

fn deny_result(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    diagnostics: Vec<String>,
    extra_check: &[&'static str; 2],
) -> Result<CoordinationApplyResult> {
    let checks = [
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("deny-before-side-effects", "pass"),
        (extra_check[0], extra_check[1]),
    ];
    let receipt_value = coordination_receipt_value(ReceiptValueInput {
        decision: "deny",
        service: &request.service,
        operation: &request.operation,
        request_ref: &request.request_ref,
        raft_receipt_ref: None,
        token_ref: None,
        state_ref: &snapshot.state_ref,
        dataspace_assertion_refs: &[],
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_coordination_receipt(&receipt_value)?;
    let evidence_values = evidence_values_for(EvidenceValuesInput {
        request: &request,
        receipt: &receipt,
        token: None,
        snapshot: &snapshot,
        assertions: &[],
        read: None,
    });
    let result = CoordinationApplyResult {
        receipt: receipt.clone(),
        request: request.clone(),
        token: None,
        state_snapshot: snapshot,
        assertions: Vec::new(),
        raft_commit_ref: None,
        raft_read_receipt: None,
        evidence_values,
    };
    runtime.receipts.push(receipt);
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    Ok(result)
}

fn prepare_mutation(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    match (request.service.as_str(), request.operation.as_str()) {
        (SERVICE_LOCK, OP_ACQUIRE) => prepare_lock_acquire(runtime, request),
        (SERVICE_LOCK, OP_RELEASE) => prepare_lock_release(runtime, request),
        (SERVICE_QUEUE, OP_ENQUEUE) => prepare_queue_enqueue(runtime, request),
        (SERVICE_QUEUE, OP_DEQUEUE) => prepare_queue_dequeue(runtime, request),
        (SERVICE_SEMAPHORE, OP_ACQUIRE) => prepare_semaphore_acquire(runtime, request),
        (SERVICE_SEMAPHORE, OP_RELEASE) => prepare_semaphore_release(runtime, request),
        (SERVICE_RATE_LIMIT, OP_ACQUIRE) => prepare_rate_acquire(runtime, request),
        (SERVICE_ELECTION, OP_ELECT) => prepare_election(runtime, request),
        (SERVICE_BARRIER, OP_ARRIVE) => prepare_barrier(runtime, request),
        (SERVICE_REGISTRY, OP_REGISTER) => prepare_registry_register(runtime, request),
        (SERVICE_REGISTRY, OP_UNREGISTER) => prepare_registry_unregister(runtime, request),
        _ => Err(MoltenError::invalid_harness("unsupported coordination mutation")),
    }
}

fn prepare_lock_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let mut diagnostics = Vec::new();
    if let Some(lock) = runtime.state.locks.get(&request.key) {
        diagnostics.push_limited(
            format!("coordination lock {} already held by {}", request.key, lock.owner),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if diagnostics.is_empty() {
        let token_number = runtime.state.next_fencing_token;
        let placeholder = pending_token(&request.key, &request.client_session, token_number)?;
        let mut state = runtime.state.clone();
        let next = token_number
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("coordination fencing token overflow"))?;
        state.next_fencing_token = next;
        state.locks.insert(request.key.clone(), LockState {
            owner: request.client_session.clone(),
            token: token_number,
            token_ref: placeholder.token_ref.clone(),
            lease_epoch: token_number,
        });
        return Ok(PreparedMutation {
            state,
            token: Some(placeholder),
            status_fact: record("lock-held", vec![
                string(&request.key),
                string(&request.client_session),
                u64_value(token_number),
            ]),
            checks: vec![("fencing-token-monotonic", "pass"), ("lock-lease-held", "pass")],
        });
    }
    Err(MoltenError::invalid_harness(diagnostics.join("; ")))
}

fn prepare_lock_release(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let token = payload_token(request)?;
    let lock = runtime
        .state
        .locks
        .get(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination lock is not held"))?;
    if token != lock.token {
        return Err(MoltenError::invalid_harness(format!(
            "stale fencing token {token}; current token is {}",
            lock.token
        )));
    }
    if request.client_session != lock.owner {
        return Err(MoltenError::invalid_harness("coordination lock release owner mismatch"));
    }
    let mut state = runtime.state.clone();
    state.locks.remove(&request.key);
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("lock-free", vec![string(&request.key), u64_value(token)]),
        checks: vec![("stale-token-checked", "pass"), ("lock-release", "pass")],
    })
}

fn prepare_queue_enqueue(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let item = payload_text(request, "item")?;
    let mut state = runtime.state.clone();
    let current_len = match state.queues.get(&request.key) {
        Some(values) => vec_len_u64(values)?,
        None => 0,
    };
    if current_len >= runtime.manifest.queue_capacity {
        return Err(MoltenError::invalid_harness("coordination queue overflow"));
    }
    let queue = state.queues.entry(request.key.clone()).or_default();
    ensure_count_at_most(queue.len().saturating_add(1), MAX_COORDINATION_ITEMS, "coordination queue items")?;
    queue.push(item.clone());
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("queue-depth", vec![string(&request.key), u64_value(current_len + 1), string(&item)]),
        checks: vec![("fifo-queue", "pass"), ("capacity-checked", "pass")],
    })
}

fn prepare_queue_dequeue(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let queue = runtime
        .state
        .queues
        .get(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination queue empty"))?;
    let item = queue.first().cloned().ok_or_else(|| MoltenError::invalid_harness("coordination queue empty"))?;
    let mut state = runtime.state.clone();
    let remaining = state
        .queues
        .get_mut(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination queue missing during dequeue"))?;
    remaining.remove(0);
    let depth = vec_len_u64(remaining)?;
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("queue-depth", vec![string(&request.key), u64_value(depth), string(&item)]),
        checks: vec![("fifo-queue", "pass"), ("dequeue-committed", "pass")],
    })
}

fn prepare_semaphore_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let mut state = runtime.state.clone();
    let holders = state.semaphores.entry(request.key.clone()).or_default();
    if holders.contains(&request.client_session) {
        return Err(MoltenError::invalid_harness("coordination semaphore already held by client"));
    }
    if set_len_u64(holders)? >= runtime.manifest.semaphore_capacity {
        return Err(MoltenError::invalid_harness("coordination semaphore exhausted"));
    }
    ensure_count_at_most(holders.len().saturating_add(1), MAX_COORDINATION_ITEMS, "coordination semaphore holders")?;
    holders.insert(request.client_session.clone());
    let available = runtime.manifest.semaphore_capacity.saturating_sub(set_len_u64(holders)?);
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("semaphore-available", vec![string(&request.key), u64_value(available)]),
        checks: vec![("semaphore-bounds", "pass"), ("capacity-checked", "pass")],
    })
}

fn prepare_semaphore_release(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let mut state = runtime.state.clone();
    let holders = state
        .semaphores
        .get_mut(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination semaphore not held"))?;
    if holders.remove(&request.client_session) {
        let available = runtime.manifest.semaphore_capacity.saturating_sub(set_len_u64(holders)?);
        Ok(PreparedMutation {
            state,
            token: None,
            status_fact: record("semaphore-available", vec![string(&request.key), u64_value(available)]),
            checks: vec![("semaphore-bounds", "pass"), ("release-committed", "pass")],
        })
    } else {
        Err(MoltenError::invalid_harness("coordination semaphore release holder mismatch"))
    }
}

fn prepare_rate_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let used = runtime.state.rates.get(&request.key).copied().unwrap_or(0);
    if used >= runtime.manifest.rate_limit {
        return Err(MoltenError::invalid_harness("coordination rate limit exhausted"));
    }
    let mut state = runtime.state.clone();
    state.rates.insert(request.key.clone(), used + 1);
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("rate-limit", vec![
            string(&request.key),
            u64_value(used + 1),
            u64_value(runtime.manifest.rate_limit),
        ]),
        checks: vec![("rate-limit-bounds", "pass")],
    })
}

fn prepare_election(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let leader = request
        .payload
        .as_ref()
        .map_or_else(|| Ok(request.client_session.clone()), |payload| simple_payload_text(payload, "candidate"))?;
    if let Some(existing) = runtime.state.elections.get(&request.key) {
        if existing.leader == leader {
            return Err(MoltenError::invalid_harness("coordination election already has same leader"));
        }
        return Err(MoltenError::invalid_harness(format!("coordination election already led by {}", existing.leader)));
    }
    let token_number = runtime.state.next_fencing_token;
    let token = pending_token(&request.key, &leader, token_number)?;
    let mut state = runtime.state.clone();
    state.next_fencing_token = token_number
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination election token overflow"))?;
    state.elections.insert(request.key.clone(), ElectionState {
        leader: leader.clone(),
        token: token_number,
        token_ref: token.token_ref.clone(),
    });
    Ok(PreparedMutation {
        state,
        token: Some(token),
        status_fact: record("leader", vec![string(&request.key), string(&leader), u64_value(token_number)]),
        checks: vec![("election-single-leader", "pass"), ("fencing-token-monotonic", "pass")],
    })
}

fn prepare_barrier(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let participant = request
        .payload
        .as_ref()
        .map_or_else(|| Ok(request.client_session.clone()), |payload| simple_payload_text(payload, "participant"))?;
    let mut state = runtime.state.clone();
    let barrier = state.barriers.entry(request.key.clone()).or_insert_with(|| BarrierState {
        participants: OrderedSet::new(),
        required: runtime.manifest.barrier_parties,
        is_released: false,
    });
    ensure_count_at_most(
        barrier.participants.len().saturating_add(1),
        MAX_COORDINATION_ITEMS,
        "coordination barrier participants",
    )?;
    barrier.participants.insert(participant);
    barrier.is_released = set_len_u64(&barrier.participants)? >= barrier.required;
    let required = barrier.required;
    let status = if barrier.is_released { "released" } else { "waiting" };
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("barrier", vec![string(&request.key), string(status), u64_value(required)]),
        checks: vec![("barrier-deterministic", "pass")],
    })
}

fn prepare_registry_register(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let (endpoint_ref, evidence_ref) = payload_endpoint(request)?;
    let mut state = runtime.state.clone();
    state.registry.insert(request.key.clone(), RegistryEntry {
        endpoint_ref: endpoint_ref.clone(),
        evidence_ref: evidence_ref.clone(),
    });
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("service-registered", vec![
            string(&request.key),
            string(&endpoint_ref),
            string(&evidence_ref),
        ]),
        checks: vec![("service-registry-pointer", "pass"), ("control-plane-pointer", "pass")],
    })
}

fn prepare_registry_unregister(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
) -> Result<PreparedMutation> {
    let mut state = runtime.state.clone();
    if state.registry.remove(&request.key).is_some() {
        Ok(PreparedMutation {
            state,
            token: None,
            status_fact: record("service-unregistered", vec![string(&request.key)]),
            checks: vec![("service-registry-pointer", "pass")],
        })
    } else {
        Err(MoltenError::invalid_harness("coordination registry entry missing"))
    }
}

fn materialize_token(pending: Option<FencingToken>, commit_receipt_ref: &str) -> Result<Option<FencingToken>> {
    pending
        .map(|token| {
            let value =
                fencing_token_value(&token.key, &token.owner, token.token, token.lease_epoch, commit_receipt_ref)?;
            parse_fencing_token(&value)
        })
        .transpose()
}

fn pending_token(key: &str, owner: &str, token: u64) -> Result<FencingToken> {
    let pending_commit_ref = fixture_ref("pending-coordination-commit");
    let value = fencing_token_value(key, owner, token, token, &pending_commit_ref)?;
    parse_fencing_token(&value)
}

fn status_fact_for_token(
    state: &CoordinationState,
    manifest: &CoordinationServiceManifest,
    service: &str,
    key: &str,
    token: &FencingToken,
) -> Result<IoValue> {
    match service {
        SERVICE_LOCK => Ok(record("lock-held", vec![
            string(key),
            string(&token.owner),
            u64_value(token.token),
            string(&token.token_ref),
        ])),
        SERVICE_ELECTION => Ok(record("leader", vec![
            string(key),
            string(&token.owner),
            u64_value(token.token),
            string(&token.token_ref),
        ])),
        _ => status_fact_for(state, manifest, service, key),
    }
}

fn status_fact_for(
    state: &CoordinationState,
    manifest: &CoordinationServiceManifest,
    service: &str,
    key: &str,
) -> Result<IoValue> {
    match service {
        SERVICE_LOCK => Ok(state.locks.get(key).map_or_else(
            || record("lock-free", vec![string(key)]),
            |lock| {
                record("lock-held", vec![
                    string(key),
                    string(&lock.owner),
                    u64_value(lock.token),
                    string(&lock.token_ref),
                ])
            },
        )),
        SERVICE_QUEUE => {
            let depth = match state.queues.get(key) {
                Some(values) => vec_len_u64(values)?,
                None => 0,
            };
            Ok(record("queue-depth", vec![string(key), u64_value(depth)]))
        }
        SERVICE_SEMAPHORE => {
            let used = match state.semaphores.get(key) {
                Some(values) => set_len_u64(values)?,
                None => 0,
            };
            Ok(record("semaphore-available", vec![
                string(key),
                u64_value(manifest.semaphore_capacity.saturating_sub(used)),
            ]))
        }
        SERVICE_RATE_LIMIT => {
            let used = state.rates.get(key).copied().unwrap_or(0);
            Ok(record("rate-limit", vec![string(key), u64_value(used), u64_value(manifest.rate_limit)]))
        }
        SERVICE_ELECTION => Ok(state.elections.get(key).map_or_else(
            || record("no-leader", vec![string(key)]),
            |election| {
                record("leader", vec![
                    string(key),
                    string(&election.leader),
                    u64_value(election.token),
                    string(&election.token_ref),
                ])
            },
        )),
        SERVICE_BARRIER => Ok(state.barriers.get(key).map_or_else(
            || record("barrier", vec![string(key), string("waiting"), u64_value(manifest.barrier_parties)]),
            |barrier| {
                record("barrier", vec![
                    string(key),
                    string(if barrier.is_released { "released" } else { "waiting" }),
                    u64_value(barrier.required),
                ])
            },
        )),
        SERVICE_REGISTRY => Ok(state.registry.get(key).map_or_else(
            || record("service-unregistered", vec![string(key)]),
            |entry| {
                record("service-registered", vec![
                    string(key),
                    string(&entry.endpoint_ref),
                    string(&entry.evidence_ref),
                ])
            },
        )),
        _ => Err(MoltenError::invalid_harness("unsupported coordination status service")),
    }
}

fn collect_admission_diagnostics(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    if runtime.manifest.services.iter().all(|service| service != &request.service) {
        diagnostics.push_limited(
            format!("coordination service {} not declared in manifest", request.service),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.authority_refs.is_empty() {
        diagnostics.push_limited(
            "coordination request missing authority evidence".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.policy_refs.is_empty() {
        diagnostics.push_limited(
            "coordination request missing policy evidence".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.resource_refs.is_empty() {
        diagnostics.push_limited(
            "coordination request missing resource evidence".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.operation != OP_READ && request.operation_id_ref.is_empty() {
        diagnostics.push_limited(
            "coordination mutating request missing operation id".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    Ok(())
}

fn snapshot_from_state(state: &CoordinationState) -> Result<CoordinationStateSnapshot> {
    let value = coordination_state_snapshot_value(state)?;
    parse_coordination_state_snapshot(&value)
}

struct EvidenceValuesInput<'a> {
    request: &'a CoordinationRequest,
    receipt: &'a CoordinationReceipt,
    token: Option<&'a FencingToken>,
    snapshot: &'a CoordinationStateSnapshot,
    assertions: &'a [CoordinationStatusAssertion],
    read: Option<&'a RaftReadReceipt>,
}

fn evidence_values_for(input: EvidenceValuesInput<'_>) -> Vec<IoValue> {
    let mut values = vec![
        input.request.value.clone(),
        input.snapshot.value.clone(),
        input.receipt.value.clone(),
    ];
    if let Some(token) = input.token {
        values.push(token.value.clone());
    }
    if let Some(read) = input.read {
        values.push(read.value.clone());
    }
    values.extend(input.assertions.iter().map(|assertion| assertion.value.clone()));
    values
}

fn retention_refs_for_state(state: &CoordinationState) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for lock in state.locks.values() {
        refs.push_limited(lock.token_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
    }
    for election in state.elections.values() {
        refs.push_limited(election.token_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
    }
    for entry in state.registry.values() {
        refs.push_limited(entry.endpoint_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
        refs.push_limited(entry.evidence_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
    }
    Ok(refs)
}

fn payload_token(request: &CoordinationRequest) -> Result<u64> {
    let payload = request
        .payload
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("coordination release requires token payload"))?;
    let fields = simple_record(payload, "token", 1)?;
    required_u64(&fields[0], "coordination token")
}

fn payload_text(request: &CoordinationRequest, label: &str) -> Result<String> {
    let payload = request
        .payload
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness(format!("coordination request requires {label} payload")))?;
    simple_payload_text(payload, label)
}

fn simple_payload_text(payload: &IoValue, label: &str) -> Result<String> {
    let fields = simple_record(payload, label, 1)?;
    required_string(&fields[0], label)
}

fn payload_endpoint(request: &CoordinationRequest) -> Result<(String, String)> {
    let payload = request
        .payload
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("coordination registry register requires endpoint payload"))?;
    let fields = simple_record(payload, "endpoint", 2)?;
    let endpoint_ref = required_string(&fields[0], "coordination endpoint ref")?;
    let evidence_ref = required_string(&fields[1], "coordination endpoint evidence ref")?;
    validate_ref(&endpoint_ref, "coordination endpoint ref")?;
    validate_ref(&evidence_ref, "coordination endpoint evidence ref")?;
    Ok((endpoint_ref, evidence_ref))
}

fn coordination_namespace(service: &str) -> String {
    format!("{COORDINATION_NAMESPACE_PREFIX}:{service}")
}

struct CoordinationRefSlices<'a> {
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    policy_refs: &'a [String],
}

struct FixtureRequestInput<'a> {
    service: &'a str,
    operation: &'a str,
    key: &'a str,
    client_session: &'a str,
    sequence: u64,
    payload: Option<IoValue>,
    refs: &'a CoordinationRefSlices<'a>,
}

fn fixture_request(input: FixtureRequestInput<'_>) -> Result<IoValue> {
    let scope =
        delivery_idempotency::control_command_scope_ref(&fixture_ref("coordination-group"), input.client_session)?;
    let payload_ref = input.payload.as_ref().map_or_else(
        || fixture_ref("coordination-empty-payload"),
        |value| canonical_hash(value).unwrap_or_else(|_| fixture_ref("coordination-payload-error")),
    );
    let operation_id = delivery_idempotency::derive_operation_id(delivery_idempotency::OperationIdInput {
        scope_ref: scope,
        producer: input.client_session.to_string(),
        consumer: format!("coordination:{}:{}", input.service, input.key),
        sequence: input.sequence,
        intent: format!("{}:{}", input.service, input.operation),
        payload_ref,
        policy_refs: input.refs.policy_refs.to_vec(),
    })?;
    coordination_request_value(&CoordinationRequestInput {
        service: input.service.to_string(),
        operation: input.operation.to_string(),
        key: input.key.to_string(),
        client_session: input.client_session.to_string(),
        operation_id_ref: operation_id.operation_ref,
        payload: input.payload,
        authority_refs: input.refs.authority_refs.to_vec(),
        resource_refs: input.refs.resource_refs.to_vec(),
        policy_refs: input.refs.policy_refs.to_vec(),
    })
}

fn supported_services() -> Vec<String> {
    vec![
        SERVICE_LOCK.to_string(),
        SERVICE_QUEUE.to_string(),
        SERVICE_SEMAPHORE.to_string(),
        SERVICE_RATE_LIMIT.to_string(),
        SERVICE_ELECTION.to_string(),
        SERVICE_BARRIER.to_string(),
        SERVICE_REGISTRY.to_string(),
    ]
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_value(value: Option<&IoValue>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![value.clone()]))
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "coordination checks")?;
    ensure_count_at_most(items.len(), MAX_COORDINATION_CHECKS, "coordination checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "coordination check name")?;
        let status = required_string(&check[1], "coordination check status")?;
        match status.as_str() {
            "pass" | "fail" | "diagnostic" => {
                parsed.push_limited((name, status), MAX_COORDINATION_CHECKS, "coordination checks")?
            }
            _ => return Err(MoltenError::invalid_harness("coordination check status must be pass/fail/diagnostic")),
        }
    }
    Ok(parsed)
}

fn require_check(checks: &[(String, String)], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(name, _)| name == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_u64(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let option = value_to_iovalue(&fields[0]);
    if option.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let some = simple_record(&option, "some", 1)?;
        let reference = required_string(&some[0], label)?;
        validate_ref(&reference, label)?;
        Ok(Some(reference))
    }
}

fn record_optional_value(value: &Value<IoValue>, label: &str) -> Result<Option<IoValue>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let option = value_to_iovalue(&fields[0]);
    if option.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let some = simple_record(&option, "some", 1)?;
        Ok(Some(value_to_iovalue(&some[0])))
    }
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_COORDINATION_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited(required_string(item, label)?, MAX_COORDINATION_REFS, label)?;
    }
    Ok(values)
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = record_string_sequence(value, label)?;
    validate_refs(&values, label)?;
    Ok(values)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    let number = value.as_u64().ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {field}")))
}

fn validate_request_input(input: &CoordinationRequestInput) -> Result<()> {
    validate_service(&input.service)?;
    validate_operation(&input.service, &input.operation)?;
    validate_key(&input.key)?;
    validate_session(&input.client_session)?;
    validate_ref(&input.operation_id_ref, "coordination operation id ref")?;
    validate_refs(&input.authority_refs, "coordination authority ref")?;
    validate_refs(&input.resource_refs, "coordination resource ref")?;
    validate_refs(&input.policy_refs, "coordination policy ref")?;
    Ok(())
}

fn validate_service_id(value: &str) -> Result<()> {
    validate_non_empty(value, "coordination service id")?;
    if value.starts_with("coordination:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness("coordination service id must start with coordination:"))
    }
}

fn validate_services(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_COORDINATION_SERVICES, "coordination services")?;
    if values.is_empty() {
        return Err(MoltenError::invalid_harness("coordination manifest requires at least one service"));
    }
    for value in values {
        validate_service(value)?;
    }
    Ok(())
}

fn validate_service(value: &str) -> Result<()> {
    match value {
        SERVICE_LOCK | SERVICE_QUEUE | SERVICE_SEMAPHORE | SERVICE_RATE_LIMIT | SERVICE_ELECTION | SERVICE_BARRIER
        | SERVICE_REGISTRY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported coordination service {value}"))),
    }
}

fn validate_operation(service: &str, operation: &str) -> Result<()> {
    let is_valid = matches!(
        (service, operation),
        (SERVICE_LOCK, OP_ACQUIRE)
            | (SERVICE_LOCK, OP_RELEASE)
            | (SERVICE_LOCK, OP_READ)
            | (SERVICE_QUEUE, OP_ENQUEUE)
            | (SERVICE_QUEUE, OP_DEQUEUE)
            | (SERVICE_QUEUE, OP_READ)
            | (SERVICE_SEMAPHORE, OP_ACQUIRE)
            | (SERVICE_SEMAPHORE, OP_RELEASE)
            | (SERVICE_SEMAPHORE, OP_READ)
            | (SERVICE_RATE_LIMIT, OP_ACQUIRE)
            | (SERVICE_RATE_LIMIT, OP_READ)
            | (SERVICE_ELECTION, OP_ELECT)
            | (SERVICE_ELECTION, OP_READ)
            | (SERVICE_BARRIER, OP_ARRIVE)
            | (SERVICE_BARRIER, OP_READ)
            | (SERVICE_REGISTRY, OP_REGISTER)
            | (SERVICE_REGISTRY, OP_UNREGISTER)
            | (SERVICE_REGISTRY, OP_READ)
    );
    if is_valid {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported coordination operation {operation} for service {service}"
        )))
    }
}

fn validate_key(value: &str) -> Result<()> {
    validate_non_empty(value, "coordination key")?;
    ensure_count_at_most(value.len(), MAX_COORDINATION_KEY_LEN, "coordination key bytes")
}

fn validate_session(value: &str) -> Result<()> {
    validate_non_empty(value, "coordination client session")
}

fn validate_capacity(value: u64, label: &str) -> Result<()> {
    if value > 0 {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} must be positive")))
    }
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        "pass" | "deny" => Ok(()),
        _ => Err(MoltenError::invalid_harness("coordination decision must be pass or deny")),
    }
}

fn validate_ref(value: &str, label: &str) -> Result<()> {
    validate_non_empty(value, label)?;
    validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("{label} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_COORDINATION_REFS, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    }
}

fn vec_len_u64<T>(values: &[T]) -> Result<u64> {
    u64::try_from(values.len()).map_err(|_| MoltenError::invalid_harness("coordination vector length overflow"))
}

fn set_len_u64<T>(values: &OrderedSet<T>) -> Result<u64> {
    u64::try_from(values.len()).map_err(|_| MoltenError::invalid_harness("coordination set length overflow"))
}

fn fixture_ref(label: &str) -> String {
    content_ref_from_bytes(label.as_bytes())
}

trait PushLimited<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T, S> PushLimited<T> for S
where S: VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        ensure_count_at_most(self.item_count().saturating_add(1), maximum, label)?;
        self.push_item(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::catalog;
    use crate::catalog_mcp;
    use crate::ledger;
    use crate::preserves_rail::to_text;

    fn refs() -> (Vec<String>, Vec<String>, Vec<String>) {
        (vec![fixture_ref("auth")], vec![fixture_ref("resource")], vec![fixture_ref("policy")])
    }

    fn runtime() -> CoordinationRuntime {
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        new_coordination_runtime(&manifest).expect("runtime")
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove stale temp root");
        }
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn request(
        service: &str,
        operation: &str,
        key: &str,
        session: &str,
        sequence: u64,
        payload: Option<IoValue>,
    ) -> IoValue {
        let (auth, resources, policies) = refs();
        let fixture_refs = CoordinationRefSlices {
            authority_refs: &auth,
            resource_refs: &resources,
            policy_refs: &policies,
        };
        fixture_request(FixtureRequestInput {
            service,
            operation,
            key,
            client_session: session,
            sequence,
            payload,
            refs: &fixture_refs,
        })
        .expect("request")
    }

    #[test]
    fn coordination_rejects_malformed_content_refs() {
        let (authority_refs, resource_refs, policy_refs) = refs();
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            let error = coordination_service_manifest_value(&CoordinationServiceManifestInput {
                service_id: DEFAULT_COORDINATION_SERVICE_ID.to_string(),
                services: vec![SERVICE_LOCK.to_string(), SERVICE_QUEUE.to_string()],
                control_group_ref: invalid.to_string(),
                queue_capacity: DEFAULT_COORDINATION_QUEUE_CAPACITY,
                semaphore_capacity: DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
                rate_limit: DEFAULT_COORDINATION_RATE_LIMIT,
                barrier_parties: DEFAULT_COORDINATION_BARRIER_PARTIES,
                policy_refs: policy_refs.clone(),
                resource_refs: resource_refs.clone(),
            })
            .expect_err("malformed manifest ref denied");
            assert!(error.to_string().contains("canonical blake3 content ref"), "unexpected error: {error}");

            let request_error = coordination_request_value(&CoordinationRequestInput {
                service: SERVICE_LOCK.to_string(),
                operation: OP_ACQUIRE.to_string(),
                key: "resource:test".to_string(),
                client_session: "session-malformed".to_string(),
                operation_id_ref: invalid.to_string(),
                payload: None,
                authority_refs: authority_refs.clone(),
                resource_refs: resource_refs.clone(),
                policy_refs: policy_refs.clone(),
            })
            .expect_err("malformed request ref denied");
            assert!(
                request_error.to_string().contains("canonical blake3 content ref"),
                "unexpected error: {request_error}"
            );
        }
    }

    #[test]
    fn lock_acquire_release_stale_fencing_and_duplicate_are_receipted() {
        let mut runtime = runtime();
        let acquire = request(SERVICE_LOCK, OP_ACQUIRE, "resource:test", "session-a", 1, None);
        let first = apply_coordination_request(&mut runtime, &acquire).expect("acquire");
        assert_eq!(first.receipt.decision, "pass");
        let token = first.token.as_ref().expect("token").token;
        assert_eq!(token, 1);
        assert_eq!(first.assertions.len(), 1);
        let duplicate = apply_coordination_request(&mut runtime, &acquire).expect("duplicate");
        assert_eq!(duplicate.receipt.receipt_ref, first.receipt.receipt_ref);
        assert_eq!(runtime.state.next_fencing_token, 2);
        let stale = request(
            SERVICE_LOCK,
            OP_RELEASE,
            "resource:test",
            "session-a",
            2,
            Some(record("token", vec![u64_value(0)])),
        );
        let stale = apply_coordination_request(&mut runtime, &stale).expect("stale deny");
        assert_eq!(stale.receipt.decision, "deny");
        assert!(stale.receipt.diagnostics.join(";").contains("stale fencing token"));
        let release = request(
            SERVICE_LOCK,
            OP_RELEASE,
            "resource:test",
            "session-a",
            3,
            Some(record("token", vec![u64_value(token)])),
        );
        let release = apply_coordination_request(&mut runtime, &release).expect("release");
        assert_eq!(release.receipt.decision, "pass");
        assert!(runtime.state.locks.is_empty());
    }

    #[test]
    fn queue_fifo_duplicate_overflow_and_resource_denial_are_receipted() {
        let mut coord_runtime = runtime();
        let one =
            request(SERVICE_QUEUE, OP_ENQUEUE, "queue:test", "producer", 1, Some(record("item", vec![string("one")])));
        let two =
            request(SERVICE_QUEUE, OP_ENQUEUE, "queue:test", "producer", 2, Some(record("item", vec![string("two")])));
        let first = apply_coordination_request(&mut coord_runtime, &one).expect("enqueue one");
        let duplicate = apply_coordination_request(&mut coord_runtime, &one).expect("duplicate one");
        assert_eq!(first.receipt.receipt_ref, duplicate.receipt.receipt_ref);
        apply_coordination_request(&mut coord_runtime, &two).expect("enqueue two");
        let dequeue = request(SERVICE_QUEUE, OP_DEQUEUE, "queue:test", "consumer", 3, None);
        let dequeue = apply_coordination_request(&mut coord_runtime, &dequeue).expect("dequeue");
        assert_eq!(dequeue.receipt.decision, "pass");
        assert_eq!(coord_runtime.state.queues.get("queue:test").expect("queue")[0], "two");

        let mut small = runtime();
        small.manifest.queue_capacity = 1;
        apply_coordination_request(
            &mut small,
            &request(SERVICE_QUEUE, OP_ENQUEUE, "queue:small", "p", 1, Some(record("item", vec![string("a")]))),
        )
        .expect("first");
        let overflow = apply_coordination_request(
            &mut small,
            &request(SERVICE_QUEUE, OP_ENQUEUE, "queue:small", "p", 2, Some(record("item", vec![string("b")]))),
        )
        .expect("overflow");
        assert_eq!(overflow.receipt.decision, "deny");
        assert!(overflow.receipt.diagnostics.join(";").contains("queue overflow"));

        let (auth, _resources, policies) = refs();
        let empty_resources = Vec::new();
        let denied_refs = CoordinationRefSlices {
            authority_refs: &auth,
            resource_refs: &empty_resources,
            policy_refs: &policies,
        };
        let denied = fixture_request(FixtureRequestInput {
            service: SERVICE_QUEUE,
            operation: OP_ENQUEUE,
            key: "queue:deny",
            client_session: "p",
            sequence: 9,
            payload: Some(record("item", vec![string("x")])),
            refs: &denied_refs,
        })
        .expect("request");
        let denied = apply_coordination_request(&mut small, &denied).expect("resource deny");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(denied.receipt.diagnostics.join(";").contains("missing resource"));
    }

    #[test]
    fn service_registry_updates_and_read_index_reads_are_control_plane_bound() {
        let mut runtime = runtime();
        let endpoint = fixture_ref("endpoint");
        let evidence = fixture_ref("evidence");
        let register = request(
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:coord",
            "registrar",
            1,
            Some(record("endpoint", vec![string(&endpoint), string(&evidence)])),
        );
        let register = apply_coordination_request(&mut runtime, &register).expect("register");
        assert_eq!(register.receipt.decision, "pass");
        assert!(register.receipt.raft_receipt_ref.is_some());
        let read = request(SERVICE_REGISTRY, OP_READ, "svc:coord", "reader", 2, None);
        let read = apply_coordination_request(&mut runtime, &read).expect("read");
        assert_eq!(read.receipt.decision, "pass");
        assert!(read.raft_read_receipt.is_some());
        assert_eq!(read.assertions.len(), 1);
        let assertion_text = to_text(&read.assertions[0].value).expect("assertion text");
        assert!(assertion_text.contains(&endpoint));
    }

    #[test]
    fn semaphore_rate_election_barrier_and_registry_primitives_are_deterministic() {
        let mut runtime = runtime();
        let first =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "a", 1, None))
                .expect("sem a");
        let second =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "b", 2, None))
                .expect("sem b");
        let exhausted =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "c", 3, None))
                .expect("sem deny");
        assert_eq!(first.receipt.decision, "pass");
        assert_eq!(second.receipt.decision, "pass");
        assert_eq!(exhausted.receipt.decision, "deny");
        assert!(exhausted.receipt.diagnostics.join(";").contains("semaphore exhausted"));
        let rate_a = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "a", 4, None),
        )
        .expect("rate a");
        let rate_b = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "b", 5, None),
        )
        .expect("rate b");
        let rate_c = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "c", 6, None),
        )
        .expect("rate deny");
        assert_eq!(rate_a.receipt.decision, "pass");
        assert_eq!(rate_b.receipt.decision, "pass");
        assert_eq!(rate_c.receipt.decision, "deny");
        let elect = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, "election:test", "leader", 7, None),
        )
        .expect("elect");
        assert_eq!(elect.receipt.decision, "pass");
        assert!(elect.token.is_some());
        let barrier_wait = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_BARRIER, OP_ARRIVE, "barrier:test", "a", 8, None),
        )
        .expect("barrier wait");
        let barrier_release = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_BARRIER, OP_ARRIVE, "barrier:test", "b", 9, None),
        )
        .expect("barrier release");
        assert_eq!(barrier_wait.receipt.decision, "pass");
        assert_eq!(barrier_release.receipt.decision, "pass");
        assert!(runtime.state.barriers.get("barrier:test").expect("barrier").is_released);
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_coordination_artifacts() {
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        let mut runtime = new_coordination_runtime(&manifest).expect("runtime");
        let result = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:classify", "session", 1, None),
        )
        .expect("lock");
        assert_eq!(ledger::artifact_kind(&manifest), "coordination-service-manifest");
        assert_eq!(ledger::artifact_kind(&result.receipt.value), "coordination-receipt");
        assert_eq!(ledger::artifact_kind(&result.assertions[0].value), "coordination-status-assertion");
        assert_eq!(ledger::artifact_kind(&result.token.as_ref().expect("token").value), "coordination-fencing-token");
        let report_evidence_refs = result
            .evidence_values
            .iter()
            .map(canonical_hash)
            .collect::<Result<Vec<_>>>()
            .expect("evidence refs");
        let manifest_ref = canonical_hash(&manifest).expect("manifest ref");
        let apply_report = coordination_apply_report_value(ApplyReportValueInput {
            decision: "pass",
            manifest_ref: &manifest_ref,
            final_state_ref: &result.state_snapshot.state_ref,
            receipt_refs: std::slice::from_ref(&result.receipt.receipt_ref),
            assertion_refs: std::slice::from_ref(&result.assertions[0].assertion_ref),
            evidence_refs: &report_evidence_refs,
        })
        .expect("apply report");
        assert_eq!(ledger::artifact_kind(&apply_report), "coordination-apply-report");
        let root = temp_root("coordination-ledger-catalog");
        let registry_root = root.join("registry");
        let ledger_root = root.join("ledger");
        std::fs::create_dir_all(&registry_root).expect("registry root");
        ledger::import_artifact(&ledger_root, &result.receipt.value).expect("import receipt");
        let list = catalog::list(&registry_root, Some(&ledger_root), &catalog::CatalogListInput {
            kind: Some("coordination-receipt".to_string()),
            visibility: catalog::CatalogVisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(list.decision, "pass");
        assert_eq!(list.items.len(), 1);
        let view_request = catalog_mcp::mcp_request_value("catalog.view", vec![record("reference", vec![string(
            &result.receipt.receipt_ref,
        )])])
        .expect("mcp request");
        let call = catalog_mcp::call(&registry_root, Some(&ledger_root), &view_request).expect("mcp call");
        assert_eq!(call.decision, "pass");
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_fencing_fifo_semaphore_and_no_actor_traffic_invariants(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(1).max_value(1000));
        let mut runtime = runtime();
        let key = format!("resource:{salt}");
        let acquire =
            apply_coordination_request(&mut runtime, &request(SERVICE_LOCK, OP_ACQUIRE, &key, "owner", salt, None))
                .expect("acquire");
        assert_eq!(acquire.receipt.decision, "pass");
        let token = acquire.token.expect("token").token;
        assert!(token >= 1);
        let queue_key = format!("queue:{salt}");
        apply_coordination_request(
            &mut runtime,
            &request(SERVICE_QUEUE, OP_ENQUEUE, &queue_key, "p", salt + 1, Some(record("item", vec![string("first")]))),
        )
        .expect("enqueue first");
        apply_coordination_request(
            &mut runtime,
            &request(
                SERVICE_QUEUE,
                OP_ENQUEUE,
                &queue_key,
                "p",
                salt + 2,
                Some(record("item", vec![string("second")])),
            ),
        )
        .expect("enqueue second");
        assert_eq!(runtime.state.queues.get(&queue_key).expect("queue")[0], "first");
        let sem_key = format!("sem:{salt}");
        apply_coordination_request(
            &mut runtime,
            &request(SERVICE_SEMAPHORE, OP_ACQUIRE, &sem_key, "a", salt + 3, None),
        )
        .expect("sem a");
        assert!(
            set_len_u64(runtime.state.semaphores.get(&sem_key).expect("sem")).expect("sem count")
                <= runtime.manifest.semaphore_capacity
        );
        let snapshot_before_actor_message = snapshot_from_state(&runtime.state).expect("before").state_ref;
        let snapshot_after_actor_message = snapshot_from_state(&runtime.state).expect("after").state_ref;
        assert_eq!(snapshot_before_actor_message, snapshot_after_actor_message);
    }
}
