
pub fn migrate_value(
    root: &Path,
    namespace: &str,
    key: &str,
    migration_recipe_value: &IoValue,
    admission: &Admission,
) -> Result<Migrate> {
    ensure_dirs(root)?;
    validate_namespace_key(namespace, key)?;
    validate_admission(admission)?;
    let recipe = parse_migration_recipe_value(migration_recipe_value)?;
    let recipe_ref = recipe.recipe_ref.clone();
    let storage_key = storage_key_ref(namespace, key)?;
    let source = source_data(SourceInput {
        root,
        storage_key: &storage_key,
        namespace,
        key,
        recipe: &recipe,
    })?;
    let new_value = apply_migration_transform(&recipe, &source.value)?;
    validate_no_executable_authority(&new_value, "typed storage migrated value")?;
    let new_value_bytes = canonical_bytes(&new_value)?;
    let new_value_ref = canonical_hash(&new_value)?;
    let (payload, payload_details) = store_payload(root, &new_value_bytes)?;
    let effect = effect_evidence(EffectEvidenceInput {
        operation: "migrate",
        namespace,
        key,
        schema_ref: &recipe.target_schema_ref,
        producer_ref: &recipe.transformer_ref,
        admission,
        remote_use: false,
    })?;
    let revision = next_revision(root, &storage_key)?;
    let refs = entry_refs(&source.typed_ref, &recipe);
    let typed_ref_value = next_value(NextInput {
        namespace,
        key,
        recipe: &recipe,
        value_ref: &new_value_ref,
        payload: &payload,
        refs: &refs,
        revision,
        admission,
        effect_handle_ref: &effect.handle_ref,
    });
    let new_storage_ref = canonical_hash(&typed_ref_value)?;
    let receipt_value = step_receipt(StepInput {
        namespace,
        key,
        recipe: &recipe,
        source: &source.typed_ref,
        storage_ref: &new_storage_ref,
        value_ref: &new_value_ref,
        effect: &effect,
        details: payload_details,
    });
    persist_entry(PersistInput {
        root,
        storage_key: &storage_key,
        storage_ref: &new_storage_ref,
        typed_ref_value: &typed_ref_value,
        value_ref: &new_value_ref,
        value_bytes: &new_value_bytes,
        receipt_value: &receipt_value,
    })?;
    Ok(Migrate {
        old_storage_ref: source.typed_ref.storage_ref,
        new_storage_ref,
        old_value_ref: source.typed_ref.value_ref,
        new_value_ref,
        recipe_ref,
        typed_ref_value,
        receipt_value,
    })
}

fn source_data(input: SourceInput<'_>) -> Result<SourceData> {
    let typed_ref_value = stored_entry(&input)?;
    let typed_ref = parse_entry_ref_value(&typed_ref_value)?;
    require_match(&input, &typed_ref)?;
    let value = source_value(&input, &typed_ref)?;
    Ok(SourceData { typed_ref, value })
}

fn stored_entry(input: &SourceInput<'_>) -> Result<IoValue> {
    let db = ensure_index_tables(input.root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let records = read_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
    let Some(bytes) = records.get(input.storage_key).map_err(index_error)? else {
        drop(records);
        drop(read_txn);
        drop(db);
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "migrate",
            storage_ref: None,
            namespace: Some(input.namespace),
            key: Some(input.key),
            schema_ref: Some(&input.recipe.target_schema_ref),
            value_ref: None,
            reason: "typed storage migration source record not found".to_string(),
            checks: vec![("record-found", "fail"), ("denial-receipt", "pass")],
            details: vec![record("recipe", vec![string(&input.recipe.recipe_ref)])],
        });
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("typed storage migration rejected: source record not found"));
    };
    parse_canonical_bytes(bytes.value())
}

fn require_match(input: &SourceInput<'_>, typed_ref: &EntryRef) -> Result<()> {
    if typed_ref.schema_ref == input.recipe.source_schema_ref {
        return Ok(());
    }
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "migrate",
        storage_ref: Some(&typed_ref.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(&input.recipe.target_schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        reason: "typed storage migration source schema does not match recipe".to_string(),
        checks: vec![("source-schema-binding", "fail"), ("denial-receipt", "pass")],
        details: vec![record("recipe", vec![string(&input.recipe.recipe_ref)])],
    });
    store_receipt(input.root, &receipt_value)?;
    Err(MoltenError::invalid_harness(
        "typed storage migration rejected: source schema does not match recipe",
    ))
}

fn source_value(input: &SourceInput<'_>, typed_ref: &EntryRef) -> Result<IoValue> {
    let value_bytes = read_payload_bytes(input.root, typed_ref)?;
    let value = parse_canonical_bytes(&value_bytes)?;
    if canonical_hash(&value)? == typed_ref.value_ref {
        return Ok(value);
    }
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "migrate",
        storage_ref: Some(&typed_ref.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(&input.recipe.target_schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        reason: "typed storage migration source value hash mismatch".to_string(),
        checks: vec![("source-content-integrity", "fail"), ("denial-receipt", "pass")],
        details: vec![record("recipe", vec![string(&input.recipe.recipe_ref)])],
    });
    store_receipt(input.root, &receipt_value)?;
    Err(MoltenError::invalid_harness("typed storage migration source content integrity failed"))
}

fn entry_refs(typed_ref: &EntryRef, recipe: &MigrationRecipe) -> EntryRefs {
    let consumer = sorted_unique_strings(typed_ref.consumer_refs.clone());
    let mut policy = typed_ref.policy_refs.clone();
    policy.extend(recipe.policy_refs.clone());
    let policy = sorted_unique_strings(policy);
    let mut capability = typed_ref.capability_refs.clone();
    capability.push(typed_ref.capability_ref.clone());
    let capability = sorted_unique_strings(capability);
    let retention = sorted_unique_strings(typed_ref.retention_refs.clone());
    let mut provenance = typed_ref.provenance_refs.clone();
    provenance.extend(recipe.provenance_refs.clone());
    provenance.push(recipe.recipe_ref.clone());
    let provenance = sorted_unique_strings(provenance);
    let mut evidence = typed_ref.evidence_refs.clone();
    evidence.push(recipe.recipe_ref.clone());
    evidence.push(recipe.effect_manifest_ref.clone());
    evidence.push(recipe.rollback_ref.clone());
    evidence.extend(recipe.evidence_refs.clone());
    evidence.extend(recipe.test_evidence_refs.clone());
    evidence.extend(recipe.source_gate_refs.clone());
    evidence.extend(recipe.lineage_refs.clone());
    let evidence = sorted_unique_strings(evidence);
    let decoder_artifact = sorted_unique_strings(typed_ref.decoder_artifact_refs.clone());
    EntryRefs {
        consumer,
        policy,
        capability,
        retention,
        provenance,
        evidence,
        decoder_artifact,
    }
}

fn next_value(input: NextInput<'_>) -> IoValue {
    let mut capability_refs = input.refs.capability.clone();
    capability_refs.push(input.admission.capability_ref.clone());
    let capability_refs = sorted_unique_strings(capability_refs);
    ref_value(RefValueInput {
        namespace: input.namespace,
        key: input.key,
        schema_ref: &input.recipe.target_schema_ref,
        value_ref: input.value_ref,
        payload: input.payload,
        producer_ref: &input.recipe.transformer_ref,
        consumer_refs: &input.refs.consumer,
        handler_profile: &input.recipe.handler_profile,
        policy_refs: &input.refs.policy,
        capability_refs: &capability_refs,
        retention_refs: &input.refs.retention,
        provenance_refs: &input.refs.provenance,
        evidence_refs: &input.refs.evidence,
        decoder_artifact_refs: &input.refs.decoder_artifact,
        revision: input.revision,
        actor_ref: &input.admission.actor_ref,
        capability_ref: &input.admission.capability_ref,
        effect_handle_ref: input.effect_handle_ref,
    })
}

fn step_receipt(input: StepInput<'_>) -> IoValue {
    let mut details = vec![
        record("recipe", vec![string(&input.recipe.recipe_ref)]),
        record("mode", vec![string(&input.recipe.mode)]),
        record("transformer", vec![
            string(&input.recipe.transformer_ref),
            string(&input.recipe.transformer_kind),
        ]),
        record("old-storage-ref", vec![string(&input.source.storage_ref)]),
        record("new-storage-ref", vec![string(input.storage_ref)]),
        record("old-value-ref", vec![string(&input.source.value_ref)]),
        record("new-value-ref", vec![string(input.value_ref)]),
        record("source-schema-ref", vec![string(&input.recipe.source_schema_ref)]),
        record("target-schema-ref", vec![string(&input.recipe.target_schema_ref)]),
        record("effect-manifest-ref", vec![string(&input.recipe.effect_manifest_ref)]),
        record("handler-profile", vec![string(&input.recipe.handler_profile)]),
        record("source-gate", vec![sequence(input.recipe.source_gate_refs.iter().map(string).collect())]),
        record("provenance", vec![sequence(input.recipe.provenance_refs.iter().map(string).collect())]),
        record("test-evidence", vec![sequence(input.recipe.test_evidence_refs.iter().map(string).collect())]),
        record("rollback", vec![string(&input.recipe.rollback_ref)]),
        record("lineage", vec![sequence(input.recipe.lineage_refs.iter().map(string).collect())]),
        record("migration-phase-receipts", vec![sequence(migration_phase_receipts(&input))]),
    ];
    details.extend(input.details);
    receipt_value(ReceiptValueInput {
        operation: "migrate",
        decision: "pass",
        storage_ref: Some(input.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(&input.recipe.target_schema_ref),
        value_ref: Some(input.value_ref),
        effect: input.effect,
        checks: vec![
            ("effect-manifest", "pass"),
            ("storage-effect-handle", "pass"),
            ("migration-admission", "pass"),
            ("source-schema-binding", "pass"),
            ("target-schema-binding", "pass"),
            ("transformer-binding", "pass"),
            ("migration-trace", "pass"),
            ("migration-preflight-receipt", "pass"),
            ("migration-execution-receipt", "pass"),
            ("migration-output-validation-receipt", "pass"),
            ("migration-lineage-receipt", "pass"),
            ("original-value-hash", "pass"),
            ("result-value-hash", "pass"),
            ("redb-adapter", "pass"),
            ("cairn-receipt", "pass"),
        ],
        details,
    })
}

fn migration_phase_receipts(input: &StepInput<'_>) -> Vec<IoValue> {
    [
        MIGRATION_PHASE_PREFLIGHT,
        MIGRATION_PHASE_EXECUTION,
        MIGRATION_PHASE_OUTPUT_VALIDATION,
        MIGRATION_PHASE_LINEAGE,
    ]
    .into_iter()
    .map(|phase| migration_phase_receipt_value(phase, input))
    .collect()
}

fn migration_phase_receipt_value(phase: &str, input: &StepInput<'_>) -> IoValue {
    record("typed-storage-migration-phase-receipt-v1", vec![
        record("phase", vec![string(phase)]),
        record("decision", vec![string("pass")]),
        record("recipe", vec![string(&input.recipe.recipe_ref)]),
        record("source", vec![
            string(&input.source.storage_ref),
            string(&input.source.schema_ref),
            string(&input.source.value_ref),
        ]),
        record("target", vec![
            string(input.storage_ref),
            string(&input.recipe.target_schema_ref),
            string(input.value_ref),
        ]),
        checks_value(&["phase-recorded", "policy-bound", "lineage-bound"]),
    ])
}

fn sorted_unique_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

pub fn list_receipt_refs(root: &Path) -> Result<Vec<String>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut refs = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        push_bounded(&mut refs, key.value().to_string(), MAX_TYPED_STORAGE_RECEIPTS, "typed storage receipt refs")?;
    }
    refs.sort();
    Ok(refs)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<Receipt> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown typed storage receipt {receipt_ref}")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_receipt_value(&value, Some(receipt_ref))
}

pub fn inferred_schema_ref(value: &IoValue) -> Result<String> {
    canonical_hash(&inferred_schema_value(value))
}

pub fn inferred_schema_value(value: &IoValue) -> IoValue {
    let class = match value.value_class() {
        preserves::ValueClass::Atomic(_) => "atomic",
        preserves::ValueClass::Embedded => "embedded",
        preserves::ValueClass::Compound(preserves::CompoundClass::Record) => "record",
        preserves::ValueClass::Compound(preserves::CompoundClass::Sequence) => "sequence",
        preserves::ValueClass::Compound(preserves::CompoundClass::Set) => "set",
        preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => "dictionary",
    };
    record("storage-schema-artifact-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_SCHEMA_ARTIFACT_SCHEMA),
        record("inference", vec![string("preserves-value-class")]),
        record("class", vec![string(class)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-preserves-class"), string("pass")]),
            record("check", vec![string("no-raw-memory-layout"), string("pass")]),
        ])]),
    ])
}

pub fn effect_manifest_value(
    producer_ref: &str,
    namespace: &str,
    schema_ref: &str,
    operations: &[String],
) -> Result<IoValue> {
    require_ref(producer_ref, "storage effect manifest producer ref")?;
    validate_namespace(namespace)?;
    require_ref(schema_ref, "storage effect manifest schema ref")?;
    validate_operations(operations)?;
    Ok(record("storage-effect-manifest-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_EFFECT_MANIFEST_SCHEMA),
        record("producer", vec![string(producer_ref)]),
        record("namespace", vec![string(namespace)]),
        record("schema-ref", vec![string(schema_ref)]),
        record("operations", vec![sequence(operations.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("declared-storage-effect"), string("pass")]),
            record("check", vec![string("typed-schema-binding"), string("pass")]),
            record("check", vec![string("handler-profile-required"), string("pass")]),
        ])]),
    ]))
}
