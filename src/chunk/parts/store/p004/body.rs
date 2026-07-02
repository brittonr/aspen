
pub fn sync_missing_chunks(source_root: &Path, dest_root: &Path, manifest_ref: &str) -> Result<ChunkStoreSync> {
    ensure_dirs(dest_root)?;
    let manifest = read_manifest(source_root, manifest_ref)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("remote-sync", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let manifest_bytes = fs::read(manifest_path(source_root, manifest_ref)?).map_err(MoltenError::from)?;
    write_immutable_bytes(
        &manifest_path(dest_root, manifest_ref)?,
        &manifest_bytes,
        manifest_ref,
        parse_canonical_bytes,
    )?;

    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let scan = scan_refs(dest_root, &manifest, chunk_size)?;
    index_set_partial_fetch(dest_root, &manifest.manifest_ref, "in-progress", &scan.missing_before, &[])?;
    index_set_manifest_chunk_availability(dest_root, &manifest, &scan.already_available, &scan.missing_before, None)?;

    let fetched_chunks = copy_refs(CopyInput {
        source_root,
        dest_root,
        manifest: &manifest,
        chunk_size,
        missing_before: &scan.missing_before,
    })?;
    verify_manifest(dest_root, manifest_ref)?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "remote-sync",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &fetched_chunks,
        checks: vec![
            ("manifest-identity-preserved", "pass"),
            ("missing-chunk-calculation", "pass"),
            ("redb-partial-fetch-state", "pass"),
            ("resumable-fetch", "pass"),
            ("streaming-chunk-verification", "pass"),
        ],
        details: vec![
            record("missing-before", vec![sequence(scan.missing_before.iter().map(string).collect())]),
            record("fetched", vec![sequence(fetched_chunks.iter().map(string).collect())]),
        ],
    });
    let available_after = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    index_set_manifest_chunk_availability(dest_root, &manifest, &available_after, &[], Some(&receipt_value))?;
    index_set_partial_fetch(dest_root, &manifest.manifest_ref, "complete", &scan.missing_before, &fetched_chunks)?;
    Ok(ChunkStoreSync {
        manifest_ref: manifest.manifest_ref,
        missing_before: scan.missing_before,
        fetched_chunks,
        receipt_value,
    })
}

struct Scan {
    missing_before: Vec<String>,
    already_available: Vec<String>,
}

fn scan_refs(dest_root: &Path, manifest: &ChunkManifest, chunk_size: usize) -> Result<Scan> {
    let mut missing_before = Vec::new();
    let mut already_available = Vec::new();
    for chunk in &manifest.chunks {
        let dest_chunk_path = chunk_path(dest_root, &chunk.chunk_ref)?;
        if dest_chunk_path.exists() {
            read_verified_chunk(dest_root, chunk, chunk_size)?;
            push_bounded(
                &mut already_available,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store already available chunks",
            )?;
        } else {
            push_bounded(
                &mut missing_before,
                chunk.chunk_ref.clone(),
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

struct CopyInput<'a> {
    source_root: &'a Path,
    dest_root: &'a Path,
    manifest: &'a ChunkManifest,
    chunk_size: usize,
    missing_before: &'a [String],
}

fn copy_refs(input: CopyInput<'_>) -> Result<Vec<String>> {
    let mut fetched_chunks = Vec::new();
    for chunk_ref in input.missing_before {
        let chunk = input
            .manifest
            .chunks
            .iter()
            .find(|candidate| &candidate.chunk_ref == chunk_ref)
            .ok_or_else(|| MoltenError::invalid_harness(format!("manifest missing expected chunk {chunk_ref}")))?;
        let bytes = read_verified_chunk(input.source_root, chunk, input.chunk_size)?;
        fs::write(chunk_path(input.dest_root, &chunk.chunk_ref)?, bytes).map_err(MoltenError::from)?;
        push_bounded(
            &mut fetched_chunks,
            chunk.chunk_ref.clone(),
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

struct HeadInput<'a> {
    store_root: &'a Path,
    iroh_root: &'a Path,
    manifest: &'a ChunkManifest,
    chunk_refs: &'a [String],
}

fn write_head(input: HeadInput<'_>) -> Result<String> {
    let manifest_bytes =
        fs::read(manifest_path(input.store_root, &input.manifest.manifest_ref)?).map_err(MoltenError::from)?;
    let manifest_blob_ref = hash_blob_bytes(&manifest_bytes);
    if manifest_blob_ref != input.manifest.manifest_ref {
        let message = format!(
            "Iroh manifest blob ref {manifest_blob_ref} does not preserve manifest identity {}",
            input.manifest.manifest_ref
        );
        let receipt_value =
            denial_receipt_value("iroh-publish", Some(&input.manifest.manifest_ref), input.chunk_refs, &message, vec![
                ("manifest-identity-preserved", "fail"),
                ("transport-does-not-grant-trust", "pass"),
            ]);
        store_receipt(input.store_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    write_immutable_blob(&iroh_blob_path(input.iroh_root, &manifest_blob_ref)?, &manifest_bytes, &manifest_blob_ref)?;
    Ok(manifest_blob_ref)
}

struct PartsInput<'a> {
    store_root: &'a Path,
    iroh_root: &'a Path,
    manifest: &'a ChunkManifest,
    chunk_refs: &'a [String],
    chunk_size: usize,
}

fn write_parts(input: PartsInput<'_>) -> Result<Vec<IrohChunkBlob>> {
    let mut chunk_blobs = Vec::new();
    for chunk in &input.manifest.chunks {
        let bytes = match read_verified_chunk(input.store_root, chunk, input.chunk_size) {
            Ok(bytes) => bytes,
            Err(error) => {
                let receipt_value = denial_receipt_value(
                    "iroh-publish",
                    Some(&input.manifest.manifest_ref),
                    input.chunk_refs,
                    error.to_string(),
                    vec![
                        ("streaming-chunk-verification", "fail"),
                        ("deny-corrupt-or-missing-chunk", "pass"),
                    ],
                );
                store_receipt(input.store_root, &receipt_value)?;
                return Err(error);
            }
        };
        let blob_ref = hash_blob_bytes(&bytes);
        write_immutable_blob(&iroh_blob_path(input.iroh_root, &blob_ref)?, &bytes, &blob_ref)?;
        push_bounded(
            &mut chunk_blobs,
            IrohChunkBlob {
                chunk_ref: chunk.chunk_ref.clone(),
                blob_ref,
                length: chunk.length,
            },
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh chunk blobs",
        )?;
    }
    Ok(chunk_blobs)
}

struct FinishInput<'a> {
    store_root: &'a Path,
    iroh_root: &'a Path,
    node: &'a str,
    manifest: ChunkManifest,
    manifest_blob_ref: String,
    chunk_blobs: Vec<IrohChunkBlob>,
    chunk_refs: &'a [String],
}

fn finish_pass(input: FinishInput<'_>) -> Result<ChunkStoreIrohPublish> {
    let ticket_value = iroh_ticket_value(&input.manifest.manifest_ref, &input.manifest_blob_ref, &input.chunk_blobs);
    let ticket_ref = canonical_hash(&ticket_value)?;
    write_immutable_bytes(
        &iroh_ticket_path(input.iroh_root, &input.manifest.manifest_ref)?,
        &canonical_bytes(&ticket_value)?,
        &ticket_ref,
        parse_canonical_bytes,
    )?;
    let ticket = format!("iroh-local-chunk:{}", input.manifest.manifest_ref);
    let chunk_blob_refs = input.chunk_blobs.iter().map(|chunk| chunk.blob_ref.clone()).collect::<Vec<_>>();
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "iroh-publish",
        decision: "pass",
        manifest_ref: Some(&input.manifest.manifest_ref),
        chunk_refs: input.chunk_refs,
        checks: vec![
            ("manifest-identity-preserved", "pass"),
            ("chunk-blob-verification", "pass"),
            ("iroh-location-hints-only", "pass"),
            ("transport-does-not-grant-trust", "pass"),
        ],
        details: vec![
            record("node", vec![string(input.node)]),
            record("ticket", vec![string(&ticket)]),
            record("ticket-ref", vec![string(&ticket_ref)]),
            record("manifest-blob-ref", vec![string(&input.manifest_blob_ref)]),
            record("chunk-blob-refs", vec![sequence(chunk_blob_refs.iter().map(string).collect())]),
        ],
    });
    store_receipt(input.store_root, &receipt_value)?;
    Ok(ChunkStoreIrohPublish {
        ticket,
        manifest_ref: input.manifest.manifest_ref,
        manifest_blob_ref: input.manifest_blob_ref,
        chunk_blob_refs,
        receipt_value,
    })
}

fn manifest_refs(manifest: &ChunkManifest) -> Vec<String> {
    manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect()
}

fn claim_manifest(dest_root: &Path, ticket: &str, expected_manifest_ref: Option<&str>) -> Result<String> {
    let advertised_manifest_ref = match ticket.strip_prefix("iroh-local-chunk:") {
        Some(manifest_ref) => manifest_ref,
        None => {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                None,
                &[],
                "unsupported Iroh chunk ticket; expected iroh-local-chunk:<manifest-ref>",
                vec![("ticket-shape", "fail"), ("deny-unsupported-ticket", "pass")],
            );
            store_receipt(dest_root, &receipt_value)?;
            return Err(MoltenError::invalid_harness(
                "unsupported Iroh chunk ticket; expected iroh-local-chunk:<manifest-ref>",
            ));
        }
    };
    if let Some(expected) = expected_manifest_ref
        && expected != advertised_manifest_ref
    {
        let message = format!("Iroh chunk ticket advertises manifest {advertised_manifest_ref}, expected {expected}");
        let receipt_value = denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], &message, vec![
            ("ticket-manifest-binding", "fail"),
            ("deny-wrong-manifest", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(advertised_manifest_ref.to_string())
}
