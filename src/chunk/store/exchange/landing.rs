//! Iroh fetch completion: verified chunk blob copies into the destination root, index updates, and
//! the receipt.

struct BlobInput<'a> {
    iroh_root: &'a crate::blocks::CapabilityChunkRoot,
    dest_root: &'a crate::blocks::CapabilityChunkRoot,
    manifest: &'a crate::blocks::ChunkManifest,
    refs: &'a [String],
    blob: &'a crate::blocks::exchange::token::IrohChunkBlob,
}

fn blob_bytes(input: BlobInput<'_>) -> crate::error::Result<Vec<u8>> {
    let bytes: Vec<u8> = match crate::blocks::exchange::token::read_iroh_blob(input.iroh_root, &input.blob.blob_ref) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
                "iroh-fetch",
                Some(&input.manifest.manifest_ref),
                input.refs,
                error.to_string(),
                vec![("chunk-blob-availability", "fail"), ("deny-missing-chunk-blob", "pass")],
            );
            crate::blocks::registry::receipt(input.dest_root, &receipt_value)?;
            return Err(error);
        }
    };
    if crate::blocks::integrity::hash_blob_bytes(&bytes) == input.blob.blob_ref {
        return Ok(bytes);
    }
    let message = format!("Iroh chunk blob {} failed blob hash verification", input.blob.blob_ref);
    let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
        "iroh-fetch",
        Some(&input.manifest.manifest_ref),
        input.refs,
        &message,
        vec![("chunk-blob-verification", "fail"), ("deny-corrupt-chunk-blob", "pass")],
    );
    crate::blocks::registry::receipt(input.dest_root, &receipt_value)?;
    Err(crate::error::Failure::invalid_harness(message))
}

struct IncomingInput<'a> {
    iroh_root: &'a crate::blocks::CapabilityChunkRoot,
    dest_root: &'a crate::blocks::CapabilityChunkRoot,
    manifest: &'a crate::blocks::ChunkManifest,
    refs: &'a [String],
    parts: &'a std::collections::BTreeMap<String, crate::blocks::exchange::token::IrohChunkBlob>,
    missing_before: &'a [String],
    part_size: usize,
}

fn copy_incoming(input: IncomingInput<'_>) -> crate::error::Result<Vec<String>> {
    let mut fetched_chunks = Vec::new();
    for part_ref in input.missing_before {
        let part = crate::blocks::exchange::intake::part_for(input.manifest, part_ref)?;
        let blob = crate::blocks::exchange::intake::blob_for(input.parts, part_ref)?;
        crate::blocks::exchange::intake::require_blob_len(input.dest_root, input.manifest, input.refs, part, blob)?;
        let bytes = blob_bytes(BlobInput {
            iroh_root: input.iroh_root,
            dest_root: input.dest_root,
            manifest: input.manifest,
            refs: input.refs,
            blob,
        })?;
        if let Err(error) =
            crate::blocks::integrity::verify_raw_chunk_bytes(&bytes, &part.chunk_ref, part.length, input.part_size)
        {
            let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
                "iroh-fetch",
                Some(&input.manifest.manifest_ref),
                input.refs,
                error.to_string(),
                vec![
                    ("streaming-chunk-verification", "fail"),
                    ("deny-corrupt-chunk-blob", "pass"),
                ],
            );
            crate::blocks::registry::receipt(input.dest_root, &receipt_value)?;
            return Err(error);
        }
        input.dest_root.root().write(&crate::blocks::layout::chunk_path(&part.chunk_ref)?, &bytes)?;
        crate::bounded::push(
            &mut fetched_chunks,
            part.chunk_ref.clone(),
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

struct FinishIncoming<'a> {
    dest_root: &'a crate::blocks::CapabilityChunkRoot,
    ticket_text: &'a str,
    peer: &'a str,
    manifest: crate::blocks::ChunkManifest,
    parsed_ticket: crate::blocks::exchange::token::IrohChunkTicket,
    missing_before: Vec<String>,
    fetched_chunks: Vec<String>,
}

fn finish_incoming(input: FinishIncoming<'_>) -> crate::error::Result<crate::blocks::ChunkIrohFetch> {
    crate::blocks::verify_manifest_with_root(input.dest_root, &input.manifest.manifest_ref)?;
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
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
                crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer)]),
                crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(input.ticket_text)]),
                crate::preserves_rail::record("manifest-blob-ref", vec![crate::preserves_rail::string(
                    &input.parsed_ticket.manifest_blob_ref,
                )]),
                crate::preserves_rail::record("missing-before", vec![crate::preserves_rail::sequence(
                    input.missing_before.iter().map(crate::preserves_rail::string).collect(),
                )]),
                crate::preserves_rail::record("fetched", vec![crate::preserves_rail::sequence(
                    input.fetched_chunks.iter().map(crate::preserves_rail::string).collect(),
                )]),
            ],
        });
    let available_after = crate::blocks::exchange::announce::manifest_refs(&input.manifest);
    crate::blocks::registry::update::index_set_manifest_chunk_availability(
        input.dest_root,
        &input.manifest,
        &available_after,
        &[],
        Some(&receipt_value),
    )?;
    crate::blocks::registry::update::index_set_partial_fetch(
        input.dest_root,
        &input.manifest.manifest_ref,
        "complete",
        &input.missing_before,
        &input.fetched_chunks,
    )?;
    Ok(crate::blocks::ChunkIrohFetch {
        ticket: input.ticket_text.to_string(),
        manifest_ref: input.manifest.manifest_ref,
        manifest_blob_ref: input.parsed_ticket.manifest_blob_ref,
        missing_before: input.missing_before,
        fetched_chunks: input.fetched_chunks,
        receipt_value,
    })
}

pub fn fetch_iroh_blobs(
    iroh_root: &std::path::Path,
    dest_root: &std::path::Path,
    ticket: &str,
    expected_manifest_ref: Option<&str>,
    peer: &str,
) -> crate::error::Result<crate::blocks::ChunkIrohFetch> {
    let iroh_root = crate::blocks::open_capability_chunk_root(iroh_root)?;
    let dest_root = crate::blocks::open_capability_chunk_root(dest_root)?;
    fetch_iroh_blobs_with_roots(&iroh_root, &dest_root, ticket, expected_manifest_ref, peer)
}

pub fn fetch_iroh_blobs_with_roots(
    iroh_root: &crate::blocks::CapabilityChunkRoot,
    dest_root: &crate::blocks::CapabilityChunkRoot,
    ticket: &str,
    expected_manifest_ref: Option<&str>,
    peer: &str,
) -> crate::error::Result<crate::blocks::ChunkIrohFetch> {
    crate::blocks::layout::ensure_dirs(dest_root)?;
    crate::blocks::layout::ensure_iroh_dirs(iroh_root)?;
    let advertised_manifest_ref =
        crate::blocks::exchange::intake::claim_manifest(dest_root, ticket, expected_manifest_ref)?;
    let parsed_ticket = crate::blocks::exchange::intake::loaded_ticket(iroh_root, dest_root, &advertised_manifest_ref)?;
    let manifest = crate::blocks::exchange::intake::received_manifest(iroh_root, dest_root, &parsed_ticket)?;
    let chunk_refs = crate::blocks::exchange::announce::manifest_refs(&manifest);
    crate::blocks::registry::deny_unsupported_transform(dest_root, "iroh-fetch", &manifest, &chunk_refs)?;

    let ticket_chunks = crate::blocks::exchange::intake::ticket_parts(&parsed_ticket);
    let chunk_size = crate::blocks::limits::chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    let scan =
        crate::blocks::exchange::intake::plan_incoming(dest_root, &manifest, &chunk_refs, &ticket_chunks, chunk_size)?;
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
