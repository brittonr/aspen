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
    let value = transition_value(TransitionValueInput {
        state,
        event,
        next: &next_state,
        decision,
        shell_intents: &shell_intents,
        diagnostics: &diagnostics,
    })?;
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
    push_event_evidence_diagnostics(event, &mut diagnostics);
    diagnostics
}

/// Evidence every event must carry regardless of the state it arrives in.
fn push_event_evidence_diagnostics(event: &ServiceFsmEvent, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if event.event_name != EVENT_INIT && event.startup_ref.is_none() {
        diagnostics.push_item("missing-startup-evidence".to_string());
    }
    if event.duplicate_runner_observed && event.event_name != EVENT_DUPLICATE_RUNNER {
        diagnostics.push_item("duplicate-runner-observed-on-non-duplicate-event".to_string());
    }
    if event.event_name == EVENT_SERVE && event.authority_refs.is_empty() {
        diagnostics.push_item("serve-missing-authority-ref".to_string());
    }
    if event.event_name == EVENT_SERVE && event.policy_refs.is_empty() {
        diagnostics.push_item("serve-missing-policy-ref".to_string());
    }
    if event.event_name == EVENT_SERVE && event.resource_refs.is_empty() {
        diagnostics.push_item("serve-missing-resource-ref".to_string());
    }
}

fn require_startup(event: &ServiceFsmEvent, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if event.startup_ref.is_none() {
        diagnostics.push_item("missing-startup-evidence".to_string());
    }
}

fn require_startup_match(
    state: &ServiceFsmState,
    event: &ServiceFsmEvent,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    require_startup(event, diagnostics);
    if state.startup_ref.as_ref() != event.startup_ref.as_ref() {
        diagnostics.push_item("stale-startup-binding".to_string());
    }
}

fn require_lock_match(
    state: &ServiceFsmState,
    event: &ServiceFsmEvent,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    require_startup_match(state, event, diagnostics);
    if state.service_lock_ref.as_ref() != event.service_lock_ref.as_ref() {
        diagnostics.push_item("service-lock-binding-mismatch".to_string());
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
