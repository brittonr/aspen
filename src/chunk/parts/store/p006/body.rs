
fn finish_incoming(input: FinishIncoming<'_>) -> Result<ChunkStoreIrohFetch> {
    verify_manifest_with_root(input.dest_root, &input.manifest.manifest_ref)?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "iroh-fetch",
        decision: "pass",
        manifest_ref: Some(&input.manifest.manifest_ref),
        chunk_refs: &input.fetched_chunks,
        checks: vec![
            ("ticket-manifest-binding", "pass"),
            ("manifest-identity-preserved", "pass"),
            ("missing-chunk-calculation", "pass"),
            ("resumable-fetch", "pass"),
            ("streaming-chunk-verification", "pass"),
            ("transport-does-not-grant-trust", "pass"),
        ],
        details: vec![
            record("peer", vec![string(input.peer)]),
            record("ticket", vec![string(input.ticket_text)]),
            record("manifest-blob-ref", vec![string(&input.parsed_ticket.manifest_blob_ref)]),
            record("missing-before", vec![sequence(input.missing_before.iter().map(string).collect())]),
            record("fetched", vec![sequence(input.fetched_chunks.iter().map(string).collect())]),
        ],
    });
    let available_after = manifest_refs(&input.manifest);
    index_set_manifest_chunk_availability(
        input.dest_root,
        &input.manifest,
        &available_after,
        &[],
        Some(&receipt_value),
    )?;
    index_set_partial_fetch(
        input.dest_root,
        &input.manifest.manifest_ref,
        "complete",
        &input.missing_before,
        &input.fetched_chunks,
    )?;
    Ok(ChunkStoreIrohFetch {
        ticket: input.ticket_text.to_string(),
        manifest_ref: input.manifest.manifest_ref,
        manifest_blob_ref: input.parsed_ticket.manifest_blob_ref,
        missing_before: input.missing_before,
        fetched_chunks: input.fetched_chunks,
        receipt_value,
    })
}

pub fn publish_iroh_blobs(
    store_root: &Path,
    iroh_root: &Path,
    manifest_ref: &str,
    node: &str,
) -> Result<ChunkStoreIrohPublish> {
    let store_root = open_capability_chunk_root(store_root)?;
    let iroh_root = open_capability_chunk_root(iroh_root)?;
    publish_iroh_blobs_with_roots(&store_root, &iroh_root, manifest_ref, node)
}

pub fn publish_iroh_blobs_with_roots(
    store_root: &CapabilityChunkRoot,
    iroh_root: &CapabilityChunkRoot,
    manifest_ref: &str,
    node: &str,
) -> Result<ChunkStoreIrohPublish> {
    ensure_dirs(store_root)?;
    ensure_iroh_dirs(iroh_root)?;
    let manifest = match read_manifest_with_root(store_root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("iroh-publish", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(store_root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("iroh-publish", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(store_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }

    let manifest_blob_ref = write_head(HeadInput {
        store_root,
        iroh_root,
        manifest: &manifest,
        chunk_refs: &chunk_refs,
    })?;

    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let chunk_blobs = write_parts(PartsInput {
        store_root,
        iroh_root,
        manifest: &manifest,
        chunk_refs: &chunk_refs,
        chunk_size,
    })?;

    finish_pass(FinishInput {
        store_root,
        iroh_root,
        node,
        manifest,
        manifest_blob_ref,
        chunk_blobs,
        chunk_refs: &chunk_refs,
    })
}

pub fn fetch_iroh_blobs(
    iroh_root: &Path,
    dest_root: &Path,
    ticket: &str,
    expected_manifest_ref: Option<&str>,
    peer: &str,
) -> Result<ChunkStoreIrohFetch> {
    let iroh_root = open_capability_chunk_root(iroh_root)?;
    let dest_root = open_capability_chunk_root(dest_root)?;
    fetch_iroh_blobs_with_roots(&iroh_root, &dest_root, ticket, expected_manifest_ref, peer)
}

pub fn fetch_iroh_blobs_with_roots(
    iroh_root: &CapabilityChunkRoot,
    dest_root: &CapabilityChunkRoot,
    ticket: &str,
    expected_manifest_ref: Option<&str>,
    peer: &str,
) -> Result<ChunkStoreIrohFetch> {
    ensure_dirs(dest_root)?;
    ensure_iroh_dirs(iroh_root)?;
    let advertised_manifest_ref = claim_manifest(dest_root, ticket, expected_manifest_ref)?;
    let parsed_ticket = loaded_ticket(iroh_root, dest_root, &advertised_manifest_ref)?;
    let manifest = received_manifest(iroh_root, dest_root, &parsed_ticket)?;
    let chunk_refs = manifest_refs(&manifest);
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }

    let ticket_chunks = ticket_parts(&parsed_ticket);
    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let scan = plan_incoming(dest_root, &manifest, &chunk_refs, &ticket_chunks, chunk_size)?;
    index_set_partial_fetch(dest_root, &manifest.manifest_ref, "in-progress", &scan.missing_before, &[])?;
    index_set_manifest_chunk_availability(dest_root, &manifest, &scan.already_available, &scan.missing_before, None)?;

    let fetched_chunks = copy_incoming(IncomingInput {
        iroh_root,
        dest_root,
        manifest: &manifest,
        refs: &chunk_refs,
        parts: &ticket_chunks,
        missing_before: &scan.missing_before,
        part_size: chunk_size,
    })?;

    finish_incoming(FinishIncoming {
        dest_root,
        ticket_text: ticket,
        peer,
        manifest,
        parsed_ticket,
        missing_before: scan.missing_before,
        fetched_chunks,
    })
}

pub fn pin_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkStorePin> {
    let root = open_capability_chunk_root(root)?;
    pin_manifest_with_root(&root, manifest_ref)
}

pub fn pin_manifest_with_root(root: &CapabilityChunkRoot, manifest_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let manifest = read_manifest_with_root(root, manifest_ref)?;
    root.root().write(&manifest_pin_path(manifest_ref)?, manifest_ref.as_bytes())?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "pin",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: &chunk_refs,
        checks: vec![("manifest-exists", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("manifest")])],
    });
    index_set_pin(root, "manifest", manifest_ref, true, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "manifest".to_string(),
        reference: manifest_ref.to_string(),
        pinned: true,
        receipt_value,
    })
}

pub fn unpin_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkStorePin> {
    let root = open_capability_chunk_root(root)?;
    unpin_manifest_with_root(&root, manifest_ref)
}

pub fn unpin_manifest_with_root(root: &CapabilityChunkRoot, manifest_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let pin_path = manifest_pin_path(manifest_ref)?;
    if root.root().try_exists(&pin_path)? {
        root.root().remove_file(&pin_path)?;
    }
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "unpin",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: &[],
        checks: vec![("pin-removal-idempotent", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("manifest")])],
    });
    index_set_pin(root, "manifest", manifest_ref, false, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "manifest".to_string(),
        reference: manifest_ref.to_string(),
        pinned: false,
        receipt_value,
    })
}

pub fn pin_chunk(root: &Path, chunk_ref: &str) -> Result<ChunkStorePin> {
    let root = open_capability_chunk_root(root)?;
    pin_chunk_with_root(&root, chunk_ref)
}

pub fn pin_chunk_with_root(root: &CapabilityChunkRoot, chunk_ref: &str) -> Result<ChunkStorePin> {
    ensure_dirs(root)?;
    let path = chunk_path(chunk_ref)?;
    if !root.root().try_exists(&path)? {
        let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
            operation: "pin",
            decision: "deny",
            manifest_ref: None,
            chunk_refs: &[chunk_ref.to_string()],
            checks: vec![("chunk-exists", "fail"), ("deny-missing-chunk-pin", "pass")],
            details: vec![record("pin-kind", vec![string("chunk")])],
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(format!("cannot pin missing chunk {chunk_ref}")));
    }
    root.root().write(&chunk_pin_path(chunk_ref)?, chunk_ref.as_bytes())?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "pin",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: &[chunk_ref.to_string()],
        checks: vec![("chunk-exists", "pass"), ("pin-index-update", "pass")],
        details: vec![record("pin-kind", vec![string("chunk")])],
    });
    index_set_pin(root, "chunk", chunk_ref, true, Some(&receipt_value))?;
    Ok(ChunkStorePin {
        kind: "chunk".to_string(),
        reference: chunk_ref.to_string(),
        pinned: true,
        receipt_value,
    })
}

pub fn unpin_chunk(root: &Path, chunk_ref: &str) -> Result<ChunkStorePin> {
    let root = open_capability_chunk_root(root)?;
    unpin_chunk_with_root(&root, chunk_ref)
}

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
