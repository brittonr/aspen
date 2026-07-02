
struct PassInput<'a> {
    input: &'a PutInput,
    storage_ref: &'a str,
    schema_ref: &'a str,
    value_ref: &'a str,
    effect: &'a EffectEvidence,
    details: Vec<IoValue>,
}

struct SourceInput<'a> {
    root: &'a Path,
    storage_key: &'a str,
    namespace: &'a str,
    key: &'a str,
    recipe: &'a MigrationRecipe,
}

struct SourceData {
    typed_ref: EntryRef,
    value: IoValue,
}

struct EntryRefs {
    policy: Vec<String>,
    evidence: Vec<String>,
}

struct NextInput<'a> {
    namespace: &'a str,
    key: &'a str,
    recipe: &'a MigrationRecipe,
    value_ref: &'a str,
    payload: &'a IoValue,
    refs: &'a EntryRefs,
    revision: u64,
    admission: &'a Admission,
    effect_handle_ref: &'a str,
}

struct StepInput<'a> {
    namespace: &'a str,
    key: &'a str,
    recipe: &'a MigrationRecipe,
    source: &'a EntryRef,
    storage_ref: &'a str,
    value_ref: &'a str,
    effect: &'a EffectEvidence,
    details: Vec<IoValue>,
}

pub fn put_value(root: &Path, input: &PutInput) -> Result<Put> {
    ensure_dirs(root)?;
    validate_namespace_key(&input.namespace, &input.key)?;
    require_ref(&input.producer_ref, "typed storage producer ref")?;
    validate_refs(&input.policy_refs, "typed storage policy ref")?;
    validate_refs(&input.evidence_refs, "typed storage evidence ref")?;
    validate_admission(&input.admission)?;

    let inferred_schema_ref = inferred_schema_ref(&input.value)?;
    let schema_ref = accepted_schema(root, input, &inferred_schema_ref)?;
    let effect = effect_evidence(EffectEvidenceInput {
        operation: "put",
        namespace: &input.namespace,
        key: &input.key,
        schema_ref: &schema_ref,
        producer_ref: &input.producer_ref,
        admission: &input.admission,
        remote_use: false,
    })?;
    let value_bytes = canonical_bytes(&input.value)?;
    let value_ref = canonical_hash(&input.value)?;
    let payload = payload_parts(root, &value_bytes)?;
    let storage_key = storage_key_ref(&input.namespace, &input.key)?;
    let revision = next_revision(root, &storage_key)?;
    let typed_ref_value = entry_value(EntryInput {
        input,
        schema_ref: &schema_ref,
        value_ref: &value_ref,
        payload: &payload.value,
        revision,
        effect_handle_ref: &effect.handle_ref,
    });
    let storage_ref = canonical_hash(&typed_ref_value)?;
    let receipt_value = pass_receipt(PassInput {
        input,
        storage_ref: &storage_ref,
        schema_ref: &schema_ref,
        value_ref: &value_ref,
        effect: &effect,
        details: payload.details,
    });
    persist_entry(PersistInput {
        root,
        storage_key: &storage_key,
        storage_ref: &storage_ref,
        typed_ref_value: &typed_ref_value,
        value_ref: &value_ref,
        value_bytes: &value_bytes,
        receipt_value: &receipt_value,
    })?;
    Ok(Put {
        storage_ref,
        typed_ref_value,
        schema_ref,
        value_ref,
        receipt_value,
    })
}

fn entry_value(input: EntryInput<'_>) -> IoValue {
    ref_value(RefValueInput {
        namespace: &input.input.namespace,
        key: &input.input.key,
        schema_ref: input.schema_ref,
        value_ref: input.value_ref,
        payload: input.payload,
        producer_ref: &input.input.producer_ref,
        policy_refs: &input.input.policy_refs,
        evidence_refs: &input.input.evidence_refs,
        revision: input.revision,
        actor_ref: &input.input.admission.actor_ref,
        capability_ref: &input.input.admission.capability_ref,
        effect_handle_ref: input.effect_handle_ref,
    })
}

fn pass_receipt(input: PassInput<'_>) -> IoValue {
    receipt_value(ReceiptValueInput {
        operation: "put",
        decision: "pass",
        storage_ref: Some(input.storage_ref),
        namespace: Some(&input.input.namespace),
        key: Some(&input.input.key),
        schema_ref: Some(input.schema_ref),
        value_ref: Some(input.value_ref),
        effect: input.effect,
        checks: vec![
            ("effect-manifest", "pass"),
            ("storage-effect-handle", "pass"),
            ("write-admission", "pass"),
            ("schema-binding", "pass"),
            ("canonical-value", "pass"),
            ("redb-adapter", "pass"),
            ("cairn-receipt", "pass"),
            ("no-raw-memory", "pass"),
        ],
        details: input.details,
    })
}

fn accepted_schema(root: &Path, input: &PutInput, inferred_schema_ref: &str) -> Result<String> {
    let schema_ref = input.schema_ref.clone().unwrap_or_else(|| inferred_schema_ref.to_string());
    if schema_ref == inferred_schema_ref {
        return Ok(schema_ref);
    }
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "put",
        storage_ref: None,
        namespace: Some(&input.namespace),
        key: Some(&input.key),
        schema_ref: Some(&schema_ref),
        value_ref: None,
        reason: "declared schema ref does not match inferred Preserves value schema".to_string(),
        checks: vec![
            ("schema-binding", "fail"),
            ("no-raw-memory", "pass"),
            ("denial-receipt", "pass"),
        ],
        details: Vec::new(),
    });
    store_receipt(root, &receipt_value)?;
    Err(MoltenError::invalid_harness(
        "typed storage write rejected: declared schema ref does not match inferred value schema",
    ))
}

fn payload_parts(root: &Path, value_bytes: &[u8]) -> Result<PayloadParts> {
    let value_len = u64::try_from(value_bytes.len())
        .map_err(|_| MoltenError::invalid_harness("typed storage value length exceeds u64"))?;
    if value_bytes.len() <= INLINE_VALUE_LIMIT {
        return Ok(PayloadParts {
            value: record("inline", vec![u64_value(value_len)]),
            details: vec![record("payload", vec![string("inline"), u64_value(value_len)])],
        });
    }
    let put = crate::chunk_store::put_bytes(
        &chunk_root(root),
        "typed-storage-value",
        value_bytes,
        crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
    )?;
    Ok(PayloadParts {
        value: record("content-ref", vec![string(&put.manifest_ref), u64_value(value_len)]),
        details: vec![
            record("payload", vec![string("content-ref"), string(&put.manifest_ref)]),
            record("chunk-store-receipt", vec![string(canonical_hash(&put.receipt_value)?)]),
        ],
    })
}

fn persist_entry(input: PersistInput<'_>) -> Result<()> {
    let db = ensure_index_tables(input.root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let typed_ref_bytes = canonical_bytes(input.typed_ref_value)?;
        let mut records = write_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        records.insert(input.storage_key, typed_ref_bytes.as_slice()).map_err(index_error)?;
        let mut refs = write_txn.open_table(INDEX_REFS).map_err(index_error)?;
        refs.insert(input.storage_ref, typed_ref_bytes.as_slice()).map_err(index_error)?;
        if input.value_bytes.len() <= INLINE_VALUE_LIMIT {
            let mut inline_values = write_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
            inline_values.insert(input.value_ref, input.value_bytes).map_err(index_error)?;
        }
        store_receipt_in_tx(&write_txn, input.receipt_value)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(())
}

pub fn get_value(
    root: &Path,
    namespace: &str,
    key: &str,
    expected_schema_ref: Option<&str>,
    admission: &Admission,
) -> Result<Get> {
    get_value_inner(GetValueInnerInput {
        root,
        namespace,
        key,
        expected_schema_ref,
        admission,
        migration_receipt_value: None,
        schema_compatibility_value: None,
    })
}

pub fn get_value_with_schema_compatibility(input: SchemaCompatibilityGetInput<'_>) -> Result<Get> {
    get_value_inner(GetValueInnerInput {
        root: input.root,
        namespace: input.namespace,
        key: input.key,
        expected_schema_ref: Some(input.expected_schema_ref),
        admission: input.admission,
        migration_receipt_value: None,
        schema_compatibility_value: Some(input.schema_compatibility_value),
    })
}

pub fn get_value_with_migration(input: MigrationGetInput<'_>) -> Result<Get> {
    match get_value_inner(GetValueInnerInput {
        root: input.root,
        namespace: input.namespace,
        key: input.key,
        expected_schema_ref: Some(input.expected_schema_ref),
        admission: input.admission,
        migration_receipt_value: None,
        schema_compatibility_value: None,
    }) {
        Ok(value) => Ok(value),
        Err(first_error) => {
            let recipe = parse_migration_recipe_value(input.migration_recipe_value)?;
            if recipe.target_schema_ref != input.expected_schema_ref {
                return Err(MoltenError::invalid_harness(
                    "typed storage lazy migration rejected: recipe target schema does not match expected schema ref",
                ));
            }
            if !matches!(recipe.mode.as_str(), "lazy-on-read" | "explicit") {
                return Err(MoltenError::invalid_harness(format!(
                    "typed storage lazy migration rejected: recipe mode {} cannot run on read",
                    recipe.mode
                )));
            }
            let migrated =
                migrate_value(input.root, input.namespace, input.key, input.migration_recipe_value, input.admission)
                    .map_err(|migration_error| {
                        MoltenError::invalid_harness(format!(
                            "typed storage lazy migration failed after load miss {first_error}: {migration_error}"
                        ))
                    })?;
            get_value_inner(GetValueInnerInput {
                root: input.root,
                namespace: input.namespace,
                key: input.key,
                expected_schema_ref: Some(input.expected_schema_ref),
                admission: input.admission,
                migration_receipt_value: Some(&migrated.receipt_value),
                schema_compatibility_value: None,
            })
        }
    }
}
