//! Exact Mantle bundle and producer-receipt admission.

pub(crate) fn decode_bundle(bytes: &[u8]) -> Result<super::model::Bundle, super::Error> {
    let bundle = serde_json::from_slice::<super::model::Bundle>(bytes).map_err(|_error| super::Error::Json)?;
    validate_bundle(&bundle)?;
    Ok(bundle)
}

pub(crate) fn decode_receipt(
    bytes: &[u8],
    bundle: &super::model::Bundle,
) -> Result<super::model::Receipt, super::Error> {
    let receipt = serde_json::from_slice::<super::model::Receipt>(bytes).map_err(|_error| super::Error::Json)?;
    validate_receipt(&receipt, bundle)?;
    Ok(receipt)
}

fn validate_bundle(bundle: &super::model::Bundle) -> Result<(), super::Error> {
    if bundle.schema != super::model::BUNDLE_SCHEMA {
        return Err(super::Error::Schema);
    }
    if bundle.profile_id != super::model::CLOSED_PROFILE_ID
        || bundle.format != super::model::CLOSED_FORMAT
        || bundle.architecture != super::model::CLOSED_ARCHITECTURE
        || bundle.abi != super::model::CLOSED_ABI
        || bundle.endianness != super::model::CLOSED_ENDIANNESS
        || bundle.relocation_model != super::model::CLOSED_RELOCATION_MODEL
    {
        return Err(super::Error::Profile);
    }
    if bundle.source_length_bytes != super::model::CLOSED_PAGE_BYTES
        || bundle.page_size_bytes != super::model::CLOSED_PAGE_BYTES
        || bundle.maximum_virtual_bytes != super::model::CLOSED_PAGE_BYTES
        || bundle.extents.len() != super::model::CLOSED_MEMBER_COUNT
        || !exact_claims(&bundle.non_claims, &super::model::BUNDLE_NON_CLAIMS)
        || bundle.parser_id.is_empty()
        || bundle.toolchain_id.is_empty()
    {
        return Err(super::Error::Shape);
    }
    let Some(member) = bundle.extents.first() else {
        return Err(super::Error::Shape);
    };
    if member.ordinal != 0
        || member.source_offset_bytes != 0
        || member.virtual_offset_bytes != 0
        || member.length_bytes != super::model::CLOSED_PAGE_BYTES
        || member.permission != super::model::CLOSED_PERMISSION
        || !valid_leaf(&member.member_leaf)
    {
        return Err(super::Error::Shape);
    }
    for digest in [
        &bundle.bundle_identity_blake3,
        &bundle.source_artifact_blake3,
        &bundle.layout_identity_blake3,
        &bundle.plan_identity_blake3,
        &member.content_blake3,
    ] {
        decode_digest(digest)?;
    }
    if bundle.source_artifact_blake3 != member.content_blake3 {
        return Err(super::Error::Linkage);
    }
    let material = super::model::BundleMaterial {
        schema: &bundle.schema,
        profile_id: &bundle.profile_id,
        source_artifact_blake3: &bundle.source_artifact_blake3,
        source_length_bytes: bundle.source_length_bytes,
        format: &bundle.format,
        architecture: &bundle.architecture,
        abi: &bundle.abi,
        endianness: &bundle.endianness,
        page_size_bytes: bundle.page_size_bytes,
        maximum_virtual_bytes: bundle.maximum_virtual_bytes,
        parser_id: &bundle.parser_id,
        toolchain_id: &bundle.toolchain_id,
        relocation_model: &bundle.relocation_model,
        layout_identity_blake3: &bundle.layout_identity_blake3,
        plan_identity_blake3: &bundle.plan_identity_blake3,
        extents: &bundle.extents,
        non_claims: &bundle.non_claims,
    };
    require_identity(super::model::BUNDLE_IDENTITY_DOMAIN, &material, &bundle.bundle_identity_blake3)
}

fn validate_receipt(receipt: &super::model::Receipt, bundle: &super::model::Bundle) -> Result<(), super::Error> {
    if receipt.schema != super::model::RECEIPT_SCHEMA {
        return Err(super::Error::Schema);
    }
    if receipt.bundle_identity_blake3 != bundle.bundle_identity_blake3
        || receipt.source_artifact_blake3 != bundle.source_artifact_blake3
        || receipt.layout_identity_blake3 != bundle.layout_identity_blake3
        || receipt.plan_identity_blake3 != bundle.plan_identity_blake3
        || receipt.bundle_manifest_leaf.is_empty()
    {
        return Err(super::Error::Linkage);
    }
    if receipt.publication_disposition != super::model::PUBLISHED_DISPOSITION
        || !receipt.source_remeasured
        || !receipt.extents_remeasured
        || !receipt.manifest_published
        || !exact_claims(&receipt.non_claims, &super::model::RECEIPT_NON_CLAIMS)
        || receipt.producer_implementation_id.is_empty()
    {
        return Err(super::Error::Publication);
    }
    for digest in [
        &receipt.receipt_identity_blake3,
        &receipt.layout_corpus_identity_blake3,
        &receipt.layout_receipt_identity_blake3,
        &receipt.transition_corpus_identity_blake3,
        &receipt.transition_receipt_identity_blake3,
    ] {
        decode_digest(digest)?;
    }
    let material = super::model::ReceiptMaterial {
        schema: &receipt.schema,
        bundle_identity_blake3: &receipt.bundle_identity_blake3,
        bundle_manifest_leaf: &receipt.bundle_manifest_leaf,
        source_artifact_blake3: &receipt.source_artifact_blake3,
        layout_identity_blake3: &receipt.layout_identity_blake3,
        plan_identity_blake3: &receipt.plan_identity_blake3,
        producer_implementation_id: &receipt.producer_implementation_id,
        layout_corpus_identity_blake3: &receipt.layout_corpus_identity_blake3,
        layout_receipt_identity_blake3: &receipt.layout_receipt_identity_blake3,
        transition_corpus_identity_blake3: &receipt.transition_corpus_identity_blake3,
        transition_receipt_identity_blake3: &receipt.transition_receipt_identity_blake3,
        publication_disposition: &receipt.publication_disposition,
        source_remeasured: receipt.source_remeasured,
        extents_remeasured: receipt.extents_remeasured,
        manifest_published: receipt.manifest_published,
        non_claims: &receipt.non_claims,
    };
    require_identity(super::model::RECEIPT_IDENTITY_DOMAIN, &material, &receipt.receipt_identity_blake3)?;
    super::conformance::validate(receipt)
}

pub(crate) fn decode_digest(text: &str) -> Result<[u8; blake3::OUT_LEN], super::Error> {
    let bytes = data_encoding::HEXLOWER.decode(text.as_bytes()).map_err(|_error| super::Error::Digest)?;
    bytes.try_into().map_err(|_error| super::Error::Digest)
}

pub(crate) fn encode_digest(identity: &[u8; blake3::OUT_LEN]) -> String {
    data_encoding::HEXLOWER.encode(identity)
}

fn require_identity<T: serde::Serialize>(domain: &[u8], value: &T, expected: &str) -> Result<(), super::Error> {
    let canonical = serde_json::to_vec(value).map_err(|_error| super::Error::Json)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&canonical);
    if encode_digest(hasher.finalize().as_bytes()) != expected {
        return Err(super::Error::Identity);
    }
    Ok(())
}

fn exact_claims(observed: &[String], expected: &[&str]) -> bool {
    observed.len() == expected.len() && observed.iter().zip(expected).all(|(actual, required)| actual == required)
}

fn valid_leaf(leaf: &str) -> bool {
    const MAX_LEAF_BYTES: usize = 255;
    !leaf.is_empty()
        && leaf.len() <= MAX_LEAF_BYTES
        && std::path::Path::new(leaf).components().count() == super::model::CLOSED_MEMBER_COUNT
        && matches!(std::path::Path::new(leaf).components().next(), Some(std::path::Component::Normal(_)))
}
