
fn validate_near_far_refs(state: &RuntimeNearFarRefState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if crate::preserves_rail::validate_content_ref(&state.reference_ref).is_err() {
        diagnostics.push("reference-ref-noncanonical".to_string());
    }
    if state.caller_vat_id.is_empty() {
        diagnostics.push("caller-vat-id-empty".to_string());
    }
    if state.target_vat_id.is_empty() {
        diagnostics.push("target-vat-id-empty".to_string());
    }
    if !state.is_live {
        diagnostics.push("reference-not-live".to_string());
    }

    let is_same_vat = state.caller_vat_id == state.target_vat_id;
    match state.reference_kind {
        RuntimeReferenceKind::Near => {
            if !is_same_vat {
                diagnostics.push("near-ref-cross-vat".to_string());
            }
            if matches!(state.call_mode, RuntimeReferenceCallMode::Synchronous) && !is_same_vat {
                diagnostics.push("synchronous-call-not-live-same-vat-near-ref".to_string());
            }
        }
        RuntimeReferenceKind::Far => {
            if matches!(state.call_mode, RuntimeReferenceCallMode::Synchronous) {
                diagnostics.push("far-ref-synchronous-call-denied".to_string());
            }
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn validate_promise_pipeline(state: &RuntimePromisePipelineState) -> Vec<String> {
    let mut diagnostics = validate_promise_shape(&state.source, "source");
    if state.max_queue == 0 && !state.entries.is_empty() {
        diagnostics.push("pipeline-queue-nonempty-with-zero-bound".to_string());
    }
    if (state.entries.len() as u64) > state.max_queue {
        diagnostics.push("pipeline-queue-bound-exceeded".to_string());
    }
    if state.source.status.is_terminal() && !state.entries.is_empty() {
        diagnostics.push("terminal-promise-pipeline-not-cleaned".to_string());
    }
    let mut previous_sequence = None;
    let mut seen_sequences = OrderedSet::new();
    for entry in state.entries.as_slice() {
        if !seen_sequences.insert(entry.sequence) {
            diagnostics.push("pipeline-forwarding-sequence-duplicate".to_string());
        }
        if let Some(previous) = previous_sequence
            && entry.sequence <= previous
        {
            diagnostics.push("pipeline-forwarding-order-violation".to_string());
        }
        previous_sequence = Some(entry.sequence);
        if entry.operation.is_empty() {
            diagnostics.push("pipeline-operation-empty".to_string());
        }
        if crate::preserves_rail::validate_content_ref(&entry.target_ref).is_err() {
            diagnostics.push("pipeline-target-ref-noncanonical".to_string());
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn validate_promise_use(state: &RuntimePromiseUseState) -> Vec<String> {
    let mut diagnostics = validate_promise_shape(&state.source, "source");
    if crate::preserves_rail::validate_content_ref(&state.dependent_call_ref).is_err() {
        diagnostics.push("promise-use-dependent-call-ref-noncanonical".to_string());
    }
    if let Some(admitted_resolution_ref) = state.admitted_resolution_ref.as_deref()
        && crate::preserves_rail::validate_content_ref(admitted_resolution_ref).is_err()
    {
        diagnostics.push("promise-use-resolution-ref-noncanonical".to_string());
    }
    if let Some(admitted_pipeline_ref) = state.admitted_pipeline_ref.as_deref()
        && crate::preserves_rail::validate_content_ref(admitted_pipeline_ref).is_err()
    {
        diagnostics.push("promise-use-pipeline-ref-noncanonical".to_string());
    }

    match state.use_kind {
        RuntimePromiseUseKind::ResolvedValue => validate_resolved_promise_use(state, &mut diagnostics),
        RuntimePromiseUseKind::PipelineForward => validate_pipeline_promise_use(state, &mut diagnostics),
    }

    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn validate_resolved_promise_use(state: &RuntimePromiseUseState, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if state.source.status != RuntimePromiseStatus::Resolved {
        diagnostics.push_item("promise-use-requires-resolved-source".to_string());
    }
    match (state.source.value_ref.as_deref(), state.admitted_resolution_ref.as_deref()) {
        (Some(value_ref), Some(admitted_ref)) if value_ref == admitted_ref => {}
        (Some(_), Some(_)) => diagnostics.push_item("promise-use-resolution-ref-mismatch".to_string()),
        (Some(_), None) => diagnostics.push_item("promise-use-resolution-proof-missing".to_string()),
        (None, Some(_)) => diagnostics.push_item("promise-use-resolution-without-value".to_string()),
        (None, None) => diagnostics.push_item("promise-use-resolution-proof-missing".to_string()),
    }
    if state.admitted_pipeline_ref.is_some() {
        diagnostics.push_item("promise-use-resolution-has-pipeline-proof".to_string());
    }
}

fn validate_pipeline_promise_use(state: &RuntimePromiseUseState, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if state.source.status != RuntimePromiseStatus::Pending {
        diagnostics.push_item("promise-pipeline-forward-requires-pending-source".to_string());
    }
    if state.admitted_resolution_ref.is_some() {
        diagnostics.push_item("promise-pipeline-forward-has-resolution-proof".to_string());
    }
    if state.admitted_pipeline_ref.is_none() {
        diagnostics.push_item("promise-use-pipeline-proof-missing".to_string());
    }
}

fn validate_revocation_cleanup(state: &RuntimeRevocationCleanupState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(16);
    diagnostics.extend(validate_sorted_content_refs(&state.revoked_refs, "revocation", "revoked"));
    diagnostics.extend(validate_sorted_content_refs(&state.attempted_use_refs, "revocation", "attempted-use"));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_assertion_refs,
        "revocation",
        "remaining-assertion",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_subscription_refs,
        "revocation",
        "remaining-subscription",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_pending_call_refs,
        "revocation",
        "remaining-pending-call",
    ));
    diagnostics.extend(validate_sorted_content_refs(&state.remaining_child_refs, "revocation", "remaining-child"));

    let revoked_refs: OrderedSet<&str> = state.revoked_refs.as_slice().iter().map(String::as_str).collect();
    if has_revoked_intersection(&revoked_refs, &state.attempted_use_refs) {
        diagnostics.push("revoked-ref-used-after-revocation".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_assertion_refs) {
        diagnostics.push("revoked-dependent-assertion-not-cleaned".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_subscription_refs) {
        diagnostics.push("revoked-dependent-subscription-not-cleaned".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_pending_call_refs) {
        diagnostics.push("revoked-pending-call-not-cleaned".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_child_refs) {
        diagnostics.push("revoked-child-ref-not-cleaned".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn has_revoked_intersection(revoked_refs: &OrderedSet<&str>, refs: &[String]) -> bool {
    for reference in refs {
        if revoked_refs.contains(reference.as_str()) {
            return true;
        }
    }
    false
}

fn validate_vat_rollback_cleanup(state: &RuntimeVatRollbackCleanupState) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if crate::preserves_rail::validate_content_ref(&state.rollback_receipt_ref).is_err() {
        diagnostics.push("vat-rollback-receipt-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.before_snapshot_ref).is_err() {
        diagnostics.push("vat-rollback-before-snapshot-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.final_snapshot_ref).is_err() {
        diagnostics.push("vat-rollback-final-snapshot-ref-noncanonical".to_string());
    }
    diagnostics.extend(validate_sorted_content_refs(&state.rolled_back_refs, "vat-rollback", "rolled-back"));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_assertion_refs,
        "vat-rollback",
        "remaining-assertion",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_observer_refs,
        "vat-rollback",
        "remaining-observer",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_pending_call_refs,
        "vat-rollback",
        "remaining-pending-call",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_authority_snapshot_refs,
        "vat-rollback",
        "remaining-authority-snapshot",
    ));

    if state.before_snapshot_ref != state.final_snapshot_ref {
        diagnostics.push("vat-rollback-final-snapshot-changed".to_string());
    }
    let rolled_back_refs = string_set(&state.rolled_back_refs);
    if has_revoked_intersection(&rolled_back_refs, &state.remaining_assertion_refs) {
        diagnostics.push("vat-rollback-assertion-leak".to_string());
    }
    if has_revoked_intersection(&rolled_back_refs, &state.remaining_observer_refs) {
        diagnostics.push("vat-rollback-observer-leak".to_string());
    }
    if has_revoked_intersection(&rolled_back_refs, &state.remaining_pending_call_refs) {
        diagnostics.push("vat-rollback-pending-call-leak".to_string());
    }
    if has_revoked_intersection(&rolled_back_refs, &state.remaining_authority_snapshot_refs) {
        diagnostics.push("vat-rollback-authority-snapshot-leak".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}
