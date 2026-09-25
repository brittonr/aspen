
fn parse_entity_kind(value: &str) -> Result<EntityKind> {
    match value {
        "actor" => Ok(EntityKind::Actor),
        "service" => Ok(EntityKind::Service),
        "vat" => Ok(EntityKind::Vat),
        "session" => Ok(EntityKind::Session),
        "handler" => Ok(EntityKind::Handler),
        "job" => Ok(EntityKind::Job),
        _ => Err(MoltenError::invalid_harness(format!("unknown lifecycle entity kind {value}"))),
    }
}

fn parse_state(value: &str) -> Result<State> {
    match value {
        "declared" => Ok(State::Declared),
        "spawning" => Ok(State::Spawning),
        "starting" => Ok(State::Starting),
        "ready" => Ok(State::Ready),
        "degraded" => Ok(State::Degraded),
        "stopping" => Ok(State::Stopping),
        "stopped" => Ok(State::Stopped),
        "failed" => Ok(State::Failed),
        "restarting" => Ok(State::Restarting),
        "cleaned" => Ok(State::Cleaned),
        _ => Err(MoltenError::invalid_harness(format!("unknown lifecycle state {value}"))),
    }
}

fn parse_action(value: &str) -> Result<Action> {
    match value {
        "spawn" => Ok(Action::Spawn),
        "start" => Ok(Action::Start),
        "ready" => Ok(Action::Ready),
        "degrade" => Ok(Action::Degrade),
        "fail" => Ok(Action::Fail),
        "restart" => Ok(Action::Restart),
        "stop" => Ok(Action::Stop),
        "cleanup" => Ok(Action::Cleanup),
        "supervisor-decision" => Ok(Action::SupervisorDecision),
        _ => Err(MoltenError::invalid_harness(format!("unknown lifecycle action {value}"))),
    }
}

fn restart_window_value(window: Option<&RestartWindow>) -> IoValue {
    match window {
        Some(window) => record("restart-window", vec![
            u64_value(window.start_step),
            u64_value(window.end_step),
            u64_value(window.max_restarts),
        ]),
        None => record("restart-window-none", Vec::new()),
    }
}

fn cleanup_removed_anything(cleanup: &RuntimeScopeCleanup) -> bool {
    !cleanup.assertion_refs.is_empty() || !cleanup.observer_refs.is_empty() || !cleanup.message_refs.is_empty()
}

fn pending_turn_value(turn: &PendingTurn) -> Result<IoValue> {
    let mut actions = Vec::with_capacity(turn.actions.len());
    for action in &turn.actions {
        actions.push(turn_action_value(action));
    }
    Ok(record("runtime-pending-turn-v1", vec![sequence(actions)]))
}

fn pending_action_refs(turn: &PendingTurn) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(turn.actions.len());
    for action in &turn.actions {
        refs.push(canonical_hash(&turn_action_value(action))?);
    }
    refs.sort();
    Ok(refs)
}

fn turn_action_value(action: &TurnAction) -> IoValue {
    match action {
        TurnAction::Send(message) => record("runtime-turn-action-send-v1", vec![message.to_value()]),
        TurnAction::Observe(observer) => record("runtime-turn-action-observe-v1", vec![observer.to_value()]),
        TurnAction::Assert(assertion) => record("runtime-turn-action-assert-v1", vec![assertion.to_value()]),
        TurnAction::Retract(assertion) => record("runtime-turn-action-retract-v1", vec![assertion.to_value()]),
    }
}

pub fn transition_diagnostics(input: &TransitionInput) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(2));
    if !action_matches_target(input.action, input.to_state) {
        diagnostics.push(format!(
            "action {} does not match target state {}",
            input.action.as_str(),
            input.to_state.as_str()
        ));
    }
    if !allowed_transition(input.from_state, input.to_state) {
        diagnostics.push(format!("invalid transition {} -> {}", input.from_state.as_str(), input.to_state.as_str()));
    }
    diagnostics
}

pub fn action_matches_target(action: Action, to_state: State) -> bool {
    action == Action::SupervisorDecision
        || LIFECYCLE_ACTION_TARGETS
            .iter()
            .any(|target| target.action == action && target.to_state == to_state)
}

pub fn allowed_transition(from_state: State, to_state: State) -> bool {
    LIFECYCLE_TRANSITIONS
        .iter()
        .any(|transition| transition.from_state == from_state && transition.to_state == to_state)
}

pub fn lifecycle_successor_states(from_state: State) -> Vec<State> {
    LIFECYCLE_TRANSITIONS
        .iter()
        .filter(|transition| transition.from_state == from_state)
        .map(|transition| transition.to_state)
        .collect()
}

pub fn reachable_lifecycle_states(from_state: State) -> Vec<State> {
    let mut reachable = Vec::with_capacity(LIFECYCLE_STATE_COUNT);
    reachable.push(from_state);
    let mut cursor = 0;
    while cursor < reachable.len() {
        let current = reachable[cursor];
        cursor += 1;
        for transition in LIFECYCLE_TRANSITIONS
            .iter()
            .filter(|transition| transition.from_state == current)
        {
            if !reachable.contains(&transition.to_state) {
                reachable.push(transition.to_state);
            }
        }
    }
    reachable
}

pub fn lifecycle_state_reachable(from_state: State, to_state: State) -> bool {
    reachable_lifecycle_states(from_state).contains(&to_state)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |reference| record("some", vec![string(reference)]))
}

fn checks_value() -> IoValue {
    record("checks", vec![
        bool_value(true),
        sequence(vec![
            string("molten-lifecycle-local-semantics"),
            string("no-otp-compatibility-claim"),
            string("canonical-transition-evidence"),
        ]),
    ])
}
