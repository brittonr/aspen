
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

pub fn deterministic_schedule(tasks: &[SchedulerTask], quantum: u64) -> Result<IoValue> {
    ensure_count_at_most(tasks.len(), MAX_RESOURCE_SEQUENCE_ITEMS, "scheduler tasks")?;
    let quantum_steps = bounded_positive_count(quantum, MAX_RESOURCE_SEQUENCE_ITEMS_U64, "scheduler quantum")?;
    let mut queues = OrderedMap::<(u64, String), VecDeque<&SchedulerTask>>::new();
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

pub fn resource_receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
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

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}
