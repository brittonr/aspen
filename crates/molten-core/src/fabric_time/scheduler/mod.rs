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
    pub occurrence_sequence: u64,
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
    pub terminal_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerState {
    pub profile_ref: String,
    pub generation: u64,
    pub next_enqueue_sequence: u64,
    pub choice_sequence: u64,
    pub terminal_retention_limit: u64,
    pub highest_occurrence_sequence: u64,
    pub next_terminal_sequence: u64,
    pub runnables: Vec<Option<RunnableState>>,
}

impl SchedulerState {
    /// Propose fresh issuance; only an admitted Wake advances the fence.
    pub fn next_occurrence_sequence(&self) -> Result<u64, SchedulerError> {
        self.highest_occurrence_sequence.checked_add(1).ok_or(SchedulerError::Overflow)
    }
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
pub struct ReplayChoiceMismatch {
    pub expected: RunnableKey,
    pub actual: RunnableKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    MalformedServiceId(String),
    MalformedRunnableId(String),
    ZeroGeneration,
    ZeroOccurrenceSequence,
    StaleOccurrence {
        highest: u64,
        actual: u64,
    },
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
    UnexpectedReplayChoice(Box<ReplayChoiceMismatch>),
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
        terminal_retention_limit: profile.max_runnables,
        highest_occurrence_sequence: 0,
        next_terminal_sequence: 0,
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
    let decision = preview_scheduler_command(profile, policy, state, active_generation, command)?;
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
    let selected = state.runnables[decision.index].as_ref().ok_or(SchedulerError::NoReadyRunnable)?.key.clone();
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

pub fn cleanup_scheduler_generation(state: &SchedulerState, generation: u64) -> Result<SchedulerState, SchedulerError> {
    let mut next = state.clone();
    cleanup_scheduler_generation_in_place(&mut next, generation)?;
    Ok(next)
}

// r[impl molten.audit_f11.retention]
// r[impl molten.audit_f11.atomicity]
pub fn cleanup_scheduler_generation_in_place(
    state: &mut SchedulerState,
    generation: u64,
) -> Result<(), SchedulerError> {
    let affected = u64::try_from(
        state
            .runnables
            .iter()
            .flatten()
            .filter(|runnable| runnable.key.generation == generation && runnable.terminal_sequence.is_none())
            .count(),
    )
    .map_err(|_| SchedulerError::Overflow)?;
    state.next_terminal_sequence.checked_add(affected).ok_or(SchedulerError::Overflow)?;
    for runnable in state.runnables.iter_mut().flatten() {
        if runnable.key.generation == generation && runnable.terminal_sequence.is_none() {
            runnable.phase = RunnablePhase::Cancelled;
            runnable.terminal_sequence = Some(state.next_terminal_sequence);
            state.next_terminal_sequence += 1;
        }
    }
    reclaim_terminal_history(state);
    Ok(())
}

// Terminal positions are unique and contiguous. Keeping the last admitted window
// reclaims oldest terminals in one pass without moving any occupied slot.
fn reclaim_terminal_history(state: &mut SchedulerState) {
    let oldest_retained = state.next_terminal_sequence.saturating_sub(state.terminal_retention_limit);
    for slot in &mut state.runnables {
        if slot
            .as_ref()
            .and_then(|runnable| runnable.terminal_sequence)
            .is_some_and(|sequence| sequence < oldest_retained)
        {
            *slot = None;
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
            .flatten()
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
    if state.terminal_retention_limit != profile.max_runnables {
        return Err(SchedulerError::ProfileMismatch);
    }
    let physical_limit = profile.max_runnables.checked_mul(2).ok_or(SchedulerError::Overflow)?;
    if u64::try_from(state.runnables.len()).map_err(|_| SchedulerError::Overflow)? > physical_limit {
        return Err(SchedulerError::Overflow);
    }
    Ok(())
}

pub fn validate_runnable_key(key: &RunnableKey) -> Result<(), SchedulerError> {
    if !valid_time_id(&key.service_id) {
        return Err(SchedulerError::MalformedServiceId(key.service_id.clone()));
    }
    if !valid_time_id(&key.runnable_id) {
        return Err(SchedulerError::MalformedRunnableId(key.runnable_id.clone()));
    }
    if key.generation == 0 {
        return Err(SchedulerError::ZeroGeneration);
    }
    if key.occurrence_sequence == 0 {
        return Err(SchedulerError::ZeroOccurrenceSequence);
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
    u64::try_from(state.runnables.iter().flatten().filter(|runnable| runnable.phase == phase).count())
        .map_err(|_| SchedulerError::Overflow)
}
