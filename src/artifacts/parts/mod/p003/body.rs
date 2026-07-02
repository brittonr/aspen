
fn name_pointer_value(input: &NamePointerValueInput<'_>) -> Result<IoValue> {
    validate_pointer_kind(input.pointer_kind)?;
    validate_non_empty(input.name, "artifact pointer name")?;
    validate_ref(input.artifact_ref, "artifact pointer artifact ref")?;
    if let Some(previous_ref) = input.previous_ref {
        validate_ref(previous_ref, "artifact pointer previous ref")?;
    }
    validate_refs(input.policy_refs, "artifact pointer policy ref")?;
    validate_ref(input.receipt_ref, "artifact pointer receipt ref")?;
    Ok(record("artifact-name-pointer-v1", vec![
        string(crate::preserves_rail::ARTIFACT_NAME_POINTER_SCHEMA),
        record("kind", vec![string(input.pointer_kind)]),
        record("name", vec![string(input.name)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("previous", vec![optional_ref_value(input.previous_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("receipt", vec![string(input.receipt_ref)]),
        checks_value(&["names-are-metadata", "artifact-content-immutable"]),
    ]))
}

fn parse_name_pointer_value(value: &IoValue) -> Result<ArtifactNamePointer> {
    let fields = value
        .collect_simple_record("artifact-name-pointer-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-name-pointer-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_NAME_POINTER_SCHEMA, "artifact name pointer")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "names-are-metadata", "artifact name pointer")?;
    Ok(ArtifactNamePointer {
        pointer_ref: canonical_hash(value)?,
        pointer_kind: record_string(&fields[1], "kind")?,
        name: record_string(&fields[2], "name")?,
        artifact_ref: record_ref(&fields[3], "artifact")?,
        previous_ref: record_optional_ref(&fields[4], "previous")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        receipt_ref: record_ref(&fields[6], "receipt")?,
        value: value.clone(),
    })
}

pub fn list_name_pointers(root: &Path) -> Result<Vec<ArtifactNamePointer>> {
    all_name_pointers(root)
}

fn all_name_pointers(root: &Path) -> Result<Vec<ArtifactNamePointer>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let names = read_txn.open_table(INDEX_NAMES).map_err(index_error)?;
    let mut pointers = Vec::new();
    for item in names.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        push_bounded(
            &mut pointers,
            parse_name_pointer_value(&parse_canonical_bytes(bytes.value())?)?,
            MAX_ARTIFACT_POINTERS,
            "artifact name pointers",
        )?;
    }
    Ok(pointers)
}

fn closure_value(roots: &[String], closure_refs: &[String], missing_refs: &[String]) -> Result<IoValue> {
    validate_refs(roots, "artifact closure root")?;
    validate_refs(closure_refs, "artifact closure ref")?;
    validate_refs(missing_refs, "artifact closure missing ref")?;
    Ok(record("artifact-closure-v1", vec![
        string(crate::preserves_rail::ARTIFACT_CLOSURE_SCHEMA),
        refs_record("roots", &sorted_unique(roots)),
        refs_record("closure", closure_refs),
        refs_record("missing", missing_refs),
        checks_value(&["ordered-refs", "closure-hash", "missing-dependency-denial"]),
    ]))
}

fn payload_value(payload: &ArtifactPayloadRef) -> Result<IoValue> {
    Ok(record("payload", vec![match payload {
        ArtifactPayloadRef::Inline { value_ref, length } => {
            validate_ref(value_ref, "inline payload value ref")?;
            record("inline", vec![string(value_ref), crate::preserves_rail::u64_value(*length)])
        }
        ArtifactPayloadRef::ContentRef { manifest_ref, length } => {
            validate_ref(manifest_ref, "content payload manifest ref")?;
            record("content-ref", vec![string(manifest_ref), crate::preserves_rail::u64_value(*length)])
        }
    }]))
}

fn parse_payload_ref(value: &RailValue) -> Result<ArtifactPayloadRef> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "payload", 1)?;
    let payload = value_to_iovalue(&fields[0]);
    if let Some(inline) = payload.collect_simple_record("inline", Some(2)) {
        return Ok(ArtifactPayloadRef::Inline {
            value_ref: required_ref(&inline[0], "inline payload ref")?,
            length: required_u64(&inline[1], "inline payload length")?,
        });
    }
    if let Some(content) = payload.collect_simple_record("content-ref", Some(2)) {
        return Ok(ArtifactPayloadRef::ContentRef {
            manifest_ref: required_ref(&content[0], "content payload manifest ref")?,
            length: required_u64(&content[1], "content payload length")?,
        });
    }
    Err(MoltenError::invalid_harness("artifact payload must be inline or content-ref"))
}

fn refs_value(refs: &[String]) -> IoValue {
    record("refs", vec![refs_sequence(refs)])
}

fn parse_refs_value(value: &IoValue, label: &str) -> Result<Vec<String>> {
    let fields = simple_record(value, "refs", 1)?;
    parse_ref_sequence_value(&fields[0], label)
}

fn refs_record(label: &'static str, refs: &[String]) -> IoValue {
    record(label, vec![refs_sequence(refs)])
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect()
}

fn registry_text_contains_ref(root: &Path, target_ref: &str) -> Result<bool> {
    for receipt in receipt_values(root)? {
        if crate::preserves_rail::to_text(&receipt)?.contains(target_ref) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn receipt_values(root: &Path) -> Result<Vec<IoValue>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut values = Vec::new();
    for item in receipts.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        push_bounded(
            &mut values,
            parse_canonical_bytes(bytes.value())?,
            MAX_ARTIFACT_RECEIPTS,
            "artifact registry receipts",
        )?;
    }
    Ok(values)
}

fn store_receipt(root: &Path, receipt_value: &IoValue) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt_value: &IoValue) -> Result<()> {
    let parsed = parse_artifact_receipt(receipt_value)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts
        .insert(parsed.receipt_ref.as_str(), canonical_bytes(receipt_value)?.as_slice())
        .map_err(index_error)?;
    Ok(())
}

fn clear_derived_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> Result<()> {
    clear_bytes_table(write_txn, INDEX_DEPS)?;
    clear_bytes_table(write_txn, INDEX_REVERSE)?;
    clear_str_table(write_txn, INDEX_KIND)?;
    clear_str_table(write_txn, INDEX_SCHEMA)?;
    clear_str_table(write_txn, INDEX_EFFECT)?;
    clear_str_table(write_txn, INDEX_POLICY)?;
    clear_str_table(write_txn, INDEX_EVIDENCE)
}

fn clear_bytes_table(write_txn: &redb::WriteTransaction, table_definition: TableDef<&str, &[u8]>) -> Result<()> {
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    let keys = bytes_table_keys(&table)?;
    for key in keys {
        table.remove(key.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn clear_str_table(write_txn: &redb::WriteTransaction, table_definition: TableDef<&str, &str>) -> Result<()> {
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    let keys = str_table_keys(&table)?;
    for key in keys {
        table.remove(key.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn bytes_table_keys(table: &redb::Table<'_, &str, &[u8]>) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _) = item.map_err(index_error)?;
        push_bounded(&mut keys, key.value().to_string(), MAX_ARTIFACT_RECORDS, "artifact byte-table keys")?;
    }
    Ok(keys)
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _) = item.map_err(index_error)?;
        push_bounded(&mut keys, key.value().to_string(), MAX_ARTIFACT_RECORDS, "artifact string-table keys")?;
    }
    Ok(keys)
}

fn ensure_dirs(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    std::fs::create_dir_all(chunk_root(root)).map_err(MoltenError::from)
}

fn ensure_index_tables(root: &Path) -> Result<redb::Database> {
    ensure_dirs(root)?;
    let db = redb::Database::create(index_path(root)).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
        write_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
        write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        write_txn.open_table(INDEX_DEPS).map_err(index_error)?;
        write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
        write_txn.open_table(INDEX_KIND).map_err(index_error)?;
        write_txn.open_table(INDEX_SCHEMA).map_err(index_error)?;
        write_txn.open_table(INDEX_EFFECT).map_err(index_error)?;
        write_txn.open_table(INDEX_POLICY).map_err(index_error)?;
        write_txn.open_table(INDEX_EVIDENCE).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn index_path(root: &Path) -> std::path::PathBuf {
    root.join(INDEX_FILE)
}

fn chunk_root(root: &Path) -> std::path::PathBuf {
    root.join("chunks")
}

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn value_to_iovalue(value: &RailValue) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn put_payload_bytes(root: &Path, payload_bytes: &[u8]) -> Result<crate::chunk_store::ChunkStorePut> {
    crate::chunk_store::put_bytes(
        root,
        "artifact-payload",
        payload_bytes,
        crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
    )
}

fn read_chunk_object(root: &Path, manifest_ref: &str) -> Result<crate::chunk_store::ChunkStoreRead> {
    crate::chunk_store::read_object(root, manifest_ref)
}

fn name_key(pointer_kind: &str, name: &str) -> Result<String> {
    canonical_hash(&record("artifact-name-key", vec![string(pointer_kind), string(name)]))
}

fn local_ref(kind: &'static str, refs: &[String]) -> Result<String> {
    canonical_hash(&record(kind, vec![refs_sequence(refs)]))
}

fn domain_for_kind(kind: &str) -> String {
    format!("molten.artifacts.domain.v1:{kind}")
}
