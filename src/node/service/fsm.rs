type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;

const SERVICE_FSM_SCHEMA: &str = "molten.node.service-fsm-transition.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const STATE_UNINITIALIZED: &str = "uninitialized";
const STATE_INITIALIZED: &str = "initialized";
const STATE_STARTUP_LOCKED: &str = "startup-locked";
const STATE_SERVICE_LOCK_HELD: &str = "service-lock-held";
const STATE_SERVING: &str = "serving";
const STATE_DRAINING: &str = "draining";
const STATE_STOPPED: &str = "stopped";
const STATE_STALE_LOCK_RECOVERY_PENDING: &str = "stale-lock-recovery-pending";
const STATE_STALE_LOCK_RECOVERED: &str = "stale-lock-recovered";
const STATE_FAILED: &str = "failed";
const EVENT_INIT: &str = "init";
const EVENT_STARTUP: &str = "startup";
const EVENT_ACQUIRE_SERVICE_LOCK: &str = "acquire-service-lock";
const EVENT_SERVE: &str = "serve";
const EVENT_HEARTBEAT: &str = "heartbeat";
const EVENT_DUPLICATE_RUNNER: &str = "duplicate-runner-observed";
const EVENT_STALE_LOCK_DETECTED: &str = "stale-lock-detected";
const EVENT_STALE_LOCK_RECOVER: &str = "stale-lock-recover";
const EVENT_RESTART_REQUEST: &str = "supervisor-restart-request";
const EVENT_SHUTDOWN_REQUESTED: &str = "shutdown-requested";
const EVENT_DRAIN_COMPLETE: &str = "drain-complete";
const EVENT_STOP: &str = "stop";
const EVENT_FAILURE: &str = "failure";
const INTENT_ACQUIRE_LOCK: &str = "acquire-service-lock";
const INTENT_RELEASE_LOCK: &str = "release-service-lock";
const INTENT_WRITE_HEARTBEAT: &str = "write-heartbeat";
const INTENT_SCAN_INGRESS: &str = "scan-ingress";
const INTENT_DRAIN_INBOX: &str = "drain-inbox";
const INTENT_WRITE_SHUTDOWN: &str = "write-shutdown";
const MAX_DIAGNOSTICS: usize = 256;
const MAX_REFS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFsmState {
    pub state_name: String,
    pub state_ref: String,
    pub startup_ref: Option<String>,
    pub service_lock_ref: Option<String>,
    pub supervisor_policy_ref: Option<String>,
    pub heartbeat_count: u64,
    pub restart_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFsmEvent {
    pub event_name: String,
    pub startup_ref: Option<String>,
    pub service_lock_ref: Option<String>,
    pub supervisor_policy_ref: Option<String>,
    pub heartbeat_tick: u64,
    pub max_heartbeat_gap: u64,
    pub pending_inbox: u64,
    pub drain_bound: u64,
    pub max_restarts: u64,
    pub stale_lock_observed: bool,
    pub duplicate_runner_observed: bool,
    pub shutdown_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFsmTransition {
    pub decision: String,
    pub prior_state_ref: String,
    pub next_state_ref: String,
    pub next_state: ServiceFsmState,
    pub shell_intents: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub transition_ref: String,
}

// r[impl molten.node_runtime.service_fsm_model]
// r[impl molten.node_runtime.service_fsm_receipts]
// r[impl molten.node_runtime.service_fsm_lock_recovery]
pub fn evaluate_service_transition(state: &ServiceFsmState, event: &ServiceFsmEvent) -> Result<ServiceFsmTransition> {
    validate_state(state)?;
    validate_event(event)?;
    let mut diagnostics = transition_diagnostics(state, event);
    diagnostics.sort();
    diagnostics.dedup();
    crate::bounded::ensure_count_at_most(diagnostics.len(), MAX_DIAGNOSTICS, "service FSM diagnostics")?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let next_state = if decision == DECISION_PASS {
        next_state(state, event)?
    } else {
        state.clone()
    };
    let shell_intents = if decision == DECISION_PASS {
        shell_intents(event)
    } else {
        Vec::new()
    };
    let value = transition_value(state, event, &next_state, decision, &shell_intents, &diagnostics)?;
    let transition_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ServiceFsmTransition {
        decision: decision.to_string(),
        prior_state_ref: state.state_ref.clone(),
        next_state_ref: next_state.state_ref.clone(),
        next_state,
        shell_intents,
        diagnostics,
        value,
        transition_ref,
    })
}

fn transition_diagnostics(state: &ServiceFsmState, event: &ServiceFsmEvent) -> Vec<String> {
    let mut diagnostics = Vec::new();
    match (state.state_name.as_str(), event.event_name.as_str()) {
        (STATE_UNINITIALIZED, EVENT_INIT) => {}
        (STATE_INITIALIZED, EVENT_STARTUP) => require_startup(event, &mut diagnostics),
        (STATE_STARTUP_LOCKED, EVENT_ACQUIRE_SERVICE_LOCK) => {
            require_startup_match(state, event, &mut diagnostics);
            if event.service_lock_ref.is_none() {
                diagnostics.push("missing-service-lock-ref".to_string());
            }
        }
        (STATE_SERVICE_LOCK_HELD, EVENT_SERVE) => require_lock_match(state, event, &mut diagnostics),
        (STATE_SERVING, EVENT_HEARTBEAT) => {
            require_lock_match(state, event, &mut diagnostics);
            if event.heartbeat_tick > state.heartbeat_count.saturating_add(event.max_heartbeat_gap) {
                diagnostics.push("heartbeat-timeout".to_string());
            }
        }
        (STATE_SERVING, EVENT_SHUTDOWN_REQUESTED) => {
            require_lock_match(state, event, &mut diagnostics);
            if event.shutdown_ref.is_none() {
                diagnostics.push("missing-shutdown-ref".to_string());
            }
        }
        (STATE_DRAINING, EVENT_DRAIN_COMPLETE) => {
            require_lock_match(state, event, &mut diagnostics);
            if event.pending_inbox > event.drain_bound {
                diagnostics.push("shutdown-drain-bound-exceeded".to_string());
            }
        }
        (STATE_STOPPED, EVENT_STOP) => {}
        (STATE_SERVING, EVENT_DUPLICATE_RUNNER) => diagnostics.push("duplicate-runner-preserves-state".to_string()),
        (STATE_SERVING, EVENT_STALE_LOCK_DETECTED) => {
            if !event.stale_lock_observed {
                diagnostics.push("stale-lock-not-observed".to_string());
            }
        }
        (STATE_STALE_LOCK_RECOVERY_PENDING, EVENT_STALE_LOCK_RECOVER) => {
            if event.supervisor_policy_ref.is_none() {
                diagnostics.push("stale-lock-recovery-missing-policy".to_string());
            }
            require_startup_match(state, event, &mut diagnostics);
        }
        (STATE_SERVING, EVENT_RESTART_REQUEST) => {
            if event.supervisor_policy_ref.is_none() {
                diagnostics.push("restart-missing-supervisor-policy".to_string());
            }
            if state.restart_count >= event.max_restarts {
                diagnostics.push("restart-bound-exhausted".to_string());
            }
        }
        (_, EVENT_FAILURE) => {}
        _ => diagnostics.push(format!("illegal-service-transition:{}->{}", state.state_name, event.event_name)),
    }
    if event.event_name != EVENT_INIT && event.startup_ref.is_none() {
        diagnostics.push("missing-startup-evidence".to_string());
    }
    if event.duplicate_runner_observed && event.event_name != EVENT_DUPLICATE_RUNNER {
        diagnostics.push("duplicate-runner-observed-on-non-duplicate-event".to_string());
    }
    if event.event_name == EVENT_SERVE && event.authority_refs.is_empty() {
        diagnostics.push("serve-missing-authority-ref".to_string());
    }
    if event.event_name == EVENT_SERVE && event.policy_refs.is_empty() {
        diagnostics.push("serve-missing-policy-ref".to_string());
    }
    if event.event_name == EVENT_SERVE && event.resource_refs.is_empty() {
        diagnostics.push("serve-missing-resource-ref".to_string());
    }
    diagnostics
}

fn require_startup(event: &ServiceFsmEvent, diagnostics: &mut Vec<String>) {
    if event.startup_ref.is_none() {
        diagnostics.push("missing-startup-evidence".to_string());
    }
}

fn require_startup_match(state: &ServiceFsmState, event: &ServiceFsmEvent, diagnostics: &mut Vec<String>) {
    require_startup(event, diagnostics);
    if state.startup_ref.as_ref() != event.startup_ref.as_ref() {
        diagnostics.push("stale-startup-binding".to_string());
    }
}

fn require_lock_match(state: &ServiceFsmState, event: &ServiceFsmEvent, diagnostics: &mut Vec<String>) {
    require_startup_match(state, event, diagnostics);
    if state.service_lock_ref.as_ref() != event.service_lock_ref.as_ref() {
        diagnostics.push("service-lock-binding-mismatch".to_string());
    }
}

fn next_state(state: &ServiceFsmState, event: &ServiceFsmEvent) -> Result<ServiceFsmState> {
    let state_name = match event.event_name.as_str() {
        EVENT_INIT => STATE_INITIALIZED,
        EVENT_STARTUP => STATE_STARTUP_LOCKED,
        EVENT_ACQUIRE_SERVICE_LOCK => STATE_SERVICE_LOCK_HELD,
        EVENT_SERVE => STATE_SERVING,
        EVENT_HEARTBEAT => STATE_SERVING,
        EVENT_STALE_LOCK_DETECTED => STATE_STALE_LOCK_RECOVERY_PENDING,
        EVENT_STALE_LOCK_RECOVER => STATE_STALE_LOCK_RECOVERED,
        EVENT_RESTART_REQUEST => STATE_SERVICE_LOCK_HELD,
        EVENT_SHUTDOWN_REQUESTED => STATE_DRAINING,
        EVENT_DRAIN_COMPLETE => STATE_STOPPED,
        EVENT_STOP => STATE_STOPPED,
        EVENT_FAILURE => STATE_FAILED,
        _ => state.state_name.as_str(),
    };
    let startup_ref = event.startup_ref.clone().or_else(|| state.startup_ref.clone());
    let service_lock_ref = match event.event_name.as_str() {
        EVENT_DRAIN_COMPLETE | EVENT_STOP => None,
        _ => event.service_lock_ref.clone().or_else(|| state.service_lock_ref.clone()),
    };
    let supervisor_policy_ref = event.supervisor_policy_ref.clone().or_else(|| state.supervisor_policy_ref.clone());
    let heartbeat_count = if event.event_name == EVENT_HEARTBEAT {
        event.heartbeat_tick
    } else {
        state.heartbeat_count
    };
    let restart_count = if event.event_name == EVENT_RESTART_REQUEST {
        state.restart_count.saturating_add(1)
    } else {
        state.restart_count
    };
    let state_ref = service_state_ref(
        state_name,
        startup_ref.as_deref(),
        service_lock_ref.as_deref(),
        heartbeat_count,
        restart_count,
    )?;
    Ok(ServiceFsmState {
        state_name: state_name.to_string(),
        state_ref,
        startup_ref,
        service_lock_ref,
        supervisor_policy_ref,
        heartbeat_count,
        restart_count,
    })
}

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

fn transition_value(
    state: &ServiceFsmState,
    event: &ServiceFsmEvent,
    next: &ServiceFsmState,
    decision: &str,
    shell_intents: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn state(name: &str, startup: Option<String>, lock: Option<String>) -> ServiceFsmState {
        let state_ref = service_state_ref(name, startup.as_deref(), lock.as_deref(), 0, 0).expect("state ref");
        ServiceFsmState {
            state_name: name.to_string(),
            state_ref,
            startup_ref: startup,
            service_lock_ref: lock,
            supervisor_policy_ref: None,
            heartbeat_count: 0,
            restart_count: 0,
        }
    }

    fn event(name: &str, startup: Option<String>, lock: Option<String>) -> ServiceFsmEvent {
        ServiceFsmEvent {
            event_name: name.to_string(),
            startup_ref: startup,
            service_lock_ref: lock,
            supervisor_policy_ref: Some(local_ref("supervisor-policy")),
            heartbeat_tick: 1,
            max_heartbeat_gap: 2,
            pending_inbox: 0,
            drain_bound: 1,
            max_restarts: 1,
            stale_lock_observed: false,
            duplicate_runner_observed: false,
            shutdown_ref: Some(local_ref("shutdown")),
            authority_refs: refs("authority"),
            policy_refs: refs("policy"),
            resource_refs: refs("resource"),
        }
    }

    // r[verify molten.node_runtime.service_fsm_model]
    // r[verify molten.node_runtime.service_fsm_receipts]
    // r[verify molten.node_runtime.service_fsm_tests]
    #[test]
    fn normal_service_trace_reaches_serving_then_stopped() {
        let startup = local_ref("startup");
        let lock = local_ref("lock");
        let initialized = state(STATE_INITIALIZED, None, None);
        let startup_transition =
            evaluate_service_transition(&initialized, &event(EVENT_STARTUP, Some(startup.clone()), None))
                .expect("startup transition");
        assert_eq!(startup_transition.decision, DECISION_PASS);
        let lock_transition = evaluate_service_transition(
            &startup_transition.next_state,
            &event(EVENT_ACQUIRE_SERVICE_LOCK, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("lock transition");
        assert_eq!(lock_transition.next_state.state_name, STATE_SERVICE_LOCK_HELD);
        assert!(lock_transition.shell_intents.contains(&INTENT_ACQUIRE_LOCK.to_string()));
        let serve = evaluate_service_transition(
            &lock_transition.next_state,
            &event(EVENT_SERVE, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("serve transition");
        assert_eq!(serve.next_state.state_name, STATE_SERVING);
        let shutdown = evaluate_service_transition(
            &serve.next_state,
            &event(EVENT_SHUTDOWN_REQUESTED, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("shutdown transition");
        assert_eq!(shutdown.next_state.state_name, STATE_DRAINING);
        let drained =
            evaluate_service_transition(&shutdown.next_state, &event(EVENT_DRAIN_COMPLETE, Some(startup), Some(lock)))
                .expect("drain transition");
        assert_eq!(drained.next_state.state_name, STATE_STOPPED);
    }

    #[test]
    fn serve_without_startup_denies_without_intents() {
        let state = state(STATE_SERVICE_LOCK_HELD, None, Some(local_ref("lock")));
        let denied = evaluate_service_transition(&state, &event(EVENT_SERVE, None, Some(local_ref("lock"))))
            .expect("denied serve");
        assert_eq!(denied.decision, DECISION_DENY);
        assert!(denied.shell_intents.is_empty());
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic == "missing-startup-evidence"));
    }

    // r[verify molten.node_runtime.service_fsm_lock_recovery]
    #[test]
    fn duplicate_runner_stale_lock_restart_and_drain_denials_preserve_state() {
        let startup = local_ref("startup");
        let lock = local_ref("lock");
        let serving = state(STATE_SERVING, Some(startup.clone()), Some(lock.clone()));
        let duplicate = evaluate_service_transition(
            &serving,
            &event(EVENT_DUPLICATE_RUNNER, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("duplicate");
        assert_eq!(duplicate.decision, DECISION_DENY);
        assert_eq!(duplicate.next_state_ref, serving.state_ref);

        let pending = state(STATE_STALE_LOCK_RECOVERY_PENDING, Some(startup.clone()), Some(lock.clone()));
        let mut stale_recover = event(EVENT_STALE_LOCK_RECOVER, Some(startup.clone()), Some(lock.clone()));
        stale_recover.supervisor_policy_ref = None;
        let stale = evaluate_service_transition(&pending, &stale_recover).expect("stale recovery");
        assert_eq!(stale.decision, DECISION_DENY);
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic == "stale-lock-recovery-missing-policy"));

        let mut restart_state = serving.clone();
        restart_state.restart_count = 1;
        let restart = evaluate_service_transition(
            &restart_state,
            &event(EVENT_RESTART_REQUEST, Some(startup.clone()), Some(lock.clone())),
        )
        .expect("restart");
        assert!(restart.diagnostics.iter().any(|diagnostic| diagnostic == "restart-bound-exhausted"));

        let draining = state(STATE_DRAINING, Some(startup.clone()), Some(lock.clone()));
        let mut drain = event(EVENT_DRAIN_COMPLETE, Some(startup), Some(lock));
        drain.pending_inbox = 2;
        drain.drain_bound = 1;
        let drain = evaluate_service_transition(&draining, &drain).expect("drain");
        assert_eq!(drain.decision, DECISION_DENY);
        assert!(drain.diagnostics.iter().any(|diagnostic| diagnostic == "shutdown-drain-bound-exceeded"));
    }
}
