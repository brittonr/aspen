use preserves::IOValue;
use preserves::Value;
use redb::ReadableDatabase;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;

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

const MAX_DELIVERY_REFS: usize = 4096;
const MAX_DELIVERY_DIAGNOSTICS: usize = 128;
const MAX_SCOPE_NAME_LEN: usize = 256;
const _: () = assert!(MAX_DELIVERY_REFS <= 100_000);
const _: () = assert!(MAX_DELIVERY_DIAGNOSTICS <= 10_000);
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryWindow {
    pub window_ref: String,
    pub scope_ref: String,
    pub scope_profile: String,
    pub next_sequence: u64,
    pub lowest_retained: u64,
    pub retention_refs: Vec<String>,
    pub value: IOValue,
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub operation_ref: String,
    pub scope_ref: String,
    pub window_ref: String,
    pub prior_receipt_ref: Option<String>,
    pub semantic_result_ref: Option<String>,
    pub side_effect: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryDecision {
    pub operation: OperationId,
    pub window: DeliveryWindow,
    pub receipt: IdempotencyReceipt,
    pub entry: Option<DedupEntry>,
    pub should_commit_side_effect: bool,
    pub prior_semantic_result_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryCheckInput<'a> {
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

pub fn scope_profile_value(profile: &str, scope_name: &str, retention_refs: &[String]) -> Result<IOValue> {
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
    canonical_hash(&scope_profile_value(profile, scope_name, &[])?)
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

pub fn operation_id_value(input: &OperationIdInput) -> Result<IOValue> {
    validate_operation_input(input)?;
    Ok(record("operation-id-v1", vec![
        string(crate::preserves_rail::DELIVERY_OPERATION_ID_SCHEMA),
        record("scope", vec![string(&input.scope_ref)]),
        record("producer", vec![string(&input.producer)]),
        record("consumer", vec![string(&input.consumer)]),
        record("sequence", vec![u64_value(input.sequence)]),
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

pub fn parse_operation_id(value: &IOValue) -> Result<OperationId> {
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
        operation_ref: canonical_hash(value)?,
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

pub fn delivery_window_value(
    scope_profile: &str,
    scope_ref: &str,
    next_sequence: u64,
    lowest_retained: u64,
    retention_refs: &[String],
) -> Result<IOValue> {
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
        record("next-sequence", vec![u64_value(next_sequence)]),
        record("lowest-retained", vec![u64_value(lowest_retained)]),
        record("retention", vec![strings_sequence(retention_refs)]),
        checks_value(&[("dedup-window-scoped", "pass"), ("retention-pinned", "pass")]),
    ]))
}

pub fn parse_delivery_window(value: &IOValue) -> Result<DeliveryWindow> {
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
    Ok(DeliveryWindow {
        window_ref: canonical_hash(value)?,
        scope_ref,
        scope_profile,
        next_sequence,
        lowest_retained,
        retention_refs,
        value: value.clone(),
    })
}

pub fn check_delivery(input: DeliveryCheckInput<'_>) -> Result<DeliveryDecision> {
    validate_scope_profile(input.scope_profile)?;
    require_ref(input.scope_ref, "delivery scope ref")?;
    validate_refs(input.policy_refs, "delivery policy ref")?;
    validate_refs(input.evidence_refs, "delivery evidence ref")?;
    if let Some(result_ref) = input.semantic_result_ref {
        require_ref(result_ref, "delivery semantic result ref")?;
    }
    let operation = derive_operation_id(OperationIdInput {
        scope_ref: input.scope_ref.to_owned(),
        producer: input.producer.to_owned(),
        consumer: input.consumer.to_owned(),
        sequence: input.sequence,
        intent: input.intent.to_owned(),
        payload_ref: input.payload_ref.to_owned(),
        policy_refs: input.policy_refs.to_vec(),
    })?;
    let dedup_key = dedup_key_ref(&operation)?;
    let db = ensure_store_tables(input.root)?;
    let existing_entry = read_entry_from_store(&db, &dedup_key)?;
    let current_window = read_or_create_window(&db, input.scope_profile, input.scope_ref, input.policy_refs)?;
    let decision = if let Some(entry) = existing_entry {
        duplicate_or_conflict_decision(input, &db, operation, current_window, entry)?
    } else if input.sequence < current_window.next_sequence {
        stale_decision(input, &db, operation, current_window)?
    } else if input.sequence > current_window.next_sequence {
        gap_or_retry_decision(input, &db, operation, current_window)?
    } else {
        first_decision(input, &db, operation, current_window, dedup_key)?
    };
    Ok(decision)
}

pub fn read_idempotency_receipt(root: &std::path::Path, receipt_ref: &str) -> Result<IOValue> {
    require_ref(receipt_ref, "delivery idempotency receipt ref")?;
    let db = ensure_store_tables(root)?;
    let read_txn = db.begin_read().map_err(store_error)?;
    let table = read_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(store_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown delivery idempotency receipt {receipt_ref}")));
    };
    parse_canonical_bytes(bytes.value())
}

pub fn retry_receipt_value(
    operation: &OperationId,
    window: &DeliveryWindow,
    diagnostics: &[String],
) -> Result<IOValue> {
    validate_diagnostics(diagnostics)?;
    Ok(record("retry-receipt-v1", vec![
        string(crate::preserves_rail::DELIVERY_RETRY_RECEIPT_SCHEMA),
        record("operation", vec![string(&operation.operation_ref)]),
        record("scope", vec![string(&operation.scope_ref)]),
        record("window", vec![string(&window.window_ref)]),
        record("retry-after-sequence", vec![u64_value(window.next_sequence)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[("retry-before-side-effects", "pass"), ("sequence-window-bound", "pass")]),
    ]))
}

pub fn parse_idempotency_receipt(value: &IOValue) -> Result<IdempotencyReceipt> {
    let fields = value
        .collect_simple_record("delivery-idempotency-receipt-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <delivery-idempotency-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA,
        "delivery idempotency receipt schema",
    )?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let side_effect = record_string(&fields[7], "side-effect")?;
    validate_side_effect(&side_effect)?;
    require_check(&parse_checks(&fields[9])?, "dedup-before-commit", "delivery idempotency receipt")?;
    Ok(IdempotencyReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        operation_ref: record_ref(&fields[2], "operation")?,
        scope_ref: record_ref(&fields[3], "scope")?,
        window_ref: record_ref(&fields[4], "window")?,
        prior_receipt_ref: record_optional_ref(&fields[5], "prior")?,
        semantic_result_ref: record_optional_ref(&fields[6], "semantic-result")?,
        side_effect,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn parse_dedup_entry(value: &IOValue) -> Result<DedupEntry> {
    let fields = value
        .collect_simple_record("dedup-entry-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <dedup-entry-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DELIVERY_DEDUP_ENTRY_SCHEMA, "delivery dedup entry schema")?;
    require_check(&parse_checks(&fields[12])?, "first-receipt-bound", "delivery dedup entry")?;
    Ok(DedupEntry {
        entry_ref: canonical_hash(value)?,
        dedup_key: record_ref(&fields[1], "dedup-key")?,
        operation_ref: record_ref(&fields[2], "operation")?,
        scope_ref: record_ref(&fields[3], "scope")?,
        producer: record_string(&fields[4], "producer")?,
        consumer: record_string(&fields[5], "consumer")?,
        sequence: record_u64(&fields[6], "sequence")?,
        intent: record_string(&fields[7], "intent")?,
        payload_ref: record_ref(&fields[8], "payload")?,
        semantic_result_ref: record_optional_ref(&fields[9], "semantic-result")?,
        first_receipt_ref: record_ref(&fields[10], "first-receipt")?,
        evidence_refs: record_ref_sequence(&fields[11], "evidence")?,
        value: value.clone(),
    })
}

pub fn delivery_summary(value: &IOValue) -> Result<String> {
    if let Ok(operation) = parse_operation_id(value) {
        return Ok(format!(
            "delivery operation ref={} scope={} producer={} consumer={} sequence={} intent={} payload={}",
            operation.operation_ref,
            operation.scope_ref,
            operation.producer,
            operation.consumer,
            operation.sequence,
            operation.intent,
            operation.payload_ref
        ));
    }
    if let Ok(window) = parse_delivery_window(value) {
        return Ok(format!(
            "delivery window ref={} scope={} profile={} next_sequence={} lowest_retained={} retention_refs={}",
            window.window_ref,
            window.scope_ref,
            window.scope_profile,
            window.next_sequence,
            window.lowest_retained,
            window.retention_refs.len()
        ));
    }
    if let Ok(entry) = parse_dedup_entry(value) {
        return Ok(format!(
            "delivery dedup entry ref={} operation={} scope={} sequence={} first_receipt={} evidence_refs={}",
            entry.entry_ref,
            entry.operation_ref,
            entry.scope_ref,
            entry.sequence,
            entry.first_receipt_ref,
            entry.evidence_refs.len()
        ));
    }
    if let Ok(receipt) = parse_idempotency_receipt(value) {
        return Ok(format!(
            "delivery idempotency receipt ref={} decision={} operation={} scope={} side_effect={} diagnostics={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.operation_ref,
            receipt.scope_ref,
            receipt.side_effect,
            receipt.diagnostics.len()
        ));
    }
    if let Some(fields) = value.collect_simple_record("retry-receipt-v1", Some(7)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::DELIVERY_RETRY_RECEIPT_SCHEMA,
            "delivery retry receipt schema",
        )?;
        require_check(&parse_checks(&fields[6])?, "retry-before-side-effects", "delivery retry receipt")?;
        return Ok(format!(
            "delivery retry receipt ref={} operation={} scope={} retry_after_sequence={} diagnostics={}",
            canonical_hash(value)?,
            record_ref(&fields[1], "operation")?,
            record_ref(&fields[2], "scope")?,
            record_u64(&fields[4], "retry-after-sequence")?,
            record_string_sequence(&fields[5], "diagnostics")?.len()
        ));
    }
    Err(MoltenError::invalid_harness("unsupported delivery artifact"))
}

fn first_decision(
    input: DeliveryCheckInput<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: DeliveryWindow,
    dedup_key: String,
) -> Result<DeliveryDecision> {
    let next_sequence = operation
        .sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("delivery sequence overflow"))?;
    let updated_window = parse_delivery_window(&delivery_window_value(
        input.scope_profile,
        input.scope_ref,
        next_sequence,
        window.lowest_retained,
        input.policy_refs,
    )?)?;
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision: "first",
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &updated_window.window_ref,
        prior_receipt_ref: None,
        semantic_result_ref: input.semantic_result_ref,
        side_effect: "commit",
        diagnostics: &[],
        checks: &[
            ("dedup-before-commit", "pass"),
            ("sequence-window-advanced", "pass"),
            ("retention-pinned", "pass"),
        ],
    })?;
    let receipt = parse_idempotency_receipt(&receipt_value)?;
    let entry_value = dedup_entry_value(DedupEntryValueInput {
        dedup_key: &dedup_key,
        operation: &operation,
        semantic_result_ref: input.semantic_result_ref,
        first_receipt_ref: &receipt.receipt_ref,
        evidence_refs: input.evidence_refs,
    })?;
    let entry = parse_dedup_entry(&entry_value)?;
    store_first_decision(db, &updated_window, &entry, &receipt)?;
    Ok(DeliveryDecision {
        operation,
        window: updated_window,
        receipt,
        entry: Some(entry),
        should_commit_side_effect: true,
        prior_semantic_result_ref: None,
    })
}

fn duplicate_or_conflict_decision(
    input: DeliveryCheckInput<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: DeliveryWindow,
    entry: DedupEntry,
) -> Result<DeliveryDecision> {
    let has_same_operation = entry.operation_ref == operation.operation_ref;
    let has_same_payload = entry.payload_ref == operation.payload_ref;
    let has_same_evidence = entry.evidence_refs == input.evidence_refs;
    let (decision, prior, semantic, diagnostics) = if has_same_operation && has_same_payload && has_same_evidence {
        (
            "duplicate",
            Some(entry.first_receipt_ref.as_str()),
            entry.semantic_result_ref.as_deref(),
            Vec::new(),
        )
    } else {
        ("conflict", Some(entry.first_receipt_ref.as_str()), None, vec![
            "delivery operation sequence reused with different payload or evidence".to_string(),
        ])
    };
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision,
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &window.window_ref,
        prior_receipt_ref: prior,
        semantic_result_ref: semantic,
        side_effect: "suppress",
        diagnostics: &diagnostics,
        checks: &[
            ("dedup-before-commit", "pass"),
            ("duplicate-suppresses-side-effects", if decision == "duplicate" { "pass" } else { "n/a" }),
            ("conflict-denies-before-side-effects", if decision == "conflict" { "pass" } else { "n/a" }),
        ],
    })?;
    let receipt = parse_idempotency_receipt(&receipt_value)?;
    store_receipt(db, &receipt)?;
    Ok(DeliveryDecision {
        operation,
        window,
        receipt,
        entry: Some(entry.clone()),
        should_commit_side_effect: false,
        prior_semantic_result_ref: entry.semantic_result_ref,
    })
}

fn stale_decision(
    input: DeliveryCheckInput<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: DeliveryWindow,
) -> Result<DeliveryDecision> {
    let diagnostics = vec![format!(
        "delivery sequence {} is stale for window next {}",
        input.sequence, window.next_sequence
    )];
    suppressed_decision(db, operation, window, "stale", diagnostics)
}

fn gap_or_retry_decision(
    input: DeliveryCheckInput<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: DeliveryWindow,
) -> Result<DeliveryDecision> {
    let diagnostics = vec![format!(
        "delivery sequence {} leaves gap before expected {}",
        input.sequence, window.next_sequence
    )];
    let decision = match input.gap_policy {
        GapPolicy::Deny => "gap",
        GapPolicy::Retry => "retry",
    };
    let result = suppressed_decision(db, operation, window, decision, diagnostics)?;
    if matches!(input.gap_policy, GapPolicy::Retry) {
        let retry_value = retry_receipt_value(&result.operation, &result.window, &result.receipt.diagnostics)?;
        let retry_ref = canonical_hash(&retry_value)?;
        store_raw_receipt(db, &retry_ref, &retry_value)?;
    }
    Ok(result)
}

fn suppressed_decision(
    db: &redb::Database,
    operation: OperationId,
    window: DeliveryWindow,
    decision: &str,
    diagnostics: Vec<String>,
) -> Result<DeliveryDecision> {
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision,
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &window.window_ref,
        prior_receipt_ref: None,
        semantic_result_ref: None,
        side_effect: "suppress",
        diagnostics: &diagnostics,
        checks: &[
            ("dedup-before-commit", "pass"),
            ("sequence-window-bound", "pass"),
            ("no-side-effects", "pass"),
        ],
    })?;
    let receipt = parse_idempotency_receipt(&receipt_value)?;
    store_receipt(db, &receipt)?;
    Ok(DeliveryDecision {
        operation,
        window,
        receipt,
        entry: None,
        should_commit_side_effect: false,
        prior_semantic_result_ref: None,
    })
}

fn read_or_create_window(
    db: &redb::Database,
    scope_profile: &str,
    scope_ref: &str,
    retention_refs: &[String],
) -> Result<DeliveryWindow> {
    let read_txn = db.begin_read().map_err(store_error)?;
    let windows = read_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
    if let Some(bytes) = windows.get(scope_ref).map_err(store_error)? {
        let value = parse_canonical_bytes(bytes.value())?;
        return parse_delivery_window(&value);
    }
    drop(windows);
    drop(read_txn);
    let value = delivery_window_value(scope_profile, scope_ref, 1, 1, retention_refs)?;
    let window = parse_delivery_window(&value)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let bytes = crate::preserves_rail::canonical_bytes(&window.value)?;
        let mut windows = write_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
        windows.insert(scope_ref, bytes.as_slice()).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)?;
    Ok(window)
}

fn read_entry_from_store(db: &redb::Database, dedup_key: &str) -> Result<Option<DedupEntry>> {
    let read_txn = db.begin_read().map_err(store_error)?;
    let entries = read_txn.open_table(STORE_ENTRIES).map_err(store_error)?;
    let Some(bytes) = entries.get(dedup_key).map_err(store_error)? else {
        return Ok(None);
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_dedup_entry(&value).map(Some)
}

fn store_first_decision(
    db: &redb::Database,
    window: &DeliveryWindow,
    entry: &DedupEntry,
    receipt: &IdempotencyReceipt,
) -> Result<()> {
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let mut windows = write_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
        let window_bytes = crate::preserves_rail::canonical_bytes(&window.value)?;
        windows.insert(window.scope_ref.as_str(), window_bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut entries = write_txn.open_table(STORE_ENTRIES).map_err(store_error)?;
        let entry_bytes = crate::preserves_rail::canonical_bytes(&entry.value)?;
        entries.insert(entry.dedup_key.as_str(), entry_bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut receipts = write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        let receipt_bytes = crate::preserves_rail::canonical_bytes(&receipt.value)?;
        receipts.insert(receipt.receipt_ref.as_str(), receipt_bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut pins = write_txn.open_table(STORE_PINS).map_err(store_error)?;
        pins.insert(entry.operation_ref.as_str(), entry.entry_ref.as_bytes()).map_err(store_error)?;
        pins.insert(window.scope_ref.as_str(), window.window_ref.as_bytes()).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)
}

fn store_receipt(db: &redb::Database, receipt: &IdempotencyReceipt) -> Result<()> {
    store_raw_receipt(db, &receipt.receipt_ref, &receipt.value)
}

fn store_raw_receipt(db: &redb::Database, receipt_ref: &str, receipt_value: &IOValue) -> Result<()> {
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let mut receipts = write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        let bytes = crate::preserves_rail::canonical_bytes(receipt_value)?;
        receipts.insert(receipt_ref, bytes.as_slice()).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)
}

fn dedup_entry_value(input: DedupEntryValueInput<'_>) -> Result<IOValue> {
    validate_refs(input.evidence_refs, "delivery dedup evidence ref")?;
    if let Some(result_ref) = input.semantic_result_ref {
        require_ref(result_ref, "delivery dedup semantic result ref")?;
    }
    require_ref(input.first_receipt_ref, "delivery dedup first receipt ref")?;
    Ok(record("dedup-entry-v1", vec![
        string(crate::preserves_rail::DELIVERY_DEDUP_ENTRY_SCHEMA),
        record("dedup-key", vec![string(input.dedup_key)]),
        record("operation", vec![string(&input.operation.operation_ref)]),
        record("scope", vec![string(&input.operation.scope_ref)]),
        record("producer", vec![string(&input.operation.producer)]),
        record("consumer", vec![string(&input.operation.consumer)]),
        record("sequence", vec![u64_value(input.operation.sequence)]),
        record("intent", vec![string(&input.operation.intent)]),
        record("payload", vec![string(&input.operation.payload_ref)]),
        record("semantic-result", vec![optional_ref_value(input.semantic_result_ref)]),
        record("first-receipt", vec![string(input.first_receipt_ref)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[
            ("first-receipt-bound", "pass"),
            ("payload-ref-bound", "pass"),
            ("retention-pinned", "pass"),
        ]),
    ]))
}

struct DedupEntryValueInput<'a> {
    dedup_key: &'a str,
    operation: &'a OperationId,
    semantic_result_ref: Option<&'a str>,
    first_receipt_ref: &'a str,
    evidence_refs: &'a [String],
}

fn idempotency_receipt_value(input: IdempotencyReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    require_ref(input.operation_ref, "delivery idempotency operation ref")?;
    require_ref(input.scope_ref, "delivery idempotency scope ref")?;
    require_ref(input.window_ref, "delivery idempotency window ref")?;
    if let Some(prior) = input.prior_receipt_ref {
        require_ref(prior, "delivery idempotency prior receipt ref")?;
    }
    if let Some(result) = input.semantic_result_ref {
        require_ref(result, "delivery idempotency semantic result ref")?;
    }
    validate_side_effect(input.side_effect)?;
    validate_diagnostics(input.diagnostics)?;
    Ok(record("delivery-idempotency-receipt-v1", vec![
        string(crate::preserves_rail::DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation_ref)]),
        record("scope", vec![string(input.scope_ref)]),
        record("window", vec![string(input.window_ref)]),
        record("prior", vec![optional_ref_value(input.prior_receipt_ref)]),
        record("semantic-result", vec![optional_ref_value(input.semantic_result_ref)]),
        record("side-effect", vec![string(input.side_effect)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("checks", vec![sequence(
            input
                .checks
                .iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
    ]))
}

struct IdempotencyReceiptValueInput<'a> {
    decision: &'a str,
    operation_ref: &'a str,
    scope_ref: &'a str,
    window_ref: &'a str,
    prior_receipt_ref: Option<&'a str>,
    semantic_result_ref: Option<&'a str>,
    side_effect: &'a str,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn dedup_key_ref(operation: &OperationId) -> Result<String> {
    canonical_hash(&record("dedup-key-v1", vec![
        record("scope", vec![string(&operation.scope_ref)]),
        record("producer", vec![string(&operation.producer)]),
        record("consumer", vec![string(&operation.consumer)]),
        record("sequence", vec![u64_value(operation.sequence)]),
        record("intent", vec![string(&operation.intent)]),
    ]))
}

fn validate_operation_input(input: &OperationIdInput) -> Result<()> {
    require_ref(&input.scope_ref, "delivery operation scope ref")?;
    validate_name(&input.producer, "delivery operation producer")?;
    validate_name(&input.consumer, "delivery operation consumer")?;
    validate_name(&input.intent, "delivery operation intent")?;
    require_ref(&input.payload_ref, "delivery operation payload ref")?;
    validate_refs(&input.policy_refs, "delivery operation policy ref")?;
    ensure_count_at_most(input.policy_refs.len(), MAX_DELIVERY_REFS, "delivery operation policy refs")
}

fn validate_scope_profile(profile: &str) -> Result<()> {
    match profile {
        SCOPE_ACTOR_TURN
        | SCOPE_SERVICE_LIFECYCLE
        | SCOPE_PROTOCOL_SESSION
        | SCOPE_REMOTE_TOPIC
        | SCOPE_JOB_WORKER
        | SCOPE_CONTROL_COMMAND => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported delivery scope profile {profile}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "first" | "duplicate" | "conflict" | "stale" | "gap" | "retry" | "deny" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported idempotency decision {decision}"))),
    }
}

fn validate_side_effect(side_effect: &str) -> Result<()> {
    match side_effect {
        "commit" | "suppress" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported delivery side effect {side_effect}"))),
    }
}

fn validate_diagnostics(diagnostics: &[String]) -> Result<()> {
    ensure_count_at_most(diagnostics.len(), MAX_DELIVERY_DIAGNOSTICS, "delivery diagnostics")?;
    for diagnostic in diagnostics {
        validate_name(diagnostic, "delivery diagnostic")?;
    }
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains('\0') || value.len() > MAX_SCOPE_NAME_LEN {
        return Err(MoltenError::invalid_harness(format!("invalid {label} {value:?}")));
    }
    Ok(())
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_DELIVERY_REFS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "unsupported {label} {reference}; expected canonical content ref: {error}"
        ))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}

fn ensure_store_tables(root: &std::path::Path) -> Result<redb::Database> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    let db = redb::Database::create(store_path(root)).map_err(store_error)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        write_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
        write_txn.open_table(STORE_ENTRIES).map_err(store_error)?;
        write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        write_txn.open_table(STORE_PINS).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)?;
    Ok(db)
}

fn store_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(STORE_FILE)
}

fn store_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("delivery idempotency redb store error: {error}"))
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected checks record"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected checks sequence"))?;
    let mut checks = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let check_value = value_to_iovalue(entry);
        let check_fields = check_value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
        checks.push((
            required_string(&check_fields[0], "check name")?,
            required_string(&check_fields[1], "check status")?,
        ));
    }
    Ok(checks)
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} missing pass check {name}")))
    }
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let inner = value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    let reference = required_string(&some[0], label)?;
    require_ref(&reference, label)?;
    Ok(Some(reference))
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let refs = record_string_sequence(value, label)?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    entries.iter().map(|entry| required_string(entry, label)).collect()
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn require_schema(value: &Value<IOValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {label} {expected}, got {actual}")))
    }
}

fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_identity_is_canonical_and_payload_sensitive() {
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let payload_a = fake_ref("payload-a");
        let payload_b = fake_ref("payload-b");
        let left = derive_operation_id(OperationIdInput {
            scope_ref: scope.clone(),
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 7,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_a.clone(),
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("left operation");
        let right = derive_operation_id(OperationIdInput {
            scope_ref: scope.clone(),
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 7,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_a,
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("right operation");
        let changed = derive_operation_id(OperationIdInput {
            scope_ref: scope,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 7,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_b,
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("changed operation");
        assert_eq!(left.operation_ref, right.operation_ref);
        assert_ne!(left.operation_ref, changed.operation_ref);
    }

    #[test]
    fn duplicate_delivery_suppresses_second_side_effect_and_returns_prior_result() {
        let root = temp_dir("delivery-duplicate");
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("evidence")];
        let result_ref = fake_ref("semantic-result");
        let first = check_delivery(DeliveryCheckInput {
            root: &root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: 1,
            intent: "remote-dataspace-assert",
            payload_ref: &fake_ref("payload"),
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            semantic_result_ref: Some(&result_ref),
            gap_policy: GapPolicy::Deny,
        })
        .expect("first delivery");
        assert_eq!(first.receipt.decision, "first");
        assert!(first.should_commit_side_effect);
        let duplicate = check_delivery(DeliveryCheckInput {
            root: &root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: 1,
            intent: "remote-dataspace-assert",
            payload_ref: &fake_ref("payload"),
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            semantic_result_ref: Some(&result_ref),
            gap_policy: GapPolicy::Deny,
        })
        .expect("duplicate delivery");
        assert_eq!(duplicate.receipt.decision, "duplicate");
        assert!(!duplicate.should_commit_side_effect);
        assert_eq!(duplicate.prior_semantic_result_ref.as_deref(), Some(result_ref.as_str()));
    }

    #[test]
    fn conflict_stale_gap_and_retry_are_canonical_denials() {
        let case = negative_case();
        assert_first(&case);
        assert_conflict(&case);
        assert_denied(&case, attempt(0, "stale", None, GapPolicy::Deny), "stale");
        assert_denied(&case, attempt(4, "gap", None, GapPolicy::Deny), "gap");
        assert_denied(&case, attempt(4, "retry", None, GapPolicy::Retry), "retry");
    }

    struct Case {
        root: std::path::PathBuf,
        scope: String,
        policy_refs: Vec<String>,
        evidence_refs: Vec<String>,
        payload_ref: String,
        result_ref: String,
    }

    struct Attempt {
        sequence: u64,
        payload_ref: String,
        semantic_result_ref: Option<String>,
        gap_policy: GapPolicy,
    }

    fn negative_case() -> Case {
        Case {
            root: temp_dir("delivery-negative"),
            scope: remote_topic_scope_ref("services", "peer:b").expect("scope"),
            policy_refs: vec![fake_ref("policy")],
            evidence_refs: vec![fake_ref("evidence")],
            payload_ref: fake_ref("payload"),
            result_ref: fake_ref("result"),
        }
    }

    fn attempt(sequence: u64, payload_label: &str, result_label: Option<&str>, gap_policy: GapPolicy) -> Attempt {
        Attempt {
            sequence,
            payload_ref: fake_ref(payload_label),
            semantic_result_ref: result_label.map(fake_ref),
            gap_policy,
        }
    }

    fn assert_first(case: &Case) {
        let first = check_case(
            case,
            Attempt {
                sequence: 1,
                payload_ref: case.payload_ref.clone(),
                semantic_result_ref: Some(case.result_ref.clone()),
                gap_policy: GapPolicy::Deny,
            },
            "first",
        );
        assert_eq!(first.receipt.decision, "first");
    }

    fn assert_conflict(case: &Case) {
        let conflict =
            check_case(case, attempt(1, "changed-payload", Some("changed-result"), GapPolicy::Deny), "conflict");
        assert_eq!(conflict.receipt.decision, "conflict");
        assert!(!conflict.should_commit_side_effect);
    }

    fn assert_denied(case: &Case, attempt: Attempt, decision: &str) {
        let denied = check_case(case, attempt, decision);
        assert_eq!(denied.receipt.decision, decision);
    }

    fn check_case(case: &Case, attempt: Attempt, context: &str) -> DeliveryDecision {
        check_delivery(DeliveryCheckInput {
            root: &case.root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &case.scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: attempt.sequence,
            intent: "remote-dataspace-message",
            payload_ref: &attempt.payload_ref,
            policy_refs: &case.policy_refs,
            evidence_refs: &case.evidence_refs,
            semantic_result_ref: attempt.semantic_result_ref.as_deref(),
            gap_policy: attempt.gap_policy,
        })
        .expect(context)
    }

    #[test]
    fn hegel_like_no_global_sequence_invariant_for_independent_scopes() {
        for sequence in 1..8_u64 {
            let root = temp_dir("delivery-scopes");
            let left_scope = remote_topic_scope_ref("services", "peer:left").expect("left scope");
            let right_scope = remote_topic_scope_ref("services", "peer:right").expect("right scope");
            let policy_refs = vec![fake_ref("policy")];
            let evidence_refs = vec![fake_ref("evidence")];
            let left = check_delivery(DeliveryCheckInput {
                root: &root,
                scope_profile: SCOPE_REMOTE_TOPIC,
                scope_ref: &left_scope,
                producer: "peer:a/producer",
                consumer: "peer:left",
                sequence,
                intent: "remote-dataspace-assert",
                payload_ref: &fake_ref("payload-left"),
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: Some(&fake_ref("left-result")),
                gap_policy: if sequence == 1 {
                    GapPolicy::Deny
                } else {
                    GapPolicy::Retry
                },
            })
            .expect("left delivery");
            let right = check_delivery(DeliveryCheckInput {
                root: &root,
                scope_profile: SCOPE_REMOTE_TOPIC,
                scope_ref: &right_scope,
                producer: "peer:a/producer",
                consumer: "peer:right",
                sequence: 1,
                intent: "remote-dataspace-assert",
                payload_ref: &fake_ref("payload-right"),
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: Some(&fake_ref("right-result")),
                gap_policy: GapPolicy::Deny,
            })
            .expect("right delivery");
            assert_eq!(right.receipt.decision, "first");
            if sequence == 1 {
                assert_eq!(left.receipt.decision, "first");
            } else {
                assert_eq!(left.receipt.decision, "retry");
            }
        }
    }

    fn fake_ref(label: &str) -> String {
        canonical_hash(&crate::preserves_rail::record("fake-ref", vec![crate::preserves_rail::string(label)]))
            .expect("fake ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
