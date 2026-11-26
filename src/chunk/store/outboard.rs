//! Remote byte-source hints and range readback verified before any byte is exposed.

const REMOTE_BYTE_SOURCE_HINT_SCHEMA: &str = "molten.chunk-store.remote-byte-source-hint.v1";
const REMOTE_BYTE_SOURCE_READBACK_SCHEMA: &str = "molten.chunk-store.remote-byte-source-readback-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteByteSourceHintInput<'a> {
    pub manifest_ref: &'a str,
    pub location: &'a str,
    pub outboard_ref: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteByteSourceHint {
    pub hint_ref: String,
    pub manifest_ref: String,
    pub location: String,
    pub outboard_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRangeReadbackInput<'a> {
    pub hint: &'a RemoteByteSourceHint,
    pub manifest: &'a crate::blocks::ChunkManifest,
    pub offset: u64,
    pub length: u64,
    pub chunk_bytes: &'a [(String, Vec<u8>)],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRangeReadbackReceipt {
    pub decision: String,
    pub bytes: Vec<u8>,
    pub diagnostics: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

pub fn remote_byte_source_hint(input: &RemoteByteSourceHintInput<'_>) -> crate::error::Result<RemoteByteSourceHint> {
    crate::preserves_rail::validate_content_ref(input.manifest_ref)?;
    crate::preserves_rail::validate_content_ref(input.outboard_ref)?;
    crate::blocks::planner::validate_chunk_refs(input.evidence_refs, "remote byte source evidence ref")?;
    validate_remote_location(input.location)?;
    let value = crate::preserves_rail::record("remote-byte-source-hint-v1", vec![
        crate::preserves_rail::string(REMOTE_BYTE_SOURCE_HINT_SCHEMA),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(input.manifest_ref)]),
        crate::preserves_rail::record("location", vec![crate::preserves_rail::string(input.location)]),
        crate::preserves_rail::record("outboard", vec![crate::preserves_rail::string(input.outboard_ref)]),
        crate::preserves_rail::record("evidence", vec![crate::blocks::wire::encode::string_sequence(
            input.evidence_refs,
        )]),
        crate::preserves_rail::record("caveat", vec![crate::preserves_rail::string(
            "remote byte-source hints are locations, not object identity or mutation authority",
        )]),
    ]);
    let hint_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(RemoteByteSourceHint {
        hint_ref,
        manifest_ref: input.manifest_ref.to_string(),
        location: input.location.to_string(),
        outboard_ref: input.outboard_ref.to_string(),
        value,
    })
}

pub fn verify_remote_range_readback(
    input: &RemoteRangeReadbackInput<'_>,
) -> crate::error::Result<RemoteRangeReadbackReceipt> {
    crate::blocks::planner::validate_chunk_manifest_identity(input.manifest)?;
    crate::bounded::ensure_count_at_most(
        input.chunk_bytes.len(),
        crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
        "remote byte-source supplied chunks",
    )?;
    let mut diagnostics = Vec::with_capacity(crate::blocks::planner::CHUNK_SYNC_DIAGNOSTIC_CAPACITY);
    if input.hint.manifest_ref != input.manifest.manifest_ref {
        diagnostics.push("remote byte-source hint manifest does not match requested manifest".to_string());
    }
    let required_chunks =
        required_chunks_for_remote_range(input.manifest, input.offset, input.length, &mut diagnostics)?;
    let mut bytes = verified_range_bytes(input, &required_chunks, &mut diagnostics)?;
    if !diagnostics.is_empty() {
        bytes.clear();
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = readback_receipt(input, decision, &required_chunks, &diagnostics);
    Ok(RemoteRangeReadbackReceipt {
        decision: decision.to_string(),
        bytes,
        diagnostics,
        receipt_value,
    })
}

/// Concatenates the supplied bytes of every required chunk whose content ref verifies; each
/// mismatch or missing chunk becomes a diagnostic instead.
fn verified_range_bytes(
    input: &RemoteRangeReadbackInput<'_>,
    required_chunks: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<Vec<u8>> {
    let supplied = input
        .chunk_bytes
        .iter()
        .map(|(reference, chunk_bytes)| (reference.as_str(), chunk_bytes.as_slice()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut bytes = Vec::new();
    for chunk_ref in required_chunks {
        match supplied.get(chunk_ref.as_str()) {
            Some(chunk_bytes) => {
                crate::bounded::ensure_count_at_most(
                    chunk_bytes.len(),
                    crate::blocks::limits::MAX_CHUNK_STORE_OBJECT_BYTES,
                    "remote byte-source chunk bytes",
                )?;
                let chunk_size = usize::try_from(input.manifest.chunk_size).map_err(|error| {
                    crate::error::Failure::invalid_harness(format!(
                        "remote byte-source chunk size cannot fit usize: {error}"
                    ))
                })?;
                let actual_ref = crate::blocks::hash_chunk(chunk_bytes, chunk_size);
                if actual_ref != *chunk_ref {
                    diagnostics.push_item(format!("remote byte-source chunk {chunk_ref} verified as {actual_ref}"));
                } else {
                    crate::blocks::limits::extend_bytes_bounded(
                        &mut bytes,
                        chunk_bytes,
                        crate::blocks::limits::MAX_CHUNK_STORE_OBJECT_BYTES,
                        "remote byte-source readback bytes",
                    )?;
                }
            }
            None => diagnostics.push_item(format!("remote byte-source missing required chunk {chunk_ref}")),
        }
    }
    Ok(bytes)
}

fn readback_receipt(
    input: &RemoteRangeReadbackInput<'_>,
    decision: &str,
    required_chunks: &[String],
    diagnostics: &[String],
) -> preserves::IOValue {
    crate::preserves_rail::record("remote-byte-source-readback-receipt-v1", vec![
        crate::preserves_rail::string(REMOTE_BYTE_SOURCE_READBACK_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("hint", vec![crate::preserves_rail::string(&input.hint.hint_ref)]),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(&input.manifest.manifest_ref)]),
        crate::preserves_rail::record("offset", vec![crate::preserves_rail::string(input.offset.to_string())]),
        crate::preserves_rail::record("length", vec![crate::preserves_rail::string(input.length.to_string())]),
        crate::preserves_rail::record("required-chunks", vec![crate::blocks::wire::encode::string_sequence(
            required_chunks,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::blocks::wire::encode::string_sequence(diagnostics)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::blocks::wire::encode::check_record("source-hint-not-identity", "pass"),
            crate::blocks::wire::encode::check_record(
                "range-verified-before-exposure",
                if diagnostics.is_empty() { "pass" } else { "fail" },
            ),
        ])]),
    ])
}

fn validate_remote_location(location: &str) -> crate::error::Result<()> {
    if location.starts_with("s3://")
        || location.starts_with("http://")
        || location.starts_with("https://")
        || location.starts_with("iroh://")
    {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "unsupported remote byte-source location {location}"
        )))
    }
}

fn required_chunks_for_remote_range(
    manifest: &crate::blocks::ChunkManifest,
    offset: u64,
    length: u64,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<Vec<String>> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| crate::error::Failure::invalid_harness("remote byte-source range end overflow"))?;
    if end > manifest.total_len {
        diagnostics.push_item("remote byte-source range exceeds manifest length".to_string());
    }
    let mut refs = Vec::with_capacity(manifest.chunks.len());
    let mut chunk_start = 0_u64;
    for chunk in &manifest.chunks {
        let chunk_end = chunk_start
            .checked_add(chunk.length)
            .ok_or_else(|| crate::error::Failure::invalid_harness("remote byte-source chunk range overflow"))?;
        if ranges_overlap(offset, end, chunk_start, chunk_end) {
            refs.push(chunk.chunk_ref.clone());
        }
        chunk_start = chunk_end;
    }
    Ok(refs)
}

fn ranges_overlap(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
    left_start < right_end && right_start < left_end
}
