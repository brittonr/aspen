
fn parse_observation(value: &Value<IoValue>) -> Result<Observation> {
    let value = value_to_iovalue(value);
    let observation = value
        .collect_simple_record("turn-observation-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <turn-observation-v1 ...>"))?;
    let arity = observation.len();
    if arity != 6 && arity != 7 {
        return Err(MoltenError::invalid_harness(format!(
            "turn observation arity {arity} is unsupported; expected 6 or 7"
        )));
    }
    let schema = required_string(&observation[0], "observation schema")?;
    if schema != crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported observation schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA
        )));
    }
    let events_index = if arity == 7 { 6 } else { 5 };
    let event_values = required_sequence(&observation[events_index], "observation events")?;
    let mut events = Vec::with_capacity(event_values.len());
    for event in event_values.iter() {
        events.push(value_to_iovalue(&event));
    }
    let mut computed_event_refs = Vec::with_capacity(events.len());
    for event in &events {
        computed_event_refs.push(canonical_hash(event)?);
    }
    let event_refs = if arity == 7 {
        required_record_hash_sequence(&observation[5], "event-refs", "observation event ref")?
    } else {
        computed_event_refs
    };
    let observation_ref = canonical_hash(&value)?;
    let index = required_u64(&observation[1], "observation index")?;
    let step_ref = required_hash(&observation[2], "observation step ref")?;
    let before_state_hash = required_hash(&observation[3], "observation before state hash")?;
    let after_state_hash = required_hash(&observation[4], "observation after state hash")?;
    Ok(Observation {
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

fn parse_step(value: &Value<IoValue>) -> Result<super::core::CoreStep> {
    if let Some(record) = value.collect_simple_record("send", Some(3)) {
        return Ok(super::core::CoreStep::Send {
            from: required_string(&record[0], "send from")?,
            to: required_string(&record[1], "send to")?,
            body: required_runtime_value(&record[2], "send body")?,
        });
    }
    if let Some(record) = value.collect_simple_record("observe", Some(2)) {
        return Ok(super::core::CoreStep::Observe {
            actor: required_string(&record[0], "observe actor")?,
            pattern: required_runtime_value(&record[1], "observe pattern")?,
        });
    }
    if let Some(record) = value.collect_simple_record("assert", Some(2)) {
        return Ok(super::core::CoreStep::Assert {
            actor: required_string(&record[0], "assert actor")?,
            value: required_runtime_value(&record[1], "assert value")?,
        });
    }
    if let Some(record) = value.collect_simple_record("retract", Some(2)) {
        return Ok(super::core::CoreStep::Retract {
            actor: required_string(&record[0], "retract actor")?,
            value: required_runtime_value(&record[1], "retract value")?,
        });
    }
    if let Some(record) = value.collect_simple_record("clock", Some(1)) {
        return Ok(super::core::CoreStep::Clock {
            actor: required_string(&record[0], "clock actor")?,
        });
    }
    if let Some(record) = value.collect_simple_record("random", Some(2)) {
        return Ok(super::core::CoreStep::Random {
            actor: required_string(&record[0], "random actor")?,
            upper: required_u64(&record[1], "random upper bound")?,
        });
    }
    Err(MoltenError::invalid_harness("unknown harness step record"))
}

fn tuple_set<T, F>(label: &'static str, values: &OrderedSet<T>, mut render: F) -> IoValue
where F: FnMut(&T) -> IoValue {
    record(label, vec![sequence(values.iter().map(&mut render).collect())])
}

fn effect_name(effect: &super::core::CoreEffect) -> &'static str {
    match effect {
        super::core::CoreEffect::Clock => "clock",
        super::core::CoreEffect::Random => "random",
    }
}

fn error_kind(error: &MoltenError) -> String {
    match error {
        MoltenError::Io(_) => "io".to_string(),
        MoltenError::Preserves(_) => "preserves".to_string(),
        MoltenError::InvalidHarness(_) => "invalid-harness".to_string(),
        MoltenError::HarnessDivergence(divergence) => divergence.kind.clone(),
    }
}

fn error_diagnostics(error: &MoltenError) -> Vec<IoValue> {
    match error {
        MoltenError::HarnessDivergence(divergence) => {
            let mut diagnostics = Vec::new();
            if let Some(step) = divergence.step {
                diagnostics.push(record("step", vec![u64_value(step)]));
            }
            diagnostics.push(record("expected", vec![string(&divergence.expected)]));
            diagnostics.push(record("actual", vec![string(&divergence.actual)]));
            diagnostics.push(record("detail", vec![string(&divergence.detail)]));
            diagnostics
        }
        MoltenError::Io(_) | MoltenError::Preserves(_) | MoltenError::InvalidHarness(_) => Vec::new(),
    }
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

fn value_has_record_label(value: &Value<IoValue>, label: &str) -> bool {
    value.collect_simple_record(label, None).is_some()
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_bool(value: &Value<IoValue>, field: &str) -> Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected boolean for {field}")))
}

fn optional_string(value: &Value<IoValue>, field: &str) -> Result<Option<String>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        required_string(value, field).map(Some)
    }
}

fn optional_request_string(value: &Value<IoValue>, field: &str) -> Result<Option<String>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], field).map(Some);
    }
    // Compatibility with early reports that encoded present optional strings directly.
    required_string(value, field).map(Some)
}

fn optional_request_runtime_value(value: &Value<IoValue>, _field: &str) -> Result<Option<super::core::RuntimeValue>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return super::core::RuntimeValue::new(value_to_iovalue(&some[0])).map(Some);
    }
    // Compatibility with early reports that encoded present optional values directly.
    super::core::RuntimeValue::new(value_to_iovalue(value)).map(Some)
}

fn optional_request_u64(value: &Value<IoValue>, field: &str) -> Result<Option<u64>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&some[0], field).map(Some);
    }
    // Compatibility with early reports that encoded present optional integers directly.
    required_u64(value, field).map(Some)
}

fn optional_action(value: &Value<IoValue>, field: &str) -> Result<Option<crate::runtime::AdmissionAction>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        parse_admission_action(&required_string(value, field)?).map(Some)
    }
}

fn optional_runtime_match_value(value: &Value<IoValue>) -> Result<Option<super::core::RuntimeValue>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        super::core::RuntimeValue::new(value_to_iovalue(value)).map(Some)
    }
}

fn required_hash(value: &Value<IoValue>, field: &str) -> Result<String> {
    let hash = required_string(value, field)?;
    validate_content_ref(&hash).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {hash}: {error}"))
    })?;
    Ok(hash)
}

fn required_runtime_value(value: &Value<IoValue>, _field: &str) -> Result<super::core::RuntimeValue> {
    super::core::RuntimeValue::new(value_to_iovalue(value))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_schema_roundtrip_preserves_canonical_hash() {
        let suite = parse_text(r#"<harness-suite-v1 "molten.harness.suite.v1" "roundtrip" 1 [<clock "actor">]>"#)
            .expect("parse suite");
        let parsed = parse_suite(&suite).expect("parse suite schema");
        let rendered = to_text(&parsed.source_value).expect("render suite");
        let reparsed = parse_text(&rendered).expect("reparse rendered suite");
        assert_eq!(canonical_hash(&suite).unwrap(), canonical_hash(&reparsed).unwrap());
    }
}
