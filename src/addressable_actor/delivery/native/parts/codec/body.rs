fn read_receipt(value: &crate::system_extension::NativeCallbackValue) -> crate::error::Result<Receipt> {
    let decoded = crate::preserves_rail::strict_canonical_decode_with_ref(
        &value.bytes,
        &value.value_ref,
        "native actor callback receipt",
    )?;
    let fields = crate::preserves_rail::simple_record_fields(
        &decoded.value,
        "system-extension-callback-receipt-v1",
        CALLBACK_RECEIPT_FIELDS,
    )?;
    if crate::preserves_rail::required_string_field(&fields[0], "callback schema")?
        != crate::system_extension::SYSTEM_EXTENSION_CALLBACK_SCHEMA
        || crate::preserves_rail::record_string_field(&fields[4], "callback", "callback kind")? != "request"
        || crate::preserves_rail::record_string_field(&fields[9], "decision", "callback decision")? != "succeeded"
        || !crate::preserves_rail::record_content_ref_strings(&fields[11], "effect-refs", "callback effects", 0)?
            .is_empty()
    {
        return Err(super::invalid("native actor requires original successful effect-free request callback receipt"));
    }
    let state_field = crate::preserves_rail::value_to_iovalue(&fields[12]);
    let state_record = crate::preserves_rail::simple_record_fields(&state_field, "state-ref", 1)?;
    let state_ref = crate::preserves_rail::optional_content_ref_string(&state_record[0], "callback state")?
        .ok_or_else(|| super::invalid("native actor callback has no semantic state result"))?;
    Ok(Receipt {
        manifest_ref: crate::preserves_rail::record_content_ref_string(
            &fields[1],
            "manifest-ref",
            "callback manifest",
        )?,
        generation: number_field(&fields[5], "generation")?,
        sequence: number_field(&fields[6], "sequence")?,
        event_ref: crate::preserves_rail::record_content_ref_string(&fields[7], "event-ref", "callback event")?,
        state_ref,
    })
}

fn number_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, label, 1)?;
    fields[0]
        .as_u64()
        .ok_or_else(|| super::invalid(format!("native actor callback {label} is not u64")))?
        .map_err(super::problem)
}

fn publication_ref(parent_ref: &str, role: &str, value_ref: &str, generation: u64) -> String {
    crate::system_extension::native_identity_ref(&[
        "native-value-publication-operation-v2",
        parent_ref,
        role,
        value_ref,
        &generation.to_string(),
    ])
}

fn publication_completed(
    record: &crate::system_extension::NativeInstanceRecord,
    operation_ref: &str,
    parent_ref: &str,
    generation: u64,
) -> bool {
    record.completed_operations.iter().any(|operation| {
        completed(operation, crate::system_extension::NativeOperationKind::ValuePublication, generation)
            && operation.operation_ref == operation_ref
            && operation.parent_ref == parent_ref
    })
}

fn completed(
    operation: &crate::system_extension::NativeOperationRecord,
    kind: crate::system_extension::NativeOperationKind,
    generation: u64,
) -> bool {
    operation.kind == kind
        && operation.generation.get() == generation
        && operation.state == crate::system_extension::NativeOperationState::Terminal
        && operation.terminal_ref.is_some()
}