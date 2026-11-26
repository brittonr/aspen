//! Manifest and chunk-ref parsing with canonical-ref, bound, root, and codec-facade checks.

pub fn parse_manifest_value(
    value: &preserves::IOValue,
    expected_manifest_ref: Option<&str>,
) -> crate::error::Result<crate::blocks::ChunkManifest> {
    let fields = crate::blocks::wire::fields::simple_record_any(value, "chunk-manifest-v1")?;
    let arity = crate::blocks::wire::fields::record_arity(&fields);
    if arity != 10 && arity != 11 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <chunk-manifest-v1 ...> with arity 10 or 11, got {arity}"
        )));
    }
    crate::blocks::wire::fields::require_schema(
        &fields[0],
        crate::preserves_rail::CHUNK_MANIFEST_SCHEMA,
        "chunk manifest",
    )?;
    let object_kind = crate::blocks::wire::fields::record_string(&fields[1], "object-kind")?;
    let total_len = crate::blocks::wire::fields::record_u64(&fields[2], "total-len")?;
    let chunker = crate::blocks::wire::fields::record_string(&fields[3], "chunker")?;
    let chunk_size = crate::blocks::wire::fields::record_u64(&fields[4], "chunk-size")?;
    let (transforms, metadata_index) = if arity == 11 {
        (crate::blocks::wire::policy::parse_transforms_field(&fields[5])?, 6)
    } else {
        (crate::blocks::ChunkTransforms::public_plaintext(), 5)
    };
    let metadata_ref = crate::blocks::wire::fields::record_string(&fields[metadata_index], "metadata-ref")?;
    let policy_refs = crate::blocks::wire::fields::record_string_sequence(&fields[metadata_index + 1], "policy-refs")?;
    let chunk_values = crate::blocks::wire::fields::record_sequence(&fields[metadata_index + 2], "chunks")?;
    let root_ref = crate::blocks::wire::fields::record_string(&fields[metadata_index + 3], "root-ref")?;
    let evidence_refs =
        crate::blocks::wire::fields::record_string_sequence(&fields[metadata_index + 4], "evidence-refs")?;
    validate_manifest_refs(&ManifestRefs {
        metadata_ref: &metadata_ref,
        policy_refs: &policy_refs,
        chunk_count: chunk_values.len(),
        root_ref: &root_ref,
        evidence_refs: &evidence_refs,
    })?;
    let manifest_ref = codec_checked_manifest_ref(value)?;
    if let Some(expected) = expected_manifest_ref
        && manifest_ref != expected
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk manifest hash mismatch: got {manifest_ref}, expected {expected}"
        )));
    }
    validate_chunker_shape(&chunker, chunk_size)?;
    crate::blocks::wire::policy::validate_transform_shape(&transforms)?;
    let chunks = refs_from_values(&chunk_values, chunk_size, &transforms)?;
    crate::blocks::wire::policy::validate_fixed_chunk_lengths(total_len, chunk_size, &chunks)?;
    ensure_distinct_commitments(&chunks)?;
    require_root(&chunks, &root_ref)?;
    Ok(crate::blocks::ChunkManifest {
        manifest_ref,
        object_kind,
        total_len,
        chunker,
        chunk_size,
        transforms,
        metadata_ref,
        policy_refs,
        chunks,
        root_ref,
        evidence_refs,
        value: value.clone(),
    })
}

struct ManifestRefs<'a> {
    metadata_ref: &'a str,
    policy_refs: &'a [String],
    chunk_count: usize,
    root_ref: &'a str,
    evidence_refs: &'a [String],
}

/// Bounds the manifest's ref and chunk counts, then requires canonical content refs, in decode
/// order.
fn validate_manifest_refs(refs: &ManifestRefs<'_>) -> crate::error::Result<()> {
    crate::bounded::ensure_count_at_most(
        refs.policy_refs.len(),
        crate::blocks::limits::MAX_CHUNK_STORE_REFS,
        "chunk manifest policy refs",
    )?;
    crate::bounded::ensure_count_at_most(
        refs.chunk_count,
        crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
        "chunk manifest chunks",
    )?;
    crate::bounded::ensure_count_at_most(
        refs.evidence_refs.len(),
        crate::blocks::limits::MAX_CHUNK_STORE_REFS,
        "chunk manifest evidence refs",
    )?;
    validate_content_ref_field(refs.metadata_ref, "chunk manifest metadata-ref")?;
    validate_content_ref_sequence(refs.policy_refs, "chunk manifest policy-ref")?;
    validate_content_ref_field(refs.root_ref, "chunk manifest root-ref")?;
    validate_content_ref_sequence(refs.evidence_refs, "chunk manifest evidence-ref")
}

/// Requires the declared root ref to equal the root recomputed from the parsed chunk refs.
fn require_root(chunks: &[crate::blocks::ChunkRef], root_ref: &str) -> crate::error::Result<()> {
    let recomputed_root = crate::blocks::wire::encode::chunk_root_ref(chunks)?;
    if recomputed_root != root_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk manifest root mismatch: got {root_ref}, expected {recomputed_root}"
        )));
    }
    Ok(())
}

fn validate_chunker_shape(chunker: &str, chunk_size: u64) -> crate::error::Result<()> {
    if chunker != crate::blocks::FIXED_V1_CHUNKER {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported chunker {chunker}")));
    }
    if chunk_size == 0 {
        return Err(crate::error::Failure::invalid_harness("chunk manifest chunk-size must be non-zero"));
    }
    Ok(())
}

fn refs_from_values(
    values: &[preserves::IOValue],
    chunk_size: u64,
    transforms: &crate::blocks::ChunkTransforms,
) -> crate::error::Result<Vec<crate::blocks::ChunkRef>> {
    let mut chunks = Vec::new();
    for value in values {
        let chunk = parse_chunk_ref_value(value, chunk_size)?;
        if chunk.transforms != *transforms {
            return Err(crate::error::Failure::invalid_harness(format!(
                "chunk transform mismatch for {}: manifest transforms differ from chunk ref transforms",
                chunk.chunk_ref
            )));
        }
        crate::bounded::push(
            &mut chunks,
            chunk,
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk manifest chunks",
        )?;
    }
    Ok(chunks)
}

pub(super) fn validate_content_ref_field(value: &str, label: &str) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("{label} is invalid: {error}")))
}

fn validate_content_ref_sequence(values: &[String], label: &str) -> crate::error::Result<()> {
    for value in values {
        validate_content_ref_field(value, label)?;
    }
    Ok(())
}

fn ensure_distinct_commitments(chunks: &[crate::blocks::ChunkRef]) -> crate::error::Result<()> {
    for chunk in chunks {
        if chunk.transforms.protected_commitment_ref.as_deref() == Some(chunk.chunk_ref.as_str()) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "protected commitment ref for chunk {} must differ from the plaintext chunk ref",
                chunk.chunk_ref
            )));
        }
    }
    Ok(())
}

pub fn parse_chunk_ref_value(
    value: &preserves::IOValue,
    expected_chunk_size: u64,
) -> crate::error::Result<crate::blocks::ChunkRef> {
    let fields = crate::blocks::wire::fields::simple_record_any(value, "chunk-ref-v1")?;
    let arity = crate::blocks::wire::fields::record_arity(&fields);
    if arity != 7 && arity != 8 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <chunk-ref-v1 ...> with arity 7 or 8, got {arity}"
        )));
    }
    crate::blocks::wire::fields::require_schema(&fields[0], crate::preserves_rail::CHUNK_REF_SCHEMA, "chunk ref")?;
    let chunk_ref = crate::blocks::wire::fields::record_string(&fields[1], "hash")?;
    let length = crate::blocks::wire::fields::record_u64(&fields[2], "length")?;
    let domain = crate::blocks::wire::fields::record_string(&fields[3], "domain")?;
    let chunker = crate::blocks::wire::fields::record_string(&fields[4], "chunker")?;
    let (transforms, location_index) = if arity == 8 {
        (crate::blocks::wire::policy::parse_transforms_field(&fields[5])?, 6)
    } else {
        (crate::blocks::ChunkTransforms::public_plaintext(), 5)
    };
    let _location_hints = crate::blocks::wire::fields::record_sequence(&fields[location_index], "location-hints")?;
    let evidence_refs =
        crate::blocks::wire::fields::record_string_sequence(&fields[location_index + 1], "evidence-refs")?;
    validate_content_ref_field(&chunk_ref, "chunk ref hash")?;
    validate_content_ref_sequence(&evidence_refs, "chunk ref evidence-ref")?;
    if chunker != crate::blocks::FIXED_V1_CHUNKER {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported chunk ref chunker {chunker}")));
    }
    let expected_chunk_size_usize =
        crate::blocks::limits::chunk_size_to_usize(expected_chunk_size, "expected chunk size")?;
    let expected_domain = crate::blocks::integrity::chunk_domain(expected_chunk_size_usize);
    if domain != expected_domain {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk ref domain mismatch: got {domain}, expected {expected_domain}"
        )));
    }
    if length == 0 || length > expected_chunk_size {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk ref length {length} outside fixed_v1 bounds 1..={expected_chunk_size}"
        )));
    }
    crate::blocks::wire::policy::validate_transform_shape(&transforms)?;
    Ok(crate::blocks::ChunkRef {
        chunk_ref,
        length,
        domain,
        chunker,
        transforms,
    })
}

/// Hashes a manifest value and requires the codec facade to accept it as a chunk-store manifest.
fn codec_checked_manifest_ref(value: &preserves::IOValue) -> crate::error::Result<String> {
    let manifest_ref = crate::preserves_rail::canonical_hash(value)?;
    molten_core::codec::validate_domain_artifact(&molten_core::codec::DomainArtifactInput {
        domain: "chunk-store",
        label: "chunk-manifest-v1",
        schema: crate::preserves_rail::CHUNK_MANIFEST_SCHEMA,
        artifact_ref: &manifest_ref,
        expected_schema: crate::preserves_rail::CHUNK_MANIFEST_SCHEMA,
        supported_labels: &["chunk-manifest-v1"],
    })
    .map_err(|issue| {
        crate::error::Failure::invalid_harness(format!("chunk manifest codec facade rejected artifact: {issue}"))
    })?;
    Ok(manifest_ref)
}
