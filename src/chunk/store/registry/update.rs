//! Index writes for puts, availability, partial fetches, pins, and GC, and their index records.

const CHUNK_INDEX_SCHEMA: &str = "molten.chunk-store.index.chunk.v1";
const PARTIAL_FETCH_SCHEMA: &str = "molten.chunk-store.index.partial-fetch.v1";

pub(super) fn chunk_index_value(chunk: &crate::blocks::ChunkRef) -> preserves::IOValue {
    crate::preserves_rail::record("chunk-index-v1", vec![
        crate::preserves_rail::string(CHUNK_INDEX_SCHEMA),
        crate::preserves_rail::record("chunk-ref", vec![crate::preserves_rail::string(&chunk.chunk_ref)]),
        crate::preserves_rail::record("length", vec![crate::preserves_rail::u64_value(chunk.length)]),
        crate::preserves_rail::record("domain", vec![crate::preserves_rail::string(&chunk.domain)]),
        crate::preserves_rail::record("chunker", vec![crate::preserves_rail::string(&chunk.chunker)]),
        crate::preserves_rail::record("transforms", vec![crate::blocks::wire::encode::transforms_value(
            &chunk.transforms,
        )]),
    ])
}

fn partial_fetch_value(
    manifest_ref: &str,
    status: &str,
    missing_before: &[String],
    fetched: &[String],
) -> preserves::IOValue {
    crate::preserves_rail::record("partial-fetch-v1", vec![
        crate::preserves_rail::string(PARTIAL_FETCH_SCHEMA),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(manifest_ref)]),
        crate::preserves_rail::record("status", vec![crate::preserves_rail::string(status)]),
        crate::preserves_rail::record("missing-before", vec![crate::preserves_rail::sequence(
            missing_before.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("fetched", vec![crate::preserves_rail::sequence(
            fetched.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ])
}

pub(in crate::blocks) fn index_put(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_value: &preserves::IOValue,
    chunks: &[crate::blocks::ChunkRef],
    receipt_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let manifest = crate::blocks::parse_manifest_value(manifest_value, None)?;
    let chunk_refs = chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(root, &manifest, &chunk_refs, &[], Some(receipt_value))
}

pub(in crate::blocks) fn index_manifest_available(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
    receipt_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(root, manifest, &chunk_refs, &[], Some(receipt_value))
}

pub(in crate::blocks) fn index_set_manifest_chunk_availability(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
    available: &[String],
    missing: &[String],
    receipt_value: Option<&preserves::IOValue>,
) -> crate::error::Result<()> {
    let available = available.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let missing = missing.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let manifest_bytes = crate::preserves_rail::canonical_bytes(&manifest.value)?;
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(crate::blocks::registry::index_error)?;
    {
        let mut manifests = write_txn
            .open_table(crate::blocks::registry::INDEX_MANIFESTS)
            .map_err(crate::blocks::registry::index_error)?;
        manifests
            .insert(manifest.manifest_ref.as_str(), manifest_bytes.as_slice())
            .map_err(crate::blocks::registry::index_error)?;
    }
    {
        let mut chunks = write_txn
            .open_table(crate::blocks::registry::INDEX_CHUNKS)
            .map_err(crate::blocks::registry::index_error)?;
        let mut availability = write_txn
            .open_table(crate::blocks::registry::INDEX_AVAILABILITY)
            .map_err(crate::blocks::registry::index_error)?;
        for chunk in &manifest.chunks {
            let chunk_value = crate::preserves_rail::canonical_bytes(&chunk_index_value(chunk))?;
            chunks
                .insert(chunk.chunk_ref.as_str(), chunk_value.as_slice())
                .map_err(crate::blocks::registry::index_error)?;
            let status = if missing.contains(&chunk.chunk_ref) {
                "missing"
            } else if available.contains(&chunk.chunk_ref)
                || root.root().try_exists(&crate::blocks::layout::chunk_path(&chunk.chunk_ref)?)?
            {
                "available"
            } else {
                "missing"
            };
            availability
                .insert(chunk.chunk_ref.as_str(), status)
                .map_err(crate::blocks::registry::index_error)?;
        }
    }
    if let Some(receipt_value) = receipt_value {
        crate::blocks::registry::receipt_in_tx(&write_txn, receipt_value)?;
    }
    write_txn.commit().map_err(crate::blocks::registry::index_error)
}

pub(in crate::blocks) fn index_set_partial_fetch(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
    status: &str,
    missing_before: &[String],
    fetched: &[String],
) -> crate::error::Result<()> {
    let value =
        crate::preserves_rail::canonical_bytes(&partial_fetch_value(manifest_ref, status, missing_before, fetched))?;
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(crate::blocks::registry::index_error)?;
    {
        let mut partial_fetches = write_txn
            .open_table(crate::blocks::registry::INDEX_PARTIAL_FETCHES)
            .map_err(crate::blocks::registry::index_error)?;
        partial_fetches
            .insert(manifest_ref, value.as_slice())
            .map_err(crate::blocks::registry::index_error)?;
    }
    write_txn.commit().map_err(crate::blocks::registry::index_error)
}

pub(in crate::blocks) fn index_set_pin(
    root: &crate::blocks::CapabilityChunkRoot,
    kind: &str,
    reference: &str,
    pinned: bool,
    receipt_value: Option<&preserves::IOValue>,
) -> crate::error::Result<()> {
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(crate::blocks::registry::index_error)?;
    {
        let mut pins = write_txn
            .open_table(crate::blocks::registry::INDEX_PINS)
            .map_err(crate::blocks::registry::index_error)?;
        let key = crate::blocks::registry::pin_key(kind, reference);
        if pinned {
            pins.insert(key.as_str(), kind).map_err(crate::blocks::registry::index_error)?;
        } else {
            pins.remove(key.as_str()).map_err(crate::blocks::registry::index_error)?;
        }
    }
    if let Some(receipt_value) = receipt_value {
        crate::blocks::registry::receipt_in_tx(&write_txn, receipt_value)?;
    }
    write_txn.commit().map_err(crate::blocks::registry::index_error)
}

pub(in crate::blocks) struct IndexApplyGcInput<'a> {
    pub(in crate::blocks) root: &'a crate::blocks::CapabilityChunkRoot,
    pub(in crate::blocks) dry_run: bool,
    pub(in crate::blocks) removed_manifests: &'a [String],
    pub(in crate::blocks) removed_chunks: &'a [String],
    pub(in crate::blocks) receipt_value: &'a preserves::IOValue,
    pub(in crate::blocks) tombstone_receipt: Option<&'a preserves::IOValue>,
}

pub(in crate::blocks) fn index_apply_gc(input: &IndexApplyGcInput<'_>) -> crate::error::Result<()> {
    let db = crate::blocks::registry::ensure_index_tables(input.root)?;
    let write_txn = db.begin_write().map_err(crate::blocks::registry::index_error)?;
    if !input.dry_run {
        {
            let mut manifests = write_txn
                .open_table(crate::blocks::registry::INDEX_MANIFESTS)
                .map_err(crate::blocks::registry::index_error)?;
            let mut partial_fetches = write_txn
                .open_table(crate::blocks::registry::INDEX_PARTIAL_FETCHES)
                .map_err(crate::blocks::registry::index_error)?;
            for manifest_ref in input.removed_manifests {
                manifests.remove(manifest_ref.as_str()).map_err(crate::blocks::registry::index_error)?;
                partial_fetches.remove(manifest_ref.as_str()).map_err(crate::blocks::registry::index_error)?;
            }
        }
        {
            let mut chunks = write_txn
                .open_table(crate::blocks::registry::INDEX_CHUNKS)
                .map_err(crate::blocks::registry::index_error)?;
            let mut availability = write_txn
                .open_table(crate::blocks::registry::INDEX_AVAILABILITY)
                .map_err(crate::blocks::registry::index_error)?;
            let mut pins = write_txn
                .open_table(crate::blocks::registry::INDEX_PINS)
                .map_err(crate::blocks::registry::index_error)?;
            for chunk_ref in input.removed_chunks {
                chunks.remove(chunk_ref.as_str()).map_err(crate::blocks::registry::index_error)?;
                availability.remove(chunk_ref.as_str()).map_err(crate::blocks::registry::index_error)?;
                pins.remove(crate::blocks::registry::pin_key("chunk", chunk_ref).as_str())
                    .map_err(crate::blocks::registry::index_error)?;
            }
        }
    }
    crate::blocks::registry::receipt_in_tx(&write_txn, input.receipt_value)?;
    if let Some(tombstone_receipt) = input.tombstone_receipt {
        crate::blocks::registry::receipt_in_tx(&write_txn, tombstone_receipt)?;
    }
    write_txn.commit().map_err(crate::blocks::registry::index_error)
}
