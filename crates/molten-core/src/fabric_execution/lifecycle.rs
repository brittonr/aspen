use super::ExecutionIdentity;

const MAX_COMPLETION_ISSUES: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionLifecycleState {
    Admitted,
    Queued,
    Started,
    Exited,
    TimedOut,
    Cancelled,
    FailedBeforeStart,
    FailedAfterStart,
    TeardownIncomplete,
    Unknown,
}

impl ExecutionLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Queued => "queued",
            Self::Started => "started",
            Self::Exited => "exited",
            Self::TimedOut => "timed-out",
            Self::Cancelled => "cancelled",
            Self::FailedBeforeStart => "failed-before-start",
            Self::FailedAfterStart => "failed-after-start",
            Self::TeardownIncomplete => "teardown-incomplete",
            Self::Unknown => "unknown",
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Exited
                | Self::TimedOut
                | Self::Cancelled
                | Self::FailedBeforeStart
                | Self::FailedAfterStart
                | Self::TeardownIncomplete
                | Self::Unknown
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionLifecycleEvent {
    Queue,
    Start,
    Exit,
    Timeout,
    Cancel,
    FailBeforeStart,
    FailAfterStart,
    TeardownIncomplete,
    LoseDefinitiveObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLifecycleTransition {
    pub previous: ExecutionLifecycleState,
    pub event: ExecutionLifecycleEvent,
    pub next: ExecutionLifecycleState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionLifecycleIssue {
    InvalidTransition {
        state: ExecutionLifecycleState,
        event: ExecutionLifecycleEvent,
    },
}

// r[impl molten.fabric_execution.lifecycle]
pub fn plan_execution_lifecycle_transition(
    state: ExecutionLifecycleState,
    event: ExecutionLifecycleEvent,
) -> Result<ExecutionLifecycleTransition, ExecutionLifecycleIssue> {
    let next = match (state, event) {
        (ExecutionLifecycleState::Admitted, ExecutionLifecycleEvent::Queue) => ExecutionLifecycleState::Queued,
        (ExecutionLifecycleState::Admitted | ExecutionLifecycleState::Queued, ExecutionLifecycleEvent::Start) => {
            ExecutionLifecycleState::Started
        }
        (
            ExecutionLifecycleState::Admitted | ExecutionLifecycleState::Queued,
            ExecutionLifecycleEvent::FailBeforeStart,
        ) => ExecutionLifecycleState::FailedBeforeStart,
        (ExecutionLifecycleState::Started, ExecutionLifecycleEvent::Exit) => ExecutionLifecycleState::Exited,
        (ExecutionLifecycleState::Started, ExecutionLifecycleEvent::Timeout) => ExecutionLifecycleState::TimedOut,
        (ExecutionLifecycleState::Started, ExecutionLifecycleEvent::Cancel) => ExecutionLifecycleState::Cancelled,
        (ExecutionLifecycleState::Started, ExecutionLifecycleEvent::FailAfterStart) => {
            ExecutionLifecycleState::FailedAfterStart
        }
        (ExecutionLifecycleState::Started, ExecutionLifecycleEvent::TeardownIncomplete) => {
            ExecutionLifecycleState::TeardownIncomplete
        }
        (
            ExecutionLifecycleState::Started
            | ExecutionLifecycleState::FailedAfterStart
            | ExecutionLifecycleState::TeardownIncomplete,
            ExecutionLifecycleEvent::LoseDefinitiveObservation,
        ) => ExecutionLifecycleState::Unknown,
        _ => return Err(ExecutionLifecycleIssue::InvalidTransition { state, event }),
    };
    Ok(ExecutionLifecycleTransition {
        previous: state,
        event,
        next,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionCompletionIssue {
    StaleGeneration { actual: u64, active: u64 },
    IdentityMismatch(&'static str),
    NonTerminalState(ExecutionLifecycleState),
}

// r[impl molten.fabric_execution.generation]
pub fn admit_execution_completion(
    expected: &ExecutionIdentity,
    observed: &ExecutionIdentity,
    observed_state: ExecutionLifecycleState,
    active_generation: u64,
) -> Result<(), Vec<ExecutionCompletionIssue>> {
    let mut issues = Vec::with_capacity(MAX_COMPLETION_ISSUES);
    if observed.generation != active_generation {
        issues.push(ExecutionCompletionIssue::StaleGeneration {
            actual: observed.generation,
            active: active_generation,
        });
    }
    for (field, matches) in [
        ("extension-id", observed.extension_id == expected.extension_id),
        ("service-id", observed.service_id == expected.service_id),
        ("generation", observed.generation == expected.generation),
        ("callback-ref", observed.callback_ref == expected.callback_ref),
        ("effect-ref", observed.effect_ref == expected.effect_ref),
        ("operation-ref", observed.operation_ref == expected.operation_ref),
        ("executable-identity-ref", observed.executable_identity_ref == expected.executable_identity_ref),
        ("profile-ref", observed.profile_ref == expected.profile_ref),
        ("idempotency-ref", observed.idempotency_ref == expected.idempotency_ref),
    ] {
        if !matches {
            issues.push(ExecutionCompletionIssue::IdentityMismatch(field));
        }
    }
    if !observed_state.is_terminal() {
        issues.push(ExecutionCompletionIssue::NonTerminalState(observed_state));
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecoveryFacts {
    pub identity: ExecutionIdentity,
    pub active_generation: u64,
    pub intent_committed: bool,
    pub start_observed: bool,
    pub terminal_observed: bool,
    pub teardown_observed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionRecoveryDecision {
    NoCommittedIntent,
    DefiniteNotStarted,
    Terminal,
    UnknownRequiresReconciliation,
    Stale,
}

// r[impl molten.fabric_execution.uncertainty]
pub fn classify_execution_recovery(facts: &ExecutionRecoveryFacts) -> ExecutionRecoveryDecision {
    if facts.identity.generation != facts.active_generation {
        return ExecutionRecoveryDecision::Stale;
    }
    if !facts.intent_committed {
        return ExecutionRecoveryDecision::NoCommittedIntent;
    }
    if !facts.start_observed {
        return ExecutionRecoveryDecision::DefiniteNotStarted;
    }
    if facts.terminal_observed && facts.teardown_observed {
        return ExecutionRecoveryDecision::Terminal;
    }
    ExecutionRecoveryDecision::UnknownRequiresReconciliation
}
