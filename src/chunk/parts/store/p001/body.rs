
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub manifest_ref: Option<String>,
    pub chunk_refs: Vec<String>,
    pub checks: Vec<ChunkStoreReceiptCheck>,
    pub details: Vec<IoValue>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkLineage {
    pub lineage_ref: String,
    pub manifest_ref: String,
    pub root_ref: String,
    pub link_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub verify_receipt_ref: String,
    pub predicate_receipt_refs: Vec<String>,
    pub value: IoValue,
}

pub struct PutBytesWithMetadataInput<'a> {
    pub root: &'a Path,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
    pub chunk_size: u64,
    pub metadata: &'a IoValue,
    pub policy_refs: &'a [String],
}

pub struct PutBytesWithTransformsInput<'a> {
    pub root: &'a Path,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
    pub chunk_size: u64,
    pub metadata: &'a IoValue,
    pub policy_refs: &'a [String],
    pub transforms: &'a ChunkTransforms,
}

struct CapabilityPutBytesWithTransformsInput<'a> {
    root: &'a CapabilityChunkRoot,
    object_kind: &'a str,
    bytes: &'a [u8],
    chunk_size: u64,
    metadata: &'a IoValue,
    policy_refs: &'a [String],
    transforms: &'a ChunkTransforms,
}

pub fn put_bytes(root: &Path, object_kind: &str, bytes: &[u8], chunk_size: u64) -> Result<ChunkStorePut> {
    let root = open_capability_chunk_root(root)?;
    put_bytes_with_root(&root, object_kind, bytes, chunk_size)
}

pub fn put_bytes_with_root(
    root: &CapabilityChunkRoot,
    object_kind: &str,
    bytes: &[u8],
    chunk_size: u64,
) -> Result<ChunkStorePut> {
    let metadata = record("chunk-metadata-v1", vec![
        record("object-kind", vec![string(object_kind)]),
        record("policy", vec![string("public-local-fixture")]),
    ]);
    put_bytes_with_transforms_root(&CapabilityPutBytesWithTransformsInput {
        root,
        object_kind,
        bytes,
        chunk_size,
        metadata: &metadata,
        policy_refs: &[],
        transforms: &ChunkTransforms::public_plaintext(),
    })
}

pub fn put_bytes_with_metadata(input: &PutBytesWithMetadataInput<'_>) -> Result<ChunkStorePut> {
    let root = open_capability_chunk_root(input.root)?;
    put_bytes_with_transforms_root(&CapabilityPutBytesWithTransformsInput {
        root: &root,
        object_kind: input.object_kind,
        bytes: input.bytes,
        chunk_size: input.chunk_size,
        metadata: input.metadata,
        policy_refs: input.policy_refs,
        transforms: &ChunkTransforms::public_plaintext(),
    })
}

struct Ready {
    chunk_size: usize,
    metadata_ref: String,
}

struct Written {
    values: Vec<IoValue>,
    refs: Vec<ChunkRef>,
    dedup_refs: Vec<String>,
    dedup_hits: usize,
}

struct FinalizeInput<'a> {
    root: &'a CapabilityChunkRoot,
    object_kind: &'a str,
    total_len: u64,
    chunk_size_input: u64,
    chunk_size: usize,
    transforms: &'a ChunkTransforms,
    metadata_ref: &'a str,
    policy_refs: &'a [String],
    written: Written,
}

pub fn put_bytes_with_transforms(input: &PutBytesWithTransformsInput<'_>) -> Result<ChunkStorePut> {
    let root = open_capability_chunk_root(input.root)?;
    put_bytes_with_transforms_root(&CapabilityPutBytesWithTransformsInput {
        root: &root,
        object_kind: input.object_kind,
        bytes: input.bytes,
        chunk_size: input.chunk_size,
        metadata: input.metadata,
        policy_refs: input.policy_refs,
        transforms: input.transforms,
    })
}

fn put_bytes_with_transforms_root(input: &CapabilityPutBytesWithTransformsInput<'_>) -> Result<ChunkStorePut> {
    let ready = prepare_put(input)?;
    let written = write_put_parts(input.root, input.bytes, ready.chunk_size, input.transforms)?;
    finish_put(FinalizeInput {
        root: input.root,
        object_kind: input.object_kind,
        total_len: input.bytes.len() as u64,
        chunk_size_input: input.chunk_size,
        chunk_size: ready.chunk_size,
        transforms: input.transforms,
        metadata_ref: &ready.metadata_ref,
        policy_refs: input.policy_refs,
        written,
    })
}

fn prepare_put(input: &CapabilityPutBytesWithTransformsInput<'_>) -> Result<Ready> {
    ensure_dirs(input.root)?;
    if input.object_kind.is_empty() {
        let receipt_value =
            denial_receipt_value("manifest-create", None, &[], "chunk store object kind must not be empty", vec![
                ("object-kind", "fail"),
                ("denial-receipt", "pass"),
            ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("chunk store object kind must not be empty"));
    }
    if input.chunk_size == 0 {
        let receipt_value =
            denial_receipt_value("manifest-create", None, &[], "fixed_v1 chunk size must be non-zero", vec![
                ("chunk-size", "fail"),
                ("denial-receipt", "pass"),
            ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("fixed_v1 chunk size must be non-zero"));
    }
    let chunk_size = match chunk_size_to_usize(input.chunk_size, "fixed_v1 chunk size") {
        Ok(chunk_size) => chunk_size,
        Err(error) => {
            let receipt_value = denial_receipt_value("manifest-create", None, &[], error.to_string(), vec![
                ("chunk-size", "fail"),
                ("denial-receipt", "pass"),
            ]);
            store_receipt(input.root, &receipt_value)?;
            return Err(error);
        }
    };
    if let Err(error) = validate_put_transforms(input.transforms) {
        let receipt_value = denial_receipt_value("manifest-create", None, &[], error.to_string(), vec![
            ("confidentiality-policy", "fail"),
            ("transform-ordering", "fail"),
            ("deny-plaintext-hash-leakage", "pass"),
        ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(error);
    }
    if let Err(error) = validate_put_bounds(input.bytes.len(), chunk_size, input.policy_refs.len()) {
        let receipt_value = denial_receipt_value("manifest-create", None, &[], error.to_string(), vec![
            ("resource-bounds", "fail"),
            ("denial-receipt", "pass"),
        ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(error);
    }
    let metadata_ref = canonical_hash(input.metadata)?;
    write_immutable_bytes(
        input.root,
        &metadata_path(&metadata_ref)?,
        &canonical_bytes(input.metadata)?,
        &metadata_ref,
        parse_canonical_bytes,
    )?;
    Ok(Ready {
        chunk_size,
        metadata_ref,
    })
}

fn ensure_part(root: &CapabilityChunkRoot, chunk: &[u8], chunk_size: usize) -> Result<(String, bool)> {
    let chunk_ref = hash_chunk(chunk, chunk_size);
    let path = chunk_path(&chunk_ref)?;
    if root.root().try_exists(&path)? {
        if let Err(error) = verify_raw_chunk_file(root, &path, &chunk_ref, chunk.len() as u64, chunk_size) {
            let receipt_value =
                denial_receipt_value("dedup-hit", None, std::slice::from_ref(&chunk_ref), error.to_string(), vec![
                    ("existing-chunk-hash-binding", "fail"),
                    ("deny-corrupt-dedup-source", "pass"),
                ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
        Ok((chunk_ref, true))
    } else {
        root.root().write(&path, chunk)?;
        Ok((chunk_ref, false))
    }
}

fn write_put_parts(
    root: &CapabilityChunkRoot,
    bytes: &[u8],
    chunk_size: usize,
    transforms: &ChunkTransforms,
) -> Result<Written> {
    let mut values = Vec::new();
    let mut refs = Vec::new();
    let mut dedup_refs = Vec::new();
    let mut dedup_hits = 0usize;
    for chunk in bytes.chunks(chunk_size) {
        let (chunk_ref, was_present) = ensure_part(root, chunk, chunk_size)?;
        if was_present {
            dedup_hits += 1;
            push_bounded(&mut dedup_refs, chunk_ref.clone(), MAX_CHUNK_STORE_CHUNKS, "chunk store dedup chunks")?;
        }
        push_bounded(
            &mut refs,
            ChunkRef {
                chunk_ref: chunk_ref.clone(),
                length: chunk.len() as u64,
                domain: chunk_domain(chunk_size),
                chunker: FIXED_V1_CHUNKER.to_string(),
                transforms: transforms.clone(),
            },
            MAX_CHUNK_STORE_CHUNKS,
            "chunk store chunk refs",
        )?;
        push_bounded(
            &mut values,
            chunk_ref_value(&chunk_ref, chunk.len() as u64, chunk_size, transforms),
            MAX_CHUNK_STORE_CHUNKS,
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

fn note_reuse(root: &CapabilityChunkRoot, manifest_ref: &str, refs: &[String]) -> Result<()> {
    if refs.is_empty() {
        return Ok(());
    }
    let dedup_receipt = self::receipt_value(ChunkStoreReceiptValueInput {
        operation: "dedup-hit",
        decision: "pass",
        manifest_ref: Some(manifest_ref),
        chunk_refs: refs,
        checks: vec![
            ("existing-chunk-hash-binding", "pass"),
            ("dedup-no-rewrite", "pass"),
            ("receipt-index-update", "pass"),
        ],
        details: vec![record("dedup-hits", vec![u64_value(refs.len() as u64)])],
    });
    store_receipt(root, &dedup_receipt)
}
