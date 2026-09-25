
pub fn node_health_receipt_value(input: &HealthReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node health startup receipt ref")?;
    if let Some(shutdown_receipt_ref) = input.shutdown_receipt_ref {
        validate_ref(shutdown_receipt_ref, "node health shutdown receipt ref")?;
    }
    validate_refs(input.index_receipt_refs, "node health index receipt ref")?;
    validate_refs(input.head_refs, "node health head ref")?;
    validate_refs(input.open_job_refs, "node health open job ref")?;
    for adapter in input.adapter_receipts {
        validate_adapter_name(&adapter.name)?;
        validate_ref(&adapter.receipt_ref, "node health adapter receipt ref")?;
    }
    Ok(record("node-health-receipt-v1", vec![
        string(crate::preserves_rail::NODE_HEALTH_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("shutdown", vec![optional_ref_value(input.shutdown_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("indexes", vec![refs_sequence(input.index_receipt_refs)]),
        record("heads", vec![refs_sequence(input.head_refs)]),
        record("open-jobs", vec![refs_sequence(input.open_job_refs)]),
        record("replay", vec![string(if input.replay_is_eligible {
            "eligible"
        } else {
            "ineligible"
        })]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("startup-verified", "pass"),
            ("shutdown-verified", status(input.shutdown_receipt_ref.is_some())),
            ("adapter-indexes-current", status(!input.index_receipt_refs.is_empty())),
            ("health-heads-bound", status(!input.head_refs.is_empty())),
            ("no-open-jobs-for-replay", status(input.open_job_refs.is_empty())),
            ("replay-eligibility", status(input.replay_is_eligible)),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}
