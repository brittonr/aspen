
fn parse_operation(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<NativeOperationRecord> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, OPERATION_RECORD, OPERATION_FIELD_COUNT)?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "native operation schema")?;
    if schema != NATIVE_OPERATION_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("native operation schema mismatch"));
    }
    Ok(NativeOperationRecord {
        schema,
        operation_ref: crate::preserves_rail::required_content_ref_string(&fields[1], "native operation ref")?,
        parent_ref: crate::preserves_rail::required_content_ref_string(&fields[2], "native operation parent ref")?,
        kind: parse_operation_kind(&crate::preserves_rail::required_string_field(
            &fields[3],
            "native operation kind",
        )?)?,
        generation: required_u64(&fields[4], "native operation generation")?,
        state: parse_operation_state(&crate::preserves_rail::required_string_field(
            &fields[5],
            "native operation state",
        )?)?,
        terminal_ref: parse_optional_ref(&fields[6], "native operation terminal ref")?,
        is_retry_permitted: required_bool(&fields[7], "native operation retry state")?,
    })
}

fn parse_refs(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<Vec<String>> {
    let values = crate::preserves_rail::required_sequence_field(value, field)?;
    require_item_bound(values.len(), field)?;
    values
        .iter()
        .map(|value| crate::preserves_rail::required_content_ref_string(value, field))
        .collect()
}

fn ref_sequence(references: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(references.iter().map(crate::preserves_rail::string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> preserves::IOValue {
    reference.map_or_else(
        || crate::preserves_rail::record(NONE_RECORD, Vec::new()),
        |reference| crate::preserves_rail::record(SOME_RECORD, vec![crate::preserves_rail::string(reference)]),
    )
}

fn parse_optional_ref(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record(NONE_RECORD, Some(0)).is_some() {
        return Ok(None);
    }
    let fields = crate::preserves_rail::simple_record_fields(&value, SOME_RECORD, 1)?;
    crate::preserves_rail::required_content_ref_string(&fields[0], field).map(Some)
}

fn required_u64(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn required_bool(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected boolean for {field}")))
}

fn parse_phase(value: &str) -> crate::error::Result<LifecyclePhase> {
    match value {
        "absent" => Ok(LifecyclePhase::Absent),
        "installed" => Ok(LifecyclePhase::Installed),
        "admitted" => Ok(LifecyclePhase::Admitted),
        "initializing" => Ok(LifecyclePhase::Initializing),
        "initialized" => Ok(LifecyclePhase::Initialized),
        "starting" => Ok(LifecyclePhase::Starting),
        "running" => Ok(LifecyclePhase::Running),
        "checkpointing" => Ok(LifecyclePhase::Checkpointing),
        "recovering" => Ok(LifecyclePhase::Recovering),
        "draining" => Ok(LifecyclePhase::Draining),
        "drained" => Ok(LifecyclePhase::Drained),
        "failed" => Ok(LifecyclePhase::Failed),
        "restarting" => Ok(LifecyclePhase::Restarting),
        "upgrading" => Ok(LifecyclePhase::Upgrading),
        "rolling-back" => Ok(LifecyclePhase::RollingBack),
        "shutting-down" => Ok(LifecyclePhase::ShuttingDown),
        "quarantined" => Ok(LifecyclePhase::Quarantined),
        "stopped" => Ok(LifecyclePhase::Stopped),
        "removed" => Ok(LifecyclePhase::Removed),
        _ => Err(crate::error::MoltenError::invalid_harness("native lifecycle phase is unsupported")),
    }
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
        _ => Err(crate::error::MoltenError::invalid_harness("native health state is unsupported")),
    }
}

fn parse_operation_kind(value: &str) -> crate::error::Result<NativeOperationKind> {
    match value {
        "callback" => Ok(NativeOperationKind::Callback),
        "effect" => Ok(NativeOperationKind::Effect),
        "ingress" => Ok(NativeOperationKind::Ingress),
        "value-publication" => Ok(NativeOperationKind::ValuePublication),
        _ => Err(crate::error::MoltenError::invalid_harness("native operation kind is unsupported")),
    }
}

fn parse_operation_state(value: &str) -> crate::error::Result<NativeOperationState> {
    match value {
        "intent-committed" => Ok(NativeOperationState::IntentCommitted),
        "started" => Ok(NativeOperationState::Started),
        "terminal" => Ok(NativeOperationState::Terminal),
        "unknown" => Ok(NativeOperationState::Unknown),
        "stale" => Ok(NativeOperationState::Stale),
        _ => Err(crate::error::MoltenError::invalid_harness("native operation state is unsupported")),
    }
}

fn require_item_bound(actual: usize, field: &str) -> crate::error::Result<()> {
    if actual > MAX_INSTANCE_COLLECTION_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!("{field} exceeds its item bound")));
    }
    Ok(())
}

fn journal_invalid(error: crate::error::MoltenError) -> NativeJournalError {
    NativeJournalError::InvalidRecord(error.to_string())
}
