
pub fn decide_index(input: &IndexInput) -> Result<IndexDecision> {
    let mut diagnostics = Vec::new();
    collect_ref_diagnostics(std::slice::from_ref(&input.bundle_ref), "bundle", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.requester_ref), "requester", &mut diagnostics)?;
    collect_visibility_diagnostics(&input.visibility, &mut diagnostics)?;
    validate_count(input.members.len(), MAX_MEMBERS, "gateway index member")?;
    let hidden = input.visibility.hidden_refs.iter().collect::<Set<_>>();
    let entry_capacity = input.members.len().min(MAX_MEMBERS);
    let mut entries = Vec::with_capacity(entry_capacity);
    for member in &input.members {
        validate_member(member, &mut diagnostics)?;
        if !member.visible || hidden.contains(&member.object_ref) {
            push_diagnostic(&mut diagnostics, "hidden member omitted without leaking ref")?;
            continue;
        }
        let should_redact = member.sensitive
            && (input.visibility.profile == PUBLIC_PROFILE || input.visibility.profile == DIAGNOSTIC_PROFILE)
            && !input.visibility.allow_sensitive_names;
        if should_redact {
            entries.push(IndexEntry {
                name: "redacted".to_string(),
                object_ref: None,
                size: None,
                mime_hint: None,
                redacted: true,
            });
        } else {
            entries.push(IndexEntry {
                name: member.name.clone(),
                object_ref: Some(member.object_ref.clone()),
                size: Some(member.size),
                mime_hint: member.mime_hint.clone(),
                redacted: false,
            });
        }
    }
    let decision = if diagnostics.iter().any(|diagnostic| diagnostic.contains("invalid")) {
        "deny"
    } else {
        "pass"
    }
    .to_string();
    let receipt_value = index_receipt_value(IndexReceiptInput {
        decision: &decision,
        bundle_ref: &input.bundle_ref,
        requester_ref: &input.requester_ref,
        visibility: &input.visibility,
        entries: &entries,
        diagnostics: &diagnostics,
    })?;
    Ok(IndexDecision {
        decision,
        bundle_ref: input.bundle_ref.clone(),
        entries,
        diagnostics,
        receipt_value,
    })
}

pub fn receipt_authorizes_mutation(_receipt: &IoValue) -> bool {
    false
}

fn normalize_range(
    manifest: Option<&ChunkManifest>,
    requested: Option<Range>,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<Option<Range>> {
    let Some(manifest) = manifest else {
        if requested.is_some() {
            push_diagnostic(diagnostics, "range request requires a chunk manifest before lookup")?;
        }
        return Ok(None);
    };
    let total_len = manifest.total_len;
    let range = requested.unwrap_or(Range {
        offset: RANGE_START,
        length: total_len,
    });
    let Some(end) = range.offset.checked_add(range.length) else {
        push_diagnostic(diagnostics, "range offset and length overflow")?;
        return Ok(Some(range));
    };
    if range.offset > total_len || end > total_len {
        push_diagnostic(diagnostics, "range outside object length")?;
    }
    Ok(Some(range))
}

fn required_chunks_for_range(
    manifest: &ChunkManifest,
    range: Range,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<Vec<String>> {
    if range.length == EMPTY_RANGE_LENGTH {
        return Ok(Vec::new());
    }
    let chunk_size = usize::try_from(manifest.chunk_size)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway chunk size unsupported: {error}")))?;
    if chunk_size < MIN_CHUNK_SIZE {
        push_diagnostic(diagnostics, "manifest chunk size must be non-zero")?;
        return Ok(Vec::new());
    }
    let offset = usize::try_from(range.offset)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range offset unsupported: {error}")))?;
    let end = usize::try_from(range.offset + range.length)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range end unsupported: {error}")))?;
    let first = offset
        .checked_div(chunk_size)
        .ok_or_else(|| MoltenError::invalid_harness("gateway chunk size must be non-zero"))?;
    let last_exclusive = end.div_ceil(chunk_size);
    let chunk_count = last_exclusive.saturating_sub(first);
    let mut refs = Vec::with_capacity(chunk_count);
    for index in first..last_exclusive {
        let Some(chunk) = manifest.chunks.get(index) else {
            push_diagnostic(diagnostics, "range maps to missing manifest chunk")?;
            continue;
        };
        refs.push(chunk.chunk_ref.clone());
    }
    Ok(refs)
}

fn reconstruct_verified_range(
    manifest: &ChunkManifest,
    range: Range,
    chunks: &Map<String, Vec<u8>>,
    chunk_size: usize,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let offset = usize::try_from(range.offset)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range offset unsupported: {error}")))?;
    let end = usize::try_from(range.offset + range.length)
        .map_err(|error| MoltenError::invalid_harness(format!("gateway range end unsupported: {error}")))?;
    if range.length == EMPTY_RANGE_LENGTH {
        return Ok(output);
    }
    let first = offset
        .checked_div(chunk_size)
        .ok_or_else(|| MoltenError::invalid_harness("gateway chunk size must be non-zero"))?;
    let last_exclusive = end.div_ceil(chunk_size);
    for index in first..last_exclusive {
        let Some(chunk) = manifest.chunks.get(index) else {
            push_diagnostic(diagnostics, "range maps to missing manifest chunk")?;
            continue;
        };
        let Some(bytes) = chunks.get(&chunk.chunk_ref) else {
            push_diagnostic(diagnostics, "missing chunk denies before response")?;
            continue;
        };
        let actual_ref = hash_fixed_chunk(bytes, chunk_size);
        if actual_ref != chunk.chunk_ref {
            push_diagnostic(diagnostics, "corrupt chunk denies before response")?;
            continue;
        }
        if bytes.len() as u64 != chunk.length {
            push_diagnostic(diagnostics, "wrong chunk length denies before response")?;
            continue;
        }
        let chunk_start = index * chunk_size;
        let wanted_start = offset.saturating_sub(chunk_start);
        let wanted_end = end.saturating_sub(chunk_start).min(bytes.len());
        output.extend_from_slice(&bytes[wanted_start..wanted_end]);
        if output.len() > MAX_CHUNK_BYTES {
            push_diagnostic(diagnostics, "gateway reconstructed bytes exceed bound")?;
            return Ok(Vec::new());
        }
    }
    if output.len() as u64 != range.length {
        push_diagnostic(diagnostics, "range reconstruction length mismatch")?;
    }
    Ok(output)
}

fn hash_fixed_chunk(bytes: &[u8], chunk_size: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten.chunk-store.chunk.fixed_v1\0");
    hasher.update(format!("molten.chunk-store.chunk.fixed_v1:{chunk_size}").as_bytes());
    hasher.update(b"\0");
    hasher.update(bytes);
    content_ref_from_blake3_hash(hasher.finalize())
}

struct ReadReceiptInput<'a> {
    decision: &'a str,
    object_ref: &'a str,
    member: Option<&'a str>,
    requester_ref: &'a str,
    normalized_range: Option<Range>,
    required_chunk_refs: &'a [String],
    visibility: &'a Visibility,
    diagnostics: &'a [String],
}

fn read_receipt_value(input: ReadReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("operator-gateway-read-receipt-v1", vec![
        string(READ_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("object", vec![string(input.object_ref)]),
        record("member", vec![optional_string_value(input.member)]),
        record("requester", vec![string(input.requester_ref)]),
        record("range", vec![range_value(input.normalized_range)]),
        record("required-chunks", vec![refs_value(input.required_chunk_refs)?]),
        visibility_value(input.visibility)?,
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("readback-decision-before-io", pass_fail(input.decision == "pass")),
            ("visibility-retention-checked", pass_fail(input.decision == "pass")),
            ("gateway-receipt-evidence-only", "pass"),
        ]),
        record("caveat", vec![string(EVIDENCE_ONLY_CAVEAT)]),
    ]))
}

struct RangeReceiptInput<'a> {
    decision: &'a str,
    manifest_ref: &'a str,
    normalized_range: Range,
    chunk_refs: &'a [String],
    diagnostics: &'a [String],
}

fn range_receipt_value(input: RangeReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("operator-gateway-range-receipt-v1", vec![
        string(RANGE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("range", vec![range_value(Some(input.normalized_range))]),
        record("chunks", vec![refs_value(input.chunk_refs)?]),
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("manifest-ref-bound", "pass"),
            ("range-normalized", "pass"),
            ("chunks-verified-before-bytes", pass_fail(input.decision == "pass")),
            ("gateway-receipt-evidence-only", "pass"),
        ]),
    ]))
}

struct IndexReceiptInput<'a> {
    decision: &'a str,
    bundle_ref: &'a str,
    requester_ref: &'a str,
    visibility: &'a Visibility,
    entries: &'a [IndexEntry],
    diagnostics: &'a [String],
}

fn index_receipt_value(input: IndexReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("operator-gateway-index-receipt-v1", vec![
        string(INDEX_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("requester", vec![string(input.requester_ref)]),
        visibility_value(input.visibility)?,
        record("entries", vec![sequence(input.entries.iter().map(index_entry_value).collect())]),
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("read-only-index", "pass"),
            ("hidden-members-omitted", "pass"),
            ("sensitive-metadata-redacted", "pass"),
            ("gateway-receipt-evidence-only", "pass"),
        ]),
        record("caveat", vec![string(EVIDENCE_ONLY_CAVEAT)]),
    ]))
}

fn visibility_value(visibility: &Visibility) -> Result<IoValue> {
    Ok(record("visibility", vec![
        record("profile", vec![string(&visibility.profile)]),
        record("policy", vec![refs_value(&visibility.visibility_policy_refs)?]),
        record("retention", vec![refs_value(&visibility.retention_refs)?]),
        record("reveal", vec![refs_value(&visibility.reveal_refs)?]),
        record("redaction", vec![refs_value(&visibility.redaction_refs)?]),
    ]))
}

fn index_entry_value(entry: &IndexEntry) -> IoValue {
    record("entry", vec![
        record("name", vec![string(&entry.name)]),
        record("object", vec![optional_string_value(entry.object_ref.as_deref())]),
        record("size", vec![optional_u64_value(entry.size)]),
        record("mime", vec![optional_string_value(entry.mime_hint.as_deref())]),
        record("redacted", vec![string(if entry.redacted { "true" } else { "false" })]),
    ])
}

fn range_value(range: Option<Range>) -> IoValue {
    match range {
        Some(range) => record("some", vec![
            record("offset", vec![u64_value(range.offset)]),
            record("length", vec![u64_value(range.length)]),
        ]),
        None => record("none", Vec::new()),
    }
}
