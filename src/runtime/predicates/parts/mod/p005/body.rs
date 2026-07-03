
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

fn validate_resolved_promise_use(state: &RuntimePromiseUseState, diagnostics: &mut Vec<String>) {
    if state.source.status != RuntimePromiseStatus::Resolved {
        diagnostics.push("promise-use-requires-resolved-source".to_string());
    }
    match (state.source.value_ref.as_deref(), state.admitted_resolution_ref.as_deref()) {
        (Some(value_ref), Some(admitted_ref)) if value_ref == admitted_ref => {}
        (Some(_), Some(_)) => diagnostics.push("promise-use-resolution-ref-mismatch".to_string()),
        (Some(_), None) => diagnostics.push("promise-use-resolution-proof-missing".to_string()),
        (None, Some(_)) => diagnostics.push("promise-use-resolution-without-value".to_string()),
        (None, None) => diagnostics.push("promise-use-resolution-proof-missing".to_string()),
    }
    if state.admitted_pipeline_ref.is_some() {
        diagnostics.push("promise-use-resolution-has-pipeline-proof".to_string());
    }
}

fn validate_pipeline_promise_use(state: &RuntimePromiseUseState, diagnostics: &mut Vec<String>) {
    if state.source.status != RuntimePromiseStatus::Pending {
        diagnostics.push("promise-pipeline-forward-requires-pending-source".to_string());
    }
    if state.admitted_resolution_ref.is_some() {
        diagnostics.push("promise-pipeline-forward-has-resolution-proof".to_string());
    }
    if state.admitted_pipeline_ref.is_none() {
        diagnostics.push("promise-use-pipeline-proof-missing".to_string());
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

fn validate_actormap_transaction(state: &RuntimeActormapTransactionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(24);
    diagnostics.extend(validate_sorted_content_refs(&state.before_object_refs, "actormap", "before-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.after_object_refs, "actormap", "after-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.spawned_object_refs, "actormap", "spawned-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.removed_object_refs, "actormap", "removed-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.visible_object_refs, "actormap", "visible-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.used_object_refs, "actormap", "used-object"));

    let before_refs = string_set(&state.before_object_refs);
    let after_refs = string_set(&state.after_object_refs);
    let spawned_refs = string_set(&state.spawned_object_refs);
    let removed_refs = string_set(&state.removed_object_refs);
    let visible_refs = string_set(&state.visible_object_refs);
    let used_refs = string_set(&state.used_object_refs);

    if has_set_intersection(&spawned_refs, &before_refs) {
        diagnostics.push("spawned-object-already-existed".to_string());
    }
    if !is_subset(&removed_refs, &before_refs) {
        diagnostics.push("removed-object-missing-before".to_string());
    }

    match state.outcome {
        RuntimeActormapTransactionOutcome::Committed => {
            let mut expected_after = before_refs.clone();
            for removed in &removed_refs {
                expected_after.remove(*removed);
            }
            for spawned in &spawned_refs {
                expected_after.insert(*spawned);
            }
            if expected_after != after_refs {
                diagnostics.push("actormap-commit-delta-mismatch".to_string());
            }
            if !is_subset(&spawned_refs, &after_refs) {
                diagnostics.push("spawned-object-missing-after-commit".to_string());
            }
            if !is_subset(&spawned_refs, &visible_refs) {
                diagnostics.push("spawned-object-not-visible-after-commit".to_string());
            }
            if has_set_intersection(&removed_refs, &after_refs) {
                diagnostics.push("removed-object-present-after-commit".to_string());
            }
            if has_set_intersection(&removed_refs, &visible_refs) {
                diagnostics.push("removed-object-visible-after-commit".to_string());
            }
            if has_set_intersection(&removed_refs, &used_refs) {
                diagnostics.push("removed-object-used-after-removal".to_string());
            }
        }
        RuntimeActormapTransactionOutcome::RolledBack => {
            if before_refs != after_refs {
                diagnostics.push("actormap-rollback-state-changed".to_string());
            }
            if has_set_intersection(&spawned_refs, &visible_refs) {
                diagnostics.push("spawned-object-visible-after-rollback".to_string());
            }
            if has_set_intersection(&spawned_refs, &used_refs) {
                diagnostics.push("spawned-object-used-after-rollback".to_string());
            }
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn string_set(refs: &[String]) -> OrderedSet<&str> {
    refs.iter().map(String::as_str).collect()
}

fn is_subset(left: &OrderedSet<&str>, right: &OrderedSet<&str>) -> bool {
    for item in left {
        if !right.contains(item) {
            return false;
        }
    }
    true
}

fn has_set_intersection(left: &OrderedSet<&str>, right: &OrderedSet<&str>) -> bool {
    for item in left {
        if right.contains(item) {
            return true;
        }
    }
    false
}

fn set_intersection<'a>(left: &OrderedSet<&'a str>, right: &OrderedSet<&'a str>) -> OrderedSet<&'a str> {
    let mut intersection = OrderedSet::new();
    for item in left {
        if right.contains(item) {
            intersection.insert(*item);
        }
    }
    intersection
}

fn validate_promise_shape(state: &RuntimePromiseState, label: &str) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if state.promise_id.is_empty() {
        diagnostics.push(format!("{label}-promise-id-empty"));
    }
    match state.status {
        RuntimePromiseStatus::Pending => {
            if state.value_ref.is_some() || state.reason.is_some() || !state.caused_by.is_empty() {
                diagnostics.push(format!("{label}-pending-promise-has-terminal-data"));
            }
        }
        RuntimePromiseStatus::Resolved => {
            if !state.caused_by.is_empty() || state.reason.is_some() {
                diagnostics.push(format!("{label}-resolved-promise-has-failure-data"));
            }
            match state.value_ref.as_deref() {
                Some(value_ref) if crate::preserves_rail::validate_content_ref(value_ref).is_ok() => {}
                Some(_) => diagnostics.push(format!("{label}-resolved-value-ref-noncanonical")),
                None => diagnostics.push(format!("{label}-resolved-value-ref-missing")),
            }
        }
        RuntimePromiseStatus::Broken => {
            if state.value_ref.is_some() {
                diagnostics.push(format!("{label}-broken-promise-has-value"));
            }
            if state.reason.as_deref().is_none_or(str::is_empty) {
                diagnostics.push(format!("{label}-broken-reason-missing"));
            }
            diagnostics.extend(validate_sorted_content_refs(&state.caused_by, label, "causal-failure"));
        }
        RuntimePromiseStatus::Cancelled | RuntimePromiseStatus::TimedOut => {
            if state.value_ref.is_some() || !state.caused_by.is_empty() {
                diagnostics.push(format!("{label}-cancel-timeout-has-resolution-data"));
            }
            if state.reason.as_deref().is_none_or(str::is_empty) {
                diagnostics.push(format!("{label}-cancel-timeout-reason-missing"));
            }
        }
    }
    diagnostics
}

fn ref_list_value(label: &'static str, refs: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(
        refs.iter().map(crate::preserves_rail::string).collect(),
    )])
}

fn optional_ref_record(label: &'static str, reference: Option<&str>) -> IoValue {
    match reference {
        Some(reference) => crate::preserves_rail::record(label, vec![crate::preserves_rail::string(reference)]),
        None => crate::preserves_rail::record(label, Vec::new()),
    }
}

fn validate_sorted_content_refs(refs: &[String], label: &str, field: &str) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(refs.len() + 1);
    for reference in refs {
        if crate::preserves_rail::validate_content_ref(reference).is_err() {
            diagnostics.push(format!("{label}-{field}-ref-noncanonical"));
        }
    }
    let mut sorted_refs = refs.to_vec();
    sorted_refs.sort();
    sorted_refs.dedup();
    if sorted_refs != refs {
        diagnostics.push(format!("{label}-{field}-refs-not-sorted-unique"));
    }
    diagnostics
}
