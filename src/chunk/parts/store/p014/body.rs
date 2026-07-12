fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, maximum, label)
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    crate::bounded::checked_count_sum(left, right, maximum, label)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::push_bounded(values, value, maximum, label)
}

fn extend_bytes_bounded(
    bytes: &mut impl crate::bounded::VecSink<u8>,
    incoming: &[u8],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(bytes.item_count(), incoming.len(), maximum, label)?;
    bytes.reserve_items(final_count.saturating_sub(bytes.item_count()));
    bytes.extend_cloned_items(incoming);
    Ok(())
}

fn insert_set_bounded<T: Ord>(values: &mut OrderedSet<T>, value: T, maximum: usize, label: &str) -> Result<bool> {
    if !values.contains(&value) {
        checked_count_sum(values.len(), 1, maximum, label)?;
    }
    Ok(values.insert(value))
}

fn chunk_count_for_len(byte_len: usize, chunk_size: usize, label: &str) -> Result<usize> {
    if chunk_size == 0 {
        return Err(MoltenError::invalid_harness(format!("{label} chunk size must be non-zero")));
    }
    let count = byte_len.div_ceil(chunk_size);
    ensure_count_at_most(count, MAX_CHUNK_STORE_CHUNKS, label)?;
    Ok(count)
}

fn chunk_size_to_usize(chunk_size: u64, label: &str) -> Result<usize> {
    usize::try_from(chunk_size)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} is unsupported on this platform: {error}")))
}

fn validate_put_bounds(byte_len: usize, chunk_size: usize, policy_ref_count: usize) -> Result<()> {
    ensure_count_at_most(byte_len, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    chunk_count_for_len(byte_len, chunk_size, "chunk store object chunks")?;
    ensure_count_at_most(policy_ref_count, MAX_CHUNK_STORE_REFS, "chunk store policy refs")?;
    Ok(())
}

fn reconstruct_object(root: &CapabilityChunkRoot, manifest: &ChunkManifest) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    verify_manifest_chunks_into(root, manifest, &mut bytes)?;
    Ok(bytes)
}

fn verify_manifest_chunks(root: &CapabilityChunkRoot, manifest: &ChunkManifest) -> Result<()> {
    let mut sink = Vec::new();
    verify_manifest_chunks_into(root, manifest, &mut sink)
}

fn verify_manifest_chunks_into(
    root: &CapabilityChunkRoot,
    manifest: &ChunkManifest,
    bytes: &mut impl crate::bounded::VecSink<u8>,
) -> Result<()> {
    let total_len = usize::try_from(manifest.total_len).map_err(|error| {
        MoltenError::invalid_harness(format!("chunk manifest total length is unsupported on this platform: {error}"))
    })?;
    ensure_count_at_most(total_len, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    let chunk_size = chunk_size_to_usize(manifest.chunk_size, "manifest chunk size")?;
    for chunk in &manifest.chunks {
        let chunk_bytes = read_verified_chunk(root, chunk, chunk_size)?;
        extend_bytes_bounded(&mut *bytes, &chunk_bytes, MAX_CHUNK_STORE_OBJECT_BYTES, "chunk store object bytes")?;
    }
    let actual_len = u64::try_from(bytes.item_count()).map_err(|error| {
        MoltenError::invalid_harness(format!("chunk manifest actual length is unsupported on this platform: {error}"))
    })?;
    if actual_len != manifest.total_len {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest total length mismatch: got {}, expected {}",
            actual_len, manifest.total_len
        )));
    }
    Ok(())
}

fn read_verified_chunk(root: &CapabilityChunkRoot, chunk: &ChunkRef, chunk_size: usize) -> Result<Vec<u8>> {
    let path = chunk_path(&chunk.chunk_ref)?;
    let bytes = root.root().read(&path)?;
    let actual_ref = hash_chunk(&bytes, chunk_size);
    if actual_ref != chunk.chunk_ref {
        return Err(MoltenError::invalid_harness(format!(
            "chunk hash mismatch: got {actual_ref}, expected {}",
            chunk.chunk_ref
        )));
    }
    if bytes.len() as u64 != chunk.length {
        return Err(MoltenError::invalid_harness(format!(
            "chunk length mismatch: got {}, expected {}",
            bytes.len(),
            chunk.length
        )));
    }
    Ok(bytes)
}

fn verify_raw_chunk_file(
    root: &CapabilityChunkRoot,
    path: &StorePath,
    chunk_ref: &str,
    length: u64,
    chunk_size: usize,
) -> Result<()> {
    let bytes = root.root().read(path)?;
    verify_raw_chunk_bytes(&bytes, chunk_ref, length, chunk_size).map_err(|error| {
        MoltenError::invalid_harness(format!("chunk store content path for {chunk_ref} is invalid: {error}"))
    })
}

fn verify_raw_chunk_bytes(bytes: &[u8], chunk_ref: &str, length: u64, chunk_size: usize) -> Result<()> {
    let actual_ref = hash_chunk(bytes, chunk_size);
    if actual_ref != chunk_ref {
        return Err(MoltenError::invalid_harness(format!(
            "chunk bytes hash mismatch: got {actual_ref}, expected {chunk_ref}"
        )));
    }
    if bytes.len() as u64 != length {
        return Err(MoltenError::invalid_harness(format!("chunk bytes length {}, expected {length}", bytes.len())));
    }
    Ok(())
}

fn validate_fixed_chunk_lengths(total_len: u64, chunk_size: u64, chunks: &[ChunkRef]) -> Result<()> {
    let reconstructed = chunks.iter().map(|chunk| chunk.length).sum::<u64>();
    if reconstructed != total_len {
        return Err(MoltenError::invalid_harness(format!(
            "chunk manifest total length mismatch: refs sum to {reconstructed}, expected {total_len}"
        )));
    }
    if total_len == 0 {
        if !chunks.is_empty() {
            return Err(MoltenError::invalid_harness("empty object manifest must not contain chunks"));
        }
        return Ok(());
    }
    for (index, chunk) in chunks.iter().enumerate() {
        let is_last = index + 1 == chunks.len();
        if !is_last && chunk.length != chunk_size {
            return Err(MoltenError::invalid_harness(format!(
                "non-final fixed_v1 chunk {index} has length {}, expected {chunk_size}",
                chunk.length
            )));
        }
        if is_last && (chunk.length == 0 || chunk.length > chunk_size) {
            return Err(MoltenError::invalid_harness(format!(
                "final fixed_v1 chunk {index} has invalid length {} for chunk size {chunk_size}",
                chunk.length
            )));
        }
    }
    Ok(())
}

fn hash_chunk(bytes: &[u8], chunk_size: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten.chunk-store.chunk.fixed_v1\0");
    hasher.update(chunk_domain(chunk_size).as_bytes());
    hasher.update(b"\0");
    hasher.update(bytes);
    content_ref_from_blake3_hash(hasher.finalize())
}

fn chunk_domain(chunk_size: usize) -> String {
    format!("molten.chunk-store.chunk.fixed_v1:{chunk_size}")
}

fn hash_blob_bytes(bytes: &[u8]) -> String {
    content_ref_from_bytes(bytes)
}

fn ensure_iroh_dirs(root: &CapabilityChunkRoot) -> Result<()> {
    root.root().create_dir_all(&store_path("blobs")?)?;
    root.root().create_dir_all(&store_path("tickets")?)
}

fn ensure_dirs(root: &CapabilityChunkRoot) -> Result<()> {
    root.root().create_dir_all(&store_path("chunks")?)?;
    root.root().create_dir_all(&store_path("manifests")?)?;
    root.root().create_dir_all(&store_path("metadata")?)?;
    root.root().create_dir_all(&store_path("pins/manifests")?)?;
    root.root().create_dir_all(&store_path("pins/chunks")?)
}

fn chunk_path(chunk_ref: &str) -> Result<StorePath> {
    store_path("chunks")?.join(&filename_for_ref(chunk_ref)?)
}

#[cfg(test)]
fn test_chunk_path(root: &Path, chunk_ref: &str) -> Result<PathBuf> {
    Ok(root.join("chunks").join(filename_for_ref(chunk_ref)?))
}

fn iroh_blob_path(blob_ref: &str) -> Result<StorePath> {
    store_path("blobs")?.join(&filename_for_ref(blob_ref)?)
}

fn iroh_ticket_path(manifest_ref: &str) -> Result<StorePath> {
    store_path("tickets")?.join(&filename_for_ref(manifest_ref)?)
}

fn manifest_path(manifest_ref: &str) -> Result<StorePath> {
    store_path("manifests")?.join(&filename_for_ref(manifest_ref)?)
}

#[cfg(test)]
fn test_manifest_path(root: &Path, manifest_ref: &str) -> Result<PathBuf> {
    Ok(root.join("manifests").join(filename_for_ref(manifest_ref)?))
}

fn metadata_path(metadata_ref: &str) -> Result<StorePath> {
    store_path("metadata")?.join(&filename_for_ref(metadata_ref)?)
}

fn manifest_pin_path(manifest_ref: &str) -> Result<StorePath> {
    store_path("pins/manifests")?.join(&filename_for_ref(manifest_ref)?)
}

fn chunk_pin_path(chunk_ref: &str) -> Result<StorePath> {
    store_path("pins/chunks")?.join(&filename_for_ref(chunk_ref)?)
}

fn filename_for_ref(reference: &str) -> Result<String> {
    let hex = content_ref_hex(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("unsupported chunk-store ref {reference}: {error}")))?;
    Ok(format!("blake3_{hex}.bin"))
}

fn ref_from_filename(filename: &str) -> Option<String> {
    let hex = filename.strip_prefix("blake3_").and_then(|value| value.strip_suffix(".bin"))?;
    content_ref_from_hex(hex).ok()
}

fn refs_from_dir(root: &CapabilityChunkRoot, dir: &StorePath) -> Result<Vec<String>> {
    if !root.root().try_exists(dir)? {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in root.root().list_entries(dir)? {
        if entry.kind != StoreEntryKind::File {
            return Err(MoltenError::invalid_harness(format!(
                "chunk store directory entry {} must be a regular file, got {:?}",
                entry.path.display(),
                entry.kind
            )));
        }
        if let Some(reference) = ref_from_filename(&entry.name) {
            push_bounded(&mut refs, reference, MAX_CHUNK_STORE_REFS, "chunk store refs from directory")?;
        }
    }
    Ok(refs)
}

fn pinned_refs(root: &CapabilityChunkRoot, dir: &StorePath) -> Result<Vec<String>> {
    if !root.root().try_exists(dir)? {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in root.root().list_entries(dir)? {
        if entry.kind != StoreEntryKind::File {
            return Err(MoltenError::invalid_harness(format!(
                "chunk pin entry {} must be a regular file, got {:?}",
                entry.path.display(),
                entry.kind
            )));
        }
        push_bounded(
            &mut refs,
            root.root().read_to_string(&entry.path)?,
            MAX_CHUNK_STORE_REFS,
            "chunk store pinned refs",
        )?;
    }
    refs.sort();
    Ok(refs)
}

fn write_immutable_bytes(
    root: &CapabilityChunkRoot,
    path: &StorePath,
    bytes: &[u8],
    expected_ref: &str,
    parser: fn(&[u8]) -> Result<IoValue>,
) -> Result<()> {
    if root.root().try_exists(path)? {
        let existing = root.root().read(path)?;
        let existing_value = parser(&existing)?;
        let existing_ref = canonical_hash(&existing_value)?;
        if existing_ref != expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "immutable content path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        root.root().write(path, bytes)?;
    }
    Ok(())
}
