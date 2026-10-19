use super::AdmittedTimeProfile;
use super::RunnableKey;
use super::RunnablePhase;
use super::RunnableState;
use super::SchedulerAction;
use super::SchedulerCommand;
use super::SchedulerError;
use super::SchedulerOrdering;
use super::SchedulerPolicy;
use super::SchedulerReplayPolicy;
use super::SchedulerState;
use super::command_key;
use super::count_phase;
use super::overload_action;
use super::ready_overload_action;
use super::validate_key;
use super::validate_scheduler_policy;
use super::validate_scheduler_state;

/// A validated command, staged without changing scheduler state or allocating a snapshot.
/// Apply only to the state and command used for preview, without intervening mutation.
#[derive(Debug)]
pub struct SchedulerCommandDecision {
    pub action: SchedulerAction,
    index: Option<usize>,
    phase: Option<RunnablePhase>,
    priority: Option<i32>,
    enqueue: Option<u64>,
}

impl SchedulerCommandDecision {
    pub fn occurrence_index(&self, state: &SchedulerState) -> usize {
        self.index.unwrap_or(state.runnables.len())
    }

    pub fn usage_change(&self, state: &SchedulerState) -> (i64, i64) {
        let before = self.index.map(|index| state.runnables[index].phase);
        let active = |phase| match phase {
            Some(RunnablePhase::Ready | RunnablePhase::Running | RunnablePhase::Blocked) => 1,
            _ => 0,
        };
        let ready = |phase| i64::from(phase == Some(RunnablePhase::Ready));
        match self.phase {
            Some(after) => (active(Some(after)) - active(before), ready(Some(after)) - ready(before)),
            None => (0, 0),
        }
    }

    pub fn apply(self, state: &mut SchedulerState, command: SchedulerCommand) -> SchedulerAction {
        if let Some(phase) = self.phase {
            if let Some(index) = self.index {
                let runnable = &mut state.runnables[index];
                runnable.phase = phase;
                if let Some(priority) = self.priority {
                    runnable.priority = priority;
                    runnable.wait_turns = 0;
                }
                if let Some(sequence) = self.enqueue {
                    runnable.enqueue_sequence = sequence;
                }
            } else if let SchedulerCommand::Wake { key, priority } = command {
                state.runnables.push(RunnableState {
                    key,
                    priority,
                    enqueue_sequence: state.next_enqueue_sequence,
                    wait_turns: 0,
                    phase,
                });
            }
            if self.enqueue.is_some() {
                // Preview checked this increment before any publication.
                state.next_enqueue_sequence += 1;
            }
        }
        self.action
    }
}

// r[impl molten.audit_f09.resume]
// r[impl molten.audit_f09.ordering]
// r[impl molten.audit_f10.queue]
// r[impl molten.audit_f10.atomicity]
pub fn preview_scheduler_command(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    active_generation: u64,
    command: &SchedulerCommand,
    has_new_occurrence_slot: bool,
) -> Result<SchedulerCommandDecision, SchedulerError> {
    validate_scheduler_state(profile, state, active_generation)?;
    validate_scheduler_policy(profile, policy)?;
    let key = command_key(command);
    validate_key(key)?;
    let index = state.runnables.iter().position(|runnable| &runnable.key == key);
    let unchanged = |action| SchedulerCommandDecision {
        action,
        index,
        phase: None,
        priority: None,
        enqueue: None,
    };
    if key.generation != active_generation {
        return Ok(unchanged(SchedulerAction::DiscardedStaleGeneration));
    }
    let before = index.map(|index| state.runnables[index].phase);
    let (phase, action, priority, should_enqueue) = match command {
        SchedulerCommand::Wake { priority, .. } => {
            if before.is_some() && before != Some(RunnablePhase::Blocked) {
                return Err(SchedulerError::DuplicateRunnable(key.clone()));
            }
            if let Some(action) = ready_overload_action(profile, policy, state, before.is_none())? {
                return Ok(unchanged(action));
            }
            if before.is_none() && !has_new_occurrence_slot {
                return Ok(unchanged(overload_action(policy)));
            }
            (RunnablePhase::Ready, SchedulerAction::Woken, Some(*priority), true)
        }
        SchedulerCommand::Yield { .. } => {
            require_phase(key, before, RunnablePhase::Running)?;
            if let Some(action) = ready_overload_action(profile, policy, state, false)? {
                return Ok(unchanged(action));
            }
            (RunnablePhase::Ready, SchedulerAction::Yielded, None, true)
        }
        SchedulerCommand::Block { .. } => {
            if !matches!(before, Some(RunnablePhase::Ready | RunnablePhase::Running)) {
                require_phase(key, before, RunnablePhase::Running)?;
            }
            (RunnablePhase::Blocked, SchedulerAction::Blocked, None, false)
        }
        SchedulerCommand::Complete { .. } => {
            require_phase(key, before, RunnablePhase::Running)?;
            (RunnablePhase::Completed, SchedulerAction::Completed, None, false)
        }
        SchedulerCommand::Cancel { .. } => {
            if !matches!(before, Some(RunnablePhase::Ready | RunnablePhase::Running | RunnablePhase::Blocked)) {
                require_phase(key, before, RunnablePhase::Ready)?;
            }
            (RunnablePhase::Cancelled, SchedulerAction::Cancelled, None, false)
        }
    };
    if should_enqueue {
        state.next_enqueue_sequence.checked_add(1).ok_or(SchedulerError::Overflow)?;
    }
    Ok(SchedulerCommandDecision {
        action,
        index,
        phase: Some(phase),
        priority,
        enqueue: should_enqueue.then_some(state.next_enqueue_sequence),
    })
}

fn require_phase(
    key: &RunnableKey,
    actual: Option<RunnablePhase>,
    expected: RunnablePhase,
) -> Result<(), SchedulerError> {
    let actual = actual.ok_or_else(|| SchedulerError::UnknownRunnable(key.clone()))?;
    if actual != expected {
        return Err(SchedulerError::InvalidPhase {
            key: key.clone(),
            expected,
            actual,
        });
    }
    Ok(())
}

#[derive(Debug)]
pub struct SchedulerChoiceDecision {
    pub index: usize,
    pub choice_sequence: u64,
    pub eligible_count: u64,
}

impl SchedulerChoiceDecision {
    pub fn apply(self, state: &mut SchedulerState) {
        for (index, runnable) in state.runnables.iter_mut().enumerate() {
            if runnable.phase != RunnablePhase::Ready {
                continue;
            }
            if index == self.index {
                runnable.phase = RunnablePhase::Running;
                runnable.wait_turns = 0;
            } else {
                runnable.wait_turns += 1;
            }
        }
        state.choice_sequence = self.choice_sequence;
    }
}

/// `ready_indices`, when supplied by an owning shell, is its complete Ready queue.
/// Snapshot consumers pass `None` to enumerate Ready rows directly.
pub fn preview_runnable_choice(
    profile: &AdmittedTimeProfile,
    policy: SchedulerPolicy,
    state: &SchedulerState,
    active_generation: u64,
    recorded_choice: Option<&RunnableKey>,
    ready_indices: Option<&[usize]>,
) -> Result<SchedulerChoiceDecision, SchedulerError> {
    validate_scheduler_state(profile, state, active_generation)?;
    validate_scheduler_policy(profile, policy)?;
    let running = count_phase(state, RunnablePhase::Running)?;
    if running >= profile.max_scheduler_concurrency {
        return Err(SchedulerError::ConcurrencyLimitExceeded {
            running,
            maximum: profile.max_scheduler_concurrency,
        });
    }
    let ready = || {
        (0..ready_indices.map_or(state.runnables.len(), <[usize]>::len)).filter_map(|position| {
            let index = ready_indices.map_or(position, |indices| indices[position]);
            let runnable = state.runnables.get(index)?;
            (runnable.phase == RunnablePhase::Ready && runnable.key.generation == active_generation)
                .then_some((index, runnable))
        })
    };
    let (deterministic_index, deterministic) = ready()
        .min_by(|(_, left), (_, right)| compare_ready(left, right, policy.ordering, profile.fairness_bound_turns))
        .ok_or(SchedulerError::NoReadyRunnable)?;
    let index = match (policy.replay, recorded_choice) {
        (SchedulerReplayPolicy::RecordedChoiceRequired, None) => return Err(SchedulerError::ReplayChoiceRequired),
        (SchedulerReplayPolicy::RecordedChoiceRequired, Some(recorded)) => ready()
            .find(|(_, runnable)| &runnable.key == recorded)
            .map(|(index, _)| index)
            .ok_or_else(|| SchedulerError::UnexpectedReplayChoice {
                expected: deterministic.key.clone(),
                actual: recorded.clone(),
            })?,
        (SchedulerReplayPolicy::Deterministic, Some(recorded)) if recorded != &deterministic.key => {
            return Err(SchedulerError::UnexpectedReplayChoice {
                expected: deterministic.key.clone(),
                actual: recorded.clone(),
            });
        }
        _ => deterministic_index,
    };
    if let Some(maximum) = profile.fairness_bound_turns {
        // Match the sorted canonical ready order, including the tie chosen by max_by_key.
        let overdue = ready().filter(|(_, runnable)| runnable.wait_turns >= maximum).max_by(|(_, left), (_, right)| {
            left.wait_turns
                .cmp(&right.wait_turns)
                .then_with(|| compare_ready(left, right, policy.ordering, profile.fairness_bound_turns))
        });
        if let Some((_, overdue)) = overdue
            && state.runnables[index].wait_turns < overdue.wait_turns
        {
            return Err(SchedulerError::FairnessBoundExceeded {
                key: overdue.key.clone(),
                wait_turns: overdue.wait_turns,
                maximum,
            });
        }
    }
    for (other, runnable) in
        state.runnables.iter().enumerate().filter(|(_, runnable)| runnable.phase == RunnablePhase::Ready)
    {
        if other != index {
            runnable.wait_turns.checked_add(1).ok_or(SchedulerError::Overflow)?;
        }
    }
    Ok(SchedulerChoiceDecision {
        index,
        choice_sequence: state.choice_sequence.checked_add(1).ok_or(SchedulerError::Overflow)?,
        eligible_count: u64::try_from(ready().count()).map_err(|_| SchedulerError::Overflow)?,
    })
}

fn compare_ready(
    left: &RunnableState,
    right: &RunnableState,
    ordering: SchedulerOrdering,
    fairness_bound: Option<u64>,
) -> std::cmp::Ordering {
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
}
