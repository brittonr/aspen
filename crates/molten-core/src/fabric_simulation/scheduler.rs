use std::collections::BTreeSet;

use super::*;

const SCHEDULER_EVENT_INCREMENT: u64 = 1;
const SCHEDULER_CHOICE_INCREMENT: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationSchedulerIssue {
    TerminalState,
    EmptyEligibleSet,
    TooManyEligibleChoices {
        actual: usize,
        maximum: usize,
    },
    DuplicateChoiceId(String),
    UnknownNode(String),
    StaleGeneration {
        node_id: String,
        expected: u64,
        actual: u64,
    },
    ChoiceBoundExceeded {
        next: u64,
        maximum: u64,
    },
    EventBoundExceeded {
        next: u64,
        maximum: u64,
    },
    VirtualTimeBoundExceeded {
        next: u64,
        maximum: u64,
    },
    RecordedChoiceNotEligible(ReplayDivergence),
    Overflow(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationSchedulerTransition {
    pub next: SimulationSchedulerState,
    pub record: SchedulerChoiceRecord,
}

pub fn select_simulation_choice(
    world: &AdmittedSimulatedWorld,
    state: &SimulationSchedulerState,
    eligible: &[EligibleChoice],
    recorded_choice_id: Option<&str>,
) -> Result<SimulationSchedulerTransition, SimulationSchedulerIssue> {
    if state.terminal {
        return Err(SimulationSchedulerIssue::TerminalState);
    }
    if eligible.is_empty() {
        return Err(SimulationSchedulerIssue::EmptyEligibleSet);
    }
    if eligible.len() > MAX_ELIGIBLE_CHOICES {
        return Err(SimulationSchedulerIssue::TooManyEligibleChoices {
            actual: eligible.len(),
            maximum: MAX_ELIGIBLE_CHOICES,
        });
    }
    validate_eligible(world, eligible)?;

    let mut ordered = eligible.to_vec();
    ordered.sort();
    let selected = match recorded_choice_id {
        None => ordered[0].clone(),
        Some(recorded) => ordered.iter().find(|choice| choice.choice_id == recorded).cloned().ok_or_else(|| {
            let eligible_choice_ids = ordered.iter().map(|choice| choice.choice_id.clone()).collect();
            SimulationSchedulerIssue::RecordedChoiceNotEligible(ReplayDivergence {
                position: state.next_choice_position,
                expected_choice_id: recorded.to_string(),
                eligible_choice_ids,
                diagnostic: "recorded scheduler choice is not eligible".to_string(),
            })
        })?,
    };

    let next_choice_position = state
        .next_choice_position
        .checked_add(SCHEDULER_CHOICE_INCREMENT)
        .ok_or(SimulationSchedulerIssue::Overflow("choice-position"))?;
    if next_choice_position > world.manifest.bounds.max_choices {
        return Err(SimulationSchedulerIssue::ChoiceBoundExceeded {
            next: next_choice_position,
            maximum: world.manifest.bounds.max_choices,
        });
    }
    let event_count = state
        .event_count
        .checked_add(SCHEDULER_EVENT_INCREMENT)
        .ok_or(SimulationSchedulerIssue::Overflow("event-count"))?;
    if event_count > world.manifest.bounds.max_events {
        return Err(SimulationSchedulerIssue::EventBoundExceeded {
            next: event_count,
            maximum: world.manifest.bounds.max_events,
        });
    }
    let virtual_tick = state.virtual_tick.max(selected.ready_at_tick);
    if virtual_tick > world.manifest.bounds.max_virtual_ticks {
        return Err(SimulationSchedulerIssue::VirtualTimeBoundExceeded {
            next: virtual_tick,
            maximum: world.manifest.bounds.max_virtual_ticks,
        });
    }
    Ok(SimulationSchedulerTransition {
        next: SimulationSchedulerState {
            next_choice_position,
            event_count,
            virtual_tick,
            terminal: false,
        },
        record: SchedulerChoiceRecord {
            position: state.next_choice_position,
            virtual_tick,
            eligible: ordered,
            selected,
        },
    })
}

pub fn finish_simulation_scheduler(
    world: &AdmittedSimulatedWorld,
    state: &SimulationSchedulerState,
) -> Result<SimulationSchedulerState, SimulationSchedulerIssue> {
    if state.virtual_tick > world.manifest.bounds.max_virtual_ticks {
        return Err(SimulationSchedulerIssue::VirtualTimeBoundExceeded {
            next: state.virtual_tick,
            maximum: world.manifest.bounds.max_virtual_ticks,
        });
    }
    let mut next = state.clone();
    next.terminal = true;
    Ok(next)
}

fn validate_eligible(
    world: &AdmittedSimulatedWorld,
    eligible: &[EligibleChoice],
) -> Result<(), SimulationSchedulerIssue> {
    let mut choice_ids = BTreeSet::new();
    for choice in eligible {
        if !choice_ids.insert(choice.choice_id.clone()) {
            return Err(SimulationSchedulerIssue::DuplicateChoiceId(choice.choice_id.clone()));
        }
        let node = world
            .manifest
            .nodes
            .iter()
            .find(|node| node.node_id == choice.node_id)
            .ok_or_else(|| SimulationSchedulerIssue::UnknownNode(choice.node_id.clone()))?;
        if choice.generation != node.generation {
            return Err(SimulationSchedulerIssue::StaleGeneration {
                node_id: choice.node_id.clone(),
                expected: node.generation,
                actual: choice.generation,
            });
        }
    }
    Ok(())
}
