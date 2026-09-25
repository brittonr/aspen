
fn shell_intents(event: &ServiceFsmEvent) -> Vec<String> {
    match event.event_name.as_str() {
        EVENT_ACQUIRE_SERVICE_LOCK => vec![INTENT_ACQUIRE_LOCK.to_string()],
        EVENT_SERVE => vec![INTENT_SCAN_INGRESS.to_string()],
        EVENT_HEARTBEAT => vec![INTENT_WRITE_HEARTBEAT.to_string()],
        EVENT_SHUTDOWN_REQUESTED => vec![INTENT_DRAIN_INBOX.to_string(), INTENT_WRITE_SHUTDOWN.to_string()],
        EVENT_DRAIN_COMPLETE | EVENT_STOP => vec![INTENT_RELEASE_LOCK.to_string()],
        _ => Vec::new(),
    }
}

struct TransitionValueInput<'a> {
    state: &'a ServiceFsmState,
    event: &'a ServiceFsmEvent,
    next: &'a ServiceFsmState,
    decision: &'a str,
    shell_intents: &'a [String],
    diagnostics: &'a [String],
}

fn transition_value(input: TransitionValueInput<'_>) -> Result<IoValue> {
    let TransitionValueInput {
        state,
        event,
        next,
        decision,
        shell_intents,
        diagnostics,
    } = input;
    Ok(record("node-control-service-fsm-transition-v1", vec![
        string(SERVICE_FSM_SCHEMA),
        field_string("decision", decision),
        field_string("event", &event.event_name),
        field_string("prior-state", &state.state_ref),
        field_string("prior-state-name", &state.state_name),
        field_string("next-state", &next.state_ref),
        field_string("next-state-name", &next.state_name),
        field_string("startup", event.startup_ref.as_deref().unwrap_or("none")),
        field_string("service-lock", event.service_lock_ref.as_deref().unwrap_or("none")),
        field_string("supervisor-policy", event.supervisor_policy_ref.as_deref().unwrap_or("none")),
        field_sequence("authority", ref_values(&event.authority_refs)?),
        field_sequence("policy", ref_values(&event.policy_refs)?),
        field_sequence("resource", ref_values(&event.resource_refs)?),
        field_sequence("shell-intents", string_values(shell_intents)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[
                "service locks are lifecycle evidence only and do not grant operation authority".to_string(),
            ])?,
        ),
    ]))
}

fn service_state_ref(
    state_name: &str,
    startup_ref: Option<&str>,
    service_lock_ref: Option<&str>,
    heartbeat_count: u64,
    restart_count: u64,
) -> Result<String> {
    let value = record("node-control-service-fsm-state-v1", vec![
        field_string("state", state_name),
        field_string("startup", startup_ref.unwrap_or("none")),
        field_string("service-lock", service_lock_ref.unwrap_or("none")),
        field_string("heartbeat-count", &heartbeat_count.to_string()),
        field_string("restart-count", &restart_count.to_string()),
    ]);
    crate::preserves_rail::canonical_hash(&value)
}

fn validate_state(state: &ServiceFsmState) -> Result<()> {
    validate_state_name(&state.state_name)?;
    validate_ref(&state.state_ref, "service FSM state ref")?;
    validate_optional_ref(&state.startup_ref, "service FSM startup ref")?;
    validate_optional_ref(&state.service_lock_ref, "service FSM lock ref")?;
    validate_optional_ref(&state.supervisor_policy_ref, "service FSM supervisor policy ref")
}

fn validate_event(event: &ServiceFsmEvent) -> Result<()> {
    validate_event_name(&event.event_name)?;
    validate_optional_ref(&event.startup_ref, "service FSM event startup ref")?;
    validate_optional_ref(&event.service_lock_ref, "service FSM event lock ref")?;
    validate_optional_ref(&event.supervisor_policy_ref, "service FSM event supervisor policy ref")?;
    validate_optional_ref(&event.shutdown_ref, "service FSM shutdown ref")?;
    validate_refs(&event.authority_refs, "service FSM authority ref")?;
    validate_refs(&event.policy_refs, "service FSM policy ref")?;
    validate_refs(&event.resource_refs, "service FSM resource ref")
}

fn validate_state_name(name: &str) -> Result<()> {
    match name {
        STATE_UNINITIALIZED
        | STATE_INITIALIZED
        | STATE_STARTUP_LOCKED
        | STATE_SERVICE_LOCK_HELD
        | STATE_SERVING
        | STATE_DRAINING
        | STATE_STOPPED
        | STATE_STALE_LOCK_RECOVERY_PENDING
        | STATE_STALE_LOCK_RECOVERED
        | STATE_FAILED => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported service FSM state {other}"))),
    }
}

fn validate_event_name(name: &str) -> Result<()> {
    match name {
        EVENT_INIT
        | EVENT_STARTUP
        | EVENT_ACQUIRE_SERVICE_LOCK
        | EVENT_SERVE
        | EVENT_HEARTBEAT
        | EVENT_DUPLICATE_RUNNER
        | EVENT_STALE_LOCK_DETECTED
        | EVENT_STALE_LOCK_RECOVER
        | EVENT_RESTART_REQUEST
        | EVENT_SHUTDOWN_REQUESTED
        | EVENT_DRAIN_COMPLETE
        | EVENT_STOP
        | EVENT_FAILURE => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported service FSM event {other}"))),
    }
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(refs.len(), MAX_REFS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: &Option<String>, label: &str) -> Result<()> {
    if let Some(reference) = reference {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} {reference}: {error}")))
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_refs(refs, "service FSM ref")?;
    Ok(refs.iter().map(|reference| string(reference)).collect())
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    crate::bounded::ensure_count_at_most(values.len(), MAX_DIAGNOSTICS, "service FSM string values")?;
    Ok(values.iter().map(|value| string(value)).collect())
}
