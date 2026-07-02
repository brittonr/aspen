
fn parse_lineage_chain_scope(value: &Value<IoValue>) -> Result<crate::evidence_chain::ChainScope> {
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain scope field"))?;
    Ok(crate::evidence_chain::ChainScope::new(
        record_string(&chain[0], "scope")?,
        record_string(&chain[1], "id")?,
        record_string(&chain[2], "epoch")?,
    ))
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_string(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <none> or <some ref> for {label}")))
    }
}

pub fn index_status(root: &Path) -> Result<ChunkStoreIndexStatus> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let manifests = read_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?.len().map_err(index_error)?;
    let chunks = read_txn.open_table(INDEX_CHUNKS).map_err(index_error)?.len().map_err(index_error)?;
    let availability = read_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
    let mut available_chunks = 0;
    let mut missing_chunks = 0;
    for item in availability.iter().map_err(index_error)? {
        let (_key, value) = item.map_err(index_error)?;
        match value.value() {
            "available" => available_chunks += 1,
            "missing" => missing_chunks += 1,
            _ => {}
        }
    }
    let pins = read_txn.open_table(INDEX_PINS).map_err(index_error)?;
    let mut manifest_pins = 0;
    let mut chunk_pins = 0;
    for item in pins.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        if key.value().starts_with("manifest:") {
            manifest_pins += 1;
        } else if key.value().starts_with("chunk:") {
            chunk_pins += 1;
        }
    }
    let partial_fetches =
        read_txn.open_table(INDEX_PARTIAL_FETCHES).map_err(index_error)?.len().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?.len().map_err(index_error)?;
    Ok(ChunkStoreIndexStatus {
        manifests,
        chunks,
        available_chunks,
        missing_chunks,
        manifest_pins,
        chunk_pins,
        partial_fetches,
        receipts,
    })
}

pub fn rebuild_index(root: &Path) -> Result<ChunkStoreIndexRebuild> {
    ensure_dirs(root)?;
    let inputs = scan_inputs(root)?;
    let receipt_value = write_inputs(root, &inputs)?;
    Ok(ChunkStoreIndexRebuild {
        status: index_status(root)?,
        receipt_value,
    })
}

struct IndexInputs {
    manifest_entries: Vec<(String, Vec<u8>, ChunkManifest)>,
    chunk_entries: OrderedMap<String, (ChunkRef, String)>,
    pinned_manifests: Vec<String>,
    pinned_chunks: Vec<String>,
}

fn scan_inputs(root: &Path) -> Result<IndexInputs> {
    let mut manifest_entries = Vec::new();
    let mut chunk_entries: OrderedMap<String, (ChunkRef, String)> = OrderedMap::new();
    for manifest_ref in list_manifest_refs(root)? {
        let bytes = fs::read(manifest_path(root, &manifest_ref)?).map_err(MoltenError::from)?;
        let value = parse_canonical_bytes(&bytes)?;
        let manifest = parse_manifest_value(&value, Some(&manifest_ref))?;
        scan_manifest_chunks(root, &manifest, &mut chunk_entries)?;
        push_bounded(
            &mut manifest_entries,
            (manifest_ref, bytes, manifest),
            MAX_CHUNK_STORE_MANIFESTS,
            "chunk store index manifest entries",
        )?;
    }
    Ok(IndexInputs {
        manifest_entries,
        chunk_entries,
        pinned_manifests: pinned_refs(&root.join("pins").join("manifests"))?,
        pinned_chunks: pinned_refs(&root.join("pins").join("chunks"))?,
    })
}

fn scan_manifest_chunks(
    root: &Path,
    manifest: &ChunkManifest,
    chunk_entries: &mut OrderedMap<String, (ChunkRef, String)>,
) -> Result<()> {
    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    for chunk in &manifest.chunks {
        let available = if chunk_path(root, &chunk.chunk_ref)?.exists() {
            read_verified_chunk(root, chunk, chunk_size)?;
            "available"
        } else {
            "missing"
        };
        chunk_entries
            .entry(chunk.chunk_ref.clone())
            .and_modify(|(_existing, status)| {
                if available == "available" {
                    *status = "available".to_string();
                }
            })
            .or_insert_with(|| (chunk.clone(), available.to_string()));
    }
    Ok(())
}

fn write_inputs(root: &Path, inputs: &IndexInputs) -> Result<IoValue> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_index_tables_in_tx(&write_txn)?;
    write_manifest_entries(&write_txn, &inputs.manifest_entries)?;
    write_entries(&write_txn, &inputs.chunk_entries)?;
    write_pin_entries(&write_txn, &inputs.pinned_manifests, &inputs.pinned_chunks)?;
    let receipt_value = rebuild_receipt(inputs);
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    drop(db);
    Ok(receipt_value)
}

fn write_manifest_entries(
    write_txn: &redb::WriteTransaction,
    manifest_entries: &[(String, Vec<u8>, ChunkManifest)],
) -> Result<()> {
    let mut manifests = write_txn.open_table(INDEX_MANIFESTS).map_err(index_error)?;
    for (manifest_ref, bytes, _manifest) in manifest_entries {
        manifests.insert(manifest_ref.as_str(), bytes.as_slice()).map_err(index_error)?;
    }
    Ok(())
}

fn write_entries(write_txn: &redb::WriteTransaction, entries: &OrderedMap<String, (ChunkRef, String)>) -> Result<()> {
    let mut chunks = write_txn.open_table(INDEX_CHUNKS).map_err(index_error)?;
    let mut availability = write_txn.open_table(INDEX_AVAILABILITY).map_err(index_error)?;
    for (chunk_ref, (chunk, status)) in entries {
        let value = canonical_bytes(&chunk_index_value(chunk))?;
        chunks.insert(chunk_ref.as_str(), value.as_slice()).map_err(index_error)?;
        availability.insert(chunk_ref.as_str(), status.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn write_pin_entries(
    write_txn: &redb::WriteTransaction,
    pinned_manifests: &[String],
    pinned_chunks: &[String],
) -> Result<()> {
    let mut pins = write_txn.open_table(INDEX_PINS).map_err(index_error)?;
    for manifest_ref in pinned_manifests {
        pins.insert(pin_key("manifest", manifest_ref).as_str(), "manifest").map_err(index_error)?;
    }
    for chunk_ref in pinned_chunks {
        pins.insert(pin_key("chunk", chunk_ref).as_str(), "chunk").map_err(index_error)?;
    }
    Ok(())
}

fn rebuild_receipt(inputs: &IndexInputs) -> IoValue {
    let chunk_refs = inputs.chunk_entries.keys().cloned().collect::<Vec<_>>();
    receipt_value(ChunkStoreReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: &chunk_refs,
        checks: vec![
            ("redb-index-manifests", "pass"),
            ("redb-index-chunks", "pass"),
            ("redb-index-availability", "pass"),
            ("redb-index-pins", "pass"),
        ],
        details: vec![
            record("manifests", vec![u64_value(inputs.manifest_entries.len() as u64)]),
            record("chunks", vec![u64_value(inputs.chunk_entries.len() as u64)]),
            record("manifest-pins", vec![u64_value(inputs.pinned_manifests.len() as u64)]),
            record("chunk-pins", vec![u64_value(inputs.pinned_chunks.len() as u64)]),
        ],
    })
}

struct ChunkManifestValueInput<'a> {
    object_kind: &'a str,
    total_len: u64,
    chunk_size: u64,
    transforms: &'a ChunkTransforms,
    metadata_ref: &'a str,
    policy_refs: &'a [String],
    chunks: &'a [IoValue],
    root_ref: &'a str,
    evidence_refs: &'a [String],
}

fn manifest_value(input: &ChunkManifestValueInput<'_>) -> IoValue {
    record("chunk-manifest-v1", vec![
        string(CHUNK_MANIFEST_SCHEMA),
        record("object-kind", vec![string(input.object_kind)]),
        record("total-len", vec![u64_value(input.total_len)]),
        record("chunker", vec![string(FIXED_V1_CHUNKER)]),
        record("chunk-size", vec![u64_value(input.chunk_size)]),
        record("transforms", vec![transforms_value(input.transforms)]),
        record("metadata-ref", vec![string(input.metadata_ref)]),
        record("policy-refs", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("chunks", vec![sequence(input.chunks.to_vec())]),
        record("root-ref", vec![string(input.root_ref)]),
        record("evidence-refs", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
    ])
}

fn chunk_ref_value(chunk_ref: &str, length: u64, chunk_size: usize, transforms: &ChunkTransforms) -> IoValue {
    record("chunk-ref-v1", vec![
        string(CHUNK_REF_SCHEMA),
        record("hash", vec![string(chunk_ref)]),
        record("length", vec![u64_value(length)]),
        record("domain", vec![string(chunk_domain(chunk_size))]),
        record("chunker", vec![string(FIXED_V1_CHUNKER)]),
        record("transforms", vec![transforms_value(transforms)]),
        record("location-hints", vec![sequence(vec![record("local-content-key", vec![string(
            filename_for_ref(chunk_ref).unwrap_or_else(|_| "unsupported".to_string()),
        )])])]),
        record("evidence-refs", vec![sequence(Vec::new())]),
    ])
}

impl ChunkTransforms {
    pub fn public_plaintext() -> Self {
        Self {
            compression: "none".to_string(),
            encryption: "none".to_string(),
            ordering: "identity".to_string(),
            confidentiality: "public".to_string(),
            protected_commitment_ref: None,
        }
    }

    pub fn confidential_protected(commitment_ref: impl Into<String>) -> Self {
        Self {
            compression: "zstd-placeholder".to_string(),
            encryption: "protected-commitment".to_string(),
            ordering: "compress-then-encrypt".to_string(),
            confidentiality: "confidential".to_string(),
            protected_commitment_ref: Some(commitment_ref.into()),
        }
    }
}

fn transforms_value(transforms: &ChunkTransforms) -> IoValue {
    record("transforms-v1", vec![
        string(CHUNK_TRANSFORMS_SCHEMA),
        record("compression", vec![string(&transforms.compression)]),
        record("encryption", vec![string(&transforms.encryption)]),
        record("ordering", vec![string(&transforms.ordering)]),
        record("confidentiality", vec![string(&transforms.confidentiality)]),
        record("protected-commitment", vec![
            transforms.protected_commitment_ref.as_ref().map(string).unwrap_or_else(|| record("none", vec![])),
        ]),
    ])
}
