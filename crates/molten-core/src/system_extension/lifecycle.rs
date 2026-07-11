use super::FailureClass;
use super::HealthState;
use super::INITIAL_SYSTEM_EXTENSION_GENERATION;
use super::ResourceUsage;
use super::SupervisionDecision;
use super::plan_supervision;
use super::valid_ref;

const GENERATION_INCREMENT: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    Absent,
    Installed,
    Admitted,
    Initializing,
    Initialized,
    Starting,
    Running,
    Checkpointing,
    Recovering,
    Draining,
    Drained,
    Failed,
    Restarting,
    Upgrading,
    RollingBack,
    ShuttingDown,
    Quarantined,
    Stopped,
    Removed,
}

impl LifecyclePhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Installed => "installed",
            Self::Admitted => "admitted",
            Self::Initializing => "initializing",
            Self::Initialized => "initialized",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Checkpointing => "checkpointing",
            Self::Recovering => "recovering",
            Self::Draining => "draining",
            Self::Drained => "drained",
            Self::Failed => "failed",
            Self::Restarting => "restarting",
            Self::Upgrading => "upgrading",
            Self::RollingBack => "rolling-back",
            Self::ShuttingDown => "shutting-down",
            Self::Quarantined => "quarantined",
            Self::Stopped => "stopped",
            Self::Removed => "removed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleState {
    pub generation: u64,
    pub phase: LifecyclePhase,
    pub restart_attempts: u64,
    pub health: HealthState,
    pub checkpoint_ref: Option<String>,
}

impl LifecycleState {
    pub const fn absent() -> Self {
        Self {
            generation: 0,
            phase: LifecyclePhase::Absent,
            restart_attempts: 0,
            health: HealthState::Unknown,
            checkpoint_ref: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEventKind {
    Install,
    Admit,
    BeginInitialize,
    InitializeSucceeded,
    BeginStart,
    StartSucceeded,
    BeginCheckpoint,
    CheckpointSucceeded,
    CheckpointFailed,
    BeginRecovery,
    RecoverySucceeded,
    RecoveryFailed,
    BeginDrain,
    DrainSucceeded,
    Failure,
    BeginRestart,
    BeginUpgrade,
    UpgradeSucceeded,
    UpgradeFailed,
    BeginRollback,
    RollbackSucceeded,
    RollbackFailed,
    BeginShutdown,
    ShutdownSucceeded,
    Remove,
}

impl LifecycleEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Admit => "admit",
            Self::BeginInitialize => "begin-initialize",
            Self::InitializeSucceeded => "initialize-succeeded",
            Self::BeginStart => "begin-start",
            Self::StartSucceeded => "start-succeeded",
            Self::BeginCheckpoint => "begin-checkpoint",
            Self::CheckpointSucceeded => "checkpoint-succeeded",
            Self::CheckpointFailed => "checkpoint-failed",
            Self::BeginRecovery => "begin-recovery",
            Self::RecoverySucceeded => "recovery-succeeded",
            Self::RecoveryFailed => "recovery-failed",
            Self::BeginDrain => "begin-drain",
            Self::DrainSucceeded => "drain-succeeded",
            Self::Failure => "failure",
            Self::BeginRestart => "begin-restart",
            Self::BeginUpgrade => "begin-upgrade",
            Self::UpgradeSucceeded => "upgrade-succeeded",
            Self::UpgradeFailed => "upgrade-failed",
            Self::BeginRollback => "begin-rollback",
            Self::RollbackSucceeded => "rollback-succeeded",
            Self::RollbackFailed => "rollback-failed",
            Self::BeginShutdown => "begin-shutdown",
            Self::ShutdownSucceeded => "shutdown-succeeded",
            Self::Remove => "remove",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub kind: LifecycleEventKind,
    pub generation: u64,
    pub next_generation: Option<u64>,
    pub checkpoint_ref: Option<String>,
    pub failure_class: Option<FailureClass>,
}

impl LifecycleEvent {
    pub const fn simple(kind: LifecycleEventKind, generation: u64) -> Self {
        Self {
            kind,
            generation,
            next_generation: None,
            checkpoint_ref: None,
            failure_class: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleIssue {
    StaleGeneration {
        actual: u64,
        active: u64,
    },
    InitialGenerationMismatch {
        actual: u64,
        expected: u64,
    },
    MissingNextGeneration,
    UnexpectedNextGeneration(u64),
    GenerationOverflow,
    NonSequentialGeneration {
        actual: u64,
        expected: u64,
    },
    IllegalTransition {
        phase: LifecyclePhase,
        event: LifecycleEventKind,
    },
    MissingFailureClass,
    UnexpectedFailureClass(FailureClass),
    MissingCheckpointRef,
    MalformedCheckpointRef(String),
    UnexpectedCheckpointRef(String),
    RestartBudgetExhausted,
    ResourcesNotDrained(ResourceUsage),
}

// r[impl molten.system_extension.lifecycle]
pub fn plan_lifecycle_transition(
    state: &LifecycleState,
    event: &LifecycleEvent,
    usage: ResourceUsage,
    max_restart_attempts: u64,
) -> Result<LifecycleState, Vec<LifecycleIssue>> {
    let mut issues = validate_event_shape(state, event);
    if !issues.is_empty() {
        return Err(issues);
    }

    let next = match (state.phase, event.kind) {
        (LifecyclePhase::Absent, LifecycleEventKind::Install) => LifecycleState {
            generation: event.generation,
            phase: LifecyclePhase::Installed,
            restart_attempts: 0,
            health: HealthState::Stopped,
            checkpoint_ref: None,
        },
        (LifecyclePhase::Installed, LifecycleEventKind::Admit) => with_phase(state, LifecyclePhase::Admitted),
        (LifecyclePhase::Admitted, LifecycleEventKind::BeginInitialize) => LifecycleState {
            health: HealthState::Starting,
            phase: LifecyclePhase::Initializing,
            ..state.clone()
        },
        (LifecyclePhase::Initializing, LifecycleEventKind::InitializeSucceeded) => {
            with_phase(state, LifecyclePhase::Initialized)
        }
        (LifecyclePhase::Initialized | LifecyclePhase::Restarting, LifecycleEventKind::BeginStart) => LifecycleState {
            phase: LifecyclePhase::Starting,
            health: HealthState::Starting,
            ..state.clone()
        },
        (LifecyclePhase::Starting, LifecycleEventKind::StartSucceeded) => LifecycleState {
            phase: LifecyclePhase::Running,
            health: HealthState::Healthy,
            ..state.clone()
        },
        (LifecyclePhase::Running, LifecycleEventKind::BeginCheckpoint) => {
            with_phase(state, LifecyclePhase::Checkpointing)
        }
        (LifecyclePhase::Checkpointing, LifecycleEventKind::CheckpointSucceeded) => LifecycleState {
            phase: LifecyclePhase::Running,
            checkpoint_ref: event.checkpoint_ref.clone(),
            ..state.clone()
        },
        (LifecyclePhase::Checkpointing, LifecycleEventKind::CheckpointFailed)
        | (LifecyclePhase::Recovering, LifecycleEventKind::RecoveryFailed)
        | (LifecyclePhase::Upgrading, LifecycleEventKind::UpgradeFailed)
        | (LifecyclePhase::RollingBack, LifecycleEventKind::RollbackFailed) => failed_state(
            state,
            event.failure_class.ok_or_else(|| vec![LifecycleIssue::MissingFailureClass])?,
            max_restart_attempts,
        ),
        (
            LifecyclePhase::Admitted | LifecyclePhase::Failed | LifecyclePhase::Restarting,
            LifecycleEventKind::BeginRecovery,
        ) => with_phase(state, LifecyclePhase::Recovering),
        (LifecyclePhase::Recovering, LifecycleEventKind::RecoverySucceeded) => LifecycleState {
            phase: LifecyclePhase::Running,
            health: HealthState::Healthy,
            checkpoint_ref: event.checkpoint_ref.clone().or_else(|| state.checkpoint_ref.clone()),
            ..state.clone()
        },
        (LifecyclePhase::Running, LifecycleEventKind::BeginDrain) => with_phase(state, LifecyclePhase::Draining),
        (LifecyclePhase::Draining, LifecycleEventKind::DrainSucceeded) if usage.is_idle() => {
            with_phase(state, LifecyclePhase::Drained)
        }
        (LifecyclePhase::Draining, LifecycleEventKind::DrainSucceeded) => {
            issues.push(LifecycleIssue::ResourcesNotDrained(usage));
            return Err(issues);
        }
        (
            LifecyclePhase::Initializing
            | LifecyclePhase::Initialized
            | LifecyclePhase::Starting
            | LifecyclePhase::Running
            | LifecyclePhase::Checkpointing
            | LifecyclePhase::Recovering
            | LifecyclePhase::Draining
            | LifecyclePhase::Restarting
            | LifecyclePhase::Upgrading
            | LifecyclePhase::RollingBack
            | LifecyclePhase::ShuttingDown,
            LifecycleEventKind::Failure,
        ) => failed_state(
            state,
            event.failure_class.ok_or_else(|| vec![LifecycleIssue::MissingFailureClass])?,
            max_restart_attempts,
        ),
        (LifecyclePhase::Failed, LifecycleEventKind::BeginRestart) => {
            if plan_supervision(FailureClass::Retryable, state.restart_attempts, max_restart_attempts)
                == SupervisionDecision::Quarantine
            {
                issues.push(LifecycleIssue::RestartBudgetExhausted);
                return Err(issues);
            }
            let restart_attempts =
                state.restart_attempts.checked_add(1).ok_or_else(|| vec![LifecycleIssue::RestartBudgetExhausted])?;
            LifecycleState {
                phase: LifecyclePhase::Restarting,
                health: HealthState::Starting,
                restart_attempts,
                ..state.clone()
            }
        }
        (LifecyclePhase::Running | LifecyclePhase::Drained, LifecycleEventKind::BeginUpgrade) => {
            generated_state(state, event, LifecyclePhase::Upgrading)?
        }
        (LifecyclePhase::Upgrading, LifecycleEventKind::UpgradeSucceeded) => LifecycleState {
            phase: LifecyclePhase::Running,
            health: HealthState::Healthy,
            ..state.clone()
        },
        (
            LifecyclePhase::Running | LifecyclePhase::Drained | LifecyclePhase::Failed | LifecyclePhase::Quarantined,
            LifecycleEventKind::BeginRollback,
        ) => generated_state(state, event, LifecyclePhase::RollingBack)?,
        (LifecyclePhase::RollingBack, LifecycleEventKind::RollbackSucceeded) => LifecycleState {
            phase: LifecyclePhase::Running,
            health: HealthState::Healthy,
            ..state.clone()
        },
        (
            LifecyclePhase::Admitted
            | LifecyclePhase::Initialized
            | LifecyclePhase::Running
            | LifecyclePhase::Drained
            | LifecyclePhase::Failed
            | LifecyclePhase::Quarantined,
            LifecycleEventKind::BeginShutdown,
        ) => with_phase(state, LifecyclePhase::ShuttingDown),
        (LifecyclePhase::ShuttingDown, LifecycleEventKind::ShutdownSucceeded) if usage.is_idle() => LifecycleState {
            phase: LifecyclePhase::Stopped,
            health: HealthState::Stopped,
            ..state.clone()
        },
        (LifecyclePhase::ShuttingDown, LifecycleEventKind::ShutdownSucceeded) => {
            issues.push(LifecycleIssue::ResourcesNotDrained(usage));
            return Err(issues);
        }
        (LifecyclePhase::Stopped, LifecycleEventKind::Remove) => with_phase(state, LifecyclePhase::Removed),
        (phase, kind) => {
            issues.push(LifecycleIssue::IllegalTransition { phase, event: kind });
            return Err(issues);
        }
    };
    Ok(next)
}

fn validate_event_shape(state: &LifecycleState, event: &LifecycleEvent) -> Vec<LifecycleIssue> {
    let mut issues = Vec::new();
    if event.kind == LifecycleEventKind::Install {
        if event.generation != INITIAL_SYSTEM_EXTENSION_GENERATION {
            issues.push(LifecycleIssue::InitialGenerationMismatch {
                actual: event.generation,
                expected: INITIAL_SYSTEM_EXTENSION_GENERATION,
            });
        }
    } else if event.generation != state.generation {
        issues.push(LifecycleIssue::StaleGeneration {
            actual: event.generation,
            active: state.generation,
        });
    }

    let creates_generation = matches!(event.kind, LifecycleEventKind::BeginUpgrade | LifecycleEventKind::BeginRollback);
    if creates_generation {
        match event.next_generation {
            None => issues.push(LifecycleIssue::MissingNextGeneration),
            Some(actual) => match state.generation.checked_add(GENERATION_INCREMENT) {
                None => issues.push(LifecycleIssue::GenerationOverflow),
                Some(expected) if actual != expected => {
                    issues.push(LifecycleIssue::NonSequentialGeneration { actual, expected });
                }
                Some(_) => {}
            },
        }
    } else if let Some(generation) = event.next_generation {
        issues.push(LifecycleIssue::UnexpectedNextGeneration(generation));
    }

    let requires_failure = matches!(
        event.kind,
        LifecycleEventKind::Failure
            | LifecycleEventKind::CheckpointFailed
            | LifecycleEventKind::RecoveryFailed
            | LifecycleEventKind::UpgradeFailed
            | LifecycleEventKind::RollbackFailed
    );
    match (requires_failure, event.failure_class) {
        (true, None) => issues.push(LifecycleIssue::MissingFailureClass),
        (false, Some(failure)) => issues.push(LifecycleIssue::UnexpectedFailureClass(failure)),
        _ => {}
    }

    let requires_checkpoint =
        matches!(event.kind, LifecycleEventKind::CheckpointSucceeded | LifecycleEventKind::RecoverySucceeded);
    match (requires_checkpoint, event.checkpoint_ref.as_ref()) {
        (true, None) => issues.push(LifecycleIssue::MissingCheckpointRef),
        (_, Some(reference)) if !valid_ref(reference) => {
            issues.push(LifecycleIssue::MalformedCheckpointRef(reference.clone()));
        }
        (false, Some(reference)) => issues.push(LifecycleIssue::UnexpectedCheckpointRef(reference.clone())),
        _ => {}
    }
    issues
}

fn with_phase(state: &LifecycleState, phase: LifecyclePhase) -> LifecycleState {
    LifecycleState { phase, ..state.clone() }
}

fn failed_state(state: &LifecycleState, failure: FailureClass, max_restart_attempts: u64) -> LifecycleState {
    match plan_supervision(failure, state.restart_attempts, max_restart_attempts) {
        SupervisionDecision::Restart => LifecycleState {
            phase: LifecyclePhase::Failed,
            health: HealthState::Failed,
            ..state.clone()
        },
        SupervisionDecision::Quarantine => LifecycleState {
            phase: LifecyclePhase::Quarantined,
            health: HealthState::Quarantined,
            ..state.clone()
        },
    }
}

fn generated_state(
    state: &LifecycleState,
    event: &LifecycleEvent,
    phase: LifecyclePhase,
) -> Result<LifecycleState, Vec<LifecycleIssue>> {
    let generation = event.next_generation.ok_or_else(|| vec![LifecycleIssue::MissingNextGeneration])?;
    Ok(LifecycleState {
        generation,
        phase,
        health: HealthState::Starting,
        ..state.clone()
    })
}
