
fn get_value_inner(input: GetValueInnerInput<'_>) -> Result<Get> {
    ensure_dirs(input.root)?;
    validate_namespace_key(input.namespace, input.key)?;
    validate_admission(input.admission)?;
    let storage_key = storage_key_ref(input.namespace, input.key)?;
    let typed_ref_value = stored_binding(&input, storage_key.as_str())?;
    let typed_ref = parse_entry_ref_value(&typed_ref_value)?;
    require_binding(&input, &typed_ref)?;
    let effect = effect_evidence(EffectEvidenceInput {
        operation: "get",
        namespace: input.namespace,
        key: input.key,
        schema_ref: &typed_ref.schema_ref,
        producer_ref: &typed_ref.producer_ref,
        admission: input.admission,
        remote_use: false,
    })?;
    let value = checked_value(&input, &typed_ref)?;
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "get",
        decision: "pass",
        storage_ref: Some(&typed_ref.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(&typed_ref.schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        effect: &effect,
        checks: vec![
            ("effect-manifest", "pass"),
            ("storage-effect-handle", "pass"),
            ("load-admission", "pass"),
            ("schema-compatibility", "pass"),
            ("content-integrity", "pass"),
            ("receipt-validation", "pass"),
        ],
        details: get_details(&input, typed_ref.revision)?,
    });
    store_receipt(input.root, &receipt_value)?;
    Ok(Get {
        storage_ref: typed_ref.storage_ref.clone(),
        typed_ref,
        value,
        receipt_value,
    })
}

fn stored_binding(input: &GetValueInnerInput<'_>, storage_key: &str) -> Result<IoValue> {
    let db = ensure_index_tables(input.root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let records = read_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
    let Some(bytes) = records.get(storage_key).map_err(index_error)? else {
        drop(records);
        drop(read_txn);
        drop(db);
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "get",
            storage_ref: None,
            namespace: Some(input.namespace),
            key: Some(input.key),
            schema_ref: input.expected_schema_ref,
            value_ref: None,
            reason: "typed storage record not found".to_string(),
            checks: vec![("record-found", "fail"), ("denial-receipt", "pass")],
            details: Vec::new(),
        });
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("typed storage get rejected: record not found"));
    };
    parse_canonical_bytes(bytes.value())
}

fn require_binding(input: &GetValueInnerInput<'_>, typed_ref: &EntryRef) -> Result<()> {
    let Some(expected_schema_ref) = input.expected_schema_ref else {
        return Ok(());
    };
    if typed_ref.schema_ref == expected_schema_ref {
        return Ok(());
    }
    if is_binding_admitted(input, typed_ref)? {
        return Ok(());
    }
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "get",
        storage_ref: Some(&typed_ref.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(expected_schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        reason: "expected schema ref does not match stored schema ref; unique identity mismatch or missing compatibility/migration receipt".to_string(),
        checks: vec![("schema-compatibility", "fail"), ("compatibility-receipt", "fail"), ("denial-receipt", "pass")],
        details: Vec::new(),
    });
    store_receipt(input.root, &receipt_value)?;
    Err(MoltenError::invalid_harness(
        "typed storage get rejected: expected schema ref does not match stored schema ref",
    ))
}

fn is_binding_admitted(input: &GetValueInnerInput<'_>, typed_ref: &EntryRef) -> Result<bool> {
    let Some(expected_schema_ref) = input.expected_schema_ref else {
        return Ok(true);
    };
    if typed_ref.schema_ref == expected_schema_ref {
        return Ok(true);
    }
    let Some(schema_compatibility_value) = input.schema_compatibility_value else {
        return Ok(false);
    };
    let admits = crate::schema_identity::compatibility_admits_storage(
        schema_compatibility_value,
        expected_schema_ref,
        &typed_ref.schema_ref,
    )?;
    if !admits {
        return Ok(false);
    }
    let receipt_value =
        crate::schema_identity::compatibility_receipt_value(STORAGE_READ_COMPATIBILITY_OPERATION, schema_compatibility_value)?;
    let receipt = crate::schema_identity::parse_compatibility_receipt(&receipt_value)?;
    Ok(receipt.decision == "pass")
}

fn checked_value(input: &GetValueInnerInput<'_>, typed_ref: &EntryRef) -> Result<IoValue> {
    let value_bytes = read_payload_bytes(input.root, typed_ref)?;
    let value = parse_canonical_bytes(&value_bytes)?;
    let actual_value_ref = canonical_hash(&value)?;
    if actual_value_ref == typed_ref.value_ref {
        return Ok(value);
    }
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "get",
        storage_ref: Some(&typed_ref.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(&typed_ref.schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        reason: "stored value hash does not match typed ref".to_string(),
        checks: vec![("content-integrity", "fail"), ("denial-receipt", "pass")],
        details: Vec::new(),
    });
    store_receipt(input.root, &receipt_value)?;
    Err(MoltenError::invalid_harness("typed storage content integrity check failed"))
}

fn get_details(input: &GetValueInnerInput<'_>, revision: u64) -> Result<Vec<IoValue>> {
    let mut details = vec![record("revision", vec![u64_value(revision)])];
    if let Some(migration_receipt_value) = input.migration_receipt_value {
        details.push(record("migration-receipt", vec![string(canonical_hash(migration_receipt_value)?)]));
        details.push(record("migration-mode", vec![string("lazy-on-read")]));
    }
    if let Some(schema_compatibility_value) = input.schema_compatibility_value {
        let receipt_value =
            crate::schema_identity::compatibility_receipt_value(STORAGE_READ_COMPATIBILITY_OPERATION, schema_compatibility_value)?;
        details.push(record("schema-compatibility", vec![string(canonical_hash(schema_compatibility_value)?)]));
        details.push(record("schema-compatibility-receipt", vec![string(canonical_hash(&receipt_value)?)]));
        details.push(record("schema-compatibility-value", vec![schema_compatibility_value.clone()]));
        details.push(record("schema-compatibility-receipt-value", vec![receipt_value]));
    }
    Ok(details)
}

pub fn verify_ref(root: &Path, storage_ref: &str, expected_schema_ref: Option<&str>) -> Result<Verify> {
    ensure_dirs(root)?;
    require_ref(storage_ref, "typed storage ref")?;
    let typed_ref_value = read_entry_ref(root, storage_ref)?;
    let typed_ref = parse_entry_ref_value(&typed_ref_value)?;
    if let Some(expected_schema_ref) = expected_schema_ref
        && typed_ref.schema_ref != expected_schema_ref
    {
        return schema_mismatch(root, storage_ref, &typed_ref, expected_schema_ref);
    }
    let value_bytes = read_payload_bytes(root, &typed_ref)?;
    let value = parse_canonical_bytes(&value_bytes)?;
    let actual_value_ref = canonical_hash(&value)?;
    if actual_value_ref != typed_ref.value_ref {
        return content_mismatch(root, storage_ref, &typed_ref);
    }
    let effect = EffectEvidence {
        manifest_ref: typed_ref.effect_handle_ref.clone(),
        handler_binding_ref: typed_ref.effect_handle_ref.clone(),
        handle_ref: typed_ref.effect_handle_ref.clone(),
    };
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "verify",
        decision: "pass",
        storage_ref: Some(storage_ref),
        namespace: Some(&typed_ref.namespace),
        key: Some(&typed_ref.key),
        schema_ref: Some(&typed_ref.schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        effect: &effect,
        checks: vec![
            ("typed-ref-found", "pass"),
            ("content-integrity", "pass"),
            ("schema-binding", "pass"),
            ("receipt-validation", "pass"),
        ],
        details: Vec::new(),
    });
    store_receipt(root, &receipt_value)?;
    Ok(Verify {
        storage_ref: typed_ref.storage_ref.clone(),
        typed_ref,
        receipt_value,
    })
}

fn schema_mismatch(root: &Path, storage_ref: &str, typed_ref: &EntryRef, expected_schema_ref: &str) -> Result<Verify> {
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "verify",
        storage_ref: Some(storage_ref),
        namespace: Some(&typed_ref.namespace),
        key: Some(&typed_ref.key),
        schema_ref: Some(expected_schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        reason: "verify expected schema ref does not match stored schema ref".to_string(),
        checks: vec![("schema-compatibility", "fail"), ("denial-receipt", "pass")],
        details: Vec::new(),
    });
    store_receipt(root, &receipt_value)?;
    Err(MoltenError::invalid_harness(
        "typed storage verify rejected: expected schema ref does not match stored schema ref",
    ))
}

fn content_mismatch(root: &Path, storage_ref: &str, typed_ref: &EntryRef) -> Result<Verify> {
    let receipt_value = denial_receipt_value(DenialReceiptValueInput {
        operation: "verify",
        storage_ref: Some(storage_ref),
        namespace: Some(&typed_ref.namespace),
        key: Some(&typed_ref.key),
        schema_ref: Some(&typed_ref.schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        reason: "verify content hash mismatch".to_string(),
        checks: vec![("content-integrity", "fail"), ("denial-receipt", "pass")],
        details: Vec::new(),
    });
    store_receipt(root, &receipt_value)?;
    Err(MoltenError::invalid_harness("typed storage verify content integrity check failed"))
}
