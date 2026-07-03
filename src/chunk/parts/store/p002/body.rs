
fn finish_put(input: FinalizeInput<'_>) -> Result<ChunkStorePut> {
    let Written {
        values,
        refs,
        dedup_refs,
        dedup_hits,
    } = input.written;
    let root_ref = chunk_root_ref(&refs)?;
    let manifest_value = manifest_value(&ChunkManifestValueInput {
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
    let manifest_ref = canonical_hash(&manifest_value)?;
    write_immutable_bytes(
        &manifest_path(input.root, &manifest_ref)?,
        &canonical_bytes(&manifest_value)?,
        &manifest_ref,
        parse_canonical_bytes,
    )?;
    let receipt_chunk_refs = refs.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
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
            record("object-kind", vec![string(input.object_kind)]),
            record("total-len", vec![u64_value(input.total_len)]),
            record("chunk-size", vec![u64_value(input.chunk_size as u64)]),
            record("dedup-hits", vec![u64_value(dedup_hits as u64)]),
        ],
    });
    index_put(input.root, &manifest_value, &refs, &receipt_value)?;
    note_reuse(input.root, &manifest_ref, &dedup_refs)?;
    Ok(ChunkStorePut {
        manifest_ref,
        object_kind: input.object_kind.to_string(),
        total_len: input.total_len,
        chunk_refs: refs.into_iter().map(|chunk| chunk.chunk_ref).collect(),
        dedup_hits,
        manifest_value,
        receipt_value,
    })
}

pub fn read_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkManifest> {
    let bytes = fs::read(manifest_path(root, manifest_ref)?).map_err(MoltenError::from)?;
    let value = parse_canonical_bytes(&bytes)?;
    parse_manifest_value(&value, Some(manifest_ref))
}

pub fn parse_manifest_value(value: &IoValue, expected_manifest_ref: Option<&str>) -> Result<ChunkManifest> {
    let fields = simple_record_any(value, "chunk-manifest-v1")?;
    let arity = record_arity(&fields);
    if arity != 10 && arity != 11 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <chunk-manifest-v1 ...> with arity 10 or 11, got {arity}"
        )));
    }
    require_schema(&fields[0], CHUNK_MANIFEST_SCHEMA, "chunk manifest")?;
    let object_kind = record_string(&fields[1], "object-kind")?;
    let total_len = record_u64(&fields[2], "total-len")?;
    let chunker = record_string(&fields[3], "chunker")?;
    let chunk_size = record_u64(&fields[4], "chunk-size")?;
    let (transforms, metadata_index) = if arity == 11 {
        (parse_transforms_field(&fields[5])?, 6)
    } else {
        (ChunkTransforms::public_plaintext(), 5)
    };
    let metadata_ref = record_string(&fields[metadata_index], "metadata-ref")?;
    let policy_refs = record_string_sequence(&fields[metadata_index + 1], "policy-refs")?;
    let chunk_values = record_sequence(&fields[metadata_index + 2], "chunks")?;
    let root_ref = record_string(&fields[metadata_index + 3], "root-ref")?;
    let evidence_refs = record_string_sequence(&fields[metadata_index + 4], "evidence-refs")?;
    ensure_count_at_most(policy_refs.len(), MAX_CHUNK_STORE_REFS, "chunk manifest policy refs")?;
    ensure_count_at_most(chunk_values.len(), MAX_CHUNK_STORE_CHUNKS, "chunk manifest chunks")?;
    ensure_count_at_most(evidence_refs.len(), MAX_CHUNK_STORE_REFS, "chunk manifest evidence refs")?;
    validate_content_ref_field(&metadata_ref, "chunk manifest metadata-ref")?;
    validate_content_ref_sequence(&policy_refs, "chunk manifest policy-ref")?;
    validate_content_ref_field(&root_ref, "chunk manifest root-ref")?;
    validate_content_ref_sequence(&evidence_refs, "chunk manifest evidence-ref")?;
    let manifest_ref = canonical_hash(value)?;
    if let Some(expected) = expected_manifest_ref
        && manifest_ref != expected
    {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest hash mismatch: got {manifest_ref}, expected {expected}"
        )));
    }
    if chunker != FIXED_V1_CHUNKER {
        return Err(MoltenError::invalid_harness(format!("unsupported chunker {chunker}")));
    }
    if chunk_size == 0 {
        return Err(MoltenError::invalid_harness("chunk manifest chunk-size must be non-zero"));
    }
    validate_transform_shape(&transforms)?;
    let chunks = refs_from_values(&chunk_values, chunk_size, &transforms)?;
    validate_fixed_chunk_lengths(total_len, chunk_size, &chunks)?;
    ensure_distinct_commitments(&chunks)?;
    let recomputed_root = chunk_root_ref(&chunks)?;
    if recomputed_root != root_ref {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest root mismatch: got {root_ref}, expected {recomputed_root}"
        )));
    }
    Ok(ChunkManifest {
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

fn refs_from_values(values: &[IoValue], chunk_size: u64, transforms: &ChunkTransforms) -> Result<Vec<ChunkRef>> {
    let mut chunks = Vec::new();
    for value in values {
        let chunk = parse_chunk_ref_value(value, chunk_size)?;
        if chunk.transforms != *transforms {
            return Err(MoltenError::invalid_harness(format!(
                "chunk transform mismatch for {}: manifest transforms differ from chunk ref transforms",
                chunk.chunk_ref
            )));
        }
        push_bounded(&mut chunks, chunk, MAX_CHUNK_STORE_CHUNKS, "chunk manifest chunks")?;
    }
    Ok(chunks)
}

fn validate_content_ref_field(value: &str, label: &str) -> Result<()> {
    validate_content_ref(value).map_err(|error| MoltenError::invalid_harness(format!("{label} is invalid: {error}")))
}

fn validate_content_ref_sequence(values: &[String], label: &str) -> Result<()> {
    for value in values {
        validate_content_ref_field(value, label)?;
    }
    Ok(())
}

fn ensure_distinct_commitments(chunks: &[ChunkRef]) -> Result<()> {
    for chunk in chunks {
        if chunk.transforms.protected_commitment_ref.as_deref() == Some(chunk.chunk_ref.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "protected commitment ref for chunk {} must differ from the plaintext chunk ref",
                chunk.chunk_ref
            )));
        }
    }
    Ok(())
}

pub fn parse_chunk_ref_value(value: &IoValue, expected_chunk_size: u64) -> Result<ChunkRef> {
    let fields = simple_record_any(value, "chunk-ref-v1")?;
    let arity = record_arity(&fields);
    if arity != 7 && arity != 8 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <chunk-ref-v1 ...> with arity 7 or 8, got {arity}"
        )));
    }
    require_schema(&fields[0], CHUNK_REF_SCHEMA, "chunk ref")?;
    let chunk_ref = record_string(&fields[1], "hash")?;
    let length = record_u64(&fields[2], "length")?;
    let domain = record_string(&fields[3], "domain")?;
    let chunker = record_string(&fields[4], "chunker")?;
    let (transforms, location_index) = if arity == 8 {
        (parse_transforms_field(&fields[5])?, 6)
    } else {
        (ChunkTransforms::public_plaintext(), 5)
    };
    let _location_hints = record_sequence(&fields[location_index], "location-hints")?;
    let evidence_refs = record_string_sequence(&fields[location_index + 1], "evidence-refs")?;
    validate_content_ref_field(&chunk_ref, "chunk ref hash")?;
    validate_content_ref_sequence(&evidence_refs, "chunk ref evidence-ref")?;
    if chunker != FIXED_V1_CHUNKER {
        return Err(MoltenError::invalid_harness(format!("unsupported chunk ref chunker {chunker}")));
    }
    let expected_chunk_size_usize = chunk_size_to_usize(expected_chunk_size, "expected chunk size")?;
    let expected_domain = chunk_domain(expected_chunk_size_usize);
    if domain != expected_domain {
        return Err(MoltenError::invalid_harness(format!(
            "chunk ref domain mismatch: got {domain}, expected {expected_domain}"
        )));
    }
    if length == 0 || length > expected_chunk_size {
        return Err(MoltenError::invalid_harness(format!(
            "chunk ref length {length} outside fixed_v1 bounds 1..={expected_chunk_size}"
        )));
    }
    validate_transform_shape(&transforms)?;
    Ok(ChunkRef {
        chunk_ref,
        length,
        domain,
        chunker,
        transforms,
    })
}

pub fn verify_manifest(root: &Path, manifest_ref: &str) -> Result<ChunkStoreVerify> {
    let manifest = match read_manifest(root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("chunk-verify", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("chunk-verify", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    if let Err(error) = verify_manifest_chunks(root, &manifest) {
        let receipt_value =
            denial_receipt_value("chunk-verify", Some(&manifest.manifest_ref), &chunk_refs, error.to_string(), vec![
                ("chunk-hash-length", "fail"),
                ("deny-corrupt-or-missing-chunk", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(error);
    }
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "chunk-verify",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &chunk_refs,
        checks: vec![
            ("manifest-ref-binding", "pass"),
            ("chunk-hash-length", "pass"),
            ("chunk-order-root", "pass"),
            ("reconstructed-total-length", "pass"),
            ("redb-index-availability", "pass"),
        ],
        details: vec![record("total-len", vec![u64_value(manifest.total_len)])],
    });
    index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(ChunkStoreVerify {
        manifest_ref: manifest.manifest_ref,
        total_len: manifest.total_len,
        chunk_refs,
        receipt_value,
    })
}
