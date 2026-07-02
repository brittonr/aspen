
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAssertionKind {
    Demand,
    Ready,
    Failure,
    Dependency,
    ExposedRef,
    Restart,
    Stop,
}

impl ServiceAssertionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Demand => "demand",
            Self::Ready => "ready",
            Self::Failure => "failure",
            Self::Dependency => "dependency",
            Self::ExposedRef => "exposed-ref",
            Self::Restart => "restart",
            Self::Stop => "stop",
        }
    }
}

pub fn transition_value(input: &TransitionInput) -> Result<IoValue> {
    validate_transition_input(input)?;
    Ok(record("lifecycle-transition-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_TRANSITION_SCHEMA),
        record("entity", vec![string(input.entity_kind.as_str()), string(&input.entity_id)]),
        record("state", vec![
            record("from", vec![string(input.from_state.as_str())]),
            record("to", vec![string(input.to_state.as_str())]),
        ]),
        record("action", vec![string(input.action.as_str())]),
        record("cause", vec![string(&input.cause)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("resources", vec![refs_sequence(&input.resource_refs)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        record("supervisor", vec![optional_ref_value(input.supervisor_ref.as_deref())]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]))
}

pub fn transition_record(input: &TransitionInput) -> Result<TransitionRecord> {
    let value = transition_value(input)?;
    let transition_ref = canonical_hash(&value)?;
    Ok(TransitionRecord { transition_ref, value })
}

pub fn transition_receipt(input: &TransitionInput) -> Result<TransitionReceipt> {
    let transition = transition_record(input)?;
    let diagnostics = transition_diagnostics(input);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("lifecycle-transition-receipt-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_TRANSITION_RECEIPT_SCHEMA),
        record("transition", vec![string(&transition.transition_ref)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(TransitionReceipt {
        receipt_ref,
        transition_ref: transition.transition_ref,
        decision: decision.to_owned(),
        diagnostics,
        value,
    })
}

pub fn trace_event(input: &TransitionInput) -> Result<TraceEvent> {
    let transition = transition_record(input)?;
    let value = record("lifecycle-trace-event-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_TRACE_EVENT_SCHEMA),
        record("transition", vec![string(&transition.transition_ref)]),
        record("entity", vec![string(input.entity_kind.as_str()), string(&input.entity_id)]),
        record("action", vec![string(input.action.as_str())]),
        record("cause", vec![string(&input.cause)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]);
    let event_ref = canonical_hash(&value)?;
    Ok(TraceEvent {
        event_ref,
        transition_ref: transition.transition_ref,
        entity_kind: input.entity_kind,
        entity_id: input.entity_id.clone(),
        action: input.action,
        cause: input.cause.clone(),
        policy_refs: input.policy_refs.clone(),
        value,
    })
}

pub fn turn_failure_receipt(input: &TurnFailureInput<'_>) -> Result<TurnFailureReceipt> {
    validate_turn_failure_input(input)?;
    let before_ref = input.before.snapshot_ref()?;
    let after_ref = input.after_rollback.snapshot_ref()?;
    let pending_turn = pending_turn_value(input.pending_turn)?;
    let pending_turn_ref = canonical_hash(&pending_turn)?;
    let discarded_action_refs = pending_action_refs(input.pending_turn)?;
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(2));
    if before_ref != after_ref {
        diagnostics.push("rollback state differs from before state".to_owned());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("lifecycle-turn-failure-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_TURN_FAILURE_SCHEMA),
        record("entity", vec![string(input.entity_kind.as_str()), string(input.entity_id)]),
        record("failure-kind", vec![string(input.failure_kind.as_str())]),
        record("cause", vec![string(input.cause)]),
        record("before-state-ref", vec![string(&before_ref)]),
        record("after-rollback-state-ref", vec![string(&after_ref)]),
        record("pending-turn-ref", vec![string(&pending_turn_ref)]),
        record("pending-turn", vec![pending_turn]),
        record("discarded-actions", vec![refs_sequence(&discarded_action_refs)]),
        record("vat-deltas-discarded", vec![refs_sequence(input.vat_delta_refs)]),
        record("one-shot-effects", vec![refs_sequence(input.one_shot_effect_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(TurnFailureReceipt {
        receipt_ref,
        decision: decision.to_owned(),
        diagnostics,
        value,
    })
}

pub fn scope_cleanup_receipt(input: &ScopeCleanupInput<'_>) -> Result<ScopeCleanupReceipt> {
    validate_scope_cleanup_input(input)?;
    let before_ref = input.before.snapshot_ref()?;
    let after_ref = input.after_cleanup.snapshot_ref()?;
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(2));
    if before_ref == after_ref && cleanup_removed_anything(input.cleanup) {
        diagnostics.push("cleanup refs were reported but state did not change".to_owned());
    }
    if input.cleanup.actor != input.entity_id {
        diagnostics.push("cleanup actor does not match lifecycle entity".to_owned());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("lifecycle-scope-cleanup-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_SCOPE_CLEANUP_SCHEMA),
        record("entity", vec![string(input.entity_kind.as_str()), string(input.entity_id)]),
        record("cause", vec![string(input.cause)]),
        record("before-state-ref", vec![string(&before_ref)]),
        record("after-cleanup-state-ref", vec![string(&after_ref)]),
        record("retracted-assertions", vec![refs_sequence(&input.cleanup.assertion_refs)]),
        record("retracted-subscriptions", vec![refs_sequence(&input.cleanup.observer_refs)]),
        record("dropped-messages", vec![refs_sequence(&input.cleanup.message_refs)]),
        record("released-live-refs", vec![refs_sequence(input.live_ref_refs)]),
        record("released-resources", vec![refs_sequence(input.resource_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(ScopeCleanupReceipt {
        receipt_ref,
        decision: decision.to_owned(),
        diagnostics,
        value,
    })
}

pub fn monitor_receipt(input: &MonitorInput<'_>) -> Result<MonitorReceipt> {
    validate_monitor_input(input)?;
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(1));
    if input.policy_refs.is_empty() {
        diagnostics.push("monitor requires policy ref".to_owned());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("lifecycle-monitor-receipt-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_MONITOR_RECEIPT_SCHEMA),
        record("observer", vec![string(input.observer_id)]),
        record("child", vec![string(input.child_id)]),
        record("child-failure-ref", vec![string(input.child_failure_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("authority-escalated", vec![bool_value(false)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(MonitorReceipt {
        receipt_ref,
        decision: decision.to_owned(),
        diagnostics,
        value,
    })
}

pub fn supervisor_decision_receipt(input: &SupervisorDecisionInput<'_>) -> Result<SupervisorDecisionReceipt> {
    validate_supervisor_input(input)?;
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(2));
    match input.policy.strategy {
        RestartStrategy::Never => diagnostics.push("restart strategy never denies restart".to_owned()),
        RestartStrategy::OneForOne => {}
        RestartStrategy::Bounded => match input.policy.restart_window.as_ref() {
            Some(window) if input.logical_step < window.start_step || input.logical_step > window.end_step => {
                diagnostics.push("restart outside logical-time window".to_owned());
            }
            Some(window) if input.restart_count_in_window >= window.max_restarts => {
                diagnostics.push("restart budget exhausted".to_owned());
            }
            Some(_) => {}
            None => diagnostics.push("bounded restart strategy requires restart window".to_owned()),
        },
    }
    let decision = if diagnostics.is_empty() { "restart" } else { "deny" };
    let value = record("lifecycle-supervisor-decision-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_SUPERVISOR_DECISION_SCHEMA),
        record("supervisor", vec![string(&input.policy.supervisor_id)]),
        record("child", vec![string(input.child_id)]),
        record("strategy", vec![string(input.policy.strategy.as_str())]),
        restart_window_value(input.policy.restart_window.as_ref()),
        record("restart-count", vec![u64_value(input.restart_count_in_window)]),
        record("child-failure-ref", vec![string(input.child_failure_ref)]),
        record("policy", vec![refs_sequence(&input.policy.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(SupervisorDecisionReceipt {
        receipt_ref,
        decision: decision.to_owned(),
        diagnostics,
        value,
    })
}

pub fn service_lifecycle_assertion(
    service_id: &str,
    kind: ServiceAssertionKind,
    target_ref: Option<&str>,
    evidence_refs: &[String],
) -> Result<RuntimeValue> {
    if service_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("service lifecycle assertion id must be non-empty"));
    }
    if let Some(reference) = target_ref {
        validate_content_ref(reference)?;
    }
    validate_refs("evidence", evidence_refs)?;
    RuntimeValue::new(record("lifecycle-service-assertion-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_SERVICE_ASSERTION_SCHEMA),
        record("service", vec![string(service_id)]),
        record("kind", vec![string(kind.as_str())]),
        record("target", vec![optional_ref_value(target_ref)]),
        record("evidence", vec![refs_sequence(evidence_refs)]),
        checks_value(),
    ]))
}

fn validate_transition_input(input: &TransitionInput) -> Result<()> {
    if input.entity_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("lifecycle entity id must be non-empty"));
    }
    if input.cause.trim().is_empty() {
        return Err(MoltenError::invalid_harness("lifecycle transition cause must be non-empty"));
    }
    validate_refs("policy", &input.policy_refs)?;
    validate_refs("resources", &input.resource_refs)?;
    validate_refs("evidence", &input.evidence_refs)?;
    if let Some(supervisor_ref) = &input.supervisor_ref {
        validate_content_ref(supervisor_ref)?;
    }
    Ok(())
}
