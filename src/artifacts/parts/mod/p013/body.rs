
/// The registry dependencies, readable members, recomputed closure, and closure digest must all equal the
/// snapshot's exact member set.
fn exact_member_closure(
    root: &CapabilityArtifactRoot,
    artifact: &ArtifactRecord,
    snapshot: &ReleaseSnapshot,
    expected_members: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<bool> {
    let artifact_dependencies = sorted_unique(&artifact.dependency_refs);
    let mut is_exact_member_closure = artifact_dependencies == *expected_members;
    if !is_exact_member_closure {
        push_snapshot_diagnostic(diagnostics, "release snapshot registry dependencies do not match exact artifact members".to_string())?;
    }
    for member_ref in expected_members {
        if let Err(error) = read_artifact_with_root(root, member_ref) {
            is_exact_member_closure = false;
            push_snapshot_diagnostic(diagnostics, format!("tampered or missing snapshot member {member_ref}: {error}"))?;
        }
    }
    let (closure_refs, missing_refs) = compute_closure_refs(root, expected_members)?;
    if !missing_refs.is_empty() {
        is_exact_member_closure = false;
        for missing_ref in &missing_refs {
            push_snapshot_diagnostic(diagnostics, format!("missing closure member {missing_ref}"))?;
        }
    }
    for missing_member in set_difference(&closure_refs, expected_members)? {
        is_exact_member_closure = false;
        push_snapshot_diagnostic(diagnostics, format!("snapshot omitted closure member {missing_member}"))?;
    }
    for unexpected_member in set_difference(expected_members, &closure_refs)? {
        is_exact_member_closure = false;
        push_snapshot_diagnostic(diagnostics, format!("snapshot listed unexpected closure member {unexpected_member}"))?;
    }
    let closure_digest = canonical_hash(&closure_value(expected_members, &closure_refs, &missing_refs)?)?;
    if closure_digest != snapshot.dependency_closure_digest {
        is_exact_member_closure = false;
        push_snapshot_diagnostic(diagnostics, format!(
                "dependency closure digest mismatch: got {closure_digest}, expected {}",
                snapshot.dependency_closure_digest
            ))?;
    }
    Ok(is_exact_member_closure)
}

fn push_snapshot_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) -> Result<()> {
    push_bounded(diagnostics, diagnostic, MAX_ARTIFACT_DIAGNOSTICS, "release snapshot diagnostics")
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

fn registry_contains_structural_ref(root: &CapabilityArtifactRoot, target_ref: &str) -> Result<bool> {
    for receipt in receipt_values(root)? {
        if crate::preserves_rail::contains_structural_content_ref(&receipt, target_ref)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn receipt_values(root: &CapabilityArtifactRoot) -> Result<Vec<IoValue>> {
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

fn store_receipt(root: &CapabilityArtifactRoot, receipt_value: &IoValue) -> Result<()> {
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

fn ensure_dirs(root: &CapabilityArtifactRoot) -> Result<()> {
    root.root().create_dir_all(&LocalStorePath::parse("chunks")?)
}

fn ensure_index_tables(root: &CapabilityArtifactRoot) -> Result<redb::Database> {
    ensure_dirs(root)?;
    let database_file = root.root().open_database_file(&LocalStorePath::parse(INDEX_FILE)?)?;
    let db = redb::Database::builder().create_file(database_file).map_err(index_error)?;
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

fn chunk_root_with_root(root: &CapabilityArtifactRoot) -> Result<crate::local_store::ChunkStoreRoot> {
    crate::local_store::ChunkStoreRoot::open_artifact_chunks(root)
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

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn value_to_iovalue(value: &RailValue) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn put_payload_bytes(
    root: &CapabilityArtifactRoot,
    payload_bytes: &[u8],
) -> Result<crate::chunk_store::ChunkStorePut> {
    let chunk_root = chunk_root_with_root(root)?;
    crate::chunk_store::put_bytes_with_root(
        &chunk_root,
        "artifact-payload",
        payload_bytes,
        crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
    )
}

fn read_chunk_object(
    root: &CapabilityArtifactRoot,
    manifest_ref: &str,
) -> Result<crate::chunk_store::ChunkStoreRead> {
    let chunk_root = chunk_root_with_root(root)?;
    crate::chunk_store::read_object_with_root(&chunk_root, manifest_ref)
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
