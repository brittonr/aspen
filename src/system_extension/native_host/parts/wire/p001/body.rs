
fn parse_effect(
    value: &preserves::Value<preserves::IOValue>,
    maximum_value_bytes: u64,
) -> crate::error::Result<NativeMaterializedEffectRequest> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, EFFECT_RECORD, EFFECT_FIELD_COUNT)?;
    let target_value = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let target_fields =
        crate::preserves_rail::simple_record_fields(&target_value, PORT_TARGET_RECORD, PORT_TARGET_FIELD_COUNT)
            .map_err(|_| {
                crate::error::MoltenError::invalid_harness("native callback effect target must be an exact fabric port")
            })?;
    let schema = crate::preserves_rail::required_string_field(&fields[7], "effect schema")?;
    if schema != NATIVE_CALLBACK_OUTCOME_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("native callback effect schema mismatch"));
    }
    let request = parse_value(&fields[4], "effect request", maximum_value_bytes)?;
    Ok(NativeMaterializedEffectRequest {
        effect: TypedEffectRequest {
            target: EffectTarget::FabricPort(crate::fabric::FabricPortKey {
                port_id: crate::preserves_rail::required_string_field(&target_fields[0], "effect port id")?,
                version: crate::preserves_rail::required_string_field(&target_fields[1], "effect port version")?,
            }),
            operation: crate::preserves_rail::required_string_field(&fields[1], "effect operation")?,
            input_schema_ref: crate::preserves_rail::required_string_field(&fields[2], "effect input schema")?,
            output_schema_ref: crate::preserves_rail::required_string_field(&fields[3], "effect output schema")?,
            request_ref: request.value_ref.clone(),
            generation: required_u64(&fields[5], "effect generation")?,
            accounted_bytes: required_u64(&fields[6], "effect accounted bytes")?,
        },
        request,
    })
}

fn optional_value(value: Option<&NativeCallbackValue>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record(NONE_RECORD, Vec::new()),
        |value| crate::preserves_rail::record(SOME_RECORD, vec![value_value(value)]),
    )
}

fn parse_optional_value(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
    maximum_value_bytes: u64,
) -> crate::error::Result<Option<NativeCallbackValue>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record(NONE_RECORD, Some(0)).is_some() {
        return Ok(None);
    }
    let fields = crate::preserves_rail::simple_record_fields(&value, SOME_RECORD, 1)?;
    parse_value(&fields[0], field, maximum_value_bytes).map(Some)
}

fn value_value(value: &NativeCallbackValue) -> preserves::IOValue {
    crate::preserves_rail::record(VALUE_RECORD, vec![
        crate::preserves_rail::string(&value.value_ref),
        bytes_value(&value.bytes),
    ])
}

fn parse_value(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
    maximum_bytes: u64,
) -> crate::error::Result<NativeCallbackValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, VALUE_RECORD, VALUE_FIELD_COUNT)?;
    let value = NativeCallbackValue {
        value_ref: crate::preserves_rail::required_content_ref_string(&fields[0], field)?,
        bytes: parse_bytes(&fields[1], field, maximum_bytes)?,
    };
    super::admit_native_callback_value(&value, maximum_bytes).map_err(value_error)?;
    Ok(value)
}

fn bytes_value(bytes: &[u8]) -> preserves::IOValue {
    crate::preserves_rail::sequence(
        bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
    )
}

fn parse_bytes(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
    maximum_bytes: u64,
) -> crate::error::Result<Vec<u8>> {
    let values = crate::preserves_rail::required_sequence_field(value, field)?;
    require_byte_bound_len(values.len(), maximum_bytes, field)?;
    values
        .iter()
        .map(|value| {
            let number = required_u64(value, field)?;
            u8::try_from(number).map_err(|_| {
                crate::error::MoltenError::invalid_harness(format!("{field} contains a value outside the byte range"))
            })
        })
        .collect()
}

fn require_input_links(
    context: &NativeCallbackContext,
    invocation: &CallbackInvocation,
    inputs: &NativeCallbackInputs,
) -> crate::error::Result<()> {
    if invocation.payload_ref.as_deref() != inputs.payload.as_ref().map(|value| value.value_ref.as_str()) {
        return Err(crate::error::MoltenError::invalid_harness("native callback payload reference lacks exact bytes"));
    }
    if context.state_ref.as_deref() != inputs.state.as_ref().map(|value| value.value_ref.as_str()) {
        return Err(crate::error::MoltenError::invalid_harness("native callback state reference lacks exact bytes"));
    }
    for value in inputs.payload.iter().chain(inputs.state.iter()) {
        super::admit_native_callback_value(value, u64::MAX).map_err(value_error)?;
    }
    Ok(())
}

fn ref_sequence(references: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(references.iter().map(crate::preserves_rail::string).collect())
}

fn parse_ref_sequence(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
    maximum: u64,
) -> crate::error::Result<Vec<String>> {
    let values = crate::preserves_rail::required_sequence_field(value, field)?;
    require_item_bound(values.len(), maximum, field)?;
    values
        .iter()
        .map(|value| crate::preserves_rail::required_content_ref_string(value, field))
        .collect()
}

fn required_u64(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn parse_health(value: &str) -> crate::error::Result<HealthState> {
    match value {
        "unknown" => Ok(HealthState::Unknown),
        "starting" => Ok(HealthState::Starting),
        "healthy" => Ok(HealthState::Healthy),
        "degraded" => Ok(HealthState::Degraded),
        "failed" => Ok(HealthState::Failed),
        "quarantined" => Ok(HealthState::Quarantined),
        "stopped" => Ok(HealthState::Stopped),
        _ => Err(crate::error::MoltenError::invalid_harness("native callback health is unsupported")),
    }
}

fn require_byte_bound(bytes: &[u8], maximum: u64, label: &str) -> crate::error::Result<()> {
    require_byte_bound_len(bytes.len(), maximum, label)
}

fn require_byte_bound_len(actual: usize, maximum: u64, label: &str) -> crate::error::Result<()> {
    let actual = u64::try_from(actual)
        .map_err(|_| crate::error::MoltenError::invalid_harness(format!("{label} length does not fit u64")))?;
    if actual > maximum {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} exceeds {maximum} bytes")));
    }
    Ok(())
}

fn require_item_bound(actual: usize, maximum: u64, label: &str) -> crate::error::Result<()> {
    let actual = u64::try_from(actual)
        .map_err(|_| crate::error::MoltenError::invalid_harness(format!("{label} count does not fit u64")))?;
    if actual > maximum {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} exceeds its item bound")));
    }
    Ok(())
}

fn value_error(error: super::NativeValuePortFailure) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(error.message)
}
