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
    validate_scheduler_state(profile, state, active_generation)?;
    validate_scheduler_policy(profile, policy)?;
    let key = command_key(command);
    validate_key(key)?;
    if key.generation != active_generation {
        return Ok(discard_stale(state, key));
    }
    match command {
        SchedulerCommand::Wake { key, priority } => wake(profile, policy, state, key, *priority),
        SchedulerCommand::Yield { key } => {
            transition_phase(state, key, RunnablePhase::Running, RunnablePhase::Ready, SchedulerAction::Yielded, true)
        }
        SchedulerCommand::Block { key } => block(state, key),
        SchedulerCommand::Complete { key } => transition_phase(
            state,
            key,
            RunnablePhase::Running,
            RunnablePhase::Completed,
            SchedulerAction::Completed,
            false,
        ),
        SchedulerCommand::Cancel { key } => cancel(state, key),
    }
}

pub fn choose_runnable(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    active_generation: u64,
    recorded_choice: Option<&RunnableKey>,
) -> Result<SchedulerSelection, SchedulerError> {
    validate_scheduler_state(profile, state, active_generation)?;
    validate_scheduler_policy(profile, policy)?;
    let running = count_phase(state, RunnablePhase::Running)?;
    if running >= profile.max_scheduler_concurrency {
        return Err(SchedulerError::ConcurrencyLimitExceeded {
            running,
            maximum: profile.max_scheduler_concurrency,
        });
    }
    let mut ready: Vec<&RunnableState> = state
        .runnables
        .iter()
        .filter(|runnable| runnable.phase == RunnablePhase::Ready && runnable.key.generation == active_generation)
        .collect();
    if ready.is_empty() {
        return Err(SchedulerError::NoReadyRunnable);
    }
    order_ready(&mut ready, policy.ordering, profile.fairness_bound_turns);
    let deterministic_choice = ready[0].key.clone();
    let selected = match policy.replay {
        SchedulerReplayPolicy::Deterministic => {
            if let Some(recorded) = recorded_choice
                && recorded != &deterministic_choice
            {
                return Err(SchedulerError::UnexpectedReplayChoice {
                    expected: deterministic_choice,
                    actual: recorded.clone(),
                });
            }
            ready[0].key.clone()
        }
        SchedulerReplayPolicy::RecordedChoiceRequired => {
            let recorded = recorded_choice.ok_or(SchedulerError::ReplayChoiceRequired)?;
            if !ready.iter().any(|runnable| &runnable.key == recorded) {
                return Err(SchedulerError::UnexpectedReplayChoice {
                    expected: ready[0].key.clone(),
                    actual: recorded.clone(),
                });
            }
            recorded.clone()
        }
    };
    enforce_fairness(profile, &ready, &selected)?;

    let mut next = state.clone();
    for runnable in &mut next.runnables {
        if runnable.phase != RunnablePhase::Ready {
            continue;
        }
        if runnable.key == selected {
            runnable.phase = RunnablePhase::Running;
            runnable.wait_turns = 0;
        } else {
            runnable.wait_turns = runnable.wait_turns.checked_add(1).ok_or(SchedulerError::Overflow)?;
        }
    }
    next.choice_sequence = next.choice_sequence.checked_add(1).ok_or(SchedulerError::Overflow)?;
    let eligible_count = u64::try_from(ready.len()).map_err(|_| SchedulerError::Overflow)?;
    Ok(SchedulerSelection {
        choice_sequence: next.choice_sequence,
        next,
        selected,
        eligible_count,
    })
}

pub fn cleanup_scheduler_generation(state: &SchedulerState, generation: u64) -> SchedulerState {
    let mut next = state.clone();
    for runnable in &mut next.runnables {
        if runnable.key.generation == generation
            && !matches!(runnable.phase, RunnablePhase::Completed | RunnablePhase::Cancelled)
        {
            runnable.phase = RunnablePhase::Cancelled;
        }
    }
    next
}

fn wake(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    key: &RunnableKey,
    priority: i32,
) -> Result<SchedulerTransition, SchedulerError> {
    if state.runnables.iter().any(|runnable| &runnable.key == key) {
        return Err(SchedulerError::DuplicateRunnable(key.clone()));
    }
    let queued =
        u64::try_from(state.runnables.iter().filter(|runnable| runnable.phase == RunnablePhase::Ready).count())
            .map_err(|_| SchedulerError::Overflow)?;
    let active = u64::try_from(
        state
            .runnables
            .iter()
            .filter(|runnable| !matches!(runnable.phase, RunnablePhase::Completed | RunnablePhase::Cancelled))
            .count(),
    )
    .map_err(|_| SchedulerError::Overflow)?;
    if queued >= profile.max_scheduler_queue_depth || active >= profile.max_runnables {
        let action = match policy.overload {
            SchedulerOverloadPolicy::Reject => SchedulerAction::RejectedOverload,
            SchedulerOverloadPolicy::Backpressure => SchedulerAction::Backpressure,
        };
        return Ok(SchedulerTransition {
            next: state.clone(),
            action,
            runnable: key.clone(),
        });
    }
    let mut next = state.clone();
    let enqueue_sequence = next.next_enqueue_sequence;
    next.next_enqueue_sequence = next.next_enqueue_sequence.checked_add(1).ok_or(SchedulerError::Overflow)?;
    next.runnables.push(RunnableState {
        key: key.clone(),
        priority,
        enqueue_sequence,
        wait_turns: 0,
        phase: RunnablePhase::Ready,
    });
    Ok(SchedulerTransition {
        next,
        action: SchedulerAction::Woken,
        runnable: key.clone(),
    })
}

fn block(state: &SchedulerState, key: &RunnableKey) -> Result<SchedulerTransition, SchedulerError> {
    let phase = find_runnable(state, key)?.phase;
    if !matches!(phase, RunnablePhase::Ready | RunnablePhase::Running) {
        return Err(SchedulerError::InvalidPhase {
            key: key.clone(),
            expected: RunnablePhase::Running,
            actual: phase,
        });
    }
    let mut next = state.clone();
    find_runnable_mut(&mut next, key)?.phase = RunnablePhase::Blocked;
    Ok(SchedulerTransition {
        next,
        action: SchedulerAction::Blocked,
        runnable: key.clone(),
    })
}

fn cancel(state: &SchedulerState, key: &RunnableKey) -> Result<SchedulerTransition, SchedulerError> {
    let phase = find_runnable(state, key)?.phase;
    if matches!(phase, RunnablePhase::Completed | RunnablePhase::Cancelled) {
        return Err(SchedulerError::InvalidPhase {
            key: key.clone(),
            expected: RunnablePhase::Ready,
            actual: phase,
        });
    }
    let mut next = state.clone();
    find_runnable_mut(&mut next, key)?.phase = RunnablePhase::Cancelled;
    Ok(SchedulerTransition {
        next,
        action: SchedulerAction::Cancelled,
        runnable: key.clone(),
    })
}

fn transition_phase(
    state: &SchedulerState,
    key: &RunnableKey,
    expected: RunnablePhase,
    target: RunnablePhase,
    action: SchedulerAction,
    refresh_enqueue_sequence: bool,
) -> Result<SchedulerTransition, SchedulerError> {
    let actual = find_runnable(state, key)?.phase;
    if actual != expected {
        return Err(SchedulerError::InvalidPhase {
            key: key.clone(),
            expected,
            actual,
        });
    }
    let mut next = state.clone();
    if refresh_enqueue_sequence {
        let sequence = next.next_enqueue_sequence;
        next.next_enqueue_sequence = sequence.checked_add(1).ok_or(SchedulerError::Overflow)?;
        find_runnable_mut(&mut next, key)?.enqueue_sequence = sequence;
    }
    find_runnable_mut(&mut next, key)?.phase = target;
    Ok(SchedulerTransition {
        next,
        action,
        runnable: key.clone(),
    })
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

fn discard_stale(state: &SchedulerState, key: &RunnableKey) -> SchedulerTransition {
    SchedulerTransition {
        next: state.clone(),
        action: SchedulerAction::DiscardedStaleGeneration,
        runnable: key.clone(),
    }
}

fn find_runnable<'a>(state: &'a SchedulerState, key: &RunnableKey) -> Result<&'a RunnableState, SchedulerError> {
    state
        .runnables
        .iter()
        .find(|runnable| &runnable.key == key)
        .ok_or_else(|| SchedulerError::UnknownRunnable(key.clone()))
}

fn find_runnable_mut<'a>(
    state: &'a mut SchedulerState,
    key: &RunnableKey,
) -> Result<&'a mut RunnableState, SchedulerError> {
    state
        .runnables
        .iter_mut()
        .find(|runnable| &runnable.key == key)
        .ok_or_else(|| SchedulerError::UnknownRunnable(key.clone()))
}

fn count_phase(state: &SchedulerState, phase: RunnablePhase) -> Result<u64, SchedulerError> {
    u64::try_from(state.runnables.iter().filter(|runnable| runnable.phase == phase).count())
        .map_err(|_| SchedulerError::Overflow)
}

fn order_ready(ready: &mut [&RunnableState], ordering: SchedulerOrdering, fairness_bound: Option<u64>) {
    ready.sort_by(|left, right| {
        let fairness = fairness_bound.map(|bound| {
            let left_due = left.wait_turns >= bound;
            let right_due = right.wait_turns >= bound;
            right_due.cmp(&left_due).then_with(|| {
                if left_due && right_due {
                    right.wait_turns.cmp(&left.wait_turns)
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });
        fairness
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| match ordering {
                SchedulerOrdering::Fifo => left.enqueue_sequence.cmp(&right.enqueue_sequence),
                SchedulerOrdering::PriorityThenFifo => {
                    right.priority.cmp(&left.priority).then_with(|| left.enqueue_sequence.cmp(&right.enqueue_sequence))
                }
            })
            .then_with(|| left.key.cmp(&right.key))
    });
}

fn enforce_fairness(
    profile: &AdmittedTimeProfile,
    ready: &[&RunnableState],
    selected: &RunnableKey,
) -> Result<(), SchedulerError> {
    let Some(maximum) = profile.fairness_bound_turns else {
        return Ok(());
    };
    let Some(overdue) = ready
        .iter()
        .filter(|runnable| runnable.wait_turns >= maximum)
        .max_by_key(|runnable| runnable.wait_turns)
    else {
        return Ok(());
    };
    let selected_wait = ready
        .iter()
        .find(|runnable| &runnable.key == selected)
        .map(|runnable| runnable.wait_turns)
        .unwrap_or_default();
    if selected_wait < overdue.wait_turns {
        return Err(SchedulerError::FairnessBoundExceeded {
            key: overdue.key.clone(),
            wait_turns: overdue.wait_turns,
            maximum,
        });
    }
    Ok(())
}
