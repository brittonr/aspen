
fn parse_iroh_ticket_value(value: &IoValue) -> Result<IrohChunkTicket> {
    let fields = simple_record(value, "chunk-store-iroh-ticket-v1", 5)?;
    require_schema(&fields[0], CHUNK_IROH_TICKET_SCHEMA, "chunk-store Iroh ticket")?;
    let adapter = record_string(&fields[1], "adapter")?;
    if adapter != "iroh-local" {
        return Err(MoltenError::invalid_harness(format!("unsupported chunk-store Iroh adapter {adapter}")));
    }
    let manifest_ref = record_string(&fields[2], "manifest-ref")?;
    filename_for_ref(&manifest_ref)?;
    let manifest_blob_ref = record_string(&fields[3], "manifest-blob-ref")?;
    filename_for_ref(&manifest_blob_ref)?;
    let chunk_values = record_sequence(&fields[4], "chunks")?;
    let mut chunks = Vec::new();
    let mut seen = OrderedSet::new();
    for chunk_value in &chunk_values {
        let chunk_blob = simple_record(chunk_value, "chunk-blob", 3)?;
        let chunk_ref = required_string(&chunk_blob[0], "chunk ref")?;
        let blob_ref = required_string(&chunk_blob[1], "blob ref")?;
        let length = required_u64(&chunk_blob[2], "chunk blob length")?;
        filename_for_ref(&chunk_ref)?;
        filename_for_ref(&blob_ref)?;
        if length == 0 {
            return Err(MoltenError::invalid_harness(format!(
                "Iroh chunk ticket maps {chunk_ref} to zero-length blob"
            )));
        }
        if !insert_set_bounded(
            &mut seen,
            chunk_ref.clone(),
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh ticket chunk set",
        )? {
            return Err(MoltenError::invalid_harness(format!(
                "Iroh chunk ticket has duplicate chunk mapping for {chunk_ref}"
            )));
        }
        push_bounded(
            &mut chunks,
            IrohChunkBlob {
                chunk_ref,
                blob_ref,
                length,
            },
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh ticket chunks",
        )?;
    }
    Ok(IrohChunkTicket {
        manifest_ref,
        manifest_blob_ref,
        chunks,
    })
}

fn read_iroh_ticket(root: &CapabilityChunkRoot, manifest_ref: &str) -> Result<IoValue> {
    let bytes = root.root().read(&iroh_ticket_path(manifest_ref)?)?;
    parse_canonical_bytes(&bytes)
}

fn read_iroh_blob(root: &CapabilityChunkRoot, blob_ref: &str) -> Result<Vec<u8>> {
    let bytes = root.root().read(&iroh_blob_path(blob_ref)?)?;
    let actual_ref = hash_blob_bytes(&bytes);
    if actual_ref != blob_ref {
        return Err(MoltenError::invalid_harness(format!("Iroh blob {blob_ref} hashes to {actual_ref}")));
    }
    Ok(bytes)
}

fn index_put(
    root: &CapabilityChunkRoot,
    manifest_value: &IoValue,
    chunks: &[ChunkRef],
    receipt_value: &IoValue,
) -> Result<()> {
    let manifest = parse_manifest_value(manifest_value, None)?;
    let chunk_refs = chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(root, &manifest, &chunk_refs, &[], Some(receipt_value))
}

fn index_manifest_available(
    root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    receipt_value: &IoValue,
) -> Result<()> {
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(root, manifest, &chunk_refs, &[], Some(receipt_value))
}

fn index_set_manifest_chunk_availability(
    root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    available: &[String],
    missing: &[String],
    receipt_value: Option<&IoValue>,
) -> Result<()> {
    let available = available.iter().cloned().collect::<OrderedSet<_>>();
    let missing = missing.iter().cloned().collect::<OrderedSet<_>>();
    let manifest_bytes = canonical_bytes(&manifest.value)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut manifests = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        manifests.insert(manifest.manifest_ref.as_str(), manifest_bytes.as_slice()).map_err(index_error)?;
    }
    {
        let mut chunks = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        let mut availability = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        for chunk in &manifest.chunks {
            let chunk_value = canonical_bytes(&chunk_index_value(chunk))?;
            chunks.insert(chunk.chunk_ref.as_str(), chunk_value.as_slice()).map_err(index_error)?;
            let status = if missing.contains(&chunk.chunk_ref) {
                "missing"
            } else if available.contains(&chunk.chunk_ref) || root.root().try_exists(&chunk_path(&chunk.chunk_ref)?)? {
                "available"
            } else {
                "missing"
            };
            availability.insert(chunk.chunk_ref.as_str(), status).map_err(index_error)?;
        }
    }
    if let Some(receipt_value) = receipt_value {
        store_receipt_in_tx(&write_txn, receipt_value)?;
    }
    write_txn.commit().map_err(index_error)
}

fn index_set_partial_fetch(
    root: &CapabilityChunkRoot,
    manifest_ref: &str,
    status: &str,
    missing_before: &[String],
    fetched: &[String],
) -> Result<()> {
    let value = canonical_bytes(&partial_fetch_value(manifest_ref, status, missing_before, fetched))?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut partial_fetches = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        partial_fetches.insert(manifest_ref, value.as_slice()).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)
}

fn index_set_pin(
    root: &CapabilityChunkRoot,
    kind: &str,
    reference: &str,
    pinned: bool,
    receipt_value: Option<&IoValue>,
) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut pins = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        let key = pin_key(kind, reference);
        if pinned {
            pins.insert(key.as_str(), kind).map_err(index_error)?;
        } else {
            pins.remove(key.as_str()).map_err(index_error)?;
        }
    }
    if let Some(receipt_value) = receipt_value {
        store_receipt_in_tx(&write_txn, receipt_value)?;
    }
    write_txn.commit().map_err(index_error)
}

struct IndexApplyGcInput<'a> {
    root: &'a CapabilityChunkRoot,
    dry_run: bool,
    removed_manifests: &'a [String],
    removed_chunks: &'a [String],
    receipt_value: &'a IoValue,
    tombstone_receipt: Option<&'a IoValue>,
}

fn index_apply_gc(input: &IndexApplyGcInput<'_>) -> Result<()> {
    let db = ensure_index_tables(input.root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    if !input.dry_run {
        {
            let mut manifests = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
            let mut partial_fetches = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
            for manifest_ref in input.removed_manifests {
                manifests.remove(manifest_ref.as_str()).map_err(index_error)?;
                partial_fetches.remove(manifest_ref.as_str()).map_err(index_error)?;
            }
        }
        {
            let mut chunks = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
            let mut availability = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
            let mut pins = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
            for chunk_ref in input.removed_chunks {
                chunks.remove(chunk_ref.as_str()).map_err(index_error)?;
                availability.remove(chunk_ref.as_str()).map_err(index_error)?;
                pins.remove(pin_key("chunk", chunk_ref).as_str()).map_err(index_error)?;
            }
        }
    }
    store_receipt_in_tx(&write_txn, input.receipt_value)?;
    if let Some(tombstone_receipt) = input.tombstone_receipt {
        store_receipt_in_tx(&write_txn, tombstone_receipt)?;
    }
    write_txn.commit().map_err(index_error)
}

fn store_receipt(root: &CapabilityChunkRoot, receipt_value: &IoValue) -> Result<()> {
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

fn ensure_index_tables(root: &CapabilityChunkRoot) -> Result<Database> {
    let database_file = root.root().open_database_file(&store_path(INDEX_FILE)?)?;
    let db = Database::builder().create_file(database_file).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn clear_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> Result<()> {
    {
        let mut table = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
        let keys = str_table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
        let keys = str_table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    {
        let mut table = write_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?;
        let keys = table_keys(&table)?;
        for key in keys {
            table.remove(key.as_str()).map_err(index_error)?;
        }
    }
    Ok(())
}

fn table_keys(table: &redb::Table<'_, &str, &[u8]>) -> Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn pin_key(kind: &str, reference: &str) -> String {
    format!("{kind}:{reference}")
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("chunk store redb index error: {error}"))
}
