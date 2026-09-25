
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
