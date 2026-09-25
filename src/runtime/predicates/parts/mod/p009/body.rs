
pub fn evaluate_distributed_ref_lifetime(
    state: &RuntimeDistributedRefLifetimeState,
) -> Result<DistributedRefLifetimeResult> {
    let diagnostics = validate_distributed_ref_lifetime(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let lifetime_ref = state.lifetime_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-distributed-ref-lifetime-input-v1", vec![
        crate::preserves_rail::record("lifetime-ref", vec![crate::preserves_rail::string(&lifetime_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "distributed-ref-refs-canonical".to_string(),
        "active-session-required-for-original-far-ref".to_string(),
        "disconnect-fails-dependent-pending-calls".to_string(),
        "handoff-requires-admitted-replacement".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(4);
    state_refs.push(lifetime_ref);
    if crate::preserves_rail::validate_content_ref(&state.far_ref).is_ok() {
        state_refs.push(state.far_ref.clone());
    }
    if crate::preserves_rail::validate_content_ref(&state.session_ref).is_ok() {
        state_refs.push(state.session_ref.clone());
    }
    if let Some(replacement_ref) = state
        .replacement_ref
        .as_ref()
        .filter(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
    {
        state_refs.push(replacement_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: DISTRIBUTED_REF_LIFETIME_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(DistributedRefLifetimeResult { is_allowed, receipt })
}

// r[impl molten.vat_ref_state_proof.rollback_cleanup]
pub fn evaluate_vat_rollback_cleanup(
    state: &RuntimeVatRollbackCleanupState,
) -> Result<VatRollbackCleanupResult> {
    let diagnostics = validate_vat_rollback_cleanup(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let cleanup_ref = state.cleanup_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-vat-rollback-cleanup-input-v1", vec![
        crate::preserves_rail::record("cleanup-ref", vec![crate::preserves_rail::string(&cleanup_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "rollback-receipt-canonical".to_string(),
        "rollback-preserves-snapshot-ref".to_string(),
        "rolled-back-assertions-cleaned".to_string(),
        "rolled-back-observers-cleaned".to_string(),
        "rolled-back-pending-calls-cleaned".to_string(),
        "rolled-back-authority-snapshots-cleaned".to_string(),
    ];
    let mut state_refs = vec![cleanup_ref];
    for reference in [
        &state.rollback_receipt_ref,
        &state.before_snapshot_ref,
        &state.final_snapshot_ref,
    ] {
        if crate::preserves_rail::validate_content_ref(reference).is_ok() {
            state_refs.push(reference.clone());
        }
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: VAT_ROLLBACK_CLEANUP_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(VatRollbackCleanupResult { is_allowed, receipt })
}
