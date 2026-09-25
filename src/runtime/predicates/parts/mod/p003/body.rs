
pub fn evaluate_promise_pipeline(state: &RuntimePromisePipelineState) -> Result<PromisePipelineResult> {
    let diagnostics = validate_promise_pipeline(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let pipeline_ref = state.pipeline_ref()?;
    let source_ref = state.source.promise_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-promise-pipeline-input-v1", vec![
        crate::preserves_rail::record("pipeline-ref", vec![crate::preserves_rail::string(&pipeline_ref)]),
        crate::preserves_rail::record("source-promise-ref", vec![crate::preserves_rail::string(&source_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "bounded-promise-pipeline-queue".to_string(),
        "pending-source-allows-forwarding".to_string(),
        "terminal-source-cleans-pipeline".to_string(),
        "deterministic-forwarding-order".to_string(),
        "pipeline-target-refs-canonical".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: PROMISE_PIPELINE_PREDICATE,
        input_value,
        decision,
        state_refs: vec![pipeline_ref, source_ref],
        checks,
        diagnostics,
    })?;

    Ok(PromisePipelineResult { is_allowed, receipt })
}

// r[impl molten.vat_ref_state_proof.promise_lifecycle]
pub fn evaluate_promise_use(state: &RuntimePromiseUseState) -> Result<PromiseUseResult> {
    let diagnostics = validate_promise_use(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let use_ref = state.use_ref()?;
    let source_ref = state.source.promise_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-promise-use-input-v1", vec![
        crate::preserves_rail::record("use-ref", vec![crate::preserves_rail::string(&use_ref)]),
        crate::preserves_rail::record("source-promise-ref", vec![crate::preserves_rail::string(&source_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "promise-use-ref-canonical".to_string(),
        "resolved-value-use-requires-resolution".to_string(),
        "pending-pipeline-use-requires-proof".to_string(),
        "terminal-promise-use-fails-closed".to_string(),
    ];
    let mut state_refs = vec![use_ref, source_ref];
    if crate::preserves_rail::validate_content_ref(&state.dependent_call_ref).is_ok() {
        state_refs.push(state.dependent_call_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: PROMISE_USE_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(PromiseUseResult { is_allowed, receipt })
}

pub fn evaluate_revocation_cleanup(state: &RuntimeRevocationCleanupState) -> Result<RevocationCleanupResult> {
    let diagnostics = validate_revocation_cleanup(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let cleanup_ref = state.cleanup_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-revocation-cleanup-input-v1", vec![
        crate::preserves_rail::record("cleanup-ref", vec![crate::preserves_rail::string(&cleanup_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "revoked-refs-canonical".to_string(),
        "revoked-refs-deny-future-use".to_string(),
        "dependent-assertions-cleaned".to_string(),
        "dependent-subscriptions-cleaned".to_string(),
        "pending-calls-cleaned".to_string(),
        "child-refs-cleaned".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: REVOCATION_CLEANUP_PREDICATE,
        input_value,
        decision,
        state_refs: vec![cleanup_ref],
        checks,
        diagnostics,
    })?;

    Ok(RevocationCleanupResult { is_allowed, receipt })
}

pub fn evaluate_actormap_transaction(state: &RuntimeActormapTransactionState) -> Result<ActormapTransactionResult> {
    let diagnostics = validate_actormap_transaction(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let transaction_ref = state.transaction_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-actormap-transaction-input-v1", vec![
        crate::preserves_rail::record("transaction-ref", vec![crate::preserves_rail::string(&transaction_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "actormap-refs-canonical".to_string(),
        "actormap-delta-commit".to_string(),
        "actormap-rollback-preserves-state".to_string(),
        "spawned-object-visibility-after-commit".to_string(),
        "removed-object-invalidation".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: ACTORMAP_TRANSACTION_PREDICATE,
        input_value,
        decision,
        state_refs: vec![transaction_ref],
        checks,
        diagnostics,
    })?;

    Ok(ActormapTransactionResult { is_allowed, receipt })
}

pub fn evaluate_snapshot_authority(state: &RuntimeSnapshotAuthorityState) -> Result<SnapshotAuthorityResult> {
    let diagnostics = validate_snapshot_authority(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let authority_ref = state.authority_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-snapshot-authority-input-v1", vec![
        crate::preserves_rail::record("authority-ref", vec![crate::preserves_rail::string(&authority_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "snapshot-ref-canonical".to_string(),
        "snapshot-authority-refs-canonical".to_string(),
        "claimed-authority-subset-admitted".to_string(),
        "readable-assertions-subset-claimed".to_string(),
        "requested-assertions-readable-or-redacted".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(2);
    state_refs.push(authority_ref);
    if crate::preserves_rail::validate_content_ref(&state.snapshot_ref).is_ok() {
        state_refs.push(state.snapshot_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: SNAPSHOT_AUTHORITY_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(SnapshotAuthorityResult { is_allowed, receipt })
}

pub fn evaluate_object_authority(state: &RuntimeObjectAuthorityState) -> Result<ObjectAuthorityResult> {
    let diagnostics = validate_object_authority(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let authority_ref = state.authority_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-object-authority-input-v1", vec![
        crate::preserves_rail::record("authority-ref", vec![crate::preserves_rail::string(&authority_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "object-authority-refs-canonical".to_string(),
        "new-object-starts-without-ambient-authority".to_string(),
        "requested-authority-explicitly-endowed".to_string(),
        "requested-authority-policy-admitted".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(3);
    state_refs.push(authority_ref);
    if crate::preserves_rail::validate_content_ref(&state.object_ref).is_ok() {
        state_refs.push(state.object_ref.clone());
    }
    if crate::preserves_rail::validate_content_ref(&state.requested_authority_ref).is_ok() {
        state_refs.push(state.requested_authority_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: OBJECT_AUTHORITY_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(ObjectAuthorityResult { is_allowed, receipt })
}

pub fn evaluate_rights_amplification(state: &RuntimeRightsAmplificationState) -> Result<RightsAmplificationResult> {
    let diagnostics = validate_rights_amplification(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let amplification_ref = state.amplification_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-rights-amplification-input-v1", vec![
        crate::preserves_rail::record("amplification-ref", vec![crate::preserves_rail::string(&amplification_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "rights-amplification-refs-canonical".to_string(),
        "matching-private-brand-required".to_string(),
        "recovered-authority-subset-sealed".to_string(),
        "no-ambient-identity-amplification".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(4);
    state_refs.push(amplification_ref);
    if crate::preserves_rail::validate_content_ref(&state.holder_object_ref).is_ok() {
        state_refs.push(state.holder_object_ref.clone());
    }
    if crate::preserves_rail::validate_content_ref(&state.sealed_value_ref).is_ok() {
        state_refs.push(state.sealed_value_ref.clone());
    }
    if crate::preserves_rail::validate_content_ref(&state.sealer_brand_ref).is_ok() {
        state_refs.push(state.sealer_brand_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: RIGHTS_AMPLIFICATION_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(RightsAmplificationResult { is_allowed, receipt })
}
