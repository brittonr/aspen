pub fn node_adapter_lifecycle_receipt_value(input: &AdapterLifecycleReceiptInput<'_>) -> Result<IoValue> {
    validate_adapter_operation(input.operation)?;
    validate_decision(input.decision)?;
    validate_refs(input.index_receipt_refs, "node adapter index receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node adapter resource receipt ref")?;
    Ok(record("node-adapter-receipt-v1", vec![
        string(NODE_ADAPTER_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("adapter", vec![string(&input.adapter.name)]),
        record("profile", vec![string(&input.adapter.profile_ref)]),
        record("index", vec![refs_sequence(input.index_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("adapter-profile-bound", "pass"),
            ("adapter-index-verified", status(!input.index_receipt_refs.is_empty())),
            ("adapter-resource-profile-bound", status(!input.resource_receipt_refs.is_empty())),
            ("no-invisible-startup", if input.decision == "pass" { "pass" } else { "fail" }),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}