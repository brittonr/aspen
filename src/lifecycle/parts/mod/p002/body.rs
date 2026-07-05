
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

const FIELD_VALUE_INDEX: usize = 0;
const FIELD_SECOND_VALUE_INDEX: usize = 1;
const FIELD_RECORD_ARITY: usize = 1;
const EMPTY_RECORD_ARITY: usize = 0;
const ENTITY_FIELD_ARITY: usize = 2;
const STATE_FIELD_ARITY: usize = 2;
const TRANSITION_VALUE_ARITY: usize = 11;
const TRANSITION_SCHEMA_INDEX: usize = 0;
const TRANSITION_ENTITY_INDEX: usize = 1;
const TRANSITION_STATE_INDEX: usize = 2;
const TRANSITION_ACTION_INDEX: usize = 3;
const TRANSITION_CAUSE_INDEX: usize = 4;
const TRANSITION_POLICY_INDEX: usize = 5;
const TRANSITION_RESOURCES_INDEX: usize = 6;
const TRANSITION_EVIDENCE_INDEX: usize = 7;
const TRANSITION_SUPERVISOR_INDEX: usize = 8;
const TRANSITION_LOGICAL_STEP_INDEX: usize = 9;
const TRANSITION_CHECKS_INDEX: usize = 10;
const RECEIPT_VALUE_ARITY: usize = 5;
const RECEIPT_SCHEMA_INDEX: usize = 0;
const RECEIPT_TRANSITION_INDEX: usize = 1;
const RECEIPT_DECISION_INDEX: usize = 2;
const RECEIPT_DIAGNOSTICS_INDEX: usize = 3;
const RECEIPT_CHECKS_INDEX: usize = 4;

pub fn validate_transition_receipt(
    transition_value: &IoValue,
    receipt_value: &IoValue,
    expected_receipt_ref: Option<&str>,
) -> Result<TransitionReceiptValidation> {
    let input = parse_transition_value(transition_value)?;
    let expected_transition = transition_record(&input)?;
    if expected_transition.value != *transition_value {
        return Err(MoltenError::invalid_harness(
            "lifecycle transition value does not match parsed transition input",
        ));
    }
    let transition_ref = canonical_hash(transition_value)?;
    if transition_ref != expected_transition.transition_ref {
        return Err(MoltenError::invalid_harness("lifecycle transition ref mismatch"));
    }
    let receipt = parse_transition_receipt_value(receipt_value, expected_receipt_ref)?;
    if receipt.transition_ref != transition_ref {
        return Err(MoltenError::invalid_harness("lifecycle receipt transition ref mismatch"));
    }
    let expected_diagnostics = transition_diagnostics(&input);
    if receipt.diagnostics != expected_diagnostics {
        return Err(MoltenError::invalid_harness("lifecycle receipt diagnostics mismatch"));
    }
    let expected_decision = if expected_diagnostics.is_empty() { "pass" } else { "deny" };
    if receipt.decision != expected_decision {
        return Err(MoltenError::invalid_harness("lifecycle receipt decision mismatch"));
    }
    Ok(receipt)
}

fn parse_transition_value(value: &IoValue) -> Result<TransitionInput> {
    let fields = simple_record(value, "lifecycle-transition-v1", TRANSITION_VALUE_ARITY)?;
    require_schema(
        &fields[TRANSITION_SCHEMA_INDEX],
        crate::preserves_rail::LIFECYCLE_TRANSITION_SCHEMA,
        "lifecycle transition",
    )?;
    let entity = simple_record_field(&fields[TRANSITION_ENTITY_INDEX], "entity", ENTITY_FIELD_ARITY)?;
    let state = simple_record_field(&fields[TRANSITION_STATE_INDEX], "state", STATE_FIELD_ARITY)?;
    let from_state = record_state(&state[FIELD_VALUE_INDEX], "from")?;
    let to_state = record_state(&state[FIELD_SECOND_VALUE_INDEX], "to")?;
    let supervisor_ref = record_optional_ref(&fields[TRANSITION_SUPERVISOR_INDEX], "supervisor")?;
    let input = TransitionInput {
        entity_kind: parse_entity_kind(&required_string(&entity[FIELD_VALUE_INDEX], "entity kind")?)?,
        entity_id: required_string(&entity[FIELD_SECOND_VALUE_INDEX], "entity id")?,
        from_state,
        to_state,
        action: parse_action(&record_string(&fields[TRANSITION_ACTION_INDEX], "action")?)?,
        cause: record_string(&fields[TRANSITION_CAUSE_INDEX], "cause")?,
        policy_refs: record_string_sequence(&fields[TRANSITION_POLICY_INDEX], "policy")?,
        resource_refs: record_string_sequence(&fields[TRANSITION_RESOURCES_INDEX], "resources")?,
        evidence_refs: record_string_sequence(&fields[TRANSITION_EVIDENCE_INDEX], "evidence")?,
        supervisor_ref,
        logical_step: record_u64(&fields[TRANSITION_LOGICAL_STEP_INDEX], "logical-step")?,
    };
    if crate::preserves_rail::value_to_iovalue(&fields[TRANSITION_CHECKS_INDEX]) != checks_value() {
        return Err(MoltenError::invalid_harness("lifecycle transition checks mismatch"));
    }
    validate_transition_input(&input)?;
    Ok(input)
}

fn parse_transition_receipt_value(
    receipt_value: &IoValue,
    expected_receipt_ref: Option<&str>,
) -> Result<TransitionReceiptValidation> {
    let fields = simple_record(receipt_value, "lifecycle-transition-receipt-v1", RECEIPT_VALUE_ARITY)?;
    require_schema(
        &fields[RECEIPT_SCHEMA_INDEX],
        crate::preserves_rail::LIFECYCLE_TRANSITION_RECEIPT_SCHEMA,
        "lifecycle transition receipt",
    )?;
    let transition_ref = record_string(&fields[RECEIPT_TRANSITION_INDEX], "transition")?;
    validate_content_ref(&transition_ref)?;
    let decision = record_string(&fields[RECEIPT_DECISION_INDEX], "decision")?;
    if decision != "pass" && decision != "deny" {
        return Err(MoltenError::invalid_harness("lifecycle receipt decision must be pass or deny"));
    }
    let diagnostics = record_string_sequence(&fields[RECEIPT_DIAGNOSTICS_INDEX], "diagnostics")?;
    if diagnostics.len() > MAX_DIAGNOSTICS {
        return Err(MoltenError::invalid_harness("lifecycle receipt diagnostics exceed bound"));
    }
    if crate::preserves_rail::value_to_iovalue(&fields[RECEIPT_CHECKS_INDEX]) != checks_value() {
        return Err(MoltenError::invalid_harness("lifecycle receipt checks mismatch"));
    }
    let receipt_ref = canonical_hash(receipt_value)?;
    if let Some(expected) = expected_receipt_ref
        && receipt_ref != expected
    {
        return Err(MoltenError::invalid_harness("lifecycle receipt hash mismatch"));
    }
    Ok(TransitionReceiptValidation {
        receipt_ref,
        transition_ref,
        decision,
        diagnostics,
        value: receipt_value.clone(),
    })
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_record_field<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field with arity {arity}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let field = simple_record_field(value, label, FIELD_RECORD_ARITY)?;
    required_string(&field[FIELD_VALUE_INDEX], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let field = simple_record_field(value, label, FIELD_RECORD_ARITY)?;
    field[FIELD_VALUE_INDEX]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_state(value: &Value<IoValue>, label: &str) -> Result<State> {
    parse_state(&record_string(value, label)?)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_sequence(value, label)?.iter().map(|item| required_iovalue_string(item, label)).collect()
}

fn record_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let field = simple_record_field(value, label, FIELD_RECORD_ARITY)?;
    let sequence = field[FIELD_VALUE_INDEX]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(sequence.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let field = simple_record_field(value, label, FIELD_RECORD_ARITY)?;
    let optional = crate::preserves_rail::value_to_iovalue(&field[FIELD_VALUE_INDEX]);
    if optional.collect_simple_record("none", Some(EMPTY_RECORD_ARITY)).is_some() {
        return Ok(None);
    }
    let some = optional
        .collect_simple_record("some", Some(FIELD_RECORD_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    let reference = required_string(&some[FIELD_VALUE_INDEX], label)?;
    validate_content_ref(&reference)?;
    Ok(Some(reference))
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} schema {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_iovalue_string(value: &IoValue, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

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
