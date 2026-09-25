
/// Schedule a retry for a work queue item with bounded backoff.
pub fn schedule_retry(
    item: &WorkQueueItem,
    backoff_profile: &str,
    attempt: u64,
) -> Result<WorkQueueDecision> {
    if item.terminal {
        return Ok(WorkQueueDecision {
            pass: false,
            item: None,
            diagnostics: vec![format!(
                "cannot retry terminal item: {}",
                item.terminal_reason.as_deref().unwrap_or("unknown"),
            )],
        });
    }

    validate_non_empty(backoff_profile, "backoff profile")?;

    if attempt > MAX_BACKOFF_ATTEMPTS {
        return Ok(WorkQueueDecision {
            pass: false,
            item: None,
            diagnostics: vec![format!(
                "retry attempt {attempt} exceeds maximum {MAX_BACKOFF_ATTEMPTS}",
            )],
        });
    }

    Ok(WorkQueueDecision {
        pass: true,
        item: Some(WorkQueueItem {
            resource_ref: item.resource_ref.clone(),
            generation: item.generation,
            causes: item.causes.clone(),
            coalesced_event_refs: item.coalesced_event_refs.clone(),
            retry_attempt: attempt,
            backoff_profile: Some(backoff_profile.to_string()),
            terminal: false,
            terminal_reason: None,
        }),
        diagnostics: Vec::new(),
    })
}

/// Validate that a reconciliation completion claim is valid.
pub fn validate_reconcile_completion(
    input: &ReconcileCompletionInput,
) -> ReconcileCompletionDecision {
    let mut diagnostics = Vec::new();
    let mut is_pass = true;

    // Generation must match
    if input.claimed_generation != input.current_generation {
        is_pass = false;
        diagnostics.push(format!(
            "stale generation: claimed {} but current is {}",
            input.claimed_generation, input.current_generation,
        ));
    }

    // Must have an admitted plan
    if !input.has_admitted_plan {
        is_pass = false;
        diagnostics.push("no admitted plan for reconciliation".to_string());
    }

    // Every required effect intent must have a receipt
    diagnostics.reserve(input.required_effect_intents.len());
    for required in &input.required_effect_intents {
        if !input
            .has_effect_receipts
            .iter()
            .any(|receipt| receipt.contains(required))
        {
            is_pass = false;
            diagnostics.push(format!("missing effect receipt for: {required}"));
        }
    }

    // Must have status update
    if !input.has_status_update {
        is_pass = false;
        diagnostics.push("status update required for reconciliation success".to_string());
    }

    ReconcileCompletionDecision { pass: is_pass, diagnostics }
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn reconcile_receipt_to_value(receipt: &ReconcileReceipt) -> IoValue {
    record("reconcile-receipt-v1", vec![
        string(&receipt.resource_ref),
        u64_value(receipt.generation),
        string(&receipt.plan_ref),
        refs_sequence(&receipt.admission_refs),
        refs_sequence(&receipt.effect_refs),
        optional_ref_value(receipt.status_update_ref.as_deref()),
        optional_ref_value(receipt.canonical_ref.as_deref()),
    ])
}