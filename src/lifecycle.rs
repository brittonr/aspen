use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::LIFECYCLE_MONITOR_RECEIPT_SCHEMA;
use crate::preserves_rail::LIFECYCLE_SCOPE_CLEANUP_SCHEMA;
use crate::preserves_rail::LIFECYCLE_SERVICE_ASSERTION_SCHEMA;
use crate::preserves_rail::LIFECYCLE_SUPERVISOR_DECISION_SCHEMA;
use crate::preserves_rail::LIFECYCLE_TRACE_EVENT_SCHEMA;
use crate::preserves_rail::LIFECYCLE_TRANSITION_RECEIPT_SCHEMA;
use crate::preserves_rail::LIFECYCLE_TRANSITION_SCHEMA;
use crate::preserves_rail::LIFECYCLE_TURN_FAILURE_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::runtime::PendingTurn;
use crate::runtime::RuntimeScopeCleanup;
use crate::runtime::RuntimeSnapshot;
use crate::runtime::RuntimeValue;
use crate::runtime::TurnAction;

const MAX_LIFECYCLE_REFS: usize = 1024;
const MAX_DIAGNOSTICS: usize = 32;

const _: () = assert!(MAX_LIFECYCLE_REFS <= 100_000);
const _: () = assert!(MAX_DIAGNOSTICS <= 100_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleEntityKind {
    Actor,
    Service,
    Vat,
    Session,
    Handler,
    Job,
}

impl LifecycleEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Service => "service",
            Self::Vat => "vat",
            Self::Session => "session",
            Self::Handler => "handler",
            Self::Job => "job",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleState {
    Declared,
    Spawning,
    Starting,
    Ready,
    Degraded,
    Stopping,
    Stopped,
    Failed,
    Restarting,
    Cleaned,
}

impl LifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Spawning => "spawning",
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Restarting => "restarting",
            Self::Cleaned => "cleaned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleAction {
    Spawn,
    Start,
    Ready,
    Degrade,
    Fail,
    Restart,
    Stop,
    Cleanup,
    SupervisorDecision,
}

impl LifecycleAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spawn => "spawn",
            Self::Start => "start",
            Self::Ready => "ready",
            Self::Degrade => "degrade",
            Self::Fail => "fail",
            Self::Restart => "restart",
            Self::Stop => "stop",
            Self::Cleanup => "cleanup",
            Self::SupervisorDecision => "supervisor-decision",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTransitionInput {
    pub entity_kind: LifecycleEntityKind,
    pub entity_id: String,
    pub from_state: LifecycleState,
    pub to_state: LifecycleState,
    pub action: LifecycleAction,
    pub cause: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub supervisor_ref: Option<String>,
    pub logical_step: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTransitionRecord {
    pub transition_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTransitionReceipt {
    pub receipt_ref: String,
    pub transition_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTraceEvent {
    pub event_ref: String,
    pub transition_ref: String,
    pub entity_kind: LifecycleEntityKind,
    pub entity_id: String,
    pub action: LifecycleAction,
    pub cause: String,
    pub policy_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnFailureKind {
    Panic,
    Denial,
    ValidationFailure,
}

impl TurnFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Panic => "panic",
            Self::Denial => "denial",
            Self::ValidationFailure => "validation-failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFailureInput<'a> {
    pub entity_kind: LifecycleEntityKind,
    pub entity_id: &'a str,
    pub failure_kind: TurnFailureKind,
    pub cause: &'a str,
    pub before: &'a RuntimeSnapshot,
    pub after_rollback: &'a RuntimeSnapshot,
    pub pending_turn: &'a PendingTurn,
    pub vat_delta_refs: &'a [String],
    pub one_shot_effect_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub logical_step: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFailureReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeCleanupInput<'a> {
    pub entity_kind: LifecycleEntityKind,
    pub entity_id: &'a str,
    pub cause: &'a str,
    pub before: &'a RuntimeSnapshot,
    pub after_cleanup: &'a RuntimeSnapshot,
    pub cleanup: &'a RuntimeScopeCleanup,
    pub live_ref_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub logical_step: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeCleanupReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleMonitorInput<'a> {
    pub observer_id: &'a str,
    pub child_id: &'a str,
    pub child_failure_ref: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub logical_step: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleMonitorReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    Never,
    OneForOne,
    Bounded,
}

impl RestartStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::OneForOne => "one-for-one",
            Self::Bounded => "bounded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartWindow {
    pub start_step: u64,
    pub end_step: u64,
    pub max_restarts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorPolicy {
    pub supervisor_id: String,
    pub strategy: RestartStrategy,
    pub restart_window: Option<RestartWindow>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorDecisionInput<'a> {
    pub policy: &'a SupervisorPolicy,
    pub child_id: &'a str,
    pub child_failure_ref: &'a str,
    pub restart_count_in_window: u64,
    pub logical_step: u64,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorDecisionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLifecycleAssertionKind {
    Demand,
    Ready,
    Failure,
    Dependency,
    ExposedRef,
    Restart,
    Stop,
}

impl ServiceLifecycleAssertionKind {
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

pub fn lifecycle_transition_value(input: &LifecycleTransitionInput) -> Result<IOValue> {
    validate_transition_input(input)?;
    Ok(record("lifecycle-transition-v1", vec![
        string(LIFECYCLE_TRANSITION_SCHEMA),
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

pub fn lifecycle_transition_record(input: &LifecycleTransitionInput) -> Result<LifecycleTransitionRecord> {
    let value = lifecycle_transition_value(input)?;
    let transition_ref = canonical_hash(&value)?;
    Ok(LifecycleTransitionRecord { transition_ref, value })
}

pub fn lifecycle_transition_receipt(input: &LifecycleTransitionInput) -> Result<LifecycleTransitionReceipt> {
    let transition = lifecycle_transition_record(input)?;
    let diagnostics = transition_diagnostics(input);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("lifecycle-transition-receipt-v1", vec![
        string(LIFECYCLE_TRANSITION_RECEIPT_SCHEMA),
        record("transition", vec![string(&transition.transition_ref)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(LifecycleTransitionReceipt {
        receipt_ref,
        transition_ref: transition.transition_ref,
        decision: decision.to_owned(),
        diagnostics,
        value,
    })
}

pub fn lifecycle_trace_event(input: &LifecycleTransitionInput) -> Result<LifecycleTraceEvent> {
    let transition = lifecycle_transition_record(input)?;
    let value = record("lifecycle-trace-event-v1", vec![
        string(LIFECYCLE_TRACE_EVENT_SCHEMA),
        record("transition", vec![string(&transition.transition_ref)]),
        record("entity", vec![string(input.entity_kind.as_str()), string(&input.entity_id)]),
        record("action", vec![string(input.action.as_str())]),
        record("cause", vec![string(&input.cause)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        checks_value(),
    ]);
    let event_ref = canonical_hash(&value)?;
    Ok(LifecycleTraceEvent {
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
        string(LIFECYCLE_TURN_FAILURE_SCHEMA),
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
        string(LIFECYCLE_SCOPE_CLEANUP_SCHEMA),
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

pub fn lifecycle_monitor_receipt(input: &LifecycleMonitorInput<'_>) -> Result<LifecycleMonitorReceipt> {
    validate_monitor_input(input)?;
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(1));
    if input.policy_refs.is_empty() {
        diagnostics.push("monitor requires policy ref".to_owned());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("lifecycle-monitor-receipt-v1", vec![
        string(LIFECYCLE_MONITOR_RECEIPT_SCHEMA),
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
    Ok(LifecycleMonitorReceipt {
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
        string(LIFECYCLE_SUPERVISOR_DECISION_SCHEMA),
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
    kind: ServiceLifecycleAssertionKind,
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
        string(LIFECYCLE_SERVICE_ASSERTION_SCHEMA),
        record("service", vec![string(service_id)]),
        record("kind", vec![string(kind.as_str())]),
        record("target", vec![optional_ref_value(target_ref)]),
        record("evidence", vec![refs_sequence(evidence_refs)]),
        checks_value(),
    ]))
}

fn validate_transition_input(input: &LifecycleTransitionInput) -> Result<()> {
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

fn validate_refs(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_LIFECYCLE_REFS {
        return Err(MoltenError::invalid_harness(format!("{label} refs exceed lifecycle bound")));
    }
    let mut prior: Option<&str> = None;
    for reference in refs {
        validate_content_ref(reference)?;
        if let Some(prior_ref) = prior {
            if prior_ref >= reference.as_str() {
                return Err(MoltenError::invalid_harness(format!("{label} refs must be sorted and unique")));
            }
        }
        prior = Some(reference);
    }
    Ok(())
}

fn validate_turn_failure_input(input: &TurnFailureInput<'_>) -> Result<()> {
    if input.entity_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("turn failure entity id must be non-empty"));
    }
    if input.cause.trim().is_empty() {
        return Err(MoltenError::invalid_harness("turn failure cause must be non-empty"));
    }
    validate_refs("vat delta", input.vat_delta_refs)?;
    validate_refs("one-shot effect", input.one_shot_effect_refs)?;
    validate_refs("policy", input.policy_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn validate_scope_cleanup_input(input: &ScopeCleanupInput<'_>) -> Result<()> {
    if input.entity_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("scope cleanup entity id must be non-empty"));
    }
    if input.cause.trim().is_empty() {
        return Err(MoltenError::invalid_harness("scope cleanup cause must be non-empty"));
    }
    validate_refs("assertion", &input.cleanup.assertion_refs)?;
    validate_refs("subscription", &input.cleanup.observer_refs)?;
    validate_refs("message", &input.cleanup.message_refs)?;
    validate_refs("live ref", input.live_ref_refs)?;
    validate_refs("resource", input.resource_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn validate_monitor_input(input: &LifecycleMonitorInput<'_>) -> Result<()> {
    if input.observer_id.trim().is_empty() || input.child_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("monitor observer and child ids must be non-empty"));
    }
    validate_content_ref(input.child_failure_ref)?;
    validate_refs("policy", input.policy_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn validate_supervisor_input(input: &SupervisorDecisionInput<'_>) -> Result<()> {
    if input.policy.supervisor_id.trim().is_empty() || input.child_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("supervisor and child ids must be non-empty"));
    }
    validate_content_ref(input.child_failure_ref)?;
    validate_refs("policy", &input.policy.policy_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    if let Some(window) = input.policy.restart_window.as_ref() {
        if window.end_step < window.start_step {
            return Err(MoltenError::invalid_harness("restart window end precedes start"));
        }
    }
    Ok(())
}

fn restart_window_value(window: Option<&RestartWindow>) -> IOValue {
    match window {
        Some(window) => record("restart-window", vec![
            u64_value(window.start_step),
            u64_value(window.end_step),
            u64_value(window.max_restarts),
        ]),
        None => record("restart-window-none", Vec::new()),
    }
}

fn cleanup_removed_anything(cleanup: &RuntimeScopeCleanup) -> bool {
    !cleanup.assertion_refs.is_empty() || !cleanup.observer_refs.is_empty() || !cleanup.message_refs.is_empty()
}

fn pending_turn_value(turn: &PendingTurn) -> Result<IOValue> {
    let mut actions = Vec::with_capacity(turn.actions.len());
    for action in &turn.actions {
        actions.push(turn_action_value(action));
    }
    Ok(record("runtime-pending-turn-v1", vec![sequence(actions)]))
}

fn pending_action_refs(turn: &PendingTurn) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(turn.actions.len());
    for action in &turn.actions {
        refs.push(canonical_hash(&turn_action_value(action))?);
    }
    refs.sort();
    Ok(refs)
}

fn turn_action_value(action: &TurnAction) -> IOValue {
    match action {
        TurnAction::Send(message) => record("runtime-turn-action-send-v1", vec![message.to_value()]),
        TurnAction::Observe(observer) => record("runtime-turn-action-observe-v1", vec![observer.to_value()]),
        TurnAction::Assert(assertion) => record("runtime-turn-action-assert-v1", vec![assertion.to_value()]),
        TurnAction::Retract(assertion) => record("runtime-turn-action-retract-v1", vec![assertion.to_value()]),
    }
}

fn transition_diagnostics(input: &LifecycleTransitionInput) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(2));
    if !action_matches_target(input.action, input.to_state) {
        diagnostics.push(format!(
            "action {} does not match target state {}",
            input.action.as_str(),
            input.to_state.as_str()
        ));
    }
    if !allowed_transition(input.from_state, input.to_state) {
        diagnostics.push(format!("invalid transition {} -> {}", input.from_state.as_str(), input.to_state.as_str()));
    }
    diagnostics
}

fn action_matches_target(action: LifecycleAction, to_state: LifecycleState) -> bool {
    matches!(
        (action, to_state),
        (LifecycleAction::Spawn, LifecycleState::Spawning)
            | (LifecycleAction::Start, LifecycleState::Starting)
            | (LifecycleAction::Ready, LifecycleState::Ready)
            | (LifecycleAction::Degrade, LifecycleState::Degraded)
            | (LifecycleAction::Fail, LifecycleState::Failed)
            | (LifecycleAction::Restart, LifecycleState::Restarting)
            | (LifecycleAction::Stop, LifecycleState::Stopping | LifecycleState::Stopped)
            | (LifecycleAction::Cleanup, LifecycleState::Cleaned)
            | (LifecycleAction::SupervisorDecision, _)
    )
}

fn allowed_transition(from_state: LifecycleState, to_state: LifecycleState) -> bool {
    matches!(
        (from_state, to_state),
        (LifecycleState::Declared, LifecycleState::Spawning)
            | (LifecycleState::Spawning, LifecycleState::Starting)
            | (LifecycleState::Starting, LifecycleState::Ready)
            | (LifecycleState::Ready, LifecycleState::Degraded)
            | (LifecycleState::Ready, LifecycleState::Stopping)
            | (LifecycleState::Ready, LifecycleState::Failed)
            | (LifecycleState::Degraded, LifecycleState::Ready)
            | (LifecycleState::Degraded, LifecycleState::Stopping)
            | (LifecycleState::Degraded, LifecycleState::Failed)
            | (LifecycleState::Stopping, LifecycleState::Stopped)
            | (LifecycleState::Stopped, LifecycleState::Cleaned)
            | (LifecycleState::Failed, LifecycleState::Restarting)
            | (LifecycleState::Failed, LifecycleState::Cleaned)
            | (LifecycleState::Restarting, LifecycleState::Starting)
            | (LifecycleState::Restarting, LifecycleState::Cleaned)
    )
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |reference| record("some", vec![string(reference)]))
}

fn checks_value() -> IOValue {
    record("checks", vec![
        bool_value(true),
        sequence(vec![
            string("molten-lifecycle-local-semantics"),
            string("no-otp-compatibility-claim"),
            string("canonical-transition-evidence"),
        ]),
    ])
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::LifecycleAction;
    use super::LifecycleEntityKind;
    use super::LifecycleMonitorInput;
    use super::LifecycleState;
    use super::LifecycleTransitionInput;
    use super::RestartStrategy;
    use super::RestartWindow;
    use super::ScopeCleanupInput;
    use super::ServiceLifecycleAssertionKind;
    use super::SupervisorDecisionInput;
    use super::SupervisorPolicy;
    use super::TurnFailureInput;
    use super::TurnFailureKind;
    use super::lifecycle_monitor_receipt;
    use super::lifecycle_trace_event;
    use super::lifecycle_transition_receipt;
    use super::scope_cleanup_receipt;
    use super::service_lifecycle_assertion;
    use super::supervisor_decision_receipt;
    use super::turn_failure_receipt;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::preserves_rail::to_text;
    use crate::runtime::RuntimeState;
    use crate::runtime::RuntimeStep;
    use crate::runtime::RuntimeValue;

    #[test]
    fn failed_turn_rolls_back_pending_actions_and_records_discarded_refs() {
        let state = RuntimeState::new(1);
        let step = RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: RuntimeValue::string("service.ready").expect("runtime value"),
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        let events = state.rollback_turn(turn.clone(), step.primary_actor(), "policy denied");
        let after = state.snapshot();
        let policy_ref = content_ref_from_bytes(b"policy");
        let receipt = turn_failure_receipt(&TurnFailureInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1",
            failure_kind: TurnFailureKind::Denial,
            cause: "policy denied",
            before: &before,
            after_rollback: &after,
            pending_turn: &turn,
            vat_delta_refs: &[],
            one_shot_effect_refs: &[],
            policy_refs: &[policy_ref],
            evidence_refs: &[],
            logical_step: 3,
        })
        .expect("turn failure receipt");
        let rendered = to_text(&receipt.value).expect("render receipt");

        assert_eq!(receipt.decision, "pass");
        assert_eq!(before, after);
        assert!(matches!(events.as_slice(), [crate::runtime::RuntimeEvent::TurnRolledBack { .. }]));
        assert!(rendered.contains("lifecycle-turn-failure-v1"));
        assert!(rendered.contains("runtime-turn-action-assert-v1"));
    }

    #[test]
    fn failed_turn_receipt_denies_if_rollback_mutated_state() {
        let mut state = RuntimeState::new(1);
        let step = RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: RuntimeValue::string("service.ready").expect("runtime value"),
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        state.apply_step(&step);
        let after = state.snapshot();
        let receipt = turn_failure_receipt(&TurnFailureInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1",
            failure_kind: TurnFailureKind::ValidationFailure,
            cause: "validation failed",
            before: &before,
            after_rollback: &after,
            pending_turn: &turn,
            vat_delta_refs: &[],
            one_shot_effect_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
            logical_step: 4,
        })
        .expect("turn failure receipt");

        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.diagnostics, vec!["rollback state differs from before state".to_owned()]);
    }

    #[test]
    fn scope_cleanup_retracts_owned_assertions_subscriptions_and_messages() {
        let mut state = RuntimeState::new(1);
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        state.apply_step(&RuntimeStep::Observe {
            actor: "actor-1".to_owned(),
            pattern: ready.clone(),
        });
        state.apply_step(&RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: ready.clone(),
        });
        state.apply_step(&RuntimeStep::Send {
            from: "actor-1".to_owned(),
            to: "actor-2".to_owned(),
            body: ready,
        });
        let before = state.snapshot();
        let cleanup = state.cleanup_actor_scope("actor-1").expect("cleanup scope");
        let after = state.snapshot();
        let evidence_ref = content_ref_from_bytes(b"cleanup-evidence");
        let receipt = scope_cleanup_receipt(&ScopeCleanupInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1",
            cause: "stop",
            before: &before,
            after_cleanup: &after,
            cleanup: &cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[evidence_ref],
            logical_step: 5,
        })
        .expect("cleanup receipt");
        let rendered = to_text(&receipt.value).expect("render cleanup receipt");

        assert_eq!(receipt.decision, "pass");
        assert_ne!(before, after);
        assert_eq!(after.assertions.len(), 0);
        assert_eq!(after.observers.len(), 0);
        assert_eq!(after.messages.len(), 0);
        assert_eq!(cleanup.assertion_refs.len(), 1);
        assert_eq!(cleanup.observer_refs.len(), 1);
        assert_eq!(cleanup.message_refs.len(), 1);
        assert!(rendered.contains("lifecycle-scope-cleanup-v1"));
    }

    #[test]
    fn scope_cleanup_is_idempotent_and_receipt_backed() {
        let mut state = RuntimeState::new(1);
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        state.apply_step(&RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: ready,
        });
        let before_first = state.snapshot();
        let first_cleanup = state.cleanup_actor_scope("actor-1").expect("first cleanup");
        let after_first = state.snapshot();
        let first_receipt = scope_cleanup_receipt(&ScopeCleanupInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1",
            cause: "cleanup",
            before: &before_first,
            after_cleanup: &after_first,
            cleanup: &first_cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
            logical_step: 6,
        })
        .expect("first cleanup receipt");

        let before_second = state.snapshot();
        let second_cleanup = state.cleanup_actor_scope("actor-1").expect("second cleanup");
        let after_second = state.snapshot();
        let second_receipt = scope_cleanup_receipt(&ScopeCleanupInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1",
            cause: "cleanup",
            before: &before_second,
            after_cleanup: &after_second,
            cleanup: &second_cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
            logical_step: 7,
        })
        .expect("second cleanup receipt");

        assert_eq!(first_receipt.decision, "pass");
        assert_eq!(second_receipt.decision, "pass");
        assert_ne!(before_first, after_first);
        assert_eq!(before_second, after_second);
        assert!(second_cleanup.assertion_refs.is_empty());
        assert!(second_cleanup.observer_refs.is_empty());
        assert!(second_cleanup.message_refs.is_empty());
    }

    #[test]
    fn failed_turn_discloses_one_shot_effects() {
        let state = RuntimeState::new(1);
        let step = RuntimeStep::Clock {
            actor: "actor-1".to_owned(),
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        let after = state.snapshot();
        let effect_refs = vec![content_ref_from_bytes(b"irreversible-effect")];
        let receipt = turn_failure_receipt(&TurnFailureInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1",
            failure_kind: TurnFailureKind::Panic,
            cause: "panic after one-shot effect",
            before: &before,
            after_rollback: &after,
            pending_turn: &turn,
            vat_delta_refs: &[],
            one_shot_effect_refs: &effect_refs,
            policy_refs: &[],
            evidence_refs: &[],
            logical_step: 8,
        })
        .expect("turn failure receipt");
        let rendered = to_text(&receipt.value).expect("render receipt");

        assert_eq!(receipt.decision, "pass");
        assert!(rendered.contains("one-shot-effects"));
        assert!(rendered.contains(&effect_refs[0]));
    }

    #[test]
    fn monitor_observes_failure_without_authority_escalation() {
        let policy_ref = content_ref_from_bytes(b"monitor-policy");
        let failure_ref = content_ref_from_bytes(b"child-failure");
        let receipt = lifecycle_monitor_receipt(&LifecycleMonitorInput {
            observer_id: "monitor-1",
            child_id: "child-1",
            child_failure_ref: &failure_ref,
            policy_refs: &[policy_ref],
            evidence_refs: &[],
            logical_step: 9,
        })
        .expect("monitor receipt");
        let rendered = to_text(&receipt.value).expect("render monitor receipt");

        assert_eq!(receipt.decision, "pass");
        assert!(rendered.contains("authority-escalated #f"));
    }

    #[test]
    fn supervisor_restart_strategies_and_windows_are_deterministic() {
        let policy_ref = content_ref_from_bytes(b"supervisor-policy");
        let failure_ref = content_ref_from_bytes(b"child-failure");
        let one_for_one = SupervisorPolicy {
            supervisor_id: "sup".to_owned(),
            strategy: RestartStrategy::OneForOne,
            restart_window: None,
            policy_refs: vec![policy_ref.clone()],
        };
        let restart = supervisor_decision_receipt(&SupervisorDecisionInput {
            policy: &one_for_one,
            child_id: "child",
            child_failure_ref: &failure_ref,
            restart_count_in_window: 0,
            logical_step: 10,
            evidence_refs: &[],
        })
        .expect("restart decision");
        assert_eq!(restart.decision, "restart");

        let bounded = SupervisorPolicy {
            supervisor_id: "sup".to_owned(),
            strategy: RestartStrategy::Bounded,
            restart_window: Some(RestartWindow {
                start_step: 0,
                end_step: 20,
                max_restarts: 2,
            }),
            policy_refs: vec![policy_ref],
        };
        let denied = supervisor_decision_receipt(&SupervisorDecisionInput {
            policy: &bounded,
            child_id: "child",
            child_failure_ref: &failure_ref,
            restart_count_in_window: 2,
            logical_step: 10,
            evidence_refs: &[],
        })
        .expect("bounded decision");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.diagnostics, vec!["restart budget exhausted".to_owned()]);
    }

    #[test]
    fn service_lifecycle_states_are_dataspace_assertions() {
        let evidence_ref = content_ref_from_bytes(b"readiness-evidence");
        let assertion = service_lifecycle_assertion("service:frontend", ServiceLifecycleAssertionKind::Ready, None, &[
            evidence_ref,
        ])
        .expect("service assertion");
        let mut state = RuntimeState::new(1);
        state.apply_step(&RuntimeStep::Assert {
            actor: "service:frontend".to_owned(),
            value: assertion.clone(),
        });

        assert_eq!(state.snapshot().assertions.len(), 1);
        assert!(
            to_text(assertion.as_iovalue())
                .expect("render assertion")
                .contains("lifecycle-service-assertion-v1")
        );
    }

    #[hegel::test(test_cases = 8)]
    fn hegel_cleanup_idempotence_no_leaks_and_restart_bounds(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(5));
        let actor = format!("actor-{salt}");
        let mut state = RuntimeState::new(1);
        for index in 0..=((salt % 2) as usize) {
            state.apply_step(&RuntimeStep::Assert {
                actor: actor.clone(),
                value: RuntimeValue::string(format!("service.ready.{index}")).expect("runtime value"),
            });
        }
        let _first = state.cleanup_actor_scope(&actor).expect("first cleanup");
        let before_second = state.snapshot();
        let second = state.cleanup_actor_scope(&actor).expect("second cleanup");
        let after_second = state.snapshot();
        assert_eq!(before_second, after_second);
        assert!(second.assertion_refs.is_empty());
        assert!(after_second.assertions.iter().all(|assertion| assertion.actor != actor));

        let policy_ref = content_ref_from_bytes(b"restart-policy");
        let failure_ref = content_ref_from_bytes(format!("failure-{salt}").as_bytes());
        let policy = SupervisorPolicy {
            supervisor_id: "sup".to_owned(),
            strategy: RestartStrategy::Bounded,
            restart_window: Some(RestartWindow {
                start_step: 0,
                end_step: 10,
                max_restarts: 3,
            }),
            policy_refs: vec![policy_ref],
        };
        let receipt = supervisor_decision_receipt(&SupervisorDecisionInput {
            policy: &policy,
            child_id: &actor,
            child_failure_ref: &failure_ref,
            restart_count_in_window: salt,
            logical_step: 5,
            evidence_refs: &[],
        })
        .expect("restart receipt");
        if salt >= 3 {
            assert_eq!(receipt.decision, "deny");
        } else {
            assert_eq!(receipt.decision, "restart");
        }
    }

    #[test]
    fn transition_receipt_passes_valid_spawn() {
        let policy_ref = content_ref_from_bytes(b"policy");
        let evidence_ref = content_ref_from_bytes(b"evidence");
        let input = LifecycleTransitionInput {
            entity_kind: LifecycleEntityKind::Actor,
            entity_id: "actor-1".to_owned(),
            from_state: LifecycleState::Declared,
            to_state: LifecycleState::Spawning,
            action: LifecycleAction::Spawn,
            cause: "operator-request".to_owned(),
            policy_refs: vec![policy_ref],
            resource_refs: Vec::new(),
            evidence_refs: vec![evidence_ref],
            supervisor_ref: None,
            logical_step: 1,
        };

        let receipt = lifecycle_transition_receipt(&input).expect("receipt");

        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert!(receipt.receipt_ref.starts_with("blake3:"));
    }

    #[test]
    fn transition_receipt_denies_impossible_jump() {
        let input = LifecycleTransitionInput {
            entity_kind: LifecycleEntityKind::Service,
            entity_id: "svc".to_owned(),
            from_state: LifecycleState::Declared,
            to_state: LifecycleState::Ready,
            action: LifecycleAction::Ready,
            cause: "bad-adapter".to_owned(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: 2,
        };

        let receipt = lifecycle_transition_receipt(&input).expect("receipt");

        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.diagnostics, vec!["invalid transition declared -> ready".to_owned()]);
    }

    #[test]
    fn trace_event_binds_transition_cause_and_policy() {
        let policy_ref = content_ref_from_bytes(b"policy-a");
        let input = LifecycleTransitionInput {
            entity_kind: LifecycleEntityKind::Job,
            entity_id: "job-7".to_owned(),
            from_state: LifecycleState::Ready,
            to_state: LifecycleState::Failed,
            action: LifecycleAction::Fail,
            cause: "stage-denied".to_owned(),
            policy_refs: vec![policy_ref],
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: 9,
        };

        let event = lifecycle_trace_event(&input).expect("trace event");
        let rendered = to_text(&event.value).expect("render event");

        assert!(event.event_ref.starts_with("blake3:"));
        assert!(rendered.contains("lifecycle-trace-event-v1"));
        assert!(rendered.contains("stage-denied"));
    }

    #[test]
    fn refs_must_be_sorted_and_canonical() {
        let mut refs = vec![content_ref_from_bytes(b"z"), content_ref_from_bytes(b"a")];
        refs.sort();
        refs.reverse();
        let input = LifecycleTransitionInput {
            entity_kind: LifecycleEntityKind::Vat,
            entity_id: "vat".to_owned(),
            from_state: LifecycleState::Declared,
            to_state: LifecycleState::Spawning,
            action: LifecycleAction::Spawn,
            cause: "test".to_owned(),
            policy_refs: refs,
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: 0,
        };

        let error = lifecycle_transition_receipt(&input).expect_err("unsorted refs fail");
        assert!(error.to_string().contains("policy refs must be sorted and unique"));
    }
}
