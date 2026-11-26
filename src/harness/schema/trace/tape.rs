use preserves::ValueImpl;

const MAX_HARNESS_EFFECT_LOG_ENTRIES: usize = 100_000;

const _: () = assert!(MAX_HARNESS_EFFECT_LOG_ENTRIES <= 1_000_000);

pub fn effect_log_value(entries: &[super::observed::EffectLogEntry]) -> preserves::IOValue {
    crate::preserves_rail::record("effect-log-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA),
        crate::preserves_rail::sequence(
            entries
                .iter()
                .map(|entry| {
                    crate::preserves_rail::record("effect-entry", vec![
                        crate::preserves_rail::u64_value(entry.sequence),
                        entry.request.clone(),
                        entry.response.clone(),
                    ])
                })
                .collect(),
        ),
    ])
}

pub(super) fn parse_effect_log(
    value: &preserves::IOValue,
) -> crate::error::Result<Vec<super::observed::EffectLogEntry>> {
    let effect_log = super::fields::simple_record(value, "effect-log-v1", 2)?;
    let schema = super::fields::required_string(&effect_log[0], "effect log schema")?;
    if schema != crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported effect log schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA
        )));
    }
    let entry_values = super::fields::required_sequence(&effect_log[1], "effect log entries")?;
    let mut entries = Vec::with_capacity(entry_values.len());
    for (position, entry) in entry_values.iter().enumerate() {
        let entry_value = crate::preserves_rail::value_to_iovalue(&entry);
        let entry_record = super::fields::simple_record(&entry_value, "effect-entry", 3)?;
        let sequence = super::fields::required_u64(&entry_record[0], "effect entry sequence")?;
        if sequence != position as u64 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "effect log sequence mismatch at position {position}: got {sequence}"
            )));
        }
        let request = crate::preserves_rail::value_to_iovalue(&entry_record[1]);
        let response = crate::preserves_rail::value_to_iovalue(&entry_record[2]);
        let request_sequence = effect_request_sequence(&request)?;
        let response_sequence = effect_response_sequence_and_value(&response)?.0;
        if sequence != request_sequence || sequence != response_sequence {
            return Err(crate::error::Failure::invalid_harness(format!(
                "effect entry {sequence} request/response sequence mismatch"
            )));
        }
        entries.push(super::observed::EffectLogEntry {
            sequence,
            request,
            response,
        });
    }
    Ok(entries)
}

pub fn effect_log_from_observations(
    observations: &[super::observed::Observation],
) -> crate::error::Result<Vec<super::observed::EffectLogEntry>> {
    let mut entries = Vec::new();
    let mut pending_request: Option<(u64, preserves::IOValue)> = None;
    for observation in observations {
        for event in &observation.events {
            match event_boundary(event) {
                super::observed::EventBoundary::EffectRequest => {
                    let sequence = effect_request_sequence(event)?;
                    if pending_request.is_some() {
                        return Err(crate::error::Failure::invalid_harness("nested effect request without response"));
                    }
                    pending_request = Some((sequence, event.clone()));
                }
                super::observed::EventBoundary::EffectResponse => {
                    let (sequence, _value) = effect_response_sequence_and_value(event)?;
                    let Some((request_sequence, request)) = pending_request.take() else {
                        return Err(crate::error::Failure::invalid_harness("effect response without request"));
                    };
                    if sequence != request_sequence {
                        return Err(crate::error::Failure::invalid_harness(format!(
                            "effect response sequence {sequence} does not match request sequence {request_sequence}"
                        )));
                    }
                    push_bounded(
                        &mut entries,
                        super::observed::EffectLogEntry {
                            sequence,
                            request,
                            response: event.clone(),
                        },
                        MAX_HARNESS_EFFECT_LOG_ENTRIES,
                        "harness effect log entries",
                    )?;
                }
                super::observed::EventBoundary::PolicyDecision
                | super::observed::EventBoundary::ActorInput
                | super::observed::EventBoundary::HostcallRequest
                | super::observed::EventBoundary::HostcallDecision
                | super::observed::EventBoundary::ActorOutput
                | super::observed::EventBoundary::SteelExecution
                | super::observed::EventBoundary::WasmExecution
                | super::observed::EventBoundary::RuntimePredicate
                | super::observed::EventBoundary::Trace => {}
            }
        }
    }
    if pending_request.is_some() {
        return Err(crate::error::Failure::invalid_harness("effect request without response"));
    }
    Ok(entries)
}

pub fn append_effect_entries_from_events(
    events: &[preserves::IOValue],
    entries: &mut impl crate::bounded::VecSink<super::observed::EffectLogEntry>,
) -> crate::error::Result<()> {
    let mut pending_request: Option<(u64, preserves::IOValue)> = None;
    for event in events {
        match event_boundary(event) {
            super::observed::EventBoundary::EffectRequest => {
                let sequence = effect_request_sequence(event)?;
                if pending_request.is_some() {
                    return Err(crate::error::Failure::invalid_harness("nested effect request without response"));
                }
                pending_request = Some((sequence, event.clone()));
            }
            super::observed::EventBoundary::EffectResponse => {
                let (sequence, _value) = effect_response_sequence_and_value(event)?;
                let Some((request_sequence, request)) = pending_request.take() else {
                    return Err(crate::error::Failure::invalid_harness("effect response without request"));
                };
                if sequence != request_sequence {
                    return Err(crate::error::Failure::invalid_harness(format!(
                        "effect response sequence {sequence} does not match request sequence {request_sequence}"
                    )));
                }
                push_bounded(
                    &mut *entries,
                    super::observed::EffectLogEntry {
                        sequence,
                        request,
                        response: event.clone(),
                    },
                    MAX_HARNESS_EFFECT_LOG_ENTRIES,
                    "harness effect log entries",
                )?;
            }
            super::observed::EventBoundary::PolicyDecision
            | super::observed::EventBoundary::ActorInput
            | super::observed::EventBoundary::HostcallRequest
            | super::observed::EventBoundary::HostcallDecision
            | super::observed::EventBoundary::ActorOutput
            | super::observed::EventBoundary::SteelExecution
            | super::observed::EventBoundary::WasmExecution
            | super::observed::EventBoundary::RuntimePredicate
            | super::observed::EventBoundary::Trace => {}
        }
    }
    if pending_request.is_some() {
        return Err(crate::error::Failure::invalid_harness("effect request without response"));
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> crate::error::Result<()> {
    if count > maximum {
        Err(crate::error::Failure::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

pub(super) fn push_bounded<T>(
    values: &mut impl crate::bounded::VecSink<T>,
    value: T,
    maximum: usize,
    label: &str,
) -> crate::error::Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

pub fn effect_response_sequence_and_value(value: &preserves::IOValue) -> crate::error::Result<(u64, u64)> {
    let response = value
        .collect_simple_record("effect-response", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected effect-response record"))?;
    let arity = response.fields_iter().count();
    if arity != 4 && arity != 5 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "effect-response arity must be 4 or 5, got {arity}"
        )));
    }
    let sequence = super::fields::required_u64(&response[2], "effect response sequence")?;
    let value_index = arity - 1;
    let value = super::fields::required_u64(&response[value_index], "effect response value")?;
    Ok((sequence, value))
}

pub fn effect_request_sequence(value: &preserves::IOValue) -> crate::error::Result<u64> {
    let request = value
        .collect_simple_record("effect-request", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected effect-request record"))?;
    let arity = request.fields_iter().count();
    if arity != 3 && arity != 4 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "effect-request arity must be 3 or 4, got {arity}"
        )));
    }
    super::fields::required_u64(&request[2], "effect request sequence")
}

pub fn event_boundary(value: &preserves::IOValue) -> super::observed::EventBoundary {
    if value.collect_simple_record("effect-request", None).is_some() {
        return super::observed::EventBoundary::EffectRequest;
    }
    if value.collect_simple_record("effect-response", None).is_some() {
        return super::observed::EventBoundary::EffectResponse;
    }
    if value.collect_simple_record("admission-decision-v1", None).is_some() {
        return super::observed::EventBoundary::PolicyDecision;
    }
    if value.collect_simple_record("actor-input-v1", None).is_some() {
        return super::observed::EventBoundary::ActorInput;
    }
    if value.collect_simple_record("hostcall-request-v1", None).is_some() {
        return super::observed::EventBoundary::HostcallRequest;
    }
    if value.collect_simple_record("hostcall-decision-v1", None).is_some() {
        return super::observed::EventBoundary::HostcallDecision;
    }
    if value.collect_simple_record("actor-output-v1", None).is_some() {
        return super::observed::EventBoundary::ActorOutput;
    }
    if value.collect_simple_record("steel-execution-receipt-v1", None).is_some() {
        return super::observed::EventBoundary::SteelExecution;
    }
    if value.collect_simple_record("wasm-execution-receipt-v1", None).is_some() {
        return super::observed::EventBoundary::WasmExecution;
    }
    if value.collect_simple_record("runtime-predicate-receipt-v1", None).is_some() {
        return super::observed::EventBoundary::RuntimePredicate;
    }
    super::observed::EventBoundary::Trace
}
