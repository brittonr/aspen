pub mod capacity;
mod decision;
pub use decision::*;

use super::AdmittedTimeProfile;
use super::valid_time_id;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RunnableKey {
    pub service_id: String,
    pub generation: u64,
    pub runnable_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnablePhase {
    Ready,
    Running,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnableState {
    pub key: RunnableKey,
    pub priority: i32,
    pub enqueue_sequence: u64,
    pub wait_turns: u64,
    pub phase: RunnablePhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerState {
    pub profile_ref: String,
    pub generation: u64,
    pub next_enqueue_sequence: u64,
    pub choice_sequence: u64,
    pub runnables: Vec<RunnableState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerOrdering {
    Fifo,
    PriorityThenFifo,
}

impl SchedulerOrdering {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::PriorityThenFifo => "priority-then-fifo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerReplayPolicy {
    Deterministic,
    RecordedChoiceRequired,
}

impl SchedulerReplayPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::RecordedChoiceRequired => "recorded-choice-required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerOverloadPolicy {
    Reject,
    Backpressure,
}

impl SchedulerOverloadPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::Backpressure => "backpressure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerPolicy {
    pub ordering: SchedulerOrdering,
    pub replay: SchedulerReplayPolicy,
    pub overload: SchedulerOverloadPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerCommand {
    Wake { key: RunnableKey, priority: i32 },
    Yield { key: RunnableKey },
    Block { key: RunnableKey },
    Complete { key: RunnableKey },
    Cancel { key: RunnableKey },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerAction {
    Woken,
    Yielded,
    Blocked,
    Completed,
    Cancelled,
    RejectedOverload,
    Backpressure,
    DiscardedStaleGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTransition {
    pub next: SchedulerState,
    pub action: SchedulerAction,
    pub runnable: RunnableKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerSelection {
    pub next: SchedulerState,
    pub selected: RunnableKey,
    pub choice_sequence: u64,
    pub eligible_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    MalformedServiceId(String),
    MalformedRunnableId(String),
    ZeroGeneration,
    ProfileMismatch,
    PolicyMismatch {
        expected: SchedulerPolicy,
        actual: SchedulerPolicy,
    },
    StaleGeneration {
        expected: u64,
        actual: u64,
    },
    DuplicateRunnable(RunnableKey),
    UnknownRunnable(RunnableKey),
    InvalidPhase {
        key: RunnableKey,
        expected: RunnablePhase,
        actual: RunnablePhase,
    },
    NoReadyRunnable,
    ConcurrencyLimitExceeded {
        running: u64,
        maximum: u64,
    },
    ReplayChoiceRequired,
    UnexpectedReplayChoice {
        expected: RunnableKey,
        actual: RunnableKey,
    },
    FairnessBoundExceeded {
        key: RunnableKey,
        wait_turns: u64,
        maximum: u64,
    },
    Overflow,
}

pub fn new_scheduler_state(profile: &AdmittedTimeProfile, generation: u64) -> SchedulerState {
    SchedulerState {
        profile_ref: profile.profile_ref.clone(),
        generation,
        next_enqueue_sequence: 0,
        choice_sequence: 0,
        runnables: Vec::new(),
    }
}

// r[impl molten.fabric_time.scheduler]
pub fn apply_scheduler_command(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    active_generation: u64,
    command: &SchedulerCommand,
) -> Result<SchedulerTransition, SchedulerError> {
    let decision = preview_scheduler_command(profile, policy, state, active_generation, command, true)?;
    let mut next = state.clone();
    let action = decision.apply(&mut next, command.clone());
    Ok(SchedulerTransition {
        next,
        action,
        runnable: command_key(command).clone(),
    })
}

pub fn choose_runnable(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    active_generation: u64,
    recorded_choice: Option<&RunnableKey>,
) -> Result<SchedulerSelection, SchedulerError> {
    let decision = preview_runnable_choice(profile, policy, state, active_generation, recorded_choice, None)?;
    let selected = state.runnables[decision.index].key.clone();
    let choice_sequence = decision.choice_sequence;
    let eligible_count = decision.eligible_count;
    let mut next = state.clone();
    decision.apply(&mut next);
    Ok(SchedulerSelection {
        next,
        selected,
        choice_sequence,
        eligible_count,
    })
}

pub fn cleanup_scheduler_generation(state: &SchedulerState, generation: u64) -> SchedulerState {
    let mut next = state.clone();
    cleanup_scheduler_generation_in_place(&mut next, generation);
    next
}

pub fn cleanup_scheduler_generation_in_place(state: &mut SchedulerState, generation: u64) {
    for runnable in &mut state.runnables {
        if runnable.key.generation == generation
            && !matches!(runnable.phase, RunnablePhase::Completed | RunnablePhase::Cancelled)
        {
            runnable.phase = RunnablePhase::Cancelled;
        }
    }
}

// r[impl molten.audit_f10.shared_admission]
// r[impl molten.audit_f09.queue]
fn ready_overload_action(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    counts_as_new_work: bool,
) -> Result<Option<SchedulerAction>, SchedulerError> {
    let queued = count_phase(state, RunnablePhase::Ready)?;
    if queued >= profile.max_scheduler_queue_depth {
        return Ok(Some(overload_action(policy)));
    }
    if !counts_as_new_work {
        return Ok(None);
    }
    let active = u64::try_from(
        state
            .runnables
            .iter()
            .filter(|runnable| !matches!(runnable.phase, RunnablePhase::Completed | RunnablePhase::Cancelled))
            .count(),
    )
    .map_err(|_| SchedulerError::Overflow)?;
    if active >= profile.max_runnables {
        return Ok(Some(overload_action(policy)));
    }
    Ok(None)
}

fn overload_action(policy: SchedulerPolicy) -> SchedulerAction {
    match policy.overload {
        SchedulerOverloadPolicy::Reject => SchedulerAction::RejectedOverload,
        SchedulerOverloadPolicy::Backpressure => SchedulerAction::Backpressure,
    }
}

fn validate_scheduler_policy(profile: &AdmittedTimeProfile, actual: SchedulerPolicy) -> Result<(), SchedulerError> {
    if profile.scheduler_policy != actual {
        return Err(SchedulerError::PolicyMismatch {
            expected: profile.scheduler_policy,
            actual,
        });
    }
    Ok(())
}

fn validate_scheduler_state(
    profile: &AdmittedTimeProfile,
    state: &SchedulerState,
    active_generation: u64,
) -> Result<(), SchedulerError> {
    if state.profile_ref != profile.profile_ref {
        return Err(SchedulerError::ProfileMismatch);
    }
    if state.generation != active_generation {
        return Err(SchedulerError::StaleGeneration {
            expected: active_generation,
            actual: state.generation,
        });
    }
    Ok(())
}

fn validate_key(key: &RunnableKey) -> Result<(), SchedulerError> {
    if !valid_time_id(&key.service_id) {
        return Err(SchedulerError::MalformedServiceId(key.service_id.clone()));
    }
    if !valid_time_id(&key.runnable_id) {
        return Err(SchedulerError::MalformedRunnableId(key.runnable_id.clone()));
    }
    if key.generation == 0 {
        return Err(SchedulerError::ZeroGeneration);
    }
    Ok(())
}

fn command_key(command: &SchedulerCommand) -> &RunnableKey {
    match command {
        SchedulerCommand::Wake { key, .. }
        | SchedulerCommand::Yield { key }
        | SchedulerCommand::Block { key }
        | SchedulerCommand::Complete { key }
        | SchedulerCommand::Cancel { key } => key,
    }
}

fn count_phase(state: &SchedulerState, phase: RunnablePhase) -> Result<u64, SchedulerError> {
    u64::try_from(state.runnables.iter().filter(|runnable| runnable.phase == phase).count())
        .map_err(|_| SchedulerError::Overflow)
}
