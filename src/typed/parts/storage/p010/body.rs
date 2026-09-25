
struct ScopeParts {
    manifest: String,
    run: String,
    session: String,
    turn: String,
    scope: EffectScope,
    evidence: Vec<String>,
}

fn effect_evidence(input: EffectEvidenceInput<'_>) -> Result<EffectEvidence> {
    validate_operation(input.operation)?;
    let parts = scope_parts(&input)?;
    let handler = binding(&input, &parts)?;
    let handler_binding_ref = canonical_hash(&handler)?;
    let handle = handle(&input, &parts, &handler_binding_ref)?;
    let handle_ref = canonical_hash(&handle)?;
    let validation = validate_handle_for_request(&handler, &handle, &crate::effects::EffectHandleRequest {
        kind: ADAPTER_KIND_STORAGE,
        operation: input.operation,
        run_ref: &parts.run,
        session_ref: &parts.session,
        actor_ref: Some(&input.admission.actor_ref),
        turn_ref: Some(&parts.turn),
        policy_ref: &input.admission.policy_ref,
        capability_context_ref: &input.admission.capability_ref,
        context_ref: None,
        resource_refs: &input.admission.resource_refs,
        logical_time: 0,
        remote_use: input.remote_use,
        revoked_refs: &[],
    })?;
    if validation.handler_binding_ref != handler_binding_ref || validation.handle_ref != handle_ref {
        return Err(MoltenError::invalid_harness("typed storage handle validation ref mismatch"));
    }
    Ok(EffectEvidence {
        manifest_ref: parts.manifest,
        handler_binding_ref,
        handle_ref,
    })
}

fn scope_parts(input: &EffectEvidenceInput<'_>) -> Result<ScopeParts> {
    let manifest =
        effect_manifest_value(input.producer_ref, input.namespace, input.schema_ref, &[input.operation.to_string()])?;
    let manifest = canonical_hash(&manifest)?;
    let run = canonical_hash(&record("typed-storage-run", vec![string(input.namespace), string(input.schema_ref)]))?;
    let session = canonical_hash(&record("typed-storage-session", vec![
        string(input.namespace),
        string(&input.admission.policy_ref),
        string(&input.admission.capability_ref),
    ]))?;
    let turn = canonical_hash(&record("typed-storage-operation", vec![
        string(input.operation),
        string(input.namespace),
        string(input.key),
        string(input.schema_ref),
    ]))?;
    let scope = EffectScope {
        run_ref: run.clone(),
        session_ref: session.clone(),
        actor_ref: Some(input.admission.actor_ref.clone()),
        turn_ref: Some(turn.clone()),
    };
    let mut evidence = vec![manifest.clone()];
    evidence.extend(input.admission.evidence_refs.clone());
    Ok(ScopeParts {
        manifest,
        run,
        session,
        turn,
        scope,
        evidence,
    })
}

fn binding(input: &EffectEvidenceInput<'_>, parts: &ScopeParts) -> Result<IoValue> {
    let adapter_ref =
        canonical_hash(&record("typed-storage-redb-adapter", vec![string(input.namespace), string(input.schema_ref)]))?;
    handler_binding_value(&crate::effects::HandlerBindingInput {
        profile: STORAGE_HANDLER_PROFILE_REDB.to_string(),
        scope: parts.scope.clone(),
        adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
        adapter_ref,
        executor_preflight_ref: None,
        policy_ref: input.admission.policy_ref.clone(),
        capability_context_ref: input.admission.capability_ref.clone(),
        context_ref: None,
        resource_refs: input.admission.resource_refs.clone(),
        operations: vec![input.operation.to_string()],
        evidence_refs: parts.evidence.clone(),
    })
}

fn handle(input: &EffectEvidenceInput<'_>, parts: &ScopeParts, handler_binding_ref: &str) -> Result<IoValue> {
    effect_handle_value(&crate::effects::EffectHandleInput {
        kind: ADAPTER_KIND_STORAGE.to_string(),
        scope: parts.scope.clone(),
        handler_binding_ref: handler_binding_ref.to_string(),
        operations: vec![input.operation.to_string()],
        capability_context_ref: input.admission.capability_ref.clone(),
        context_ref: None,
        resource_refs: input.admission.resource_refs.clone(),
        not_before: Some(0),
        expires_at: None,
        revocation_refs: Vec::new(),
        transfer: crate::effects::TRANSFER_LOCAL_ONLY.to_string(),
        parent_handle_ref: None,
        evidence_refs: parts.evidence.clone(),
    })
}
