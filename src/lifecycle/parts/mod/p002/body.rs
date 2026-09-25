
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
