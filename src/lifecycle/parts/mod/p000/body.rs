type IoValue = preserves::IOValue;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
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

fn require_ref(reference: &str, field: &str) -> Result<()> {
    if reference.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    validate_content_ref(reference).map_err(|_| {
        MoltenError::invalid_harness(format!("{field} must be a valid content ref, got {reference}"))
    })
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    Ok(())
}

const MAX_REFS: usize = 1024;
const MAX_DIAGNOSTICS: usize = 32;
const SERVICE_READINESS_BASE_REF_COUNT: usize = 2;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTransition {
    pub from_state: State,
    pub to_state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionTarget {
    pub action: Action,
    pub to_state: State,
}

pub const LIFECYCLE_STATE_COUNT: usize = 10;
pub const LIFECYCLE_ACTION_COUNT: usize = 9;
pub const LIFECYCLE_TRANSITION_COUNT: usize = 15;
pub const LIFECYCLE_ACTION_TARGET_COUNT: usize = 9;

pub const LIFECYCLE_STATES: [State; LIFECYCLE_STATE_COUNT] = [
    State::Declared,
    State::Spawning,
    State::Starting,
    State::Ready,
    State::Degraded,
    State::Stopping,
    State::Stopped,
    State::Failed,
    State::Restarting,
    State::Cleaned,
];

pub const LIFECYCLE_ACTIONS: [Action; LIFECYCLE_ACTION_COUNT] = [
    Action::Spawn,
    Action::Start,
    Action::Ready,
    Action::Degrade,
    Action::Fail,
    Action::Restart,
    Action::Stop,
    Action::Cleanup,
    Action::SupervisorDecision,
];

pub const LIFECYCLE_TRANSITIONS: [StateTransition; LIFECYCLE_TRANSITION_COUNT] = [
    StateTransition {
        from_state: State::Declared,
        to_state: State::Spawning,
    },
    StateTransition {
        from_state: State::Spawning,
        to_state: State::Starting,
    },
    StateTransition {
        from_state: State::Starting,
        to_state: State::Ready,
    },
    StateTransition {
        from_state: State::Ready,
        to_state: State::Degraded,
    },
    StateTransition {
        from_state: State::Ready,
        to_state: State::Stopping,
    },
    StateTransition {
        from_state: State::Ready,
        to_state: State::Failed,
    },
    StateTransition {
        from_state: State::Degraded,
        to_state: State::Ready,
    },
    StateTransition {
        from_state: State::Degraded,
        to_state: State::Stopping,
    },
    StateTransition {
        from_state: State::Degraded,
        to_state: State::Failed,
    },
    StateTransition {
        from_state: State::Stopping,
        to_state: State::Stopped,
    },
    StateTransition {
        from_state: State::Stopped,
        to_state: State::Cleaned,
    },
    StateTransition {
        from_state: State::Failed,
        to_state: State::Restarting,
    },
    StateTransition {
        from_state: State::Failed,
        to_state: State::Cleaned,
    },
    StateTransition {
        from_state: State::Restarting,
        to_state: State::Starting,
    },
    StateTransition {
        from_state: State::Restarting,
        to_state: State::Cleaned,
    },
];

pub const LIFECYCLE_ACTION_TARGETS: [ActionTarget; LIFECYCLE_ACTION_TARGET_COUNT] = [
    ActionTarget {
        action: Action::Spawn,
        to_state: State::Spawning,
    },
    ActionTarget {
        action: Action::Start,
        to_state: State::Starting,
    },
    ActionTarget {
        action: Action::Ready,
        to_state: State::Ready,
    },
    ActionTarget {
        action: Action::Degrade,
        to_state: State::Degraded,
    },
    ActionTarget {
        action: Action::Fail,
        to_state: State::Failed,
    },
    ActionTarget {
        action: Action::Restart,
        to_state: State::Restarting,
    },
    ActionTarget {
        action: Action::Stop,
        to_state: State::Stopping,
    },
    ActionTarget {
        action: Action::Stop,
        to_state: State::Stopped,
    },
    ActionTarget {
        action: Action::Cleanup,
        to_state: State::Cleaned,
    },
];
