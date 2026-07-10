
fn parse_delivery_log_entry(value: &Value<IoValue>) -> Result<Delivery> {
    let value = value_to_iovalue(value);
    let (fields, has_operation_ref, has_idempotency_receipt) =
        if let Some(fields) = value.collect_simple_record("entry", Some(5)) {
            (fields, true, true)
        } else if let Some(fields) = value.collect_simple_record("entry", Some(4)) {
            (fields, true, false)
        } else {
            (
                value
                    .collect_simple_record("entry", Some(3))
                    .ok_or_else(|| MoltenError::invalid_harness("expected remote dataspace delivery log entry"))?,
                false,
                false,
            )
        };
    let _index = fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness("expected u64 delivery log entry index"))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for delivery log entry: {error}")))?;
    let envelope_value = record_iovalue(&fields[1], "envelope")?;
    let receipt_value = record_iovalue(&fields[2], "transport-receipt")?;
    let envelope = parse_envelope(&envelope_value)?;
    if has_operation_ref {
        let operation_ref = record_string(&fields[3], "operation-ref")?;
        if operation_ref != envelope.operation_ref {
            return Err(MoltenError::invalid_harness("remote delivery log operation ref mismatch"));
        }
    }
    if has_idempotency_receipt {
        let receipt_value = record_iovalue(&fields[4], "idempotency-receipt")?;
        let receipt = crate::delivery_idempotency::parse_receipt(&receipt_value)?;
        if receipt.operation_ref != envelope.operation_ref {
            return Err(MoltenError::invalid_harness("remote delivery log idempotency receipt mismatch"));
        }
        validate_replay_idempotency_receipt(&receipt)?;
    }
    Ok(Delivery {
        envelope,
        receipt_value,
    })
}

fn record_iovalue(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_bool(value: &Value<IoValue>, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    field_sequence(value, label)?.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/remote/parts/dataspace/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/remote/parts/dataspace/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/remote/parts/dataspace/tests/m000/p002/body.rs"));
}
