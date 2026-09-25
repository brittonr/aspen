
fn selected_traversal_refs(descriptor: &TraversalDescriptor) -> Vec<String> {
    let visited = descriptor.visited_refs.iter().collect::<std::collections::BTreeSet<_>>();
    descriptor
        .root_refs
        .iter()
        .filter(|reference| !visited.contains(reference))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn inventory_ref_set(inventory: &LocalInventorySummary) -> std::collections::BTreeSet<String> {
    inventory
        .verified_refs
        .iter()
        .chain(inventory.chunk_refs.iter())
        .cloned()
        .collect()
}

fn inventory_summary_ref(inventory: &LocalInventorySummary) -> String {
    let value = record("remote-sync-local-inventory-summary-v1", vec![
        record("verified", vec![string_sequence(&inventory.verified_refs)]),
        record("chunks", vec![string_sequence(&inventory.chunk_refs)]),
    ]);
    canonical_hash(&value).unwrap_or_else(|_| content_ref_from_bytes(b"invalid-local-inventory-summary"))
}

struct TraversalPlanReceiptInput<'a> {
    decision: &'a str,
    descriptor_ref: &'a str,
    local_inventory_ref: &'a str,
    selected_refs: &'a [String],
    already_present_refs: &'a [String],
    fetch_refs: &'a [String],
    diagnostics: &'a [String],
    replayable: bool,
}

fn traversal_plan_receipt_value(input: TraversalPlanReceiptInput<'_>) -> IoValue {
    let TraversalPlanReceiptInput { decision, descriptor_ref, local_inventory_ref, selected_refs, already_present_refs, fetch_refs, diagnostics, replayable } = input;
    record("remote-sync-traversal-plan-receipt-v1", vec![
        string(TRAVERSAL_PLAN_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("descriptor", vec![string(descriptor_ref)]),
        record("local-inventory", vec![string(local_inventory_ref)]),
        record("selected", vec![string_sequence(selected_refs)]),
        record("already-present", vec![string_sequence(already_present_refs)]),
        record("fetch", vec![string_sequence(fetch_refs)]),
        record("diagnostics", vec![string_sequence(diagnostics)]),
        record("replayable", vec![string(replayable.to_string())]),
        record("checks", vec![sequence(vec![
            check_record("deterministic-order", if replayable { "pass" } else { "fail" }),
            check_record("receiver-driven-missing-set", if replayable { "pass" } else { "fail" }),
        ])]),
    ])
}

fn validate_traversal_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_external_digest_algorithm(algorithm: &str) -> Result<()> {
    match algorithm {
        EXTERNAL_DIGEST_CID_SHA2_256 | EXTERNAL_DIGEST_CID_SHA2_512 | EXTERNAL_DIGEST_BLAKE3 => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported external digest algorithm {algorithm}"))),
    }
}

fn string_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn check_record(name: &str, status: &str) -> IoValue {
    record("check", vec![string(name), string(status)])
}
