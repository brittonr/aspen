
fn loaded_ticket(
    iroh_root: &CapabilityChunkRoot,
    dest_root: &CapabilityChunkRoot,
    advertised_manifest_ref: &str,
) -> Result<IrohChunkTicket> {
    let ticket_value = match read_iroh_ticket(iroh_root, advertised_manifest_ref) {
        Ok(ticket_value) => ticket_value,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], error.to_string(), vec![
                    ("ticket-availability", "fail"),
                    ("deny-missing-or-invalid-ticket", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    let parsed_ticket = match parse_iroh_ticket_value(&ticket_value) {
        Ok(parsed_ticket) => parsed_ticket,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], error.to_string(), vec![
                    ("ticket-shape", "fail"),
                    ("deny-missing-or-invalid-ticket", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if parsed_ticket.manifest_ref != advertised_manifest_ref {
        let message = format!(
            "Iroh ticket manifest {} does not match advertised manifest {advertised_manifest_ref}",
            parsed_ticket.manifest_ref
        );
        let receipt_value = denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], &message, vec![
            ("ticket-manifest-binding", "fail"),
            ("deny-wrong-manifest", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(parsed_ticket)
}

fn received_manifest(
    iroh_root: &CapabilityChunkRoot,
    dest_root: &CapabilityChunkRoot,
    parsed_ticket: &IrohChunkTicket,
) -> Result<ChunkManifest> {
    let manifest_bytes = match read_iroh_blob(iroh_root, &parsed_ticket.manifest_blob_ref) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(&parsed_ticket.manifest_ref), &[], error.to_string(), vec![
                    ("manifest-blob-availability", "fail"),
                    ("deny-missing-manifest-blob", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if hash_blob_bytes(&manifest_bytes) != parsed_ticket.manifest_blob_ref {
        let message = format!("Iroh manifest blob {} failed blob hash verification", parsed_ticket.manifest_blob_ref);
        let receipt_value = denial_receipt_value("iroh-fetch", Some(&parsed_ticket.manifest_ref), &[], &message, vec![
            ("manifest-blob-verification", "fail"),
            ("deny-corrupt-manifest-blob", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let manifest_value = parse_canonical_bytes(&manifest_bytes)?;
    let manifest_ref = canonical_hash(&manifest_value)?;
    if manifest_ref != parsed_ticket.manifest_ref {
        let message = format!("Iroh manifest blob hashes to {manifest_ref}, expected {}", parsed_ticket.manifest_ref);
        let receipt_value = denial_receipt_value("iroh-fetch", Some(&parsed_ticket.manifest_ref), &[], &message, vec![
            ("manifest-identity-preserved", "fail"),
            ("transport-does-not-grant-trust", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    write_immutable_bytes(
        dest_root,
        &manifest_path(&manifest_ref)?,
        &manifest_bytes,
        &manifest_ref,
        parse_canonical_bytes,
    )?;
    parse_manifest_value(&manifest_value, Some(&manifest_ref))
}

fn ticket_parts(parsed_ticket: &IrohChunkTicket) -> OrderedMap<String, IrohChunkBlob> {
    parsed_ticket.chunks.iter().map(|chunk| (chunk.chunk_ref.clone(), chunk.clone())).collect()
}

fn require_part(
    parts: &OrderedMap<String, IrohChunkBlob>,
    dest_root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    refs: &[String],
    part_ref: &str,
) -> Result<()> {
    if parts.contains_key(part_ref) {
        return Ok(());
    }
    let message = format!("Iroh ticket lacks blob mapping for chunk {part_ref}");
    let receipt_value = denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), refs, &message, vec![
        ("ticket-chunk-map", "fail"),
        ("deny-incomplete-ticket", "pass"),
    ]);
    store_receipt(dest_root, &receipt_value)?;
    Err(MoltenError::invalid_harness(message))
}

fn available_ref(
    dest_root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    refs: &[String],
    part: &ChunkRef,
    part_size: usize,
) -> Result<Option<String>> {
    let dest_chunk_path = chunk_path(&part.chunk_ref)?;
    if !dest_root.root().try_exists(&dest_chunk_path)? {
        return Ok(None);
    }
    match read_verified_chunk(dest_root, part, part_size) {
        Ok(_) => Ok(Some(part.chunk_ref.clone())),
        Err(error) => {
            let receipt_value =
                denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), refs, error.to_string(), vec![
                    ("existing-chunk-verification", "fail"),
                    ("deny-corrupt-dedup-source", "pass"),
                ]);
            store_receipt(dest_root, &receipt_value)?;
            Err(error)
        }
    }
}

fn plan_incoming(
    dest_root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    refs: &[String],
    parts: &OrderedMap<String, IrohChunkBlob>,
    part_size: usize,
) -> Result<Scan> {
    let mut missing_before = Vec::new();
    let mut already_available = Vec::new();
    for part in &manifest.chunks {
        require_part(parts, dest_root, manifest, refs, &part.chunk_ref)?;
        if let Some(part_ref) = available_ref(dest_root, manifest, refs, part, part_size)? {
            push_bounded(
                &mut already_available,
                part_ref,
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store already available chunks",
            )?;
        } else {
            push_bounded(
                &mut missing_before,
                part.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store missing-before chunks",
            )?;
        }
    }
    Ok(Scan {
        missing_before,
        already_available,
    })
}

fn part_for<'a>(manifest: &'a ChunkManifest, part_ref: &str) -> Result<&'a ChunkRef> {
    manifest
        .chunks
        .iter()
        .find(|candidate| candidate.chunk_ref == part_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("manifest missing expected chunk {part_ref}")))
}

fn blob_for<'a>(parts: &'a OrderedMap<String, IrohChunkBlob>, part_ref: &str) -> Result<&'a IrohChunkBlob> {
    parts
        .get(part_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("Iroh ticket lacks blob mapping for chunk {part_ref}")))
}

fn require_blob_len(
    dest_root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    refs: &[String],
    part: &ChunkRef,
    blob: &IrohChunkBlob,
) -> Result<()> {
    if blob.length == part.length {
        return Ok(());
    }
    let message = format!(
        "Iroh ticket length {} for chunk {} does not match manifest length {}",
        blob.length, part.chunk_ref, part.length
    );
    let receipt_value = denial_receipt_value("iroh-fetch", Some(&manifest.manifest_ref), refs, &message, vec![
        ("ticket-chunk-map", "fail"),
        ("deny-incomplete-ticket", "pass"),
    ]);
    store_receipt(dest_root, &receipt_value)?;
    Err(MoltenError::invalid_harness(message))
}

struct BlobInput<'a> {
    iroh_root: &'a CapabilityChunkRoot,
    dest_root: &'a CapabilityChunkRoot,
    manifest: &'a ChunkManifest,
    refs: &'a [String],
    blob: &'a IrohChunkBlob,
}

fn blob_bytes(input: BlobInput<'_>) -> Result<Vec<u8>> {
    let bytes: Vec<u8> = match read_iroh_blob(input.iroh_root, &input.blob.blob_ref) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                Some(&input.manifest.manifest_ref),
                input.refs,
                error.to_string(),
                vec![("chunk-blob-availability", "fail"), ("deny-missing-chunk-blob", "pass")],
            );
            store_receipt(input.dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if hash_blob_bytes(&bytes) == input.blob.blob_ref {
        return Ok(bytes);
    }
    let message = format!("Iroh chunk blob {} failed blob hash verification", input.blob.blob_ref);
    let receipt_value =
        denial_receipt_value("iroh-fetch", Some(&input.manifest.manifest_ref), input.refs, &message, vec![
            ("chunk-blob-verification", "fail"),
            ("deny-corrupt-chunk-blob", "pass"),
        ]);
    store_receipt(input.dest_root, &receipt_value)?;
    Err(MoltenError::invalid_harness(message))
}

struct IncomingInput<'a> {
    iroh_root: &'a CapabilityChunkRoot,
    dest_root: &'a CapabilityChunkRoot,
    manifest: &'a ChunkManifest,
    refs: &'a [String],
    parts: &'a OrderedMap<String, IrohChunkBlob>,
    missing_before: &'a [String],
    part_size: usize,
}

fn copy_incoming(input: IncomingInput<'_>) -> Result<Vec<String>> {
    let mut fetched_chunks = Vec::new();
    for part_ref in input.missing_before {
        let part = part_for(input.manifest, part_ref)?;
        let blob = blob_for(input.parts, part_ref)?;
        require_blob_len(input.dest_root, input.manifest, input.refs, part, blob)?;
        let bytes = blob_bytes(BlobInput {
            iroh_root: input.iroh_root,
            dest_root: input.dest_root,
            manifest: input.manifest,
            refs: input.refs,
            blob,
        })?;
        if let Err(error) = verify_raw_chunk_bytes(&bytes, &part.chunk_ref, part.length, input.part_size) {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                Some(&input.manifest.manifest_ref),
                input.refs,
                error.to_string(),
                vec![
                    ("streaming-chunk-verification", "fail"),
                    ("deny-corrupt-chunk-blob", "pass"),
                ],
            );
            store_receipt(input.dest_root, &receipt_value)?;
            return Err(error);
        }
        input.dest_root.root().write(&chunk_path(&part.chunk_ref)?, &bytes)?;
        push_bounded(
            &mut fetched_chunks,
            part.chunk_ref.clone(),
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store fetched chunks",
        )?;
        index_set_partial_fetch(
            input.dest_root,
            &input.manifest.manifest_ref,
            "in-progress",
            input.missing_before,
            &fetched_chunks,
        )?;
    }
    Ok(fetched_chunks)
}

struct FinishIncoming<'a> {
    dest_root: &'a CapabilityChunkRoot,
    ticket_text: &'a str,
    peer: &'a str,
    manifest: ChunkManifest,
    parsed_ticket: IrohChunkTicket,
    missing_before: Vec<String>,
    fetched_chunks: Vec<String>,
}
