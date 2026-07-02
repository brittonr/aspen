type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type PendingTurn = crate::runtime::PendingTurn;
type RuntimeScopeCleanup = crate::runtime::RuntimeScopeCleanup;
type RuntimeSnapshot = crate::runtime::RuntimeSnapshot;
type RuntimeValue = crate::runtime::RuntimeValue;
type TurnAction = crate::runtime::TurnAction;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
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

const MAX_REFS: usize = 1024;
const MAX_DIAGNOSTICS: usize = 32;

const _: () = assert!(MAX_REFS <= 100_000);
const _: () = assert!(MAX_DIAGNOSTICS <= 100_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    Actor,
    Service,
    Vat,
    Session,
    Handler,
    Job,
}

impl EntityKind {
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
pub enum State {
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

impl State {
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
pub enum Action {
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

impl Action {
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
