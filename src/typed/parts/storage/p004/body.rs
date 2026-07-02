
pub fn migration_recipe_value(input: &MigrationRecipeInput) -> Result<IoValue> {
    require_ref(&input.source_schema_ref, "migration source schema ref")?;
    require_ref(&input.target_schema_ref, "migration target schema ref")?;
    require_ref(&input.transformer_ref, "migration transformer ref")?;
    validate_transformer_kind(&input.transformer_kind)?;
    validate_migration_mode(&input.mode)?;
    validate_refs(&input.policy_refs, "migration policy ref")?;
    validate_refs(&input.evidence_refs, "migration evidence ref")?;
    Ok(record("storage-migration-recipe-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA),
        record("source-schema-ref", vec![string(&input.source_schema_ref)]),
        record("target-schema-ref", vec![string(&input.target_schema_ref)]),
        record("transformer", vec![string(&input.transformer_ref), string(&input.transformer_kind)]),
        record("mode", vec![string(&input.mode)]),
        refs_record("policy", &input.policy_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "migration-recipe-artifact",
            "source-schema-binding",
            "target-schema-binding",
            "transformer-artifact-binding",
            "policy-admission-required",
            "migration-trace-required",
        ]),
    ]))
}

pub fn parse_migration_recipe_value(value: &IoValue) -> Result<MigrationRecipe> {
    let recipe = simple_record(value, "storage-migration-recipe-v1", 8)?;
    require_schema(
        &recipe[0],
        crate::preserves_rail::TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA,
        "storage migration recipe",
    )?;
    let transformer = value_to_iovalue(&recipe[3]);
    let transformer = simple_record(&transformer, "transformer", 2)?;
    let transformer_kind = required_string(&transformer[1], "migration transformer kind")?;
    validate_transformer_kind(&transformer_kind)?;
    let mode = record_string(&recipe[4], "mode")?;
    validate_migration_mode(&mode)?;
    let checks = parse_checks(&recipe[7])?;
    require_check(&checks, "migration-recipe-artifact", "storage migration recipe")?;
    require_check(&checks, "migration-trace-required", "storage migration recipe")?;
    Ok(MigrationRecipe {
        recipe_ref: canonical_hash(value)?,
        source_schema_ref: record_ref(&recipe[1], "source-schema-ref")?,
        target_schema_ref: record_ref(&recipe[2], "target-schema-ref")?,
        transformer_ref: required_ref(&transformer[0], "migration transformer ref")?,
        transformer_kind,
        mode,
        policy_refs: record_ref_sequence(&recipe[5], "policy")?,
        evidence_refs: record_ref_sequence(&recipe[6], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_entry_ref_value(value: &IoValue) -> Result<EntryRef> {
    let fields = simple_record(value, "typed-storage-ref-v1", 12)?;
    require_schema(&fields[0], crate::preserves_rail::TYPED_STORAGE_REF_SCHEMA, "typed storage ref")?;
    let namespace = record_string(&fields[1], "namespace")?;
    let key = record_string(&fields[2], "key")?;
    let schema_ref = record_ref(&fields[3], "schema-ref")?;
    let value_ref = record_ref(&fields[4], "value-ref")?;
    let payload = parse_payload(&fields[5])?;
    let producer_ref = record_ref(&fields[6], "producer")?;
    let policy_refs = record_ref_sequence(&fields[7], "policy")?;
    let evidence_refs = record_ref_sequence(&fields[8], "evidence")?;
    let revision = record_u64(&fields[9], "revision")?;
    let authority_value = value_to_iovalue(&fields[10]);
    let authority = simple_record(&authority_value, "authority", 3)?;
    let actor_ref = required_ref(&authority[0], "typed storage actor ref")?;
    let capability_ref = required_ref(&authority[1], "typed storage capability ref")?;
    let effect_handle_ref = required_ref(&authority[2], "typed storage effect handle ref")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "typed-durable-ref", "typed storage ref")?;
    require_check(&checks, "handle-not-authority", "typed storage ref")?;
    let storage_ref = canonical_hash(value)?;
    Ok(EntryRef {
        storage_ref,
        namespace,
        key,
        schema_ref,
        value_ref,
        payload,
        producer_ref,
        policy_refs,
        evidence_refs,
        revision,
        actor_ref,
        capability_ref,
        effect_handle_ref,
        checks,
        value: value.clone(),
    })
}

pub fn parse_receipt_value(value: &IoValue, expected_receipt_ref: Option<&str>) -> Result<Receipt> {
    let fields = simple_record(value, "typed-storage-receipt-v1", 9)?;
    require_schema(&fields[0], crate::preserves_rail::TYPED_STORAGE_RECEIPT_SCHEMA, "typed storage receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    let decision = record_string(&fields[2], "decision")?;
    if decision != "pass" && decision != "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "typed storage receipt decision must be pass or deny, got {decision}"
        )));
    }
    let storage_ref = record_optional_ref(&fields[3], "storage-ref")?;
    let binding = parse_binding_record(&fields[4])?;
    let value_ref = binding.value_ref.clone();
    let checks = parse_checks(&fields[6])?;
    let receipt_ref = canonical_hash(value)?;
    if let Some(expected) = expected_receipt_ref
        && receipt_ref != expected
    {
        return Err(MoltenError::invalid_harness(format!(
            "typed storage receipt hash mismatch: got {receipt_ref}, expected {expected}"
        )));
    }
    Ok(Receipt {
        receipt_ref,
        operation,
        decision,
        storage_ref,
        namespace: binding.namespace,
        key: binding.key,
        schema_ref: binding.schema_ref,
        value_ref,
        checks,
        value: value.clone(),
    })
}

struct RefValueInput<'a> {
    namespace: &'a str,
    key: &'a str,
    schema_ref: &'a str,
    value_ref: &'a str,
    payload: &'a IoValue,
    producer_ref: &'a str,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    revision: u64,
    actor_ref: &'a str,
    capability_ref: &'a str,
    effect_handle_ref: &'a str,
}

fn ref_value(input: RefValueInput<'_>) -> IoValue {
    record("typed-storage-ref-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_REF_SCHEMA),
        record("namespace", vec![string(input.namespace)]),
        record("key", vec![string(input.key)]),
        record("schema-ref", vec![string(input.schema_ref)]),
        record("value-ref", vec![string(input.value_ref)]),
        record("payload", vec![input.payload.clone()]),
        record("producer", vec![string(input.producer_ref)]),
        refs_record("policy", input.policy_refs),
        refs_record("evidence", input.evidence_refs),
        record("revision", vec![u64_value(input.revision)]),
        record("authority", vec![
            string(input.actor_ref),
            string(input.capability_ref),
            string(input.effect_handle_ref),
        ]),
        checks_value(&[
            "typed-durable-ref",
            "schema-ref-binding",
            "value-ref-binding",
            "producer-artifact-binding",
            "handle-not-authority",
            "no-raw-memory-layout",
        ]),
    ])
}

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
        profile: "typed-storage-redb".to_string(),
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
