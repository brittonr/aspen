//! Iroh publish: copies the verified manifest and chunk blobs into the Iroh root and writes the
//! local ticket.

struct HeadInput<'a> {
    store_root: &'a crate::blocks::CapabilityChunkRoot,
    iroh_root: &'a crate::blocks::CapabilityChunkRoot,
    manifest: &'a crate::blocks::ChunkManifest,
    chunk_refs: &'a [String],
}

fn write_head(input: HeadInput<'_>) -> crate::error::Result<String> {
    let manifest_bytes =
        input.store_root.root().read(&crate::blocks::layout::manifest_path(&input.manifest.manifest_ref)?)?;
    let manifest_blob_ref = crate::blocks::integrity::hash_blob_bytes(&manifest_bytes);
    if manifest_blob_ref != input.manifest.manifest_ref {
        let message = format!(
            "Iroh manifest blob ref {manifest_blob_ref} does not preserve manifest identity {}",
            input.manifest.manifest_ref
        );
        let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
            "iroh-publish",
            Some(&input.manifest.manifest_ref),
            input.chunk_refs,
            &message,
            vec![
                ("manifest-identity-preserved", "fail"),
                ("transport-does-not-grant-trust", "pass"),
            ],
        );
        crate::blocks::registry::receipt(input.store_root, &receipt_value)?;
        return Err(crate::error::Failure::invalid_harness(message));
    }
    crate::blocks::layout::write_immutable_blob(
        input.iroh_root,
        &crate::blocks::layout::iroh_blob_path(&manifest_blob_ref)?,
        &manifest_bytes,
        &manifest_blob_ref,
    )?;
    Ok(manifest_blob_ref)
}

struct PartsInput<'a> {
    store_root: &'a crate::blocks::CapabilityChunkRoot,
    iroh_root: &'a crate::blocks::CapabilityChunkRoot,
    manifest: &'a crate::blocks::ChunkManifest,
    chunk_refs: &'a [String],
    chunk_size: usize,
}

fn write_parts(input: PartsInput<'_>) -> crate::error::Result<Vec<crate::blocks::exchange::token::IrohChunkBlob>> {
    let mut chunk_blobs = Vec::new();
    for chunk in &input.manifest.chunks {
        let bytes = match crate::blocks::read_verified_chunk(input.store_root, chunk, input.chunk_size) {
            Ok(bytes) => bytes,
            Err(error) => {
                let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
                    "iroh-publish",
                    Some(&input.manifest.manifest_ref),
                    input.chunk_refs,
                    error.to_string(),
                    vec![
                        ("streaming-chunk-verification", "fail"),
                        ("deny-corrupt-or-missing-chunk", "pass"),
                    ],
                );
                crate::blocks::registry::receipt(input.store_root, &receipt_value)?;
                return Err(error);
            }
        };
        let blob_ref = crate::blocks::integrity::hash_blob_bytes(&bytes);
        crate::blocks::layout::write_immutable_blob(
            input.iroh_root,
            &crate::blocks::layout::iroh_blob_path(&blob_ref)?,
            &bytes,
            &blob_ref,
        )?;
        crate::bounded::push(
            &mut chunk_blobs,
            crate::blocks::exchange::token::IrohChunkBlob {
                chunk_ref: chunk.chunk_ref.clone(),
                blob_ref,
                length: chunk.length,
            },
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh chunk blobs",
        )?;
    }
    Ok(chunk_blobs)
}

struct FinishInput<'a> {
    store_root: &'a crate::blocks::CapabilityChunkRoot,
    iroh_root: &'a crate::blocks::CapabilityChunkRoot,
    node: &'a str,
    manifest: crate::blocks::ChunkManifest,
    manifest_blob_ref: String,
    chunk_blobs: Vec<crate::blocks::exchange::token::IrohChunkBlob>,
    chunk_refs: &'a [String],
}

fn finish_pass(input: FinishInput<'_>) -> crate::error::Result<crate::blocks::ChunkIrohPublish> {
    let ticket_value = crate::blocks::exchange::token::iroh_ticket_value(
        &input.manifest.manifest_ref,
        &input.manifest_blob_ref,
        &input.chunk_blobs,
    );
    let ticket_ref = crate::preserves_rail::canonical_hash(&ticket_value)?;
    crate::blocks::layout::write_immutable_bytes(
        input.iroh_root,
        &crate::blocks::layout::iroh_ticket_path(&input.manifest.manifest_ref)?,
        &crate::preserves_rail::canonical_bytes(&ticket_value)?,
        &ticket_ref,
        crate::preserves_rail::parse_canonical_bytes,
    )?;
    let ticket = format!("iroh-local-chunk:{}", input.manifest.manifest_ref);
    let chunk_blob_refs = input.chunk_blobs.iter().map(|chunk| chunk.blob_ref.clone()).collect::<Vec<_>>();
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
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
                crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node)]),
                crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&ticket)]),
                crate::preserves_rail::record("ticket-ref", vec![crate::preserves_rail::string(&ticket_ref)]),
                crate::preserves_rail::record("manifest-blob-ref", vec![crate::preserves_rail::string(
                    &input.manifest_blob_ref,
                )]),
                crate::preserves_rail::record("chunk-blob-refs", vec![crate::preserves_rail::sequence(
                    chunk_blob_refs.iter().map(crate::preserves_rail::string).collect(),
                )]),
            ],
        });
    crate::blocks::registry::receipt(input.store_root, &receipt_value)?;
    Ok(crate::blocks::ChunkIrohPublish {
        ticket,
        manifest_ref: input.manifest.manifest_ref,
        manifest_blob_ref: input.manifest_blob_ref,
        chunk_blob_refs,
        receipt_value,
    })
}

pub(super) fn manifest_refs(manifest: &crate::blocks::ChunkManifest) -> Vec<String> {
    manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect()
}

pub fn publish_iroh_blobs(
    store_root: &std::path::Path,
    iroh_root: &std::path::Path,
    manifest_ref: &str,
    node: &str,
) -> crate::error::Result<crate::blocks::ChunkIrohPublish> {
    let store_root = crate::blocks::open_capability_chunk_root(store_root)?;
    let iroh_root = crate::blocks::open_capability_chunk_root(iroh_root)?;
    publish_iroh_blobs_with_roots(&store_root, &iroh_root, manifest_ref, node)
}

pub fn publish_iroh_blobs_with_roots(
    store_root: &crate::blocks::CapabilityChunkRoot,
    iroh_root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
    node: &str,
) -> crate::error::Result<crate::blocks::ChunkIrohPublish> {
    crate::blocks::layout::ensure_dirs(store_root)?;
    crate::blocks::layout::ensure_iroh_dirs(iroh_root)?;
    let manifest = crate::blocks::local::readout::read_admitted_manifest(store_root, "iroh-publish", manifest_ref)?;
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    crate::blocks::registry::deny_unsupported_transform(store_root, "iroh-publish", &manifest, &chunk_refs)?;

    let manifest_blob_ref = write_head(HeadInput {
        store_root,
        iroh_root,
        manifest: &manifest,
        chunk_refs: &chunk_refs,
    })?;

    let chunk_size = crate::blocks::limits::chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
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
