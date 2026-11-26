//! Chunk and blob verification: content refs, lengths, and reconstructed object size.

pub(super) fn reconstruct_object(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    verify_manifest_chunks_into(root, manifest, &mut bytes)?;
    Ok(bytes)
}

pub(super) fn verify_manifest_chunks(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
) -> crate::error::Result<()> {
    let mut sink = Vec::new();
    verify_manifest_chunks_into(root, manifest, &mut sink)
}

fn verify_manifest_chunks_into(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest: &crate::blocks::ChunkManifest,
    bytes: &mut impl crate::bounded::VecSink<u8>,
) -> crate::error::Result<()> {
    let total_len = usize::try_from(manifest.total_len).map_err(|error| {
        crate::error::Failure::invalid_harness(format!(
            "chunk manifest total length is unsupported on this platform: {error}"
        ))
    })?;
    crate::bounded::ensure_count_at_most(
        total_len,
        crate::blocks::limits::MAX_CHUNK_STORE_OBJECT_BYTES,
        "chunk store object bytes",
    )?;
    let chunk_size = crate::blocks::limits::chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    for chunk in &manifest.chunks {
        let chunk_bytes = crate::blocks::read_verified_chunk(root, chunk, chunk_size)?;
        crate::blocks::limits::extend_bytes_bounded(
            &mut *bytes,
            &chunk_bytes,
            crate::blocks::limits::MAX_CHUNK_STORE_OBJECT_BYTES,
            "chunk store object bytes",
        )?;
    }
    let actual_len = u64::try_from(bytes.item_count()).map_err(|error| {
        crate::error::Failure::invalid_harness(format!(
            "chunk manifest actual length is unsupported on this platform: {error}"
        ))
    })?;
    if actual_len != manifest.total_len {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk manifest total length mismatch: got {}, expected {}",
            actual_len, manifest.total_len
        )));
    }
    Ok(())
}

pub(super) fn verify_raw_chunk_file(
    root: &crate::blocks::CapabilityChunkRoot,
    path: &crate::local_store::Locator,
    chunk_ref: &str,
    length: u64,
    chunk_size: usize,
) -> crate::error::Result<()> {
    let bytes = root.root().read(path)?;
    verify_raw_chunk_bytes(&bytes, chunk_ref, length, chunk_size).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("chunk store content path for {chunk_ref} is invalid: {error}"))
    })
}

pub(super) fn verify_raw_chunk_bytes(
    bytes: &[u8],
    chunk_ref: &str,
    length: u64,
    chunk_size: usize,
) -> crate::error::Result<()> {
    let actual_ref = crate::blocks::hash_chunk(bytes, chunk_size);
    if actual_ref != chunk_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk bytes hash mismatch: got {actual_ref}, expected {chunk_ref}"
        )));
    }
    if bytes.len() as u64 != length {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk bytes length {}, expected {length}",
            bytes.len()
        )));
    }
    Ok(())
}

pub(super) fn chunk_domain(chunk_size: usize) -> String {
    format!("molten.chunk-store.chunk.fixed_v1:{chunk_size}")
}

pub(super) fn hash_blob_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}
