
pub fn parse_operation_id(value: &IoValue) -> Result<OperationId> {
    let fields = value
        .collect_simple_record("operation-id-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operation-id-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DELIVERY_OPERATION_ID_SCHEMA, "delivery operation id schema")?;
    let input = OperationIdInput {
        scope_ref: record_ref(&fields[1], "scope")?,
        producer: record_string(&fields[2], "producer")?,
        consumer: record_string(&fields[3], "consumer")?,
        sequence: record_u64(&fields[4], "sequence")?,
        intent: record_string(&fields[5], "intent")?,
        payload_ref: record_ref(&fields[6], "payload")?,
        policy_refs: record_ref_sequence(&fields[7], "policy")?,
    };
    validate_operation_input(&input)?;
    require_check(&parse_checks(&fields[8])?, "canonical-operation-ref", "delivery operation id")?;
    Ok(OperationId {
        operation_ref: crate::preserves_rail::canonical_hash(value)?,
        scope_ref: input.scope_ref,
        producer: input.producer,
        consumer: input.consumer,
        sequence: input.sequence,
        intent: input.intent,
        payload_ref: input.payload_ref,
        policy_refs: input.policy_refs,
        value: value.clone(),
    })
}

pub fn window_value(
    scope_profile: &str,
    scope_ref: &str,
    next_sequence: u64,
    lowest_retained: u64,
    retention_refs: &[String],
) -> Result<IoValue> {
    validate_scope_profile(scope_profile)?;
    require_ref(scope_ref, "delivery window scope ref")?;
    validate_refs(retention_refs, "delivery retention ref")?;
    if lowest_retained == 0 || next_sequence == 0 || lowest_retained > next_sequence {
        return Err(MoltenError::invalid_harness("invalid delivery window sequence bounds"));
    }
    Ok(record("delivery-window-v1", vec![
        string(crate::preserves_rail::DELIVERY_WINDOW_SCHEMA),
        record("scope", vec![string(scope_ref)]),
        record("profile", vec![string(scope_profile)]),
        record("next-sequence", vec![crate::preserves_rail::u64_value(next_sequence)]),
        record("lowest-retained", vec![crate::preserves_rail::u64_value(lowest_retained)]),
        record("retention", vec![strings_sequence(retention_refs)]),
        checks_value(&[("dedup-window-scoped", "pass"), ("retention-pinned", "pass")]),
    ]))
}

pub fn parse_window(value: &IoValue) -> Result<Window> {
    let fields = value
        .collect_simple_record("delivery-window-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <delivery-window-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DELIVERY_WINDOW_SCHEMA, "delivery window schema")?;
    let scope_ref = record_ref(&fields[1], "scope")?;
    let scope_profile = record_string(&fields[2], "profile")?;
    let next_sequence = record_u64(&fields[3], "next-sequence")?;
    let lowest_retained = record_u64(&fields[4], "lowest-retained")?;
    let retention_refs = record_ref_sequence(&fields[5], "retention")?;
    validate_scope_profile(&scope_profile)?;
    if lowest_retained == 0 || next_sequence == 0 || lowest_retained > next_sequence {
        return Err(MoltenError::invalid_harness("invalid parsed delivery window sequence bounds"));
    }
    require_check(&parse_checks(&fields[6])?, "dedup-window-scoped", "delivery window")?;
    Ok(Window {
        window_ref: crate::preserves_rail::canonical_hash(value)?,
        scope_ref,
        scope_profile,
        next_sequence,
        lowest_retained,
        retention_refs,
        value: value.clone(),
    })
}
