
pub fn parse_budget(value: &IoValue) -> Result<BudgetEvidence> {
    let budget = simple_record(value, "budget-v1", 3)?;
    let limits = parse_budget_schema_and_limits(&budget)?;
    let usage_value = value_to_iovalue(&budget[2]);
    let usage = simple_record(&usage_value, "usage", 4)?;
    let usage = BudgetUsage {
        steps: required_u64(&usage[0], "budget used steps")?,
        effects: required_u64(&usage[1], "budget used effects")?,
        events: required_u64(&usage[2], "budget used events")?,
        report_bytes: required_u64(&usage[3], "budget used report bytes")?,
    };
    Ok(BudgetEvidence { limits, usage })
}

pub fn parse_budget_limits(value: &IoValue) -> Result<Budget> {
    let budget = simple_record(value, "budget-v1", 2)?;
    parse_budget_schema_and_limits(&budget)
}

fn limits_value(budget: &Budget) -> IoValue {
    record("limits", vec![
        u64_value(budget.max_steps),
        u64_value(budget.max_effects),
        u64_value(budget.max_events),
        u64_value(budget.max_report_bytes),
    ])
}

fn parse_budget_schema_and_limits(budget: &Record<Value<IoValue>>) -> Result<Budget> {
    let schema = required_string(&budget[0], "budget schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_SCHEMA
        )));
    }
    let limits_value = value_to_iovalue(&budget[1]);
    let limits = simple_record(&limits_value, "limits", 4)?;
    Ok(Budget {
        max_steps: required_u64(&limits[0], "budget max steps")?,
        max_effects: required_u64(&limits[1], "budget max effects")?,
        max_events: required_u64(&limits[2], "budget max events")?,
        max_report_bytes: required_u64(&limits[3], "budget max report bytes")?,
    })
}

pub fn parse_effect_log(value: &IoValue) -> Result<Vec<EffectLogEntry>> {
    let effect_log = simple_record(value, "effect-log-v1", 2)?;
    let schema = required_string(&effect_log[0], "effect log schema")?;
    if schema != crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported effect log schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA
        )));
    }
    let entry_values = required_sequence(&effect_log[1], "effect log entries")?;
    let mut entries = Vec::with_capacity(entry_values.len());
    for (position, entry) in entry_values.iter().enumerate() {
        let entry_value = value_to_iovalue(&entry);
        let entry_record = simple_record(&entry_value, "effect-entry", 3)?;
        let sequence = required_u64(&entry_record[0], "effect entry sequence")?;
        if sequence != position as u64 {
            return Err(MoltenError::invalid_harness(format!(
                "effect log sequence mismatch at position {position}: got {sequence}"
            )));
        }
        let request = value_to_iovalue(&entry_record[1]);
        let response = value_to_iovalue(&entry_record[2]);
        let request_sequence = effect_request_sequence(&request)?;
        let response_sequence = effect_response_sequence_and_value(&response)?.0;
        if sequence != request_sequence || sequence != response_sequence {
            return Err(MoltenError::invalid_harness(format!(
                "effect entry {sequence} request/response sequence mismatch"
            )));
        }
        entries.push(EffectLogEntry {
            sequence,
            request,
            response,
        });
    }
    Ok(entries)
}

pub fn effect_log_from_observations(observations: &[Observation]) -> Result<Vec<EffectLogEntry>> {
    let mut entries = Vec::new();
    let mut pending_request: Option<(u64, IoValue)> = None;
    for observation in observations {
        for event in &observation.events {
            match event_boundary(event) {
                EventBoundary::EffectRequest => {
                    let sequence = effect_request_sequence(event)?;
                    if pending_request.is_some() {
                        return Err(MoltenError::invalid_harness("nested effect request without response"));
                    }
                    pending_request = Some((sequence, event.clone()));
                }
                EventBoundary::EffectResponse => {
                    let (sequence, _value) = effect_response_sequence_and_value(event)?;
                    let Some((request_sequence, request)) = pending_request.take() else {
                        return Err(MoltenError::invalid_harness("effect response without request"));
                    };
                    if sequence != request_sequence {
                        return Err(MoltenError::invalid_harness(format!(
                            "effect response sequence {sequence} does not match request sequence {request_sequence}"
                        )));
                    }
                    push_bounded(
                        &mut entries,
                        EffectLogEntry {
                            sequence,
                            request,
                            response: event.clone(),
                        },
                        MAX_HARNESS_EFFECT_LOG_ENTRIES,
                        "harness effect log entries",
                    )?;
                }
                EventBoundary::PolicyDecision
                | EventBoundary::ActorInput
                | EventBoundary::HostcallRequest
                | EventBoundary::HostcallDecision
                | EventBoundary::ActorOutput
                | EventBoundary::SteelExecution
                | EventBoundary::WasmExecution
                | EventBoundary::RuntimePredicate
                | EventBoundary::Trace => {}
            }
        }
    }
    if pending_request.is_some() {
        return Err(MoltenError::invalid_harness("effect request without response"));
    }
    Ok(entries)
}

pub(crate) fn append_effect_entries_from_events(
    events: &[IoValue],
    entries: &mut impl crate::bounded::VecSink<EffectLogEntry>,
) -> Result<()> {
    let mut pending_request: Option<(u64, IoValue)> = None;
    for event in events {
        match event_boundary(event) {
            EventBoundary::EffectRequest => {
                let sequence = effect_request_sequence(event)?;
                if pending_request.is_some() {
                    return Err(MoltenError::invalid_harness("nested effect request without response"));
                }
                pending_request = Some((sequence, event.clone()));
            }
            EventBoundary::EffectResponse => {
                let (sequence, _value) = effect_response_sequence_and_value(event)?;
                let Some((request_sequence, request)) = pending_request.take() else {
                    return Err(MoltenError::invalid_harness("effect response without request"));
                };
                if sequence != request_sequence {
                    return Err(MoltenError::invalid_harness(format!(
                        "effect response sequence {sequence} does not match request sequence {request_sequence}"
                    )));
                }
                push_bounded(
                    &mut *entries,
                    EffectLogEntry {
                        sequence,
                        request,
                        response: event.clone(),
                    },
                    MAX_HARNESS_EFFECT_LOG_ENTRIES,
                    "harness effect log entries",
                )?;
            }
            EventBoundary::PolicyDecision
            | EventBoundary::ActorInput
            | EventBoundary::HostcallRequest
            | EventBoundary::HostcallDecision
            | EventBoundary::ActorOutput
            | EventBoundary::SteelExecution
            | EventBoundary::WasmExecution
            | EventBoundary::RuntimePredicate
            | EventBoundary::Trace => {}
        }
    }
    if pending_request.is_some() {
        return Err(MoltenError::invalid_harness("effect request without response"));
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

pub fn effect_response_sequence_and_value(value: &IoValue) -> Result<(u64, u64)> {
    let response = value
        .collect_simple_record("effect-response", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected effect-response record"))?;
    let arity = response.fields_iter().count();
    if arity != 4 && arity != 5 {
        return Err(MoltenError::invalid_harness(format!("effect-response arity must be 4 or 5, got {arity}")));
    }
    let sequence = required_u64(&response[2], "effect response sequence")?;
    let value_index = arity - 1;
    let value = required_u64(&response[value_index], "effect response value")?;
    Ok((sequence, value))
}

pub fn effect_request_sequence(value: &IoValue) -> Result<u64> {
    let request = value
        .collect_simple_record("effect-request", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected effect-request record"))?;
    let arity = request.fields_iter().count();
    if arity != 3 && arity != 4 {
        return Err(MoltenError::invalid_harness(format!("effect-request arity must be 3 or 4, got {arity}")));
    }
    required_u64(&request[2], "effect request sequence")
}

pub fn event_boundary(value: &IoValue) -> EventBoundary {
    if value.collect_simple_record("effect-request", None).is_some() {
        return EventBoundary::EffectRequest;
    }
    if value.collect_simple_record("effect-response", None).is_some() {
        return EventBoundary::EffectResponse;
    }
    if value.collect_simple_record("admission-decision-v1", None).is_some() {
        return EventBoundary::PolicyDecision;
    }
    if value.collect_simple_record("actor-input-v1", None).is_some() {
        return EventBoundary::ActorInput;
    }
    if value.collect_simple_record("hostcall-request-v1", None).is_some() {
        return EventBoundary::HostcallRequest;
    }
    if value.collect_simple_record("hostcall-decision-v1", None).is_some() {
        return EventBoundary::HostcallDecision;
    }
    if value.collect_simple_record("actor-output-v1", None).is_some() {
        return EventBoundary::ActorOutput;
    }
    if value.collect_simple_record("steel-execution-receipt-v1", None).is_some() {
        return EventBoundary::SteelExecution;
    }
    if value.collect_simple_record("wasm-execution-receipt-v1", None).is_some() {
        return EventBoundary::WasmExecution;
    }
    if value.collect_simple_record("runtime-predicate-receipt-v1", None).is_some() {
        return EventBoundary::RuntimePredicate;
    }
    EventBoundary::Trace
}
