//! Index status counts and a full index rebuild from the files in a chunk root.

use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::ReadableTableMetadata;

pub fn index_status(root: &std::path::Path) -> crate::error::Result<crate::blocks::ChunkIndexStatus> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    index_status_with_root(&root)
}

pub fn index_status_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
) -> crate::error::Result<crate::blocks::ChunkIndexStatus> {
    crate::blocks::layout::ensure_dirs(root)?;
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(crate::blocks::registry::index_error)?;
    let manifests = read_txn
        .open_table(crate::blocks::registry::INDEX_MANIFESTS)
        .map_err(crate::blocks::registry::index_error)?
        .len()
        .map_err(crate::blocks::registry::index_error)?;
    let chunks = read_txn
        .open_table(crate::blocks::registry::INDEX_CHUNKS)
        .map_err(crate::blocks::registry::index_error)?
        .len()
        .map_err(crate::blocks::registry::index_error)?;
    let availability = read_txn
        .open_table(crate::blocks::registry::INDEX_AVAILABILITY)
        .map_err(crate::blocks::registry::index_error)?;
    let mut available_chunks = 0;
    let mut missing_chunks = 0;
    for item in availability.iter().map_err(crate::blocks::registry::index_error)? {
        let (_key, value) = item.map_err(crate::blocks::registry::index_error)?;
        match value.value() {
            "available" => available_chunks += 1,
            "missing" => missing_chunks += 1,
            _ => {}
        }
    }
    let pins = read_txn
        .open_table(crate::blocks::registry::INDEX_PINS)
        .map_err(crate::blocks::registry::index_error)?;
    let mut manifest_pins = 0;
    let mut chunk_pins = 0;
    for item in pins.iter().map_err(crate::blocks::registry::index_error)? {
        let (key, _value) = item.map_err(crate::blocks::registry::index_error)?;
        if key.value().starts_with("manifest:") {
            manifest_pins += 1;
        } else if key.value().starts_with("chunk:") {
            chunk_pins += 1;
        }
    }
    let partial_fetches = read_txn
        .open_table(crate::blocks::registry::INDEX_PARTIAL_FETCHES)
        .map_err(crate::blocks::registry::index_error)?
        .len()
        .map_err(crate::blocks::registry::index_error)?;
    let receipts = read_txn
        .open_table(crate::blocks::registry::INDEX_RECEIPTS)
        .map_err(crate::blocks::registry::index_error)?
        .len()
        .map_err(crate::blocks::registry::index_error)?;
    Ok(crate::blocks::ChunkIndexStatus {
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

pub fn rebuild_index(root: &std::path::Path) -> crate::error::Result<crate::blocks::ChunkIndexRebuild> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    rebuild_index_with_root(&root)
}

pub fn rebuild_index_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
) -> crate::error::Result<crate::blocks::ChunkIndexRebuild> {
    crate::blocks::layout::ensure_dirs(root)?;
    let inputs = scan_inputs(root)?;
    let receipt_value = write_inputs(root, &inputs)?;
    Ok(crate::blocks::ChunkIndexRebuild {
        status: index_status_with_root(root)?,
        receipt_value,
    })
}

struct IndexInputs {
    manifest_entries: Vec<(String, Vec<u8>, crate::blocks::ChunkManifest)>,
    chunk_entries: std::collections::BTreeMap<String, (crate::blocks::ChunkRef, String)>,
    pinned_manifests: Vec<String>,
    pinned_chunks: Vec<String>,
}

fn scan_inputs(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<IndexInputs> {
    let mut manifest_entries = Vec::new();
    let mut chunk_entries: std::collections::BTreeMap<String, (crate::blocks::ChunkRef, String)> =
        std::collections::BTreeMap::new();
    for manifest_ref in crate::blocks::list_manifest_refs_with_root(root)? {
        let bytes = root.root().read(&crate::blocks::layout::manifest_path(&manifest_ref)?)?;
        let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
        let manifest = crate::blocks::parse_manifest_value(&value, Some(&manifest_ref))?;
        scan_manifest_chunks(root, &manifest, &mut chunk_entries)?;
        crate::bounded::push(
            &mut manifest_entries,
            (manifest_ref, bytes, manifest),
            crate::blocks::limits::MAX_CHUNK_STORE_MANIFESTS,
            "chunk store index manifest entries",
        )?;
    }
    Ok(IndexInputs {
        manifest_entries,
        chunk_entries,
        pinned_manifests: crate::blocks::layout::pinned_refs(root, &crate::blocks::layout::path("pins/manifests")?)?,
        pinned_chunks: crate::blocks::layout::pinned_refs(root, &crate::blocks::layout::path("pins/chunks")?)?,
    })
}

fn scan_manifest_chunks(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
    chunk_entries: &mut std::collections::BTreeMap<String, (crate::blocks::ChunkRef, String)>,
) -> crate::error::Result<()> {
    let chunk_size = crate::blocks::limits::chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    for chunk in &manifest.chunks {
        let available = if root.root().try_exists(&crate::blocks::layout::chunk_path(&chunk.chunk_ref)?)? {
            crate::blocks::read_verified_chunk(root, chunk, chunk_size)?;
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

fn write_inputs(
    root: &crate::blocks::CapabilityChunkRoot,
    inputs: &IndexInputs,
) -> crate::error::Result<preserves::IOValue> {
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(crate::blocks::registry::index_error)?;
    crate::blocks::registry::clear_index_tables_in_tx(&write_txn)?;
    write_manifest_entries(&write_txn, &inputs.manifest_entries)?;
    write_entries(&write_txn, &inputs.chunk_entries)?;
    write_pin_entries(&write_txn, &inputs.pinned_manifests, &inputs.pinned_chunks)?;
    let receipt_value = rebuild_receipt(inputs);
    crate::blocks::registry::receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(crate::blocks::registry::index_error)?;
    drop(db);
    Ok(receipt_value)
}

fn write_manifest_entries(
    write_txn: &redb::WriteTransaction,
    manifest_entries: &[(String, Vec<u8>, crate::blocks::ChunkManifest)],
) -> crate::error::Result<()> {
    let mut manifests = write_txn
        .open_table(crate::blocks::registry::INDEX_MANIFESTS)
        .map_err(crate::blocks::registry::index_error)?;
    for (manifest_ref, bytes, _manifest) in manifest_entries {
        manifests
            .insert(manifest_ref.as_str(), bytes.as_slice())
            .map_err(crate::blocks::registry::index_error)?;
    }
    Ok(())
}

fn write_entries(
    write_txn: &redb::WriteTransaction,
    entries: &std::collections::BTreeMap<String, (crate::blocks::ChunkRef, String)>,
) -> crate::error::Result<()> {
    let mut chunks = write_txn
        .open_table(crate::blocks::registry::INDEX_CHUNKS)
        .map_err(crate::blocks::registry::index_error)?;
    let mut availability = write_txn
        .open_table(crate::blocks::registry::INDEX_AVAILABILITY)
        .map_err(crate::blocks::registry::index_error)?;
    for (chunk_ref, (chunk, status)) in entries {
        let value = crate::preserves_rail::canonical_bytes(&crate::blocks::registry::update::chunk_index_value(chunk))?;
        chunks.insert(chunk_ref.as_str(), value.as_slice()).map_err(crate::blocks::registry::index_error)?;
        availability
            .insert(chunk_ref.as_str(), status.as_str())
            .map_err(crate::blocks::registry::index_error)?;
    }
    Ok(())
}

fn write_pin_entries(
    write_txn: &redb::WriteTransaction,
    pinned_manifests: &[String],
    pinned_chunks: &[String],
) -> crate::error::Result<()> {
    let mut pins = write_txn
        .open_table(crate::blocks::registry::INDEX_PINS)
        .map_err(crate::blocks::registry::index_error)?;
    for manifest_ref in pinned_manifests {
        pins.insert(crate::blocks::registry::pin_key("manifest", manifest_ref).as_str(), "manifest")
            .map_err(crate::blocks::registry::index_error)?;
    }
    for chunk_ref in pinned_chunks {
        pins.insert(crate::blocks::registry::pin_key("chunk", chunk_ref).as_str(), "chunk")
            .map_err(crate::blocks::registry::index_error)?;
    }
    Ok(())
}

fn rebuild_receipt(inputs: &IndexInputs) -> preserves::IOValue {
    let chunk_refs = inputs.chunk_entries.keys().cloned().collect::<Vec<_>>();
    crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
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
            crate::preserves_rail::record("manifests", vec![crate::preserves_rail::u64_value(
                inputs.manifest_entries.len() as u64,
            )]),
            crate::preserves_rail::record("chunks", vec![crate::preserves_rail::u64_value(
                inputs.chunk_entries.len() as u64
            )]),
            crate::preserves_rail::record("manifest-pins", vec![crate::preserves_rail::u64_value(
                inputs.pinned_manifests.len() as u64,
            )]),
            crate::preserves_rail::record("chunk-pins", vec![crate::preserves_rail::u64_value(
                inputs.pinned_chunks.len() as u64,
            )]),
        ],
    })
}
