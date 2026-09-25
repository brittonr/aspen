
fn read_receipt_value(input: &ReadReceiptValueInput<'_>) -> Result<IoValue> {
    let read_index_check = if input.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE {
        input.decision
    } else {
        "diagnostic"
    };
    Ok(record("raft-read-receipt-v1", vec![
        string(crate::preserves_rail::RAFT_READ_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("state", vec![string(input.state_ref)]),
        record("committed-term", vec![u64_value(input.committed_term)]),
        record("committed-index", vec![u64_value(input.committed_index)]),
        record("namespace", vec![string(input.namespace)]),
        record("name", vec![string(input.name)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("read-consistency", vec![string(input.read_consistency_mode)]),
        record("read-index-predicate", vec![optional_ref_value(input.read_index_predicate_ref)]),
        record("authority", vec![strings_sequence(input.authority_refs)]),
        record("resource", vec![strings_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("read-consistency-declared", "pass"),
            ("read-index-bound", read_index_check),
            ("local-stale-non-authoritative", if input.read_consistency_mode == READ_CONSISTENCY_LOCAL_STALE { "pass" } else { "diagnostic" }),
            ("authority-resource-gated", "pass"),
        ]),
    ]))
}
