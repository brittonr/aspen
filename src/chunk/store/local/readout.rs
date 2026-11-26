//! Manifest reads, object reads, manifest verification, and missing-chunk scans.

pub fn read_manifest(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<crate::blocks::ChunkManifest> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    read_manifest_with_root(&root, manifest_ref)
}

pub fn read_manifest_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkManifest> {
    let bytes = root.root().read(&crate::blocks::layout::manifest_path(manifest_ref)?)?;
    let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
    crate::blocks::parse_manifest_value(&value, Some(manifest_ref))
}

/// Reads `manifest_ref` for `operation`, recording a denial receipt when the manifest is missing or
/// invalid.
pub(in crate::blocks) fn read_admitted_manifest(
    root: &crate::blocks::CapabilityChunkRoot,
    operation: &str,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkManifest> {
    match read_manifest_with_root(root, manifest_ref) {
        Ok(manifest) => Ok(manifest),
        Err(error) => {
            let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
                operation,
                Some(manifest_ref),
                &[],
                error.to_string(),
                vec![
                    ("manifest-ref-binding", "fail"),
                    ("deny-missing-or-invalid-manifest", "pass"),
                ],
            );
            crate::blocks::registry::receipt(root, &receipt_value)?;
            Err(error)
        }
    }
}

pub fn verify_manifest(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<crate::blocks::ChunkVerify> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    verify_manifest_with_root(&root, manifest_ref)
}

pub fn verify_manifest_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkVerify> {
    let manifest = read_admitted_manifest(root, "chunk-verify", manifest_ref)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    crate::blocks::registry::deny_unsupported_transform(root, "chunk-verify", &manifest, &chunk_refs)?;
    if let Err(error) = crate::blocks::integrity::verify_manifest_chunks(root, &manifest) {
        let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
            "chunk-verify",
            Some(&manifest.manifest_ref),
            &chunk_refs,
            error.to_string(),
            vec![("chunk-hash-length", "fail"), ("deny-corrupt-or-missing-chunk", "pass")],
        );
        crate::blocks::registry::receipt(root, &receipt_value)?;
        return Err(error);
    }
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "chunk-verify",
            decision: "pass",
            manifest_ref: Some(&manifest.manifest_ref),
            chunk_refs: &chunk_refs,
            checks: vec![
                ("manifest-ref-binding", "pass"),
                ("chunk-hash-length", "pass"),
                ("chunk-order-root", "pass"),
                ("reconstructed-total-length", "pass"),
                ("redb-index-availability", "pass"),
            ],
            details: vec![crate::preserves_rail::record("total-len", vec![
                crate::preserves_rail::u64_value(manifest.total_len),
            ])],
        });
    crate::blocks::registry::update::index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(crate::blocks::ChunkVerify {
        manifest_ref: manifest.manifest_ref,
        total_len: manifest.total_len,
        chunk_refs,
        receipt_value,
    })
}

pub fn read_object(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<crate::blocks::ChunkRead> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    read_object_with_root(&root, manifest_ref)
}

pub fn read_object_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkRead> {
    let manifest = read_admitted_manifest(root, "fetch", manifest_ref)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    crate::blocks::registry::deny_unsupported_transform(root, "fetch", &manifest, &chunk_refs)?;
    let bytes = match crate::blocks::integrity::reconstruct_object(root, &manifest) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
                "fetch",
                Some(&manifest.manifest_ref),
                &chunk_refs,
                error.to_string(),
                vec![
                    ("streaming-chunk-verification", "fail"),
                    ("deny-corrupt-or-missing-chunk", "pass"),
                ],
            );
            crate::blocks::registry::receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "fetch",
            decision: "pass",
            manifest_ref: Some(&manifest.manifest_ref),
            chunk_refs: &chunk_refs,
            checks: vec![
                ("manifest-ref-binding", "pass"),
                ("streaming-chunk-verification", "pass"),
                ("reconstructed-total-length", "pass"),
                ("redb-index-availability", "pass"),
            ],
            details: vec![crate::preserves_rail::record("total-len", vec![
                crate::preserves_rail::u64_value(bytes.len() as u64),
            ])],
        });
    crate::blocks::registry::update::index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(crate::blocks::ChunkRead {
        manifest_ref: manifest.manifest_ref,
        bytes,
        receipt_value,
    })
}

pub fn missing_chunks(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<Vec<String>> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    missing_chunks_with_root(&root, manifest_ref)
}

pub fn missing_chunks_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<Vec<String>> {
    let manifest = read_manifest_with_root(root, manifest_ref)?;
    let mut missing = Vec::new();
    let mut available = Vec::new();
    for chunk in &manifest.chunks {
        if root.root().try_exists(&crate::blocks::layout::chunk_path(&chunk.chunk_ref)?)? {
            crate::bounded::push(
                &mut available,
                chunk.chunk_ref.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
                "chunk store available chunks",
            )?;
        } else {
            crate::bounded::push(
                &mut missing,
                chunk.chunk_ref.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
                "chunk store missing chunks",
            )?;
        }
    }
    crate::blocks::registry::update::index_set_manifest_chunk_availability(
        root, &manifest, &available, &missing, None,
    )?;
    Ok(missing)
}
