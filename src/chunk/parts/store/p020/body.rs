
fn gc_receipt_value(input: GcReceiptInput<'_>) -> IoValue {
    receipt_value(ChunkStoreReceiptValueInput {
        operation: "gc",
        decision: input.decision,
        manifest_ref: None,
        chunk_refs: input.removed_chunks,
        checks: vec![
            ("pin-reachability", "pass"),
            ("deny-incomplete-reachability-proof", "pass"),
            ("chunk-tombstone-eligibility", if input.decision == "pass" { "pass" } else { "fail" }),
            ("retention-receipt-bound", "pass"),
            (
                "retention-execution-gate",
                pass_or_fail(input.is_dry_run || input.notes.execution_diagnostics.is_empty()),
            ),
            ("retention-authority-evidence", pass_or_fail(input.notes.admission_diagnostics.is_empty())),
            ("redb-index-update", if input.decision == "pass" { "pass" } else { "fail" }),
        ],
        details: vec![
            record("mode", vec![string(if input.is_dry_run { "dry-run" } else { "apply" })]),
            record("removed-manifests", vec![sequence(input.removed_manifests.iter().map(string).collect())]),
            record("retention", vec![sequence(input.notes.receipts.iter().map(string).collect())]),
            record("retention-execution", vec![sequence(input.notes.execution_gates.iter().map(string).collect())]),
            record("denied", vec![sequence(input.notes.denials.iter().map(string).collect())]),
            record("retention-evidence", vec![input.evidence_summary.clone()]),
            record("retention-admission", vec![sequence(input.notes.admission_refs.iter().map(string).collect())]),
            record("retention-diagnostics", vec![sequence(
                input.notes.admission_diagnostics.iter().map(string).collect(),
            )]),
            record("retention-execution-diagnostics", vec![sequence(
                input.notes.execution_diagnostics.iter().map(string).collect(),
            )]),
        ],
    })
}

fn gc_tombstone_value(input: GcReceiptInput<'_>) -> Option<IoValue> {
    if input.is_dry_run
        || input.decision != "pass"
        || (input.removed_manifests.is_empty() && input.removed_chunks.is_empty())
    {
        return None;
    }
    Some(self::receipt_value(ChunkStoreReceiptValueInput {
        operation: "tombstone",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: input.removed_chunks,
        checks: vec![
            ("pin-reachability", "pass"),
            ("tombstone-eligibility", "pass"),
            ("gc-mode-binding", "pass"),
            ("retention-receipt-bound", "pass"),
            ("retention-execution-gate", "pass"),
            ("retention-authority-evidence", "pass"),
        ],
        details: vec![
            record("mode", vec![string("apply")]),
            record("removed-manifests", vec![sequence(input.removed_manifests.iter().map(string).collect())]),
            record("retention", vec![sequence(input.notes.receipts.iter().map(string).collect())]),
            record("retention-execution", vec![sequence(input.notes.execution_gates.iter().map(string).collect())]),
            record("retention-evidence", vec![input.evidence_summary.clone()]),
            record("retention-admission", vec![sequence(input.notes.admission_refs.iter().map(string).collect())]),
        ],
    }))
}

struct GcFinishInput<'a> {
    root: &'a CapabilityChunkRoot,
    is_dry_run: bool,
    targets: GcTargets,
    notes: GcNotes,
    evidence_summary: IoValue,
}
