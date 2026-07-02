
pub fn read_object(root: &Path, manifest_ref: &str) -> Result<ChunkStoreRead> {
    let manifest = match read_manifest(root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("fetch", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value = denial_receipt_value("fetch", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
            ("transform-mode", "fail"),
            ("deny-unsupported-transform", "pass"),
        ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let bytes = match reconstruct_object(root, &manifest) {
        Ok(bytes) => bytes,
        Err(error) => {
            let receipt_value =
                denial_receipt_value("fetch", Some(&manifest.manifest_ref), &chunk_refs, error.to_string(), vec![
                    ("streaming-chunk-verification", "fail"),
                    ("deny-corrupt-or-missing-chunk", "pass"),
                ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "fetch",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &chunk_refs,
        checks: vec![
            ("manifest-ref-binding", "pass"),
            ("streaming-chunk-verification", "pass"),
            ("reconstructed-total-length", "pass"),
            ("redb-index-availability", "pass"),
        ],
        details: vec![record("total-len", vec![u64_value(bytes.len() as u64)])],
    });
    index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(ChunkStoreRead {
        manifest_ref: manifest.manifest_ref,
        bytes,
        receipt_value,
    })
}

struct SpanInput<'a> {
    root: &'a Path,
    manifest: &'a ChunkManifest,
    refs: &'a [String],
    offset: u64,
    length: u64,
}

struct SpanWindow {
    chunk_size: usize,
    offset: usize,
    end: usize,
    first: usize,
    last_exclusive: usize,
    expected_len: usize,
}

struct SpanData {
    bytes: Vec<u8>,
    refs: Vec<String>,
}

fn span_window(manifest: &ChunkManifest, offset: u64, length: u64) -> Result<SpanWindow> {
    let chunk_size = usize::try_from(manifest.chunk_size).map_err(|error| {
        MoltenError::invalid_harness(format!("manifest chunk size is unsupported on this platform: {error}"))
    })?;
    if chunk_size == 0 {
        return Err(MoltenError::invalid_harness("manifest chunk size must be greater than zero"));
    }
    let offset_usize = usize::try_from(offset).map_err(|error| {
        MoltenError::invalid_harness(format!("range offset is unsupported on this platform: {error}"))
    })?;
    let length_usize = usize::try_from(length).map_err(|error| {
        MoltenError::invalid_harness(format!("range length is unsupported on this platform: {error}"))
    })?;
    ensure_count_at_most(length_usize, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store range bytes")?;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| MoltenError::invalid_harness("range offset and length overflow"))?;
    let end_usize = usize::try_from(end)
        .map_err(|error| MoltenError::invalid_harness(format!("range end is unsupported on this platform: {error}")))?;
    let first = offset_usize
        .checked_div(chunk_size)
        .ok_or_else(|| MoltenError::invalid_harness("range chunk size must be non-zero"))?;
    Ok(SpanWindow {
        chunk_size,
        offset: offset_usize,
        end: end_usize,
        first,
        last_exclusive: end_usize.div_ceil(chunk_size),
        expected_len: length_usize,
    })
}

fn read_span(input: SpanInput<'_>) -> Result<SpanData> {
    let mut bytes = Vec::new();
    let mut refs = Vec::new();
    let mut expected_len = 0usize;
    if input.length > 0 {
        let window = span_window(input.manifest, input.offset, input.length)?;
        expected_len = window.expected_len;
        for index in window.first..window.last_exclusive {
            let chunk =
                input.manifest.chunks.get(index).ok_or_else(|| {
                    MoltenError::invalid_harness(format!("range maps to missing chunk index {index}"))
                })?;
            let chunk_bytes = match read_verified_chunk(input.root, chunk, window.chunk_size) {
                Ok(bytes) => bytes,
                Err(error) => {
                    let receipt_value = denial_receipt_value(
                        "range-read",
                        Some(&input.manifest.manifest_ref),
                        input.refs,
                        error.to_string(),
                        vec![
                            ("range-chunk-verification", "fail"),
                            ("deny-corrupt-or-missing-chunk", "pass"),
                        ],
                    );
                    store_receipt(input.root, &receipt_value)?;
                    return Err(error);
                }
            };
            let chunk_start = index * window.chunk_size;
            let wanted_start = window.offset.saturating_sub(chunk_start);
            let wanted_end = window.end.saturating_sub(chunk_start).min(chunk_bytes.len());
            extend_bytes_bounded(
                &mut bytes,
                &chunk_bytes[wanted_start..wanted_end],
                MAX_CHUNK_STORE_OBJECT_BYTES,
                "chunk store range bytes",
            )?;
            push_bounded(
                &mut refs,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store range touched chunks",
            )?;
        }
    }
    if bytes.len() != expected_len {
        let message = format!("range reconstruction length mismatch: got {}, expected {}", bytes.len(), input.length);
        let receipt_value =
            denial_receipt_value("range-read", Some(&input.manifest.manifest_ref), input.refs, &message, vec![
                ("range-chunk-verification", "fail"),
                ("deny-range-reconstruction-mismatch", "pass"),
            ]);
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(SpanData { bytes, refs })
}

pub fn range_read(root: &Path, manifest_ref: &str, offset: u64, length: u64) -> Result<ChunkStoreRangeRead> {
    let manifest = match read_manifest(root, manifest_ref) {
        Ok(manifest) => manifest,
        Err(error) => {
            let receipt_value = denial_receipt_value("range-read", Some(manifest_ref), &[], error.to_string(), vec![
                ("manifest-ref-binding", "fail"),
                ("deny-missing-or-invalid-manifest", "pass"),
            ]);
            store_receipt(root, &receipt_value)?;
            return Err(error);
        }
    };
    let chunk_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    if offset > manifest.total_len || offset.saturating_add(length) > manifest.total_len {
        let message =
            format!("range {offset}..{} outside object length {}", offset.saturating_add(length), manifest.total_len);
        let receipt_value =
            denial_receipt_value("range-read", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("range-bounds", "fail"),
                ("deny-out-of-bounds-range", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    if let Some(message) = unsupported_transform_message(&manifest) {
        let receipt_value =
            denial_receipt_value("range-read", Some(&manifest.manifest_ref), &chunk_refs, &message, vec![
                ("transform-mode", "fail"),
                ("deny-unsupported-transform", "pass"),
            ]);
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    let span = read_span(SpanInput {
        root,
        manifest: &manifest,
        refs: &chunk_refs,
        offset,
        length,
    })?;
    let receipt_value = receipt_value(ChunkStoreReceiptValueInput {
        operation: "range-read",
        decision: "pass",
        manifest_ref: Some(&manifest.manifest_ref),
        chunk_refs: &span.refs,
        checks: vec![
            ("range-bounds", "pass"),
            ("chunk-order-root", "pass"),
            ("range-chunk-verification", "pass"),
            ("redb-index-availability", "pass"),
        ],
        details: vec![
            record("offset", vec![u64_value(offset)]),
            record("length", vec![u64_value(length)]),
        ],
    });
    index_manifest_available(root, &manifest, &receipt_value)?;
    Ok(ChunkStoreRangeRead {
        manifest_ref: manifest.manifest_ref,
        offset,
        length,
        bytes: span.bytes,
        receipt_value,
    })
}

pub fn missing_chunks(root: &Path, manifest_ref: &str) -> Result<Vec<String>> {
    let manifest = read_manifest(root, manifest_ref)?;
    let mut missing = Vec::new();
    let mut available = Vec::new();
    for chunk in &manifest.chunks {
        if chunk_path(root, &chunk.chunk_ref)?.exists() {
            push_bounded(
                &mut available,
                chunk.chunk_ref.clone(),
                MAX_CHUNK_STORE_CHUNKS,
                "chunk store available chunks",
            )?;
        } else {
            push_bounded(&mut missing, chunk.chunk_ref.clone(), MAX_CHUNK_STORE_CHUNKS, "chunk store missing chunks")?;
        }
    }
    index_set_manifest_chunk_availability(root, &manifest, &available, &missing, None)?;
    Ok(missing)
}
