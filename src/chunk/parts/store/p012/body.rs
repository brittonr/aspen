
fn parse_transforms_field(value: &Value<IoValue>) -> Result<ChunkTransforms> {
    let fields = value
        .collect_simple_record("transforms", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transforms ...> field"))?;
    let transform_record = fields[0]
        .collect_simple_record("transforms-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transforms-v1 ...> value"))?;
    require_schema(&transform_record[0], CHUNK_TRANSFORMS_SCHEMA, "chunk transforms")?;
    let compression = record_string(&transform_record[1], "compression")?;
    let encryption = record_string(&transform_record[2], "encryption")?;
    let ordering = record_string(&transform_record[3], "ordering")?;
    let confidentiality = record_string(&transform_record[4], "confidentiality")?;
    let protected_commitment_ref = record_optional_string(&transform_record[5], "protected-commitment")?;
    Ok(ChunkTransforms {
        compression,
        encryption,
        ordering,
        confidentiality,
        protected_commitment_ref,
    })
}

fn validate_transform_shape(transforms: &ChunkTransforms) -> Result<()> {
    if transforms.compression != "none" && transforms.compression != "zstd-placeholder" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk compression mode {}",
            transforms.compression
        )));
    }
    if transforms.encryption != "none" && transforms.encryption != "protected-commitment" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk encryption mode {}",
            transforms.encryption
        )));
    }
    if transforms.confidentiality != "public" && transforms.confidentiality != "confidential" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk confidentiality mode {}",
            transforms.confidentiality
        )));
    }
    let expected_ordering = match (transforms.compression.as_str(), transforms.encryption.as_str()) {
        ("none", "none") => "identity",
        ("zstd-placeholder", "none") => "compress",
        ("none", "protected-commitment") => "encrypt",
        ("zstd-placeholder", "protected-commitment") => "compress-then-encrypt",
        _ => {
            return Err(MoltenError::invalid_harness(format!(
                "unsupported chunk transform pair compression={} encryption={}",
                transforms.compression, transforms.encryption
            )));
        }
    };
    if transforms.ordering != expected_ordering {
        return Err(MoltenError::invalid_harness(format!(
            "chunk transform ordering {} does not match expected {expected_ordering}",
            transforms.ordering
        )));
    }
    if transforms.confidentiality == "confidential" {
        let Some(commitment_ref) = &transforms.protected_commitment_ref else {
            return Err(MoltenError::invalid_harness(
                "confidential chunk transforms require a protected commitment ref",
            ));
        };
        if commitment_ref.trim().is_empty() {
            return Err(MoltenError::invalid_harness(
                "confidential chunk transforms require a non-empty protected commitment ref",
            ));
        }
        if transforms.encryption != "protected-commitment" {
            return Err(MoltenError::invalid_harness(
                "confidential chunk transforms require protected-commitment encryption",
            ));
        }
    }
    Ok(())
}

fn validate_put_transforms(transforms: &ChunkTransforms) -> Result<()> {
    validate_transform_shape(transforms)?;
    if transforms.confidentiality == "confidential" {
        return Err(MoltenError::invalid_harness(
            "confidential chunk-store writes require a protected encryption implementation before chunk refs may be emitted",
        ));
    }
    if transforms != &ChunkTransforms::public_plaintext() {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chunk-store transform for writes: compression={} encryption={} ordering={}",
            transforms.compression, transforms.encryption, transforms.ordering
        )));
    }
    Ok(())
}

fn unsupported_transform_message(manifest: &ChunkManifest) -> Option<String> {
    let supported = ChunkTransforms::public_plaintext();
    if manifest.transforms != supported {
        return Some(format!(
            "unsupported chunk-store transform for {}: compression={} encryption={} ordering={} confidentiality={}",
            manifest.manifest_ref,
            manifest.transforms.compression,
            manifest.transforms.encryption,
            manifest.transforms.ordering,
            manifest.transforms.confidentiality
        ));
    }
    for chunk in &manifest.chunks {
        if chunk.transforms != supported {
            return Some(format!(
                "unsupported chunk-store transform for chunk {}: compression={} encryption={} ordering={} confidentiality={}",
                chunk.chunk_ref,
                chunk.transforms.compression,
                chunk.transforms.encryption,
                chunk.transforms.ordering,
                chunk.transforms.confidentiality
            ));
        }
    }
    None
}

fn chunk_root_ref(chunks: &[ChunkRef]) -> Result<String> {
    canonical_hash(&record("chunk-root-v1", vec![
        string(crate::preserves_rail::CHUNK_ROOT_SCHEMA),
        record("chunker", vec![string(FIXED_V1_CHUNKER)]),
        record("chunks", vec![sequence(
            chunks
                .iter()
                .map(|chunk| record("chunk", vec![string(&chunk.chunk_ref), u64_value(chunk.length)]))
                .collect(),
        )]),
    ]))
}

struct ChunkStoreReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    manifest_ref: Option<&'a str>,
    chunk_refs: &'a [String],
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IoValue>,
}

fn receipt_value(input: ChunkStoreReceiptValueInput<'_>) -> IoValue {
    record("chunk-store-receipt-v1", vec![
        string(CHUNK_STORE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![input.manifest_ref.map(string).unwrap_or_else(|| record("none", vec![]))]),
        record("chunks", vec![sequence(input.chunk_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(
            input
                .checks
                .into_iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
        record("details", vec![sequence(input.details)]),
    ])
}

fn denial_receipt_value(
    operation: &str,
    manifest_ref: Option<&str>,
    chunk_refs: &[String],
    reason: impl Into<String>,
    checks: Vec<(&str, &str)>,
) -> IoValue {
    let reason = reason.into();
    receipt_value(ChunkStoreReceiptValueInput {
        operation,
        decision: "deny",
        manifest_ref,
        chunk_refs,
        checks,
        details: vec![record("reason", vec![string(&reason)])],
    })
}

pub fn parse_receipt_value(value: &IoValue, expected_receipt_ref: Option<&str>) -> Result<ChunkStoreReceipt> {
    let fields = simple_record(value, "chunk-store-receipt-v1", 7)?;
    require_schema(&fields[0], CHUNK_STORE_RECEIPT_SCHEMA, "chunk store receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    let decision = record_string(&fields[2], "decision")?;
    let manifest_ref = record_optional_string(&fields[3], "manifest")?;
    let chunk_refs = record_string_sequence(&fields[4], "chunks")?;
    let check_values = record_sequence(&fields[5], "checks")?;
    let details = record_sequence(&fields[6], "details")?;
    if operation.is_empty() {
        return Err(MoltenError::invalid_harness("chunk store receipt operation must not be empty"));
    }
    if decision != "pass" && decision != "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "chunk store receipt decision must be pass or deny, got {decision}"
        )));
    }
    if let Some(manifest_ref) = &manifest_ref {
        filename_for_ref(manifest_ref)?;
    }
    for chunk_ref in &chunk_refs {
        filename_for_ref(chunk_ref)?;
    }
    let mut checks = Vec::new();
    for check_value in &check_values {
        let check = simple_record(check_value, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if name.is_empty() {
            return Err(MoltenError::invalid_harness("chunk store receipt check name must not be empty"));
        }
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!(
                "chunk store receipt check status must be pass or fail, got {status}"
            )));
        }
        push_bounded(
            &mut checks,
            ChunkStoreReceiptCheck { name, status },
            MAX_CHUNK_STORE_CHECKS,
            "chunk store receipt checks",
        )?;
    }
    let receipt_ref = canonical_hash(value)?;
    if let Some(expected) = expected_receipt_ref
        && receipt_ref != expected
    {
        return Err(MoltenError::invalid_harness(format!(
            "chunk store receipt hash mismatch: got {receipt_ref}, expected {expected}"
        )));
    }
    Ok(ChunkStoreReceipt {
        receipt_ref,
        operation,
        decision,
        manifest_ref,
        chunk_refs,
        checks,
        details,
        value: value.clone(),
    })
}

fn chunk_index_value(chunk: &ChunkRef) -> IoValue {
    record("chunk-index-v1", vec![
        string(CHUNK_INDEX_SCHEMA),
        record("chunk-ref", vec![string(&chunk.chunk_ref)]),
        record("length", vec![u64_value(chunk.length)]),
        record("domain", vec![string(&chunk.domain)]),
        record("chunker", vec![string(&chunk.chunker)]),
        record("transforms", vec![transforms_value(&chunk.transforms)]),
    ])
}

fn partial_fetch_value(manifest_ref: &str, status: &str, missing_before: &[String], fetched: &[String]) -> IoValue {
    record("partial-fetch-v1", vec![
        string(PARTIAL_FETCH_SCHEMA),
        record("manifest", vec![string(manifest_ref)]),
        record("status", vec![string(status)]),
        record("missing-before", vec![sequence(missing_before.iter().map(string).collect())]),
        record("fetched", vec![sequence(fetched.iter().map(string).collect())]),
    ])
}

fn iroh_ticket_value(manifest_ref: &str, manifest_blob_ref: &str, chunks: &[IrohChunkBlob]) -> IoValue {
    record("chunk-store-iroh-ticket-v1", vec![
        string(CHUNK_IROH_TICKET_SCHEMA),
        record("adapter", vec![string("iroh-local")]),
        record("manifest-ref", vec![string(manifest_ref)]),
        record("manifest-blob-ref", vec![string(manifest_blob_ref)]),
        record("chunks", vec![sequence(
            chunks
                .iter()
                .map(|chunk| {
                    record("chunk-blob", vec![
                        string(&chunk.chunk_ref),
                        string(&chunk.blob_ref),
                        u64_value(chunk.length),
                    ])
                })
                .collect(),
        )]),
    ])
}
