
pub fn lifecycle_states() -> &'static [State] {
    &LIFECYCLE_STATES
}

pub fn lifecycle_actions() -> &'static [Action] {
    &LIFECYCLE_ACTIONS
}

pub fn allowed_transition_relation() -> &'static [StateTransition] {
    &LIFECYCLE_TRANSITIONS
}

pub fn action_target_relation() -> &'static [ActionTarget] {
    &LIFECYCLE_ACTION_TARGETS
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionInput {
    pub entity_kind: EntityKind,
    pub entity_id: String,
    pub from_state: State,
    pub to_state: State,
    pub action: Action,
    pub cause: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub supervisor_ref: Option<String>,
    pub logical_step: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionRecord {
    pub transition_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionReceipt {
    pub receipt_ref: String,
    pub transition_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionReceiptValidation {
    pub receipt_ref: String,
    pub transition_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub event_ref: String,
    pub transition_ref: String,
    pub entity_kind: EntityKind,
    pub entity_id: String,
    pub action: Action,
    pub cause: String,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
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
    pub entity_kind: EntityKind,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeCleanupInput<'a> {
    pub entity_kind: EntityKind,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInput<'a> {
    pub observer_id: &'a str,
    pub child_id: &'a str,
    pub child_failure_ref: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub logical_step: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceDemandEvaluationInput<'a> {
    pub service_id: &'a str,
    pub demand_ref: &'a str,
    pub manifest_ref: &'a str,
    pub required_dependency_refs: &'a [String],
    pub ready_dependency_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDemandEvaluation {
    pub decision: String,
    pub lifecycle_kind: String,
    pub diagnostics: Vec<String>,
    pub start_side_effect_admitted: bool,
    pub readiness_assertion: Option<RuntimeValue>,
}
