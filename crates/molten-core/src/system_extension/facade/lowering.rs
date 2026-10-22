use super::super::AdmittedSystemExtensionManifest;
use super::super::CallbackOutcome;
use super::super::EffectTarget;
use super::super::HealthState;
use super::super::TypedEffectRequest;
use super::AdmittedFacadeManifest;
use super::FacadePhaseEntry;
use crate::fabric::FabricPortKey;

/// The admitted event presented to a facade turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeEvent {
    pub kind: String,
    pub payload_ref: Option<String>,
    pub logical_tick: u64,
    pub expected_turn_sequence: u64,
}

/// An event the facade deferred to a later phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadePostponedEvent {
    pub kind: String,
    pub payload_ref: Option<String>,
    pub logical_tick: u64,
}

/// The committed facade state handed to a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeRuntimeState {
    pub generation: u64,
    pub incarnation: u64,
    pub phase: String,
    pub state_ref: Option<String>,
    pub turn_sequence: u64,
    pub postponed: Vec<FacadePostponedEvent>,
    pub completed: bool,
}

/// An effect intent before it binds the executing generation and incarnation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeEffectIntent {
    pub port_id: String,
    pub port_version: String,
    pub operation: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub request_ref: String,
    pub accounted_bytes: u64,
}

/// A timer intent before it binds the phase and incarnation that scheduled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeTimerRequest {
    pub timer_ref: String,
    pub phase: String,
    pub incarnation: u64,
    pub logical_tick: u64,
    pub ticks: u64,
}

/// A timer intent fenced by the phase and incarnation that scheduled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeTimerIntent {
    pub timer_ref: String,
    pub phase: String,
    pub incarnation: u64,
    pub logical_tick: u64,
    pub deadline_tick: u64,
}

/// The application's model result for one admitted turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeModelTurn {
    pub next_phase: String,
    pub next_state_ref: Option<String>,
    pub replies: Vec<String>,
    pub effect_intents: Vec<FacadeEffectIntent>,
    pub timer_requests: Vec<FacadeTimerRequest>,
    pub completed: bool,
}

/// The facade's lowered turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeTurn {
    pub next_phase: String,
    pub next_state_ref: Option<String>,
    pub replies: Vec<String>,
    pub effects: Vec<TypedEffectRequest>,
    pub timer_intents: Vec<FacadeTimerIntent>,
    pub completed: bool,
    pub turn_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacadeTurnIssue {
    StaleTurn {
        actual: u64,
        expected: u64,
    },
    IllegalEventForPhase {
        phase: String,
        event: String,
    },
    UnknownEvent {
        event: String,
    },
    UndeclaredPhase {
        phase: String,
    },
    UnknownCompletionPhase {
        phase: String,
    },
    StaleTimer {
        timer_ref: String,
        phase: String,
        incarnation: u64,
    },
    UnfencedTimer {
        timer_ref: String,
    },
    PostponementOverflow {
        phase: String,
        bound: u64,
    },
    TaskAlreadyCompleted,
    EffectNotBound {
        port_id: String,
        port_version: String,
    },
}

/// The decision the facade makes about one presented event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacadeEventAdmission {
    Accepted,
    Postponed { postponed: Vec<FacadePostponedEvent> },
    DeadLettered { dead_lettered: Vec<FacadePostponedEvent> },
}

// r[impl molten.system_extension.service_facades]
pub fn admit_facade_event(
    facade: &AdmittedFacadeManifest,
    state: &FacadeRuntimeState,
    event: &FacadeEvent,
) -> Result<FacadeEventAdmission, Vec<FacadeTurnIssue>> {
    let mut issues = Vec::new();
    if event.expected_turn_sequence != state.turn_sequence {
        issues.push(FacadeTurnIssue::StaleTurn {
            actual: event.expected_turn_sequence,
            expected: state.turn_sequence,
        });
    }
    if event.kind.trim().is_empty() {
        issues.push(FacadeTurnIssue::UnknownEvent {
            event: event.kind.clone(),
        });
    }
    if state.completed {
        issues.push(FacadeTurnIssue::TaskAlreadyCompleted);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let Some(entry) = facade.phase_entry(&state.phase) else {
        return Err(vec![FacadeTurnIssue::UndeclaredPhase {
            phase: state.phase.clone(),
        }]);
    };
    if entry.legal_events.iter().any(|legal| legal == &event.kind) {
        return Ok(FacadeEventAdmission::Accepted);
    }
    if entry.postponable_events.iter().any(|postponable| postponable == &event.kind) {
        let bound = entry.postponement_bound.unwrap_or(0);
        let postponed_count = u64::try_from(state.postponed.len()).unwrap_or(u64::MAX);
        let postponed_event = FacadePostponedEvent {
            kind: event.kind.clone(),
            payload_ref: event.payload_ref.clone(),
            logical_tick: event.logical_tick,
        };
        if postponed_count >= bound {
            return Ok(FacadeEventAdmission::DeadLettered {
                dead_lettered: vec![postponed_event],
            });
        }
        let mut postponed = state.postponed.clone();
        postponed.push(postponed_event);
        return Ok(FacadeEventAdmission::Postponed { postponed });
    }
    Err(vec![FacadeTurnIssue::IllegalEventForPhase {
        phase: state.phase.clone(),
        event: event.kind.clone(),
    }])
}

/// A timer that fired and asks the facade whether it may still act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeTimerDelivery {
    pub timer_ref: String,
    pub phase: String,
    pub incarnation: u64,
    pub logical_tick: u64,
}

/// Admit a fired timer against the current state.
///
/// A timer scheduled by an earlier phase or an earlier incarnation cannot act
/// on the current state or on a replacement instance.
pub fn admit_facade_timer(
    facade: &AdmittedFacadeManifest,
    state: &FacadeRuntimeState,
    delivery: &FacadeTimerDelivery,
) -> Result<(), Vec<FacadeTurnIssue>> {
    let mut issues = Vec::new();
    if delivery.phase != state.phase || delivery.incarnation != state.incarnation {
        issues.push(FacadeTurnIssue::StaleTimer {
            timer_ref: delivery.timer_ref.clone(),
            phase: delivery.phase.clone(),
            incarnation: delivery.incarnation,
        });
    }
    if facade.phase_entry(&delivery.phase).is_none() {
        issues.push(FacadeTurnIssue::UndeclaredPhase {
            phase: delivery.phase.clone(),
        });
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

/// Lower the application's model result into the facade's fenced turn.
///
/// The facade owns the fencing: timers bind the phase and incarnation that
/// scheduled them, effects bind the executing generation and incarnation and
/// must name a bound port, and the next phase must be declared by the facade
/// manifest.
pub fn lower_facade_turn(
    extension: &AdmittedSystemExtensionManifest,
    facade: &AdmittedFacadeManifest,
    state: &FacadeRuntimeState,
    model_turn: &FacadeModelTurn,
) -> Result<FacadeTurn, Vec<FacadeTurnIssue>> {
    let mut issues = Vec::new();
    let mut timer_intents = Vec::with_capacity(model_turn.timer_requests.len());
    for request in &model_turn.timer_requests {
        if facade.phase_entry(&request.phase).is_none() {
            issues.push(FacadeTurnIssue::UndeclaredPhase {
                phase: request.phase.clone(),
            });
            continue;
        }
        if request.phase != state.phase || request.incarnation != state.incarnation {
            issues.push(FacadeTurnIssue::StaleTimer {
                timer_ref: request.timer_ref.clone(),
                phase: request.phase.clone(),
                incarnation: state.incarnation,
            });
            continue;
        }
        let Some(deadline_tick) = request.logical_tick.checked_add(request.ticks) else {
            issues.push(FacadeTurnIssue::UnfencedTimer {
                timer_ref: request.timer_ref.clone(),
            });
            continue;
        };
        timer_intents.push(FacadeTimerIntent {
            timer_ref: request.timer_ref.clone(),
            phase: request.phase.clone(),
            incarnation: state.incarnation,
            logical_tick: request.logical_tick,
            deadline_tick,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let next_phase = if model_turn.completed {
        super::TASK_COMPLETED_PHASE.to_string()
    } else {
        model_turn.next_phase.clone()
    };
    if facade.profile == super::FacadeProfile::StateMachine
        && facade.phase_entry(&next_phase).is_none()
        && next_phase != super::TASK_COMPLETED_PHASE
    {
        return Err(vec![FacadeTurnIssue::UndeclaredPhase { phase: next_phase }]);
    }
    for intent in &model_turn.effect_intents {
        let is_bound = extension
            .all_port_requirements()
            .any(|requirement| requirement.port_id == intent.port_id && requirement.version == intent.port_version);
        if !is_bound {
            issues.push(FacadeTurnIssue::EffectNotBound {
                port_id: intent.port_id.clone(),
                port_version: intent.port_version.clone(),
            });
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let effects = model_turn
        .effect_intents
        .iter()
        .map(|intent| TypedEffectRequest {
            target: EffectTarget::FabricPort(FabricPortKey {
                port_id: intent.port_id.clone(),
                version: intent.port_version.clone(),
            }),
            operation: intent.operation.clone(),
            input_schema_ref: intent.input_schema_ref.clone(),
            output_schema_ref: intent.output_schema_ref.clone(),
            request_ref: intent.request_ref.clone(),
            generation: state.generation,
            incarnation: state.incarnation,
            accounted_bytes: intent.accounted_bytes,
        })
        .collect();
    Ok(FacadeTurn {
        next_phase,
        next_state_ref: model_turn.next_state_ref.clone(),
        replies: model_turn.replies.clone(),
        effects,
        timer_intents,
        completed: model_turn.completed,
        turn_sequence: state.turn_sequence + 1,
    })
}

/// Lower the facade's turn into the callback outcome the extension interface
/// already defines.
pub fn lower_facade_outcome(turn: &FacadeTurn) -> CallbackOutcome {
    CallbackOutcome {
        output_refs: turn.replies.clone(),
        effects: turn.effects.clone(),
        state_ref: turn.next_state_ref.clone(),
        checkpoint_ref: None,
        health: HealthState::Healthy,
    }
}

/// The state timeout the facade schedules for a phase, when the phase declares
/// one. The timeout is fenced by the phase and the current incarnation.
pub fn state_timeout_intent(
    entry: &FacadePhaseEntry,
    state: &FacadeRuntimeState,
    logical_tick: u64,
) -> Option<FacadeTimerIntent> {
    let ticks = entry.state_timeout_ticks?;
    let deadline_tick = logical_tick.checked_add(ticks)?;
    Some(FacadeTimerIntent {
        timer_ref: format!("{}-{}-{}", entry.phase, state.incarnation, deadline_tick),
        phase: entry.phase.clone(),
        incarnation: state.incarnation,
        logical_tick,
        deadline_tick,
    })
}
