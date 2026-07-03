
fn validate_refs(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_REFS {
        return Err(MoltenError::invalid_harness(format!("{label} refs exceed lifecycle bound")));
    }
    let mut prior: Option<&str> = None;
    for reference in refs {
        validate_content_ref(reference)?;
        if let Some(prior_ref) = prior
            && prior_ref >= reference.as_str()
        {
            return Err(MoltenError::invalid_harness(format!("{label} refs must be sorted and unique")));
        }
        prior = Some(reference);
    }
    Ok(())
}

fn validate_turn_failure_input(input: &TurnFailureInput<'_>) -> Result<()> {
    if input.entity_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("turn failure entity id must be non-empty"));
    }
    if input.cause.trim().is_empty() {
        return Err(MoltenError::invalid_harness("turn failure cause must be non-empty"));
    }
    validate_refs("vat delta", input.vat_delta_refs)?;
    validate_refs("one-shot effect", input.one_shot_effect_refs)?;
    validate_refs("policy", input.policy_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn validate_scope_cleanup_input(input: &ScopeCleanupInput<'_>) -> Result<()> {
    if input.entity_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("scope cleanup entity id must be non-empty"));
    }
    if input.cause.trim().is_empty() {
        return Err(MoltenError::invalid_harness("scope cleanup cause must be non-empty"));
    }
    validate_refs("assertion", &input.cleanup.assertion_refs)?;
    validate_refs("subscription", &input.cleanup.observer_refs)?;
    validate_refs("message", &input.cleanup.message_refs)?;
    validate_refs("live ref", input.live_ref_refs)?;
    validate_refs("resource", input.resource_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn validate_monitor_input(input: &MonitorInput<'_>) -> Result<()> {
    if input.observer_id.trim().is_empty() || input.child_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("monitor observer and child ids must be non-empty"));
    }
    validate_content_ref(input.child_failure_ref)?;
    validate_refs("policy", input.policy_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn validate_supervisor_input(input: &SupervisorDecisionInput<'_>) -> Result<()> {
    if input.policy.supervisor_id.trim().is_empty() || input.child_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("supervisor and child ids must be non-empty"));
    }
    validate_content_ref(input.child_failure_ref)?;
    validate_refs("policy", &input.policy.policy_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    if let Some(window) = input.policy.restart_window.as_ref()
        && window.end_step < window.start_step
    {
        return Err(MoltenError::invalid_harness("restart window end precedes start"));
    }
    Ok(())
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

fn transition_diagnostics(input: &TransitionInput) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lifecycle/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lifecycle/parts/mod/tests/m000/p001/body.rs"));
}
