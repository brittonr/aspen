use redb::ReadableDatabase;

type IoValue = preserves::IOValue;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value.as_ref())
}

pub const SCOPE_ACTOR_TURN: &str = "actor-turn";
pub const SCOPE_SERVICE_LIFECYCLE: &str = "service-lifecycle";
pub const SCOPE_PROTOCOL_SESSION: &str = "protocol-session";
pub const SCOPE_REMOTE_TOPIC: &str = "remote-dataspace-topic";
pub const SCOPE_JOB_WORKER: &str = "job-worker";
pub const SCOPE_CONTROL_COMMAND: &str = "control-plane-command";

const STORE_FILE: &str = "delivery-idempotency.redb";
const STORE_WINDOWS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("delivery_windows_v1");
const STORE_ENTRIES: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("delivery_dedup_entries_v1");
const STORE_RECEIPTS: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("delivery_idempotency_receipts_v1");
const STORE_PINS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("delivery_retention_pins_v1");

const MAX_REFS: usize = 4096;
const MAX_DIAGNOSTICS: usize = 128;
const MAX_SCOPE_NAME_LEN: usize = 256;
const _: () = assert!(MAX_REFS <= 100_000);
const _: () = assert!(MAX_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_SCOPE_NAME_LEN <= 4096);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapPolicy {
    Deny,
    Retry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationIdInput {
    pub scope_ref: String,
    pub producer: String,
    pub consumer: String,
    pub sequence: u64,
    pub intent: String,
    pub payload_ref: String,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationId {
    pub operation_ref: String,
    pub scope_ref: String,
    pub producer: String,
    pub consumer: String,
    pub sequence: u64,
    pub intent: String,
    pub payload_ref: String,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub window_ref: String,
    pub scope_ref: String,
    pub scope_profile: String,
    pub next_sequence: u64,
    pub lowest_retained: u64,
    pub retention_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedupEntry {
    pub entry_ref: String,
    pub dedup_key: String,
    pub operation_ref: String,
    pub scope_ref: String,
    pub producer: String,
    pub consumer: String,
    pub sequence: u64,
    pub intent: String,
    pub payload_ref: String,
    pub semantic_result_ref: Option<String>,
    pub first_receipt_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub decision: String,
    pub operation_ref: String,
    pub scope_ref: String,
    pub window_ref: String,
    pub prior_receipt_ref: Option<String>,
    pub semantic_result_ref: Option<String>,
    pub side_effect: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub operation: OperationId,
    pub window: Window,
    pub receipt: Receipt,
    pub entry: Option<DedupEntry>,
    pub should_commit_side_effect: bool,
    pub prior_semantic_result_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckInput<'a> {
    pub root: &'a std::path::Path,
    pub scope_profile: &'a str,
    pub scope_ref: &'a str,
    pub producer: &'a str,
    pub consumer: &'a str,
    pub sequence: u64,
    pub intent: &'a str,
    pub payload_ref: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub semantic_result_ref: Option<&'a str>,
    pub gap_policy: GapPolicy,
}

pub fn scope_profile_value(profile: &str, scope_name: &str, retention_refs: &[String]) -> Result<IoValue> {
    validate_scope_profile(profile)?;
    validate_name(scope_name, "delivery scope name")?;
    validate_refs(retention_refs, "delivery scope retention ref")?;
    Ok(record("delivery-scope-profile-v1", vec![
        string(crate::preserves_rail::DELIVERY_SCOPE_PROFILE_SCHEMA),
        record("profile", vec![string(profile)]),
        record("scope-name", vec![string(scope_name)]),
        record("retention", vec![strings_sequence(retention_refs)]),
        checks_value(&[("scoped-not-global", "pass"), ("retention-policy-declared", "pass")]),
    ]))
}

pub fn scope_ref(profile: &str, scope_name: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&scope_profile_value(profile, scope_name, &[])?)
}

pub fn remote_topic_scope_ref(topic: &str, consumer_peer: &str) -> Result<String> {
    scope_ref(SCOPE_REMOTE_TOPIC, &format!("{consumer_peer}:{topic}"))
}

pub fn protocol_session_scope_ref(protocol_ref: &str, session_id: &str) -> Result<String> {
    scope_ref(SCOPE_PROTOCOL_SESSION, &format!("{protocol_ref}:{session_id}"))
}

pub fn job_worker_scope_ref(job_ref: &str, target_peer: &str) -> Result<String> {
    scope_ref(SCOPE_JOB_WORKER, &format!("{target_peer}:{job_ref}"))
}

pub fn service_lifecycle_scope_ref(service_id: &str) -> Result<String> {
    scope_ref(SCOPE_SERVICE_LIFECYCLE, service_id)
}

pub fn control_command_scope_ref(group_ref: &str, client_session: &str) -> Result<String> {
    scope_ref(SCOPE_CONTROL_COMMAND, &format!("{group_ref}:{client_session}"))
}

pub fn operation_id_value(input: &OperationIdInput) -> Result<IoValue> {
    validate_operation_input(input)?;
    Ok(record("operation-id-v1", vec![
        string(crate::preserves_rail::DELIVERY_OPERATION_ID_SCHEMA),
        record("scope", vec![string(&input.scope_ref)]),
        record("producer", vec![string(&input.producer)]),
        record("consumer", vec![string(&input.consumer)]),
        record("sequence", vec![crate::preserves_rail::u64_value(input.sequence)]),
        record("intent", vec![string(&input.intent)]),
        record("payload", vec![string(&input.payload_ref)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        checks_value(&[
            ("canonical-operation-ref", "pass"),
            ("scoped-sequence", "pass"),
            ("no-wall-clock-or-path-identity", "pass"),
        ]),
    ]))
}

pub fn derive_operation_id(input: OperationIdInput) -> Result<OperationId> {
    let value = operation_id_value(&input)?;
    parse_operation_id(&value)
}

pub fn parse_operation_id(value: &IoValue) -> Result<OperationId> {
    let fields = value
        .collect_simple_record("operation-id-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operation-id-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DELIVERY_OPERATION_ID_SCHEMA, "delivery operation id schema")?;
    let input = OperationIdInput {
        scope_ref: record_ref(&fields[1], "scope")?,
        producer: record_string(&fields[2], "producer")?,
        consumer: record_string(&fields[3], "consumer")?,
        sequence: record_u64(&fields[4], "sequence")?,
        intent: record_string(&fields[5], "intent")?,
        payload_ref: record_ref(&fields[6], "payload")?,
        policy_refs: record_ref_sequence(&fields[7], "policy")?,
    };
    validate_operation_input(&input)?;
    require_check(&parse_checks(&fields[8])?, "canonical-operation-ref", "delivery operation id")?;
    Ok(OperationId {
        operation_ref: crate::preserves_rail::canonical_hash(value)?,
        scope_ref: input.scope_ref,
        producer: input.producer,
        consumer: input.consumer,
        sequence: input.sequence,
        intent: input.intent,
        payload_ref: input.payload_ref,
        policy_refs: input.policy_refs,
        value: value.clone(),
    })
}

pub fn window_value(
    scope_profile: &str,
    scope_ref: &str,
    next_sequence: u64,
    lowest_retained: u64,
    retention_refs: &[String],
) -> Result<IoValue> {
    validate_scope_profile(scope_profile)?;
    require_ref(scope_ref, "delivery window scope ref")?;
    validate_refs(retention_refs, "delivery retention ref")?;
    if lowest_retained == 0 || next_sequence == 0 || lowest_retained > next_sequence {
        return Err(MoltenError::invalid_harness("invalid delivery window sequence bounds"));
    }
    Ok(record("delivery-window-v1", vec![
        string(crate::preserves_rail::DELIVERY_WINDOW_SCHEMA),
        record("scope", vec![string(scope_ref)]),
        record("profile", vec![string(scope_profile)]),
        record("next-sequence", vec![crate::preserves_rail::u64_value(next_sequence)]),
        record("lowest-retained", vec![crate::preserves_rail::u64_value(lowest_retained)]),
        record("retention", vec![strings_sequence(retention_refs)]),
        checks_value(&[("dedup-window-scoped", "pass"), ("retention-pinned", "pass")]),
    ]))
}

pub fn parse_window(value: &IoValue) -> Result<Window> {
    let fields = value
        .collect_simple_record("delivery-window-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <delivery-window-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DELIVERY_WINDOW_SCHEMA, "delivery window schema")?;
    let scope_ref = record_ref(&fields[1], "scope")?;
    let scope_profile = record_string(&fields[2], "profile")?;
    let next_sequence = record_u64(&fields[3], "next-sequence")?;
    let lowest_retained = record_u64(&fields[4], "lowest-retained")?;
    let retention_refs = record_ref_sequence(&fields[5], "retention")?;
    validate_scope_profile(&scope_profile)?;
    if lowest_retained == 0 || next_sequence == 0 || lowest_retained > next_sequence {
        return Err(MoltenError::invalid_harness("invalid parsed delivery window sequence bounds"));
    }
    require_check(&parse_checks(&fields[6])?, "dedup-window-scoped", "delivery window")?;
    Ok(Window {
        window_ref: crate::preserves_rail::canonical_hash(value)?,
        scope_ref,
        scope_profile,
        next_sequence,
        lowest_retained,
        retention_refs,
        value: value.clone(),
    })
}
