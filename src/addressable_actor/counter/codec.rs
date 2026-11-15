const MAX_BYTES: usize = 65_536;
type Field = preserves::Value<preserves::IOValue>;

pub(super) fn validate_add(input: &super::Add) -> crate::error::Result<()> {
    crate::preserves_rail::ContentRef::parse(&input.actor_key_ref)?;
    crate::preserves_rail::ContentRef::parse(&input.request_id)?;
    if input.increment == 0 {
        return Err(super::invalid("counter increment must be positive"));
    }
    Ok(())
}

pub(super) fn add_value(input: &super::Add) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
    validate_add(input)?;
    materialize(crate::preserves_rail::record("addressable-counter-add-v1", vec![
        crate::preserves_rail::string(&input.actor_key_ref),
        crate::preserves_rail::string(&input.request_id),
        crate::preserves_rail::u64_value(input.increment),
    ]))
}

pub(super) fn decode_add(bytes: &[u8]) -> crate::error::Result<super::Add> {
    let value = bounded_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&value, "addressable-counter-add-v1", 3)?;
    let input = super::Add {
        actor_key_ref: reference(&fields[0])?,
        request_id: reference(&fields[1])?,
        increment: number(&fields[2])?,
    };
    validate_add(&input)?;
    Ok(input)
}

pub(super) fn state_value(state: &super::State) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
    crate::preserves_rail::ContentRef::parse(&state.actor_key_ref)?;
    if state.completed.len() > super::MAX_COMPLETED_REQUESTS {
        return Err(super::invalid("counter state exceeds completed request bound"));
    }
    materialize(crate::preserves_rail::record("addressable-counter-state-v1", vec![
        crate::preserves_rail::string(&state.actor_key_ref),
        crate::preserves_rail::u64_value(state.value),
        crate::preserves_rail::sequence(
            state
                .completed
                .iter()
                .map(|done| {
                    crate::preserves_rail::record("completed", vec![
                        crate::preserves_rail::string(&done.request_id),
                        crate::preserves_rail::u64_value(done.increment),
                        crate::preserves_rail::u64_value(done.result),
                    ])
                })
                .collect(),
        ),
    ]))
}

pub(super) fn decode_state(bytes: &[u8]) -> crate::error::Result<super::State> {
    let value = bounded_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&value, "addressable-counter-state-v1", 3)?;
    let mut state = super::State {
        actor_key_ref: reference(&fields[0])?,
        value: 0,
        completed: Vec::new(),
    };
    let completed = crate::preserves_rail::required_sequence_field(&fields[2], "counter completed requests")?;
    if completed.len() > super::MAX_COMPLETED_REQUESTS {
        return Err(super::invalid("counter state exceeds completed request bound"));
    }
    state.completed.reserve_exact(completed.len());
    for value in completed.iter() {
        let record = crate::preserves_rail::value_to_iovalue(value);
        let row = crate::preserves_rail::simple_record_fields(&record, "completed", 3)?;
        let request_id = reference(&row[0])?;
        let increment = number(&row[1])?;
        let result = number(&row[2])?;
        if increment == 0 || state.completed.iter().any(|done| done.request_id == request_id) {
            return Err(super::invalid("counter state contains invalid or duplicate logical requests"));
        }
        let next = state.value.checked_add(increment).ok_or_else(|| super::invalid("counter state overflow"))?;
        if next != result {
            return Err(super::invalid("counter state has inconsistent historical result"));
        }
        state.completed.push(super::Completed {
            request_id,
            increment,
            result,
        });
        state.value = next;
    }
    if state.value != number(&fields[1])? {
        return Err(super::invalid("counter state has inconsistent total"));
    }
    Ok(state)
}

pub(super) fn result_value(
    input: &super::Add,
    value: u64,
) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
    materialize(crate::preserves_rail::record("addressable-counter-result-v1", vec![
        crate::preserves_rail::string(&input.actor_key_ref),
        crate::preserves_rail::string(&input.request_id),
        crate::preserves_rail::u64_value(value),
    ]))
}

fn reference(value: &Field) -> crate::error::Result<String> {
    crate::preserves_rail::required_content_ref_string(value, "counter reference")
}

fn number(value: &Field) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| super::invalid("counter value must be u64"))?
        .map_err(|_| super::invalid("counter value exceeds u64"))
}

fn bounded_decode(bytes: &[u8]) -> crate::error::Result<preserves::IOValue> {
    if bytes.len() > MAX_BYTES {
        return Err(super::invalid("counter value exceeds byte bound"));
    }
    Ok(crate::preserves_rail::strict_canonical_decode(bytes)?.value)
}

fn materialize(value: preserves::IOValue) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_BYTES {
        return Err(super::invalid("counter value exceeds byte bound"));
    }
    Ok(crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    })
}
