//! Content-addressed puts: public entry points and the admission checks that run before any chunk
//! is written.

struct CapabilityPutBytesWithTransformsInput<'a> {
    root: &'a crate::blocks::CapabilityChunkRoot,
    object_kind: &'a str,
    bytes: &'a [u8],
    chunk_size: u64,
    metadata: &'a preserves::IOValue,
    policy_refs: &'a [String],
    transforms: &'a crate::blocks::ChunkTransforms,
}

pub fn put_bytes(
    root: &std::path::Path,
    object_kind: &str,
    bytes: &[u8],
    chunk_size: u64,
) -> crate::error::Result<crate::blocks::ChunkPut> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    put_bytes_with_root(&root, object_kind, bytes, chunk_size)
}

pub fn put_bytes_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    object_kind: &str,
    bytes: &[u8],
    chunk_size: u64,
) -> crate::error::Result<crate::blocks::ChunkPut> {
    let metadata = crate::preserves_rail::record("chunk-metadata-v1", vec![
        crate::preserves_rail::record("object-kind", vec![crate::preserves_rail::string(object_kind)]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::string("public-local-fixture")]),
    ]);
    put_bytes_with_transforms_root(&CapabilityPutBytesWithTransformsInput {
        root,
        object_kind,
        bytes,
        chunk_size,
        metadata: &metadata,
        policy_refs: &[],
        transforms: &crate::blocks::ChunkTransforms::public_plaintext(),
    })
}

pub fn put_bytes_with_metadata(
    input: &crate::blocks::PutBytesWithMetadataInput<'_>,
) -> crate::error::Result<crate::blocks::ChunkPut> {
    let root = crate::blocks::open_capability_chunk_root(input.root)?;
    put_bytes_with_transforms_root(&CapabilityPutBytesWithTransformsInput {
        root: &root,
        object_kind: input.object_kind,
        bytes: input.bytes,
        chunk_size: input.chunk_size,
        metadata: input.metadata,
        policy_refs: input.policy_refs,
        transforms: &crate::blocks::ChunkTransforms::public_plaintext(),
    })
}

struct Ready {
    chunk_size: usize,
    metadata_ref: String,
}

pub fn put_bytes_with_transforms(
    input: &crate::blocks::PutBytesWithTransformsInput<'_>,
) -> crate::error::Result<crate::blocks::ChunkPut> {
    let root = crate::blocks::open_capability_chunk_root(input.root)?;
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

fn put_bytes_with_transforms_root(
    input: &CapabilityPutBytesWithTransformsInput<'_>,
) -> crate::error::Result<crate::blocks::ChunkPut> {
    let ready = prepare_put(input)?;
    let written =
        crate::blocks::local::writer::write_put_parts(input.root, input.bytes, ready.chunk_size, input.transforms)?;
    crate::blocks::local::writer::finish_put(crate::blocks::local::writer::FinalizeInput {
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

fn prepare_put(input: &CapabilityPutBytesWithTransformsInput<'_>) -> crate::error::Result<Ready> {
    crate::blocks::layout::ensure_dirs(input.root)?;
    if input.object_kind.is_empty() {
        record_put_denial(input.root, "chunk store object kind must not be empty", vec![
            ("object-kind", "fail"),
            ("denial-receipt", "pass"),
        ])?;
        return Err(crate::error::Failure::invalid_harness("chunk store object kind must not be empty"));
    }
    if input.chunk_size == 0 {
        record_put_denial(input.root, "fixed_v1 chunk size must be non-zero", vec![
            ("chunk-size", "fail"),
            ("denial-receipt", "pass"),
        ])?;
        return Err(crate::error::Failure::invalid_harness("fixed_v1 chunk size must be non-zero"));
    }
    let chunk_size = match crate::blocks::limits::chunk_size_to_usize(input.chunk_size, "fixed_v1 chunk size") {
        Ok(chunk_size) => chunk_size,
        Err(error) => {
            record_put_denial(input.root, error.to_string(), vec![("chunk-size", "fail"), ("denial-receipt", "pass")])?;
            return Err(error);
        }
    };
    if let Err(error) = crate::blocks::wire::policy::validate_put_transforms(input.transforms) {
        record_put_denial(input.root, error.to_string(), vec![
            ("confidentiality-policy", "fail"),
            ("transform-ordering", "fail"),
            ("deny-plaintext-hash-leakage", "pass"),
        ])?;
        return Err(error);
    }
    if let Err(error) =
        crate::blocks::limits::validate_put_bounds(input.bytes.len(), chunk_size, input.policy_refs.len())
    {
        record_put_denial(input.root, error.to_string(), vec![
            ("resource-bounds", "fail"),
            ("denial-receipt", "pass"),
        ])?;
        return Err(error);
    }
    let metadata_ref = store_metadata(input.root, input.metadata)?;
    Ok(Ready {
        chunk_size,
        metadata_ref,
    })
}

/// Records a `manifest-create` denial receipt for a put rejected before any chunk is written.
fn record_put_denial(
    root: &crate::blocks::CapabilityChunkRoot,
    reason: impl Into<String>,
    checks: Vec<(&str, &str)>,
) -> crate::error::Result<()> {
    let receipt_value =
        crate::blocks::wire::outcome::denial_receipt_value("manifest-create", None, &[], reason, checks);
    crate::blocks::registry::receipt(root, &receipt_value)
}

/// Writes the canonical metadata record once and returns its content ref.
fn store_metadata(
    root: &crate::blocks::CapabilityChunkRoot,
    metadata: &preserves::IOValue,
) -> crate::error::Result<String> {
    let metadata_ref = crate::preserves_rail::canonical_hash(metadata)?;
    crate::blocks::layout::write_immutable_bytes(
        root,
        &crate::blocks::layout::metadata_path(&metadata_ref)?,
        &crate::preserves_rail::canonical_bytes(metadata)?,
        &metadata_ref,
        crate::preserves_rail::parse_canonical_bytes,
    )?;
    Ok(metadata_ref)
}
