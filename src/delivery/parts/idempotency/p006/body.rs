
fn first_decision(
    input: CheckRequest<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: Window,
    dedup_key: String,
) -> Result<Decision> {
    let next_sequence = operation
        .sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("delivery sequence overflow"))?;
    let updated_window = parse_window(&window_value(
        input.scope_profile,
        input.scope_ref,
        next_sequence,
        window.lowest_retained,
        input.policy_refs,
    )?)?;
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision: "first",
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &updated_window.window_ref,
        prior_receipt_ref: None,
        semantic_result_ref: input.semantic_result_ref,
        side_effect: "commit",
        diagnostics: &[],
        checks: &[
            ("dedup-before-commit", "pass"),
            ("sequence-window-advanced", "pass"),
            ("retention-pinned", "pass"),
        ],
    })?;
    let receipt = parse_receipt(&receipt_value)?;
    let entry_value = dedup_entry_value(DedupEntryValueInput {
        dedup_key: &dedup_key,
        operation: &operation,
        semantic_result_ref: input.semantic_result_ref,
        first_receipt_ref: &receipt.receipt_ref,
        evidence_refs: input.evidence_refs,
    })?;
    let entry = parse_dedup_entry(&entry_value)?;
    store_first_decision(db, &updated_window, &entry, &receipt)?;
    Ok(Decision {
        operation,
        window: updated_window,
        receipt,
        entry: Some(entry),
        should_commit_side_effect: true,
        prior_semantic_result_ref: None,
    })
}

fn duplicate_or_conflict_decision(
    db: &redb::Database,
    operation: OperationId,
    window: Window,
    entry: DedupEntry,
    law: &IdempotencyDecisionLaw,
) -> Result<Decision> {
    let decision = law.kind.as_str();
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision,
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &window.window_ref,
        prior_receipt_ref: law.prior_receipt_ref.as_deref(),
        semantic_result_ref: law.prior_semantic_result_ref.as_deref(),
        side_effect: "suppress",
        diagnostics: &law.diagnostics,
        checks: &[
            ("dedup-before-commit", "pass"),
            (
                "duplicate-suppresses-side-effects",
                if matches!(law.kind, IdempotencyDecisionKind::Duplicate) { "pass" } else { "n/a" },
            ),
            (
                "conflict-denies-before-side-effects",
                if matches!(law.kind, IdempotencyDecisionKind::Conflict) { "pass" } else { "n/a" },
            ),
        ],
    })?;
    let receipt = parse_receipt(&receipt_value)?;
    store_receipt(db, &receipt)?;
    Ok(Decision {
        operation,
        window,
        receipt,
        entry: Some(entry),
        should_commit_side_effect: law.should_commit_side_effect,
        prior_semantic_result_ref: law.prior_semantic_result_ref.clone(),
    })
}
