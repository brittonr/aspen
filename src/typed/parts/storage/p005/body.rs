
fn store_payload(root: &Path, value_bytes: &[u8]) -> Result<(IoValue, Vec<IoValue>)> {
    if value_bytes.len() <= INLINE_VALUE_LIMIT {
        Ok((record("inline", vec![u64_value(value_bytes.len() as u64)]), vec![record("payload", vec![
            string("inline"),
            u64_value(value_bytes.len() as u64),
        ])]))
    } else {
        let put = crate::chunk_store::put_bytes(
            &chunk_root(root),
            "typed-storage-value",
            value_bytes,
            crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
        )?;
        Ok((record("content-ref", vec![string(&put.manifest_ref), u64_value(value_bytes.len() as u64)]), vec![
            record("payload", vec![string("content-ref"), string(&put.manifest_ref)]),
            record("chunk-store-receipt", vec![string(canonical_hash(&put.receipt_value)?)]),
        ]))
    }
}

fn apply_migration_transform(recipe: &MigrationRecipe, old_value: &IoValue) -> Result<IoValue> {
    match recipe.transformer_kind.as_str() {
        "identity" | "schema-rename" => Ok(old_value.clone()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported typed storage migration transformer kind {other}"
        ))),
    }
}

fn read_payload_bytes(root: &Path, typed_ref: &EntryRef) -> Result<Vec<u8>> {
    match &typed_ref.payload {
        Payload::Inline { length } => {
            let db = ensure_index_tables(root)?;
            let read_txn = db.begin_read().map_err(index_error)?;
            let table = read_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
            let Some(bytes) = table.get(typed_ref.value_ref.as_str()).map_err(index_error)? else {
                return Err(MoltenError::invalid_harness(format!(
                    "missing inline typed storage value {}",
                    typed_ref.value_ref
                )));
            };
            let bytes = bytes.value().to_vec();
            if bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness(format!(
                    "inline typed storage length mismatch: got {}, expected {length}",
                    bytes.len()
                )));
            }
            Ok(bytes)
        }
        Payload::ContentRef { manifest_ref, length } => {
            let read = crate::chunk_store::read_object(&chunk_root(root), manifest_ref)?;
            if read.bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness(format!(
                    "chunk-backed typed storage length mismatch: got {}, expected {length}",
                    read.bytes.len()
                )));
            }
            Ok(read.bytes)
        }
    }
}

fn read_entry_ref(root: &Path, storage_ref: &str) -> Result<IoValue> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let refs = read_txn.open_table(INDEX_REFS).map_err(index_error)?;
    let Some(bytes) = refs.get(storage_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown typed storage ref {storage_ref}")));
    };
    parse_canonical_bytes(bytes.value())
}

fn next_revision(root: &Path, storage_key: &str) -> Result<u64> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let records = read_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
    let Some(bytes) = records.get(storage_key).map_err(index_error)? else {
        return Ok(1);
    };
    let value = parse_canonical_bytes(bytes.value())?;
    Ok(parse_entry_ref_value(&value)?.revision.saturating_add(1))
}

fn receipt_value(input: ReceiptValueInput<'_>) -> IoValue {
    record("typed-storage-receipt-v1", vec![
        string(crate::preserves_rail::TYPED_STORAGE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("storage-ref", vec![optional_ref_value(input.storage_ref)]),
        record("binding", vec![
            optional_string_value(input.namespace),
            optional_string_value(input.key),
            optional_ref_value(input.schema_ref),
            optional_ref_value(input.value_ref),
        ]),
        record("effect", vec![
            string(&input.effect.manifest_ref),
            string(&input.effect.handler_binding_ref),
            string(&input.effect.handle_ref),
        ]),
        checks_record(input.checks),
        record("details", vec![sequence(input.details)]),
        record("tool", vec![string("molten"), string(env!("CARGO_PKG_VERSION"))]),
    ])
}

fn denial_receipt_value(input: DenialReceiptValueInput<'_>) -> IoValue {
    let fallback_ref = local_ref("typed-storage-denial-effect", input.operation);
    let effect = EffectEvidence {
        manifest_ref: fallback_ref.clone(),
        handler_binding_ref: fallback_ref.clone(),
        handle_ref: fallback_ref,
    };
    let mut details = input.details;
    details.push(record("reason", vec![string(input.reason)]));
    receipt_value(ReceiptValueInput {
        operation: input.operation,
        decision: "deny",
        storage_ref: input.storage_ref,
        namespace: input.namespace,
        key: input.key,
        schema_ref: input.schema_ref,
        value_ref: input.value_ref,
        effect: &effect,
        checks: input.checks,
        details,
    })
}

fn parse_payload(value: &Value<IoValue>) -> Result<Payload> {
    let value = value_to_iovalue(value);
    let payload = simple_record(&value, "payload", 1)?;
    let payload_value = value_to_iovalue(&payload[0]);
    if let Some(inline) = payload_value.collect_simple_record("inline", Some(1)) {
        return Ok(Payload::Inline {
            length: required_u64(&inline[0], "inline payload length")?,
        });
    }
    if let Some(content) = payload_value.collect_simple_record("content-ref", Some(2)) {
        return Ok(Payload::ContentRef {
            manifest_ref: required_ref(&content[0], "payload manifest ref")?,
            length: required_u64(&content[1], "content payload length")?,
        });
    }
    Err(MoltenError::invalid_harness("typed storage payload must be inline or content-ref"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReceiptBinding {
    namespace: Option<String>,
    key: Option<String>,
    schema_ref: Option<String>,
    value_ref: Option<String>,
}

fn parse_binding_record(value: &Value<IoValue>) -> Result<ReceiptBinding> {
    let value = value_to_iovalue(value);
    let binding = simple_record(&value, "binding", 4)?;
    Ok(ReceiptBinding {
        namespace: parse_optional_string_value(&binding[0])?,
        key: parse_optional_string_value(&binding[1])?,
        schema_ref: parse_optional_ref_value(&binding[2])?,
        value_ref: parse_optional_ref_value(&binding[3])?,
    })
}

fn storage_key_ref(namespace: &str, key: &str) -> Result<String> {
    canonical_hash(&record("typed-storage-key-v1", vec![string(namespace), string(key)]))
}

fn chunk_root(root: &Path) -> std::path::PathBuf {
    root.join("chunks")
}

fn ensure_dirs(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    std::fs::create_dir_all(chunk_root(root)).map_err(MoltenError::from)
}

fn ensure_index_tables(root: &Path) -> Result<Database> {
    ensure_dirs(root)?;
    let db = Database::create(index_path(root)).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        write_txn.open_table(INDEX_REFS).map_err(index_error)?;
        write_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn store_receipt(root: &Path, receipt_value: &IoValue) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt_value: &IoValue) -> Result<()> {
    let parsed = parse_receipt_value(receipt_value, None)?;
    let receipt_bytes = canonical_bytes(receipt_value)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts.insert(parsed.receipt_ref.as_str(), receipt_bytes.as_slice()).map_err(index_error)?;
    Ok(())
}

fn index_path(root: &Path) -> std::path::PathBuf {
    root.join(INDEX_FILE)
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("typed storage redb index error: {error}"))
}

fn validate_admission(admission: &Admission) -> Result<()> {
    require_ref(&admission.actor_ref, "typed storage actor ref")?;
    require_ref(&admission.capability_ref, "typed storage capability ref")?;
    require_ref(&admission.policy_ref, "typed storage policy ref")?;
    if admission.resource_refs.is_empty() {
        return Err(MoltenError::invalid_harness("typed storage admission requires at least one resource ref"));
    }
    validate_refs(&admission.resource_refs, "typed storage resource ref")?;
    validate_refs(&admission.evidence_refs, "typed storage admission evidence ref")
}

fn validate_namespace_key(namespace: &str, key: &str) -> Result<()> {
    validate_namespace(namespace)?;
    if key.is_empty() {
        return Err(MoltenError::invalid_harness("typed storage key must not be empty"));
    }
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<()> {
    if namespace.is_empty() {
        return Err(MoltenError::invalid_harness("typed storage namespace must not be empty"));
    }
    if namespace.chars().any(char::is_whitespace) {
        return Err(MoltenError::invalid_harness("typed storage namespace must not contain whitespace"));
    }
    Ok(())
}

fn validate_operations(operations: &[String]) -> Result<()> {
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("storage effect manifest operations must not be empty"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for operation in operations {
        validate_operation(operation)?;
        if !seen.insert(operation.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate storage operation {operation}")));
        }
    }
    Ok(())
}

fn validate_operation(operation: &str) -> Result<()> {
    if !matches!(operation, "put" | "get" | "verify" | "migrate") {
        return Err(MoltenError::invalid_harness(format!("unsupported typed storage operation {operation}")));
    }
    Ok(())
}

fn validate_transformer_kind(kind: &str) -> Result<()> {
    if !matches!(kind, "identity" | "schema-rename") {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported typed storage migration transformer kind {kind}"
        )));
    }
    Ok(())
}

fn validate_migration_mode(mode: &str) -> Result<()> {
    if !matches!(mode, "explicit" | "lazy-on-read" | "batch") {
        return Err(MoltenError::invalid_harness(format!("unsupported typed storage migration mode {mode}")));
    }
    Ok(())
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}
