use preserves::ValueImpl;

pub(super) fn parse_observation(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<super::observed::Observation> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let observation = value
        .collect_simple_record("turn-observation-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <turn-observation-v1 ...>"))?;
    let arity = observation.len();
    if arity != 6 && arity != 7 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "turn observation arity {arity} is unsupported; expected 6 or 7"
        )));
    }
    let schema = super::fields::required_string(&observation[0], "observation schema")?;
    if schema != crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported observation schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA
        )));
    }
    let events_index = if arity == 7 { 6 } else { 5 };
    let event_values = super::fields::required_sequence(&observation[events_index], "observation events")?;
    let mut events = Vec::with_capacity(event_values.len());
    for event in event_values.iter() {
        events.push(crate::preserves_rail::value_to_iovalue(&event));
    }
    let mut computed_event_refs = Vec::with_capacity(events.len());
    for event in &events {
        computed_event_refs.push(crate::preserves_rail::canonical_hash(event)?);
    }
    let event_refs = if arity == 7 {
        super::fields::required_record_hash_sequence(&observation[5], "event-refs", "observation event ref")?
    } else {
        computed_event_refs
    };
    let observation_ref = crate::preserves_rail::canonical_hash(&value)?;
    let index = super::fields::required_u64(&observation[1], "observation index")?;
    let step_ref = super::fields::required_hash(&observation[2], "observation step ref")?;
    let before_state_hash = super::fields::required_hash(&observation[3], "observation before state hash")?;
    let after_state_hash = super::fields::required_hash(&observation[4], "observation after state hash")?;
    Ok(super::observed::Observation {
        value,
        observation_ref,
        index,
        step_ref,
        before_state_hash,
        after_state_hash,
        event_refs,
        events,
    })
}

pub(super) fn parse_step(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::harness::nucleus::CoreStep> {
    if let Some(record) = value.collect_simple_record("send", Some(3)) {
        return Ok(crate::harness::nucleus::CoreStep::Send {
            from: super::fields::required_string(&record[0], "send from")?,
            to: super::fields::required_string(&record[1], "send to")?,
            body: super::fields::required_runtime_value(&record[2], "send body")?,
        });
    }
    if let Some(record) = value.collect_simple_record("observe", Some(2)) {
        return Ok(crate::harness::nucleus::CoreStep::Observe {
            actor: super::fields::required_string(&record[0], "observe actor")?,
            pattern: super::fields::required_runtime_value(&record[1], "observe pattern")?,
        });
    }
    if let Some(record) = value.collect_simple_record("assert", Some(2)) {
        return Ok(crate::harness::nucleus::CoreStep::Assert {
            actor: super::fields::required_string(&record[0], "assert actor")?,
            value: super::fields::required_runtime_value(&record[1], "assert value")?,
        });
    }
    if let Some(record) = value.collect_simple_record("retract", Some(2)) {
        return Ok(crate::harness::nucleus::CoreStep::Retract {
            actor: super::fields::required_string(&record[0], "retract actor")?,
            value: super::fields::required_runtime_value(&record[1], "retract value")?,
        });
    }
    if let Some(record) = value.collect_simple_record("clock", Some(1)) {
        return Ok(crate::harness::nucleus::CoreStep::Clock {
            actor: super::fields::required_string(&record[0], "clock actor")?,
        });
    }
    if let Some(record) = value.collect_simple_record("random", Some(2)) {
        return Ok(crate::harness::nucleus::CoreStep::Random {
            actor: super::fields::required_string(&record[0], "random actor")?,
            upper: super::fields::required_u64(&record[1], "random upper bound")?,
        });
    }
    Err(crate::error::Failure::invalid_harness("unknown harness step record"))
}
