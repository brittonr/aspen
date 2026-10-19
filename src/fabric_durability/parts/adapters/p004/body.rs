fn encode_log_record(record: &LogRecord) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_byte(&mut bytes, encode_level(record.durability));
    push_blob(&mut bytes, record.value_ref.as_bytes())?;
    push_blob(&mut bytes, &record.value)?;
    Ok(bytes)
}

fn decode_log_record(
    sequence: u64,
    bytes: &[u8],
    profile: &DurableStateProfile,
    durable_bytes: u64,
    quota_bytes: u64,
) -> crate::error::Result<(LogRecord, u64)> {
    let mut cursor = ByteCursor::new(bytes);
    let durability = decode_level(cursor.take_byte()?)?;
    let value_ref_bytes = cursor.take_blob()?;
    if value_ref_bytes.len() > MAX_DURABILITY_TEXT_BYTES {
        return Err(adapter_validation_error("restore", &[DurabilityIssue::MalformedContentRef("value-ref")]));
    }
    let value_ref = std::str::from_utf8(value_ref_bytes).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("durable adapter string is not UTF-8: {error}"))
    })?;
    let value = cursor.take_blob()?;
    cursor.finish()?;
    let value_bytes = byte_count(value.len())?;
    if value_bytes > profile.max_operation_bytes {
        return Err(adapter_validation_error("restore", &[DurabilityIssue::ByteLimitExceeded {
            actual: value_bytes,
            maximum: profile.max_operation_bytes,
        }]));
    }
    let next_durable_bytes = checked_adapter_add(durable_bytes, value_bytes)?;
    let maximum = quota_bytes.min(profile.max_namespace_bytes);
    if next_durable_bytes > maximum {
        return Err(adapter_validation_error("restore", &[DurabilityIssue::NamespaceQuotaExceeded {
            actual: next_durable_bytes,
            maximum,
        }]));
    }
    Ok((
        LogRecord {
            sequence,
            value: value.to_vec(),
            value_ref: value_ref.to_string(),
            durability,
        },
        next_durable_bytes,
    ))
}
