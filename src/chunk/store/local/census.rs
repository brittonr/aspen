//! Listings of manifests, chunks, and receipts, and the in-memory chunk availability decision.

use redb::ReadableDatabase;
use redb::ReadableTable;

const CHUNK_AVAILABILITY_DIAGNOSTIC_CAPACITY: usize = 8;

pub fn evaluate_chunk_availability(
    input: crate::blocks::ChunkAvailabilityInput<'_>,
) -> crate::blocks::ChunkAvailabilityDecision {
    let manifest_refs = input
        .manifest
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_ref.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let available = ref_set(input.available_chunk_refs);
    let missing = ref_set(input.missing_chunk_refs);
    let indexed_available = ref_set(input.indexed_available_refs);
    let indexed_missing = ref_set(input.indexed_missing_refs);
    let partial_missing = ref_set(input.partial_fetch_missing_refs);
    let partial_fetched = ref_set(input.partial_fetch_fetched_refs);

    let mut diagnostics = Vec::with_capacity(CHUNK_AVAILABILITY_DIAGNOSTIC_CAPACITY);
    let union = available.union(&missing).cloned().collect::<std::collections::BTreeSet<_>>();
    if union != manifest_refs || !available.is_disjoint(&missing) {
        diagnostics.push("availability-partition-mismatch".to_string());
    }
    if indexed_available != available || indexed_missing != missing {
        diagnostics.push("index-availability-mismatch".to_string());
    }
    if !missing.is_empty() {
        diagnostics.push("chunk-missing".to_string());
    }
    if !available.is_subset(&manifest_refs)
        || !missing.is_subset(&manifest_refs)
        || !indexed_available.is_subset(&manifest_refs)
        || !indexed_missing.is_subset(&manifest_refs)
        || !partial_missing.is_subset(&manifest_refs)
        || !partial_fetched.is_subset(&manifest_refs)
    {
        diagnostics.push("unknown-chunk-ref".to_string());
    }
    if !partial_missing.is_empty()
        && (!partial_missing.is_subset(&manifest_refs) || !partial_fetched.is_subset(&manifest_refs))
    {
        diagnostics.push("partial-fetch-mismatch".to_string());
    }
    if !partial_fetched.is_disjoint(&missing) {
        diagnostics.push("partial-fetch-repair-incomplete".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    crate::blocks::ChunkAvailabilityDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn ref_set(refs: &[String]) -> std::collections::BTreeSet<String> {
    refs.iter().cloned().collect()
}

pub fn list_manifest_refs(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    list_manifest_refs_with_root(&root)
}

pub fn list_manifest_refs_with_root(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<Vec<String>> {
    crate::blocks::layout::refs_from_dir(root, &crate::blocks::layout::path("manifests")?)
}

pub fn list_chunk_refs(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    list_chunk_refs_with_root(&root)
}

pub fn list_chunk_refs_with_root(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<Vec<String>> {
    crate::blocks::layout::refs_from_dir(root, &crate::blocks::layout::path("chunks")?)
}

pub fn list_receipt_refs(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    list_receipt_refs_with_root(&root)
}

pub fn list_receipt_refs_with_root(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<Vec<String>> {
    crate::blocks::layout::ensure_dirs(root)?;
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(crate::blocks::registry::index_error)?;
    let table = read_txn
        .open_table(crate::blocks::registry::INDEX_RECEIPTS)
        .map_err(crate::blocks::registry::index_error)?;
    let mut refs = Vec::new();
    for item in table.iter().map_err(crate::blocks::registry::index_error)? {
        let (key, _value) = item.map_err(crate::blocks::registry::index_error)?;
        crate::bounded::push(
            &mut refs,
            key.value().to_string(),
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk store receipt refs",
        )?;
    }
    refs.sort();
    Ok(refs)
}

pub fn read_receipt(root: &std::path::Path, receipt_ref: &str) -> crate::error::Result<crate::blocks::ChunkReceipt> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    read_receipt_with_root(&root, receipt_ref)
}

pub fn read_receipt_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    receipt_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkReceipt> {
    crate::blocks::layout::ensure_dirs(root)?;
    let db = crate::blocks::registry::ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(crate::blocks::registry::index_error)?;
    let table = read_txn
        .open_table(crate::blocks::registry::INDEX_RECEIPTS)
        .map_err(crate::blocks::registry::index_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(crate::blocks::registry::index_error)? else {
        return Err(crate::error::Failure::invalid_harness(format!("unknown chunk store receipt {receipt_ref}")));
    };
    let value = crate::preserves_rail::parse_canonical_bytes(bytes.value())?;
    crate::blocks::parse_receipt_value(&value, Some(receipt_ref))
}
