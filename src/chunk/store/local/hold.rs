//! Manifest and chunk pins.

pub fn pin_manifest(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<crate::blocks::ChunkPin> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    pin_manifest_with_root(&root, manifest_ref)
}

pub fn pin_manifest_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkPin> {
    crate::blocks::layout::ensure_dirs(root)?;
    let manifest = crate::blocks::read_manifest_with_root(root, manifest_ref)?;
    root.root()
        .write(&crate::blocks::layout::manifest_pin_path(manifest_ref)?, manifest_ref.as_bytes())?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "pin",
            decision: "pass",
            manifest_ref: Some(manifest_ref),
            chunk_refs: &chunk_refs,
            checks: vec![("manifest-exists", "pass"), ("pin-index-update", "pass")],
            details: vec![crate::preserves_rail::record("pin-kind", vec![
                crate::preserves_rail::string("manifest"),
            ])],
        });
    crate::blocks::registry::update::index_set_pin(root, "manifest", manifest_ref, true, Some(&receipt_value))?;
    Ok(crate::blocks::ChunkPin {
        kind: "manifest".to_string(),
        reference: manifest_ref.to_string(),
        pinned: true,
        receipt_value,
    })
}

pub fn unpin_manifest(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<crate::blocks::ChunkPin> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    unpin_manifest_with_root(&root, manifest_ref)
}

pub fn unpin_manifest_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkPin> {
    crate::blocks::layout::ensure_dirs(root)?;
    let pin_path = crate::blocks::layout::manifest_pin_path(manifest_ref)?;
    if root.root().try_exists(&pin_path)? {
        root.root().remove_file(&pin_path)?;
    }
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "unpin",
            decision: "pass",
            manifest_ref: Some(manifest_ref),
            chunk_refs: &[],
            checks: vec![("pin-removal-idempotent", "pass"), ("pin-index-update", "pass")],
            details: vec![crate::preserves_rail::record("pin-kind", vec![
                crate::preserves_rail::string("manifest"),
            ])],
        });
    crate::blocks::registry::update::index_set_pin(root, "manifest", manifest_ref, false, Some(&receipt_value))?;
    Ok(crate::blocks::ChunkPin {
        kind: "manifest".to_string(),
        reference: manifest_ref.to_string(),
        pinned: false,
        receipt_value,
    })
}

pub fn pin_chunk(root: &std::path::Path, chunk_ref: &str) -> crate::error::Result<crate::blocks::ChunkPin> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    pin_chunk_with_root(&root, chunk_ref)
}

pub fn pin_chunk_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    chunk_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkPin> {
    crate::blocks::layout::ensure_dirs(root)?;
    let path = crate::blocks::layout::chunk_path(chunk_ref)?;
    if !root.root().try_exists(&path)? {
        let receipt_value =
            crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
                operation: "pin",
                decision: "deny",
                manifest_ref: None,
                chunk_refs: &[chunk_ref.to_string()],
                checks: vec![("chunk-exists", "fail"), ("deny-missing-chunk-pin", "pass")],
                details: vec![crate::preserves_rail::record("pin-kind", vec![
                    crate::preserves_rail::string("chunk"),
                ])],
            });
        crate::blocks::registry::receipt(root, &receipt_value)?;
        return Err(crate::error::Failure::invalid_harness(format!("cannot pin missing chunk {chunk_ref}")));
    }
    root.root().write(&crate::blocks::layout::chunk_pin_path(chunk_ref)?, chunk_ref.as_bytes())?;
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "pin",
            decision: "pass",
            manifest_ref: None,
            chunk_refs: &[chunk_ref.to_string()],
            checks: vec![("chunk-exists", "pass"), ("pin-index-update", "pass")],
            details: vec![crate::preserves_rail::record("pin-kind", vec![
                crate::preserves_rail::string("chunk"),
            ])],
        });
    crate::blocks::registry::update::index_set_pin(root, "chunk", chunk_ref, true, Some(&receipt_value))?;
    Ok(crate::blocks::ChunkPin {
        kind: "chunk".to_string(),
        reference: chunk_ref.to_string(),
        pinned: true,
        receipt_value,
    })
}

pub fn unpin_chunk(root: &std::path::Path, chunk_ref: &str) -> crate::error::Result<crate::blocks::ChunkPin> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    unpin_chunk_with_root(&root, chunk_ref)
}

pub fn unpin_chunk_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    chunk_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkPin> {
    crate::blocks::layout::ensure_dirs(root)?;
    let pin_path = crate::blocks::layout::chunk_pin_path(chunk_ref)?;
    if root.root().try_exists(&pin_path)? {
        root.root().remove_file(&pin_path)?;
    }
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "unpin",
            decision: "pass",
            manifest_ref: None,
            chunk_refs: &[chunk_ref.to_string()],
            checks: vec![("pin-removal-idempotent", "pass"), ("pin-index-update", "pass")],
            details: vec![crate::preserves_rail::record("pin-kind", vec![
                crate::preserves_rail::string("chunk"),
            ])],
        });
    crate::blocks::registry::update::index_set_pin(root, "chunk", chunk_ref, false, Some(&receipt_value))?;
    Ok(crate::blocks::ChunkPin {
        kind: "chunk".to_string(),
        reference: chunk_ref.to_string(),
        pinned: false,
        receipt_value,
    })
}

pub fn manifest_is_pinned(root: &std::path::Path, manifest_ref: &str) -> crate::error::Result<bool> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    manifest_is_pinned_with_root(&root, manifest_ref)
}

pub fn manifest_is_pinned_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<bool> {
    crate::preserves_rail::validate_content_ref(manifest_ref).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("chunk manifest pin ref is invalid: {error}"))
    })?;
    root.root().try_exists(&crate::blocks::layout::manifest_pin_path(manifest_ref)?)
}

pub fn chunk_is_pinned(root: &std::path::Path, chunk_ref: &str) -> crate::error::Result<bool> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    chunk_is_pinned_with_root(&root, chunk_ref)
}

pub fn chunk_is_pinned_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    chunk_ref: &str,
) -> crate::error::Result<bool> {
    crate::preserves_rail::validate_content_ref(chunk_ref)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("chunk pin ref is invalid: {error}")))?;
    root.root().try_exists(&crate::blocks::layout::chunk_pin_path(chunk_ref)?)
}
