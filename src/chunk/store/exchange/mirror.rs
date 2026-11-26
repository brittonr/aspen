//! Root-to-root sync of the chunks a destination lacks, with resumable partial-fetch state.

pub fn sync_missing_chunks(
    source_root: &std::path::Path,
    dest_root: &std::path::Path,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkSync> {
    let source_root = crate::blocks::open_capability_chunk_root(source_root)?;
    let dest_root = crate::blocks::open_capability_chunk_root(dest_root)?;
    sync_missing_chunks_with_roots(&source_root, &dest_root, manifest_ref)
}

pub fn sync_missing_chunks_with_roots(
    source_root: &crate::blocks::CapabilityChunkRoot,
    dest_root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkSync> {
    crate::blocks::layout::ensure_dirs(dest_root)?;
    let manifest = crate::blocks::read_manifest_with_root(source_root, manifest_ref)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    crate::blocks::registry::deny_unsupported_transform(dest_root, "remote-sync", &manifest, &chunk_refs)?;
    copy_manifest(source_root, dest_root, manifest_ref)?;

    let chunk_size = crate::blocks::limits::chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let scan = scan_refs(dest_root, &manifest, chunk_size)?;
    crate::blocks::registry::update::index_set_partial_fetch(
        dest_root,
        &manifest.manifest_ref,
        "in-progress",
        &scan.missing_before,
        &[],
    )?;
    crate::blocks::registry::update::index_set_manifest_chunk_availability(
        dest_root,
        &manifest,
        &scan.already_available,
        &scan.missing_before,
        None,
    )?;

    let fetched_chunks = copy_refs(CopyInput {
        source_root,
        dest_root,
        manifest: &manifest,
        chunk_size,
        missing_before: &scan.missing_before,
    })?;
    crate::blocks::verify_manifest_with_root(dest_root, manifest_ref)?;
    let receipt_value = sync_receipt(&manifest.manifest_ref, &scan.missing_before, &fetched_chunks);
    let available_after = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    crate::blocks::registry::update::index_set_manifest_chunk_availability(
        dest_root,
        &manifest,
        &available_after,
        &[],
        Some(&receipt_value),
    )?;
    crate::blocks::registry::update::index_set_partial_fetch(
        dest_root,
        &manifest.manifest_ref,
        "complete",
        &scan.missing_before,
        &fetched_chunks,
    )?;
    Ok(crate::blocks::ChunkSync {
        manifest_ref: manifest.manifest_ref,
        missing_before: scan.missing_before,
        fetched_chunks,
        receipt_value,
    })
}

/// Copies the canonical manifest bytes into the destination root, refusing a corrupted existing
/// copy.
fn copy_manifest(
    source_root: &crate::blocks::CapabilityChunkRoot,
    dest_root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<()> {
    let manifest_bytes = source_root.root().read(&crate::blocks::layout::manifest_path(manifest_ref)?)?;
    crate::blocks::layout::write_immutable_bytes(
        dest_root,
        &crate::blocks::layout::manifest_path(manifest_ref)?,
        &manifest_bytes,
        manifest_ref,
        crate::preserves_rail::parse_canonical_bytes,
    )
}

fn sync_receipt(manifest_ref: &str, missing_before: &[String], fetched_chunks: &[String]) -> preserves::IOValue {
    crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
        operation: "remote-sync",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: fetched_chunks,
        checks: vec![
            ("manifest-identity-preserved", "pass"),
            ("missing-chunk-calculation", "pass"),
            ("redb-partial-fetch-state", "pass"),
            ("resumable-fetch", "pass"),
            ("streaming-chunk-verification", "pass"),
        ],
        details: vec![
            crate::preserves_rail::record("missing-before", vec![crate::blocks::wire::encode::string_sequence(
                missing_before,
            )]),
            crate::preserves_rail::record("fetched", vec![crate::blocks::wire::encode::string_sequence(
                fetched_chunks,
            )]),
        ],
    })
}

pub(super) struct Scan {
    pub(super) missing_before: Vec<String>,
    pub(super) already_available: Vec<String>,
}

fn scan_refs(
    dest_root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
    chunk_size: usize,
) -> crate::error::Result<Scan> {
    let mut missing_before = Vec::new();
    let mut already_available = Vec::new();
    for chunk in &manifest.chunks {
        let dest_chunk_path = crate::blocks::layout::chunk_path(&chunk.chunk_ref)?;
        if dest_root.root().try_exists(&dest_chunk_path)? {
            crate::blocks::read_verified_chunk(dest_root, chunk, chunk_size)?;
            crate::bounded::push(
                &mut already_available,
                chunk.chunk_ref.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
                "chunk store already available chunks",
            )?;
        } else {
            crate::bounded::push(
                &mut missing_before,
                chunk.chunk_ref.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
                "chunk store missing-before chunks",
            )?;
        }
    }
    Ok(Scan {
        missing_before,
        already_available,
    })
}

struct CopyInput<'a> {
    source_root: &'a crate::blocks::CapabilityChunkRoot,
    dest_root: &'a crate::blocks::CapabilityChunkRoot,
    manifest: &'a crate::blocks::ChunkManifest,
    chunk_size: usize,
    missing_before: &'a [String],
}

fn copy_refs(input: CopyInput<'_>) -> crate::error::Result<Vec<String>> {
    let mut fetched_chunks = Vec::new();
    for chunk_ref in input.missing_before {
        let chunk =
            input.manifest.chunks.iter().find(|candidate| &candidate.chunk_ref == chunk_ref).ok_or_else(|| {
                crate::error::Failure::invalid_harness(format!("manifest missing expected chunk {chunk_ref}"))
            })?;
        let bytes = crate::blocks::read_verified_chunk(input.source_root, chunk, input.chunk_size)?;
        input.dest_root.root().write(&crate::blocks::layout::chunk_path(&chunk.chunk_ref)?, &bytes)?;
        crate::bounded::push(
            &mut fetched_chunks,
            chunk.chunk_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store fetched chunks",
        )?;
        crate::blocks::registry::update::index_set_partial_fetch(
            input.dest_root,
            &input.manifest.manifest_ref,
            "in-progress",
            input.missing_before,
            &fetched_chunks,
        )?;
    }
    Ok(fetched_chunks)
}
