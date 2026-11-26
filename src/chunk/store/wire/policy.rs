//! Transform policy: parsing, shape validation, write admission, and the unsupported-transform
//! message.

pub(super) fn parse_transforms_field(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::blocks::ChunkTransforms> {
    let fields = value
        .collect_simple_record("transforms", Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <transforms ...> field"))?;
    let transform_record = fields[0]
        .collect_simple_record("transforms-v1", Some(6))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <transforms-v1 ...> value"))?;
    crate::blocks::wire::fields::require_schema(
        &transform_record[0],
        crate::blocks::wire::CHUNK_TRANSFORMS_SCHEMA,
        "chunk transforms",
    )?;
    let compression = crate::blocks::wire::fields::record_string(&transform_record[1], "compression")?;
    let encryption = crate::blocks::wire::fields::record_string(&transform_record[2], "encryption")?;
    let ordering = crate::blocks::wire::fields::record_string(&transform_record[3], "ordering")?;
    let confidentiality = crate::blocks::wire::fields::record_string(&transform_record[4], "confidentiality")?;
    let protected_commitment_ref =
        crate::blocks::wire::fields::record_optional_string(&transform_record[5], "protected-commitment")?;
    Ok(crate::blocks::ChunkTransforms {
        compression,
        encryption,
        ordering,
        confidentiality,
        protected_commitment_ref,
    })
}

pub(super) fn validate_transform_shape(transforms: &crate::blocks::ChunkTransforms) -> crate::error::Result<()> {
    if transforms.compression != "none" && transforms.compression != "zstd-placeholder" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported chunk compression mode {}",
            transforms.compression
        )));
    }
    if transforms.encryption != "none" && transforms.encryption != "protected-commitment" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported chunk encryption mode {}",
            transforms.encryption
        )));
    }
    if transforms.confidentiality != "public" && transforms.confidentiality != "confidential" {
        return Err(crate::error::Failure::invalid_harness(format!(
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
            return Err(crate::error::Failure::invalid_harness(format!(
                "unsupported chunk transform pair compression={} encryption={}",
                transforms.compression, transforms.encryption
            )));
        }
    };
    if transforms.ordering != expected_ordering {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk transform ordering {} does not match expected {expected_ordering}",
            transforms.ordering
        )));
    }
    if transforms.confidentiality == "confidential" {
        let Some(commitment_ref) = &transforms.protected_commitment_ref else {
            return Err(crate::error::Failure::invalid_harness(
                "confidential chunk transforms require a protected commitment ref",
            ));
        };
        crate::blocks::wire::decode::validate_content_ref_field(
            commitment_ref,
            "confidential chunk protected commitment ref",
        )?;
        if transforms.encryption != "protected-commitment" {
            return Err(crate::error::Failure::invalid_harness(
                "confidential chunk transforms require protected-commitment encryption",
            ));
        }
    }
    Ok(())
}

pub(in crate::blocks) fn validate_put_transforms(
    transforms: &crate::blocks::ChunkTransforms,
) -> crate::error::Result<()> {
    validate_transform_shape(transforms)?;
    if transforms.confidentiality == "confidential" {
        return Err(crate::error::Failure::invalid_harness(
            "confidential chunk-store writes require a protected encryption implementation before chunk refs may be emitted",
        ));
    }
    if transforms != &crate::blocks::ChunkTransforms::public_plaintext() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported chunk-store transform for writes: compression={} encryption={} ordering={}",
            transforms.compression, transforms.encryption, transforms.ordering
        )));
    }
    Ok(())
}

pub(in crate::blocks) fn unsupported_transform_message(manifest: &crate::blocks::ChunkManifest) -> Option<String> {
    let supported = crate::blocks::ChunkTransforms::public_plaintext();
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

pub(super) fn validate_fixed_chunk_lengths(
    total_len: u64,
    chunk_size: u64,
    chunks: &[crate::blocks::ChunkRef],
) -> crate::error::Result<()> {
    let reconstructed = chunks.iter().map(|chunk| chunk.length).sum::<u64>();
    if reconstructed != total_len {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk manifest total length mismatch: refs sum to {reconstructed}, expected {total_len}"
        )));
    }
    if total_len == 0 {
        if !chunks.is_empty() {
            return Err(crate::error::Failure::invalid_harness("empty object manifest must not contain chunks"));
        }
        return Ok(());
    }
    for (index, chunk) in chunks.iter().enumerate() {
        let is_last = index + 1 == chunks.len();
        if !is_last && chunk.length != chunk_size {
            return Err(crate::error::Failure::invalid_harness(format!(
                "non-final fixed_v1 chunk {index} has length {}, expected {chunk_size}",
                chunk.length
            )));
        }
        if is_last && (chunk.length == 0 || chunk.length > chunk_size) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "final fixed_v1 chunk {index} has invalid length {} for chunk size {chunk_size}",
                chunk.length
            )));
        }
    }
    Ok(())
}
