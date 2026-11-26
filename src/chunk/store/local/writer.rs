//! Chunk writes with dedup verification, the manifest record, and the put receipts.

pub(super) struct Written {
    values: Vec<preserves::IOValue>,
    refs: Vec<crate::blocks::ChunkRef>,
    dedup_refs: Vec<String>,
    dedup_hits: usize,
}

pub(super) struct FinalizeInput<'a> {
    pub(super) root: &'a crate::blocks::CapabilityChunkRoot,
    pub(super) object_kind: &'a str,
    pub(super) total_len: u64,
    pub(super) chunk_size_input: u64,
    pub(super) chunk_size: usize,
    pub(super) transforms: &'a crate::blocks::ChunkTransforms,
    pub(super) metadata_ref: &'a str,
    pub(super) policy_refs: &'a [String],
    pub(super) written: Written,
}

fn ensure_part(
    root: &crate::blocks::CapabilityChunkRoot,
    chunk: &[u8],
    chunk_size: usize,
) -> crate::error::Result<(String, bool)> {
    let chunk_ref = crate::blocks::hash_chunk(chunk, chunk_size);
    let path = crate::blocks::layout::chunk_path(&chunk_ref)?;
    if root.root().try_exists(&path)? {
        if let Err(error) =
            crate::blocks::integrity::verify_raw_chunk_file(root, &path, &chunk_ref, chunk.len() as u64, chunk_size)
        {
            let receipt_value = crate::blocks::wire::outcome::denial_receipt_value(
                "dedup-hit",
                None,
                std::slice::from_ref(&chunk_ref),
                error.to_string(),
                vec![
                    ("existing-chunk-hash-binding", "fail"),
                    ("deny-corrupt-dedup-source", "pass"),
                ],
            );
            crate::blocks::registry::receipt(root, &receipt_value)?;
            return Err(error);
        }
        Ok((chunk_ref, true))
    } else {
        root.root().write(&path, chunk)?;
        Ok((chunk_ref, false))
    }
}

pub(super) fn write_put_parts(
    root: &crate::blocks::CapabilityChunkRoot,
    bytes: &[u8],
    chunk_size: usize,
    transforms: &crate::blocks::ChunkTransforms,
) -> crate::error::Result<Written> {
    let mut values = Vec::new();
    let mut refs = Vec::new();
    let mut dedup_refs = Vec::new();
    let mut dedup_hits = 0usize;
    for chunk in bytes.chunks(chunk_size) {
        let (chunk_ref, was_present) = ensure_part(root, chunk, chunk_size)?;
        if was_present {
            dedup_hits += 1;
            crate::bounded::push(
                &mut dedup_refs,
                chunk_ref.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
                "chunk store dedup chunks",
            )?;
        }
        crate::bounded::push(
            &mut refs,
            crate::blocks::ChunkRef {
                chunk_ref: chunk_ref.clone(),
                length: chunk.len() as u64,
                domain: crate::blocks::integrity::chunk_domain(chunk_size),
                chunker: crate::blocks::FIXED_V1_CHUNKER.to_string(),
                transforms: transforms.clone(),
            },
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store chunk refs",
        )?;
        crate::bounded::push(
            &mut values,
            crate::blocks::wire::encode::chunk_ref_value(&chunk_ref, chunk.len() as u64, chunk_size, transforms),
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store chunk values",
        )?;
    }
    Ok(Written {
        values,
        refs,
        dedup_refs,
        dedup_hits,
    })
}

fn note_reuse(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
    refs: &[String],
) -> crate::error::Result<()> {
    if refs.is_empty() {
        return Ok(());
    }
    let dedup_receipt =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "dedup-hit",
            decision: "pass",
            manifest_ref: Some(manifest_ref),
            chunk_refs: refs,
            checks: vec![
                ("existing-chunk-hash-binding", "pass"),
                ("dedup-no-rewrite", "pass"),
                ("receipt-index-update", "pass"),
            ],
            details: vec![crate::preserves_rail::record("dedup-hits", vec![
                crate::preserves_rail::u64_value(refs.len() as u64),
            ])],
        });
    crate::blocks::registry::receipt(root, &dedup_receipt)
}

pub(super) fn finish_put(input: FinalizeInput<'_>) -> crate::error::Result<crate::blocks::ChunkPut> {
    let Written {
        values,
        refs,
        dedup_refs,
        dedup_hits,
    } = input.written;
    let root_ref = crate::blocks::wire::encode::chunk_root_ref(&refs)?;
    let manifest_value =
        crate::blocks::wire::encode::manifest_value(&crate::blocks::wire::encode::ChunkManifestValueInput {
            object_kind: input.object_kind,
            total_len: input.total_len,
            chunk_size: input.chunk_size_input,
            transforms: input.transforms,
            metadata_ref: input.metadata_ref,
            policy_refs: input.policy_refs,
            chunks: &values,
            root_ref: &root_ref,
            evidence_refs: &[],
        });
    let manifest_ref = crate::preserves_rail::canonical_hash(&manifest_value)?;
    crate::blocks::layout::write_immutable_bytes(
        input.root,
        &crate::blocks::layout::manifest_path(&manifest_ref)?,
        &crate::preserves_rail::canonical_bytes(&manifest_value)?,
        &manifest_ref,
        crate::preserves_rail::parse_canonical_bytes,
    )?;
    let receipt_chunk_refs = refs.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let receipt_value =
        crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
            operation: "manifest-create",
            decision: "pass",
            manifest_ref: Some(&manifest_ref),
            chunk_refs: &receipt_chunk_refs,
            checks: vec![
                ("fixed-v1-chunking", "pass"),
                ("chunk-hash-binding", "pass"),
                ("manifest-root-binding", "pass"),
                ("immutable-chunk-store", "pass"),
                ("confidentiality-policy-admission", "pass"),
                ("transform-ordering", "pass"),
                ("protected-commitment-binding", "pass"),
                ("redb-index-update", "pass"),
            ],
            details: vec![
                crate::preserves_rail::record("object-kind", vec![crate::preserves_rail::string(input.object_kind)]),
                crate::preserves_rail::record("total-len", vec![crate::preserves_rail::u64_value(input.total_len)]),
                crate::preserves_rail::record("chunk-size", vec![crate::preserves_rail::u64_value(
                    input.chunk_size as u64,
                )]),
                crate::preserves_rail::record("dedup-hits", vec![crate::preserves_rail::u64_value(dedup_hits as u64)]),
            ],
        });
    crate::blocks::registry::update::index_put(input.root, &manifest_value, &refs, &receipt_value)?;
    note_reuse(input.root, &manifest_ref, &dedup_refs)?;
    Ok(crate::blocks::ChunkPut {
        manifest_ref,
        object_kind: input.object_kind.to_string(),
        total_len: input.total_len,
        chunk_refs: refs.into_iter().map(|chunk| chunk.chunk_ref).collect(),
        dedup_hits,
        manifest_value,
        receipt_value,
    })
}
