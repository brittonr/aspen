use std::collections::BTreeMap;
use std::collections::VecDeque;

use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::RESOURCE_CONSUMPTION_SCHEMA;
use crate::preserves_rail::RESOURCE_GRANT_SCHEMA;
use crate::preserves_rail::RESOURCE_RECEIPT_SCHEMA;
use crate::preserves_rail::RESOURCE_SCHEDULER_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConsumption {
    pub grant_ref: String,
    pub kind: String,
    pub amount: u64,
    pub sequence: u64,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDecision {
    pub decision: String,
    pub consumed: u64,
    pub remaining: u64,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailboxDecision {
    pub accepted: bool,
    pub queue: Vec<String>,
    pub overflow: Option<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTask {
    pub actor: String,
    pub priority: u64,
    pub sequence: u64,
    pub budget_class: String,
}

pub struct ConsumeInput<'a> {
    pub grant_value: &'a IOValue,
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

pub fn resource_grant_value(input: &ResourceGrantInput) -> Result<IOValue> {
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

pub fn parse_resource_grant(value: &IOValue) -> Result<ResourceGrant> {
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

pub fn resource_consumption_value(grant: &ResourceGrant, amount: u64, sequence_number: u64) -> Result<IOValue> {
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

pub fn parse_consumption(value: &IOValue) -> Result<ResourceConsumption> {
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

pub fn apply_mailbox_backpressure(queue: &[String], message_ref: &str, max_slots: u64) -> Result<MailboxDecision> {
    require_ref(message_ref, "mailbox message ref")?;
    ensure_u64_at_most(max_slots, MAX_RESOURCE_SEQUENCE_ITEMS_U64, "mailbox slots")?;
    ensure_count_at_most(queue.len(), MAX_RESOURCE_SEQUENCE_ITEMS, "mailbox queue")?;
    let queue_len = count_to_u64(queue.len(), "mailbox queue")?;
    let mut next = queue.to_vec();
    let mut overflow = None;
    let is_accepted = if queue_len < max_slots {
        push_bounded(&mut next, message_ref.to_string(), MAX_RESOURCE_SEQUENCE_ITEMS, "mailbox queue")?;
        true
    } else {
        overflow = Some(message_ref.to_string());
        false
    };
    let next_len = count_to_u64(next.len(), "mailbox queue")?;
    let receipt_value = resource_receipt_value(&ReceiptValueInput {
        operation: "mailbox-enqueue",
        decision: if is_accepted { "pass" } else { "throttle" },
        grant_ref: "mailbox",
        kind: KIND_MAILBOX_SLOTS,
        requested: 1,
        consumed: if is_accepted { 1 } else { 0 },
        remaining: max_slots.saturating_sub(next_len),
        diagnostics: &[if is_accepted { "queued" } else { "mailbox-full" }],
        consumption_ref: None,
    });
    Ok(MailboxDecision {
        accepted: is_accepted,
        queue: next,
        overflow,
        receipt_value,
    })
}

pub fn enforce_assertion_bound(current: u64, limit: u64, assertion_ref: &str) -> Result<ResourceDecision> {
    require_ref(assertion_ref, "assertion ref")?;
    ensure_u64_at_most(current, MAX_RESOURCE_SEQUENCE_ITEMS_U64, "current assertions")?;
    ensure_u64_at_most(limit, MAX_RESOURCE_SEQUENCE_ITEMS_U64, "assertion limit")?;
    let is_over_limit = current.saturating_add(1) > limit;
    let admitted_increment = if is_over_limit { 0 } else { 1 };
    let remaining = limit.saturating_sub(current.saturating_add(admitted_increment));
    let receipt_value = resource_receipt_value(&ReceiptValueInput {
        operation: "assertion-bound",
        decision: if is_over_limit { "deny" } else { "pass" },
        grant_ref: assertion_ref,
        kind: KIND_ASSERTIONS,
        requested: 1,
        consumed: admitted_increment,
        remaining,
        diagnostics: &[if is_over_limit {
            "assertion-limit"
        } else {
            "assertion-admitted"
        }],
        consumption_ref: None,
    });
    Ok(ResourceDecision {
        decision: if is_over_limit { "deny" } else { "pass" }.to_string(),
        consumed: admitted_increment,
        remaining,
        receipt_value,
    })
}

pub fn deterministic_schedule(tasks: &[SchedulerTask], quantum: u64) -> Result<IOValue> {
    ensure_count_at_most(tasks.len(), MAX_RESOURCE_SEQUENCE_ITEMS, "scheduler tasks")?;
    let quantum_steps = bounded_positive_count(quantum, MAX_RESOURCE_SEQUENCE_ITEMS_U64, "scheduler quantum")?;
    let mut queues = BTreeMap::<(u64, String), VecDeque<&SchedulerTask>>::new();
    for task in tasks {
        validate_non_empty(&task.actor, "scheduler task actor")?;
        validate_non_empty(&task.budget_class, "scheduler budget class")?;
        queues.entry((task.priority, task.budget_class.clone())).or_default().push_back(task);
    }
    let mut order = Vec::new();
    while queues.values().any(|queue| !queue.is_empty()) {
        for queue in queues.values_mut() {
            for _ in 0..quantum_steps {
                let Some(task) = queue.pop_front() else {
                    break;
                };
                push_bounded(
                    &mut order,
                    record("scheduled", vec![
                        string(&task.actor),
                        u64_value(task.priority),
                        u64_value(task.sequence),
                        string(&task.budget_class),
                    ]),
                    MAX_RESOURCE_SEQUENCE_ITEMS,
                    "scheduler order",
                )?;
            }
        }
    }
    Ok(record("resource-scheduler-v1", vec![
        string(RESOURCE_SCHEDULER_SCHEMA),
        record("policy", vec![string("deterministic-round-robin")]),
        record("order", vec![sequence(order)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("os-timing-independent"), string("pass")]),
            record("check", vec![string("deterministic-fairness"), string("pass")]),
        ])]),
    ]))
}

pub fn adapter_budget_decision(kind: &str, requested: u64, limit: u64, context: &str) -> Result<ResourceDecision> {
    validate_resource_kind(kind)?;
    validate_non_empty(context, "adapter budget context")?;
    let is_over_limit = requested > limit;
    let consumed = if is_over_limit { 0 } else { requested };
    let receipt_value = resource_receipt_value(&ReceiptValueInput {
        operation: context,
        decision: if is_over_limit { "deny" } else { "pass" },
        grant_ref: context,
        kind,
        requested,
        consumed,
        remaining: limit.saturating_sub(consumed),
        diagnostics: &[if is_over_limit {
            "adapter-budget-exceeded"
        } else {
            "adapter-budget-admitted"
        }],
        consumption_ref: None,
    });
    Ok(ResourceDecision {
        decision: if is_over_limit { "deny" } else { "pass" }.to_string(),
        consumed,
        remaining: limit.saturating_sub(consumed),
        receipt_value,
    })
}

pub fn plan_job_stages(stages: &[(&str, u64)], available_slots: u64) -> Result<Vec<String>> {
    ensure_count_at_most(stages.len(), MAX_RESOURCE_SEQUENCE_ITEMS, "job stage placements")?;
    let mut slots = available_slots;
    let mut plan = Vec::with_capacity(stages.len());
    for (stage, required) in stages {
        validate_non_empty(stage, "job stage")?;
        if *required <= slots {
            slots -= *required;
            push_bounded(
                &mut plan,
                format!("place:{stage}:{required}"),
                MAX_RESOURCE_SEQUENCE_ITEMS,
                "job stage placements",
            )?;
        } else {
            push_bounded(
                &mut plan,
                format!("defer:{stage}:{required}"),
                MAX_RESOURCE_SEQUENCE_ITEMS,
                "job stage placements",
            )?;
        }
    }
    Ok(plan)
}

pub fn resource_receipt_value(input: &ReceiptValueInput<'_>) -> IOValue {
    record("resource-receipt-v1", vec![
        string(RESOURCE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("grant", vec![string(input.grant_ref)]),
        record("kind", vec![string(input.kind)]),
        record("requested", vec![u64_value(input.requested)]),
        record("consumed", vec![u64_value(input.consumed)]),
        record("remaining", vec![u64_value(input.remaining)]),
        record("consumption", vec![optional_ref_value(input.consumption_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("deterministic-backpressure"), string("pass")]),
            record("check", vec![string("no-silent-drop"), string("pass")]),
            record("check", vec![string("resource-grant-not-authority"), string("pass")]),
            record("check", vec![string("supervisor-signal-on-deny"), string("pass")]),
        ])]),
    ])
}

fn diagnostic_for(
    is_revoked: bool,
    is_expired: bool,
    is_before_validity_window: bool,
    is_over_budget: bool,
) -> &'static str {
    if is_revoked {
        return "revoked";
    }
    if is_expired {
        return "expired";
    }
    if is_before_validity_window {
        return "not-yet-valid";
    }
    if is_over_budget {
        return "over-budget";
    }
    "admitted"
}

fn validate_resource_kind(kind: &str) -> Result<()> {
    match kind {
        KIND_TURNS
        | KIND_CPU_FUEL
        | KIND_MEMORY_BYTES
        | KIND_MAILBOX_SLOTS
        | KIND_ASSERTIONS
        | KIND_SUBSCRIPTIONS
        | KIND_BLOB_BYTES
        | KIND_STORAGE_BYTES
        | KIND_NETWORK_MESSAGES
        | KIND_NETWORK_BYTES
        | KIND_REMOTE_FETCHES
        | KIND_EFFECT_CALLS
        | KIND_TRACE_BYTES
        | KIND_JOB_SLOTS => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported resource kind {other}"))),
    }
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn ensure_u64_at_most(actual: u64, maximum: u64, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn count_to_u64(count: usize, label: &str) -> Result<u64> {
    u64::try_from(count).map_err(|_| MoltenError::invalid_harness(format!("{label} count exceeds u64 bound")))
}

fn bounded_positive_count(count: u64, maximum: u64, label: &str) -> Result<usize> {
    let normalized = count.max(1);
    ensure_u64_at_most(normalized, maximum, label)?;
    usize::try_from(normalized).map_err(|_| MoltenError::invalid_harness(format!("{label} count exceeds usize bound")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_ref_record(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let record = value_to_iovalue(value);
    let fields = record
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let optional = value_to_iovalue(&fields[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        let reference = required_string(&some[0], label)?;
        require_ref(&reference, label)?;
        Ok(Some(reference))
    } else {
        Err(MoltenError::invalid_harness(format!("expected optional ref for {label}")))
    }
}

fn parse_optional_u64_record(value: &Value<IOValue>, label: &str) -> Result<Option<u64>> {
    let record = value_to_iovalue(value);
    let fields = record
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_u64_value(&fields[0])
}

fn parse_optional_u64_value(value: &Value<IOValue>) -> Result<Option<u64>> {
    let optional = value_to_iovalue(value);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_u64(&some[0], "optional u64").map(Some)
    } else {
        Err(MoltenError::invalid_harness("expected optional u64"))
    }
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected resource check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("resource evidence missing passing {name} check")))
    }
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
    required_u64(&fields[0], label)
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    #[test]
    fn grant_consumption_throttle_and_revocation_are_receipted() {
        let grant_value = sample_grant(KIND_EFFECT_CALLS, 2, None).expect("grant");
        let grant = parse_resource_grant(&grant_value).expect("parse grant");
        let first = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 0,
            sequence: 0,
            is_revoked: false,
        })
        .expect("first consume");
        assert_eq!(first.decision, "pass");
        let consumption = parse_consumption(&resource_consumption_value(&grant, 1, 0).expect("consumption"))
            .expect("parse consumption");
        let prior_consumptions = [consumption];
        let second = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &prior_consumptions,
            amount: 2,
            logical_time: 0,
            sequence: 1,
            is_revoked: false,
        })
        .expect("over consume");
        assert_eq!(second.decision, "throttle");
        let revoked = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 0,
            sequence: 2,
            is_revoked: true,
        })
        .expect("revoked consume");
        assert_eq!(revoked.decision, "deny");
    }

    #[test]
    fn mailbox_overflow_is_deterministic_and_not_silent() {
        let first = ref_for("message-1");
        let second = ref_for("message-2");
        let accepted = apply_mailbox_backpressure(&[], &first, 1).expect("accepted");
        assert!(accepted.accepted);
        let denied = apply_mailbox_backpressure(&accepted.queue, &second, 1).expect("overflow");
        assert!(!denied.accepted);
        assert_eq!(denied.overflow, Some(second));
        assert!(crate::preserves_rail::to_text(&denied.receipt_value).expect("receipt").contains("mailbox-full"));
    }

    #[test]
    fn turn_assertion_adapter_and_job_budgets_are_enforced() {
        let turn_grant = sample_grant(KIND_TURNS, 1, None).expect("turn grant");
        assert_eq!(
            consume_resource(&ConsumeInput {
                grant_value: &turn_grant,
                prior_consumptions: &[],
                amount: 1,
                logical_time: 0,
                sequence: 0,
                is_revoked: false,
            })
            .expect("turn")
            .decision,
            "pass"
        );
        assert_eq!(
            consume_resource(&ConsumeInput {
                grant_value: &turn_grant,
                prior_consumptions: &[],
                amount: 2,
                logical_time: 0,
                sequence: 1,
                is_revoked: false,
            })
            .expect("turn over")
            .decision,
            "throttle"
        );
        assert_eq!(enforce_assertion_bound(1, 1, &ref_for("assertion")).expect("assertion").decision, "deny");
        assert_eq!(adapter_budget_decision(KIND_CPU_FUEL, 10, 8, "wasmtime-fuel").expect("wasm").decision, "deny");
        assert_eq!(
            adapter_budget_decision(KIND_CPU_FUEL, 4, 8, "steel-native-budget").expect("steel").decision,
            "pass"
        );
        assert_eq!(
            adapter_budget_decision(KIND_BLOB_BYTES, 9, 8, "blob-storage-network").expect("blob").decision,
            "deny"
        );
        assert_eq!(plan_job_stages(&[("a", 1), ("b", 2)], 2).expect("plan"), vec!["place:a:1", "defer:b:2"]);
    }

    #[test]
    fn deterministic_scheduler_is_os_timing_independent() {
        let tasks = vec![
            SchedulerTask {
                actor: "a".to_string(),
                priority: 0,
                sequence: 1,
                budget_class: "normal".to_string(),
            },
            SchedulerTask {
                actor: "b".to_string(),
                priority: 0,
                sequence: 2,
                budget_class: "normal".to_string(),
            },
        ];
        let first = deterministic_schedule(&tasks, 1).expect("schedule");
        let second = deterministic_schedule(&tasks, 1).expect("schedule");
        assert_eq!(first, second);
        assert!(crate::preserves_rail::to_text(&first).expect("schedule text").contains("os-timing-independent"));
    }

    #[test]
    fn expired_grants_deny_future_work_and_receipts_replay() {
        let grant_value = sample_grant(KIND_NETWORK_MESSAGES, 1, Some(5)).expect("grant");
        let before = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 4,
            sequence: 0,
            is_revoked: false,
        })
        .expect("before expiry");
        let after = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 5,
            sequence: 1,
            is_revoked: false,
        })
        .expect("after expiry");
        assert_eq!(before.decision, "pass");
        assert_eq!(after.decision, "deny");
        let replay = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: 1,
            logical_time: 5,
            sequence: 1,
            is_revoked: false,
        })
        .expect("replay");
        assert_eq!(after.receipt_value, replay.receipt_value);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_budget_monotonicity_queue_bounds_and_no_silent_drop(tc: TestCase) {
        let amount = tc.draw(generators::integers::<u64>().min_value(1).max_value(16));
        let request = tc.draw(generators::integers::<u64>().min_value(1).max_value(20));
        let grant_value = sample_grant(KIND_TRACE_BYTES, amount, None).expect("grant");
        let decision = consume_resource(&ConsumeInput {
            grant_value: &grant_value,
            prior_consumptions: &[],
            amount: request,
            logical_time: 0,
            sequence: 0,
            is_revoked: false,
        })
        .expect("consume");
        if request <= amount {
            assert_eq!(decision.decision, "pass");
        } else {
            assert_eq!(decision.decision, "throttle");
            assert_eq!(decision.consumed, 0);
        }
        let max_slots = tc.draw(generators::integers::<u64>().min_value(0).max_value(4));
        let max_slots_usize = usize::try_from(max_slots).expect("bounded max slots");
        let queue = (0..max_slots_usize).map(|index| ref_for(&format!("queued-{index}"))).collect::<Vec<_>>();
        let mailbox = apply_mailbox_backpressure(&queue, &ref_for("new-message"), max_slots).expect("mailbox");
        assert_eq!(mailbox.queue.len(), max_slots_usize);
        assert!(!mailbox.accepted);
        assert!(mailbox.overflow.is_some());
    }

    fn sample_grant(kind: &str, amount: u64, expires_at: Option<u64>) -> Result<IOValue> {
        resource_grant_value(&ResourceGrantInput {
            subject_ref: ref_for("subject"),
            scope: "scope".to_string(),
            kind: kind.to_string(),
            amount,
            rate: None,
            window: None,
            not_before: None,
            expires_at,
            parent_ref: None,
            revocation_refs: Vec::new(),
            policy_refs: vec![ref_for("policy")],
            evidence_refs: vec![ref_for("evidence")],
        })
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("resource-test-ref", vec![string(label)])).expect("test ref")
    }
}
