
pub fn migration_recipe_value(input: &MigrationRecipeInput) -> Result<IoValue> {
    require_ref(&input.source_schema_ref, "migration source schema ref")?;
    require_ref(&input.target_schema_ref, "migration target schema ref")?;
    require_ref(&input.transformer_ref, "migration transformer ref")?;
    validate_transformer_kind(&input.transformer_kind)?;
    validate_migration_mode(&input.mode)?;
    validate_refs(&input.policy_refs, "migration policy ref")?;
    validate_refs(&input.evidence_refs, "migration evidence ref")?;
    let effect_manifest_ref = migration_effect_manifest_ref(input)?;
    let source_gate_refs = default_recipe_refs("migration-source-gate", &input.source_schema_ref);
    let provenance_refs = default_recipe_refs("migration-provenance", &input.transformer_ref);
    let test_evidence_refs = default_recipe_refs("migration-test-evidence", &input.transformer_ref);
    let rollback_ref = local_ref("migration-rollback", &input.source_schema_ref);
    let lineage_refs = default_recipe_refs("migration-lineage", &input.target_schema_ref);
    Ok(record("storage-migration-recipe-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA),
        record("source-schema-ref", vec![string(&input.source_schema_ref)]),
        record("target-schema-ref", vec![string(&input.target_schema_ref)]),
        record("transformer", vec![string(&input.transformer_ref), string(&input.transformer_kind)]),
        record("mode", vec![string(&input.mode)]),
        record("effect-manifest-ref", vec![string(effect_manifest_ref)]),
        record("handler-profile", vec![string(STORAGE_HANDLER_PROFILE_REDB)]),
        refs_record("policy", &input.policy_refs),
        refs_record("provenance", &provenance_refs),
        refs_record("source-gate", &source_gate_refs),
        refs_record("test-evidence", &test_evidence_refs),
        record("rollback", vec![string(rollback_ref)]),
        refs_record("lineage", &lineage_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "migration-recipe-artifact",
            "source-schema-binding",
            "target-schema-binding",
            "transformer-artifact-binding",
            "effect-manifest-binding",
            "handler-profile-binding",
            "policy-admission-required",
            "provenance-binding",
            "source-gate-binding",
            "test-evidence-binding",
            "rollback-binding",
            "lineage-binding",
            "migration-trace-required",
            "no-function-serialization",
        ]),
    ]))
}

pub fn parse_migration_recipe_value(value: &IoValue) -> Result<MigrationRecipe> {
    validate_no_executable_authority(value, "storage migration recipe")?;
    let recipe = simple_record(value, "storage-migration-recipe-v1", MIGRATION_RECIPE_FIELD_COUNT)?;
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
    let handler_profile = record_string(&recipe[6], "handler-profile")?;
    validate_handler_profile(&handler_profile)?;
    let policy_refs = record_ref_sequence(&recipe[7], "policy")?;
    let provenance_refs = record_ref_sequence(&recipe[8], "provenance")?;
    let source_gate_refs = record_ref_sequence(&recipe[9], "source-gate")?;
    let test_evidence_refs = record_ref_sequence(&recipe[10], "test-evidence")?;
    let lineage_refs = record_ref_sequence(&recipe[12], "lineage")?;
    let evidence_refs = record_ref_sequence(&recipe[13], "evidence")?;
    require_non_empty_refs(&policy_refs, "migration policy refs")?;
    require_non_empty_refs(&provenance_refs, "migration provenance refs")?;
    require_non_empty_refs(&source_gate_refs, "migration source-gate refs")?;
    require_non_empty_refs(&test_evidence_refs, "migration test evidence refs")?;
    require_non_empty_refs(&lineage_refs, "migration lineage refs")?;
    require_non_empty_refs(&evidence_refs, "migration evidence refs")?;
    let checks = parse_checks(&recipe[14])?;
    require_check(&checks, "migration-recipe-artifact", "storage migration recipe")?;
    require_check(&checks, "effect-manifest-binding", "storage migration recipe")?;
    require_check(&checks, "handler-profile-binding", "storage migration recipe")?;
    require_check(&checks, "source-gate-binding", "storage migration recipe")?;
    require_check(&checks, "test-evidence-binding", "storage migration recipe")?;
    require_check(&checks, "rollback-binding", "storage migration recipe")?;
    require_check(&checks, "lineage-binding", "storage migration recipe")?;
    require_check(&checks, "migration-trace-required", "storage migration recipe")?;
    require_check(&checks, "no-function-serialization", "storage migration recipe")?;
    Ok(MigrationRecipe {
        recipe_ref: canonical_hash(value)?,
        source_schema_ref: record_ref(&recipe[1], "source-schema-ref")?,
        target_schema_ref: record_ref(&recipe[2], "target-schema-ref")?,
        transformer_ref: required_ref(&transformer[0], "migration transformer ref")?,
        transformer_kind,
        mode,
        effect_manifest_ref: record_ref(&recipe[5], "effect-manifest-ref")?,
        handler_profile,
        policy_refs,
        provenance_refs,
        source_gate_refs,
        test_evidence_refs,
        rollback_ref: record_ref(&recipe[11], "rollback")?,
        lineage_refs,
        evidence_refs,
        checks,
        value: value.clone(),
    })
}

pub fn parse_entry_ref_value(value: &IoValue) -> Result<EntryRef> {
    validate_no_executable_authority(value, "typed storage ref")?;
    let fields = simple_record(value, "typed-storage-ref-v1", TYPED_STORAGE_REF_FIELD_COUNT)?;
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
    require_check(&checks, "schema-identity-binding", "typed storage ref")?;
    require_check(&checks, "producer-artifact-binding", "typed storage ref")?;
    require_check(&checks, "retention-binding", "typed storage ref")?;
    require_check(&checks, "provenance-binding", "typed storage ref")?;
    require_check(&checks, "decoder-artifact-admission", "typed storage ref")?;
    require_check(&checks, "handle-not-authority", "typed storage ref")?;
    let schema_identity_mode = record_string(
        &fields[TYPED_STORAGE_REF_SCHEMA_IDENTITY_FIELD_INDEX],
        "schema-identity",
    )?;
    validate_schema_identity_mode(&schema_identity_mode)?;
    let consumer_refs = record_ref_sequence(&fields[TYPED_STORAGE_REF_CONSUMERS_FIELD_INDEX], "consumers")?;
    let handler_profile = record_string(&fields[TYPED_STORAGE_REF_HANDLER_PROFILE_FIELD_INDEX], "handler-profile")?;
    validate_handler_profile(&handler_profile)?;
    let capability_refs = record_ref_sequence(&fields[TYPED_STORAGE_REF_CAPABILITIES_FIELD_INDEX], "capabilities")?;
    let retention_refs = record_ref_sequence(&fields[TYPED_STORAGE_REF_RETENTION_FIELD_INDEX], "retention")?;
    let provenance_refs = record_ref_sequence(&fields[TYPED_STORAGE_REF_PROVENANCE_FIELD_INDEX], "provenance")?;
    let decoder_artifact_refs = record_ref_sequence(
        &fields[TYPED_STORAGE_REF_DECODER_ARTIFACTS_FIELD_INDEX],
        "decoder-artifacts",
    )?;
    require_non_empty_refs(&policy_refs, "typed storage policy refs")?;
    require_non_empty_refs(&capability_refs, "typed storage capability refs")?;
    require_non_empty_refs(&retention_refs, "typed storage retention refs")?;
    require_non_empty_refs(&provenance_refs, "typed storage provenance refs")?;
    let storage_ref = canonical_hash(value)?;
    Ok(EntryRef {
        storage_ref,
        namespace,
        key,
        schema_ref,
        schema_identity_mode,
        value_ref,
        payload,
        producer_ref,
        consumer_refs,
        handler_profile,
        policy_refs,
        capability_refs,
        retention_refs,
        provenance_refs,
        evidence_refs,
        decoder_artifact_refs,
        revision,
        actor_ref,
        capability_ref,
        effect_handle_ref,
        checks,
        value: value.clone(),
    })
}

pub fn parse_receipt_value(value: &IoValue, expected_receipt_ref: Option<&str>) -> Result<Receipt> {
    let fields = simple_record(value, "typed-storage-receipt-v1", TYPED_STORAGE_RECEIPT_FIELD_COUNT)?;
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
    consumer_refs: &'a [String],
    handler_profile: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    retention_refs: &'a [String],
    provenance_refs: &'a [String],
    evidence_refs: &'a [String],
    decoder_artifact_refs: &'a [String],
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
            "schema-identity-binding",
            "value-ref-binding",
            "producer-artifact-binding",
            "intended-consumer-binding",
            "handler-profile-binding",
            "capability-binding",
            "retention-binding",
            "provenance-binding",
            "evidence-binding",
            "decoder-artifact-admission",
            "handle-not-authority",
            "no-raw-memory-layout",
            "no-function-serialization",
        ]),
        record("schema-identity", vec![string(SCHEMA_IDENTITY_MODE_INFERRED_PRESERVES_CLASS)]),
        refs_record("consumers", input.consumer_refs),
        record("handler-profile", vec![string(input.handler_profile)]),
        refs_record("capabilities", input.capability_refs),
        refs_record("retention", input.retention_refs),
        refs_record("provenance", input.provenance_refs),
        refs_record("decoder-artifacts", input.decoder_artifact_refs),
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
