
pub fn unpin_chunk_with_root(root: &CapabilityChunkRoot, chunk_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let pin_path = chunk_pin_path(chunk_ref)?;
    if root.root().try_exists(&pin_path)? {
        root.root().remove_file(&pin_path)?;
    }
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "unpin",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: &[chunk_ref.to_string()],
        checks: vec![("pin-removal-idempotent", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("chunk")])],
    });
    index_set_pin(root, "chunk", chunk_ref, false, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "chunk".to_string(),
        reference: chunk_ref.to_string(),
        pinned: false,
        receipt_value,
    })
}

pub fn manifest_is_pinned(root: &Path, manifest_ref: &str) -> Result<bool> {
    let root = open_capability_chunk_root(root)?;
    manifest_is_pinned_with_root(&root, manifest_ref)
}

pub fn manifest_is_pinned_with_root(root: &CapabilityChunkRoot, manifest_ref: &str) -> Result<bool> {
    validate_content_ref(manifest_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("chunk manifest pin ref is invalid: {error}")))?;
    root.root().try_exists(&manifest_pin_path(manifest_ref)?)
}

pub fn chunk_is_pinned(root: &Path, chunk_ref: &str) -> Result<bool> {
    let root = open_capability_chunk_root(root)?;
    chunk_is_pinned_with_root(&root, chunk_ref)
}

pub fn chunk_is_pinned_with_root(root: &CapabilityChunkRoot, chunk_ref: &str) -> Result<bool> {
    validate_content_ref(chunk_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("chunk pin ref is invalid: {error}")))?;
    root.root().try_exists(&chunk_pin_path(chunk_ref)?)
}

fn pass_or_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

struct ApplyRefMatchInput<'a> {
    root: &'a crate::local_store::RetentionStoreRoot,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}
