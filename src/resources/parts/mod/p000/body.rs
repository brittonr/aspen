type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;
type VecDeque<T> = std::collections::VecDeque<T>;

const RESOURCE_CONSUMPTION_SCHEMA: &str = crate::preserves_rail::RESOURCE_CONSUMPTION_SCHEMA;
const RESOURCE_GRANT_SCHEMA: &str = crate::preserves_rail::RESOURCE_GRANT_SCHEMA;
const RESOURCE_RECEIPT_SCHEMA: &str = crate::preserves_rail::RESOURCE_RECEIPT_SCHEMA;
const RESOURCE_SCHEDULER_SCHEMA: &str = crate::preserves_rail::RESOURCE_SCHEDULER_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(fields)
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

pub const KIND_TURNS: &str = "turns";
pub const KIND_CPU_FUEL: &str = "cpu-fuel";
pub const KIND_MEMORY_BYTES: &str = "memory-bytes";
pub const KIND_MAILBOX_SLOTS: &str = "mailbox-slots";
pub const KIND_ASSERTIONS: &str = "assertions";
pub const KIND_SUBSCRIPTIONS: &str = "subscriptions";
pub const KIND_BLOB_BYTES: &str = "blob-bytes";
pub const KIND_STORAGE_BYTES: &str = "storage-bytes";
pub const KIND_NETWORK_MESSAGES: &str = "network-messages";
pub const KIND_NETWORK_BYTES: &str = "network-bytes";
pub const KIND_REMOTE_FETCHES: &str = "remote-fetches";
pub const KIND_EFFECT_CALLS: &str = "effect-calls";
pub const KIND_TRACE_BYTES: &str = "trace-bytes";
pub const KIND_JOB_SLOTS: &str = "job-slots";

const MAX_RESOURCE_SEQUENCE_ITEMS: usize = 4_096;
const MAX_RESOURCE_SEQUENCE_ITEMS_U64: u64 = 4_096;
const _: () = assert!(MAX_RESOURCE_SEQUENCE_ITEMS > 0);
const _: () = assert!(MAX_RESOURCE_SEQUENCE_ITEMS_U64 > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGrantInput {
    pub subject_ref: String,
    pub scope: String,
    pub kind: String,
    pub amount: u64,
    pub rate: Option<u64>,
    pub window: Option<u64>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub parent_ref: Option<String>,
    pub revocation_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGrant {
    pub grant_ref: String,
    pub subject_ref: String,
    pub scope: String,
    pub kind: String,
    pub amount: u64,
    pub rate: Option<u64>,
    pub window: Option<u64>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub parent_ref: Option<String>,
    pub revocation_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConsumption {
    pub grant_ref: String,
    pub kind: String,
    pub amount: u64,
    pub sequence: u64,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDecision {
    pub decision: String,
    pub consumed: u64,
    pub remaining: u64,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailboxDecision {
    pub accepted: bool,
    pub queue: Vec<String>,
    pub overflow: Option<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTask {
    pub actor: String,
    pub priority: u64,
    pub sequence: u64,
    pub budget_class: String,
}

pub struct ConsumeInput<'a> {
    pub grant_value: &'a IoValue,
    pub prior_consumptions: &'a [ResourceConsumption],
    pub amount: u64,
    pub logical_time: u64,
    pub sequence: u64,
    pub is_revoked: bool,
}

pub struct ReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub grant_ref: &'a str,
    pub kind: &'a str,
    pub requested: u64,
    pub consumed: u64,
    pub remaining: u64,
    pub diagnostics: &'a [&'a str],
    pub consumption_ref: Option<&'a str>,
}

pub fn resource_grant_value(input: &ResourceGrantInput) -> Result<IoValue> {
    require_ref(&input.subject_ref, "resource grant subject ref")?;
    validate_non_empty(&input.scope, "resource grant scope")?;
    validate_resource_kind(&input.kind)?;
    validate_refs(&input.revocation_refs, "resource grant revocation ref")?;
    validate_refs(&input.policy_refs, "resource grant policy ref")?;
    validate_refs(&input.evidence_refs, "resource grant evidence ref")?;
    if let Some(parent_ref) = input.parent_ref.as_deref() {
        require_ref(parent_ref, "resource grant parent ref")?;
    }
    Ok(record("resource-grant-v1", vec![
        string(RESOURCE_GRANT_SCHEMA),
        record("subject", vec![string(&input.subject_ref)]),
        record("scope", vec![string(&input.scope)]),
        record("kind", vec![string(&input.kind)]),
        record("amount", vec![u64_value(input.amount)]),
        record("rate", vec![optional_u64_value(input.rate)]),
        record("window", vec![optional_u64_value(input.window)]),
        record("validity", vec![
            optional_u64_value(input.not_before),
            optional_u64_value(input.expires_at),
        ]),
        record("parent", vec![optional_ref_value(input.parent_ref.as_deref())]),
        record("revocations", vec![sequence(input.revocation_refs.iter().map(string).collect())]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("resource-grant-not-data-authority"), string("pass")]),
            record("check", vec![string("bounded-resource"), string("pass")]),
            record("check", vec![string("policy-bound-grant"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_resource_grant(value: &IoValue) -> Result<ResourceGrant> {
    let fields = value
        .collect_simple_record("resource-grant-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <resource-grant-v1 ...>"))?;
    require_schema(&fields[0], RESOURCE_GRANT_SCHEMA, "resource grant schema")?;
    let validity = value_to_iovalue(&fields[7]);
    let validity_fields = validity
        .collect_simple_record("validity", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("resource grant missing validity"))?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "resource-grant-not-data-authority")?;
    Ok(ResourceGrant {
        grant_ref: canonical_hash(value)?,
        subject_ref: record_string(&fields[1], "subject")?,
        scope: record_string(&fields[2], "scope")?,
        kind: record_string(&fields[3], "kind")?,
        amount: record_u64(&fields[4], "amount")?,
        rate: parse_optional_u64_record(&fields[5], "rate")?,
        window: parse_optional_u64_record(&fields[6], "window")?,
        not_before: parse_optional_u64_value(&validity_fields[0])?,
        expires_at: parse_optional_u64_value(&validity_fields[1])?,
        parent_ref: parse_optional_ref_record(&fields[8], "parent")?,
        revocation_refs: parse_ref_sequence(&fields[9], "revocations")?,
        policy_refs: parse_ref_sequence(&fields[10], "policy")?,
        evidence_refs: parse_ref_sequence(&fields[11], "evidence")?,
        value: value.clone(),
    })
}

pub fn consume_resource(input: &ConsumeInput<'_>) -> Result<ResourceDecision> {
    let grant = parse_resource_grant(input.grant_value)?;
    let already = input
        .prior_consumptions
        .iter()
        .filter(|consumption| consumption.grant_ref == grant.grant_ref && consumption.kind == grant.kind)
        .map(|consumption| consumption.amount)
        .sum::<u64>();
    let is_expired = grant.expires_at.is_some_and(|expires_at| input.logical_time >= expires_at);
    let is_before_validity_window = grant.not_before.is_some_and(|not_before| input.logical_time < not_before);
    let would_total = already.saturating_add(input.amount);
    let is_over_budget = would_total > grant.amount;
    let decision = if input.is_revoked || is_expired || is_before_validity_window || is_over_budget {
        if is_over_budget { "throttle" } else { "deny" }
    } else {
        "pass"
    };
    let consumed = if decision == "pass" { input.amount } else { 0 };
    let remaining = grant.amount.saturating_sub(already.saturating_add(consumed));
    let consumption_value = resource_consumption_value(&grant, consumed, input.sequence)?;
    let consumption_ref = canonical_hash(&consumption_value)?;
    let receipt_value = resource_receipt_value(&ReceiptValueInput {
        operation: "consume",
        decision,
        grant_ref: &grant.grant_ref,
        kind: &grant.kind,
        requested: input.amount,
        consumed,
        remaining,
        diagnostics: &[diagnostic_for(
            input.is_revoked,
            is_expired,
            is_before_validity_window,
            is_over_budget,
        )],
        consumption_ref: Some(&consumption_ref),
    });
    Ok(ResourceDecision {
        decision: decision.to_string(),
        consumed,
        remaining,
        receipt_value,
    })
}

pub fn resource_consumption_value(grant: &ResourceGrant, amount: u64, sequence_number: u64) -> Result<IoValue> {
    Ok(record("resource-consumption-v1", vec![
        string(RESOURCE_CONSUMPTION_SCHEMA),
        record("grant", vec![string(&grant.grant_ref)]),
        record("kind", vec![string(&grant.kind)]),
        record("amount", vec![u64_value(amount)]),
        record("sequence", vec![u64_value(sequence_number)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("deterministic-consumption"), string("pass")]),
            record("check", vec![string("no-silent-drop"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_consumption(value: &IoValue) -> Result<ResourceConsumption> {
    let fields = value
        .collect_simple_record("resource-consumption-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <resource-consumption-v1 ...>"))?;
    require_schema(&fields[0], RESOURCE_CONSUMPTION_SCHEMA, "resource consumption schema")?;
    Ok(ResourceConsumption {
        grant_ref: record_string(&fields[1], "grant")?,
        kind: record_string(&fields[2], "kind")?,
        amount: record_u64(&fields[3], "amount")?,
        sequence: record_u64(&fields[4], "sequence")?,
        value: value.clone(),
    })
}
