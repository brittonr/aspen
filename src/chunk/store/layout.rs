//! On-disk layout of a chunk root: directories, content-addressed file names, and immutable writes.

pub(super) fn path(path: &str) -> crate::error::Result<crate::local_store::Locator> {
    crate::local_store::Locator::parse(path)
}

pub(super) fn ensure_iroh_dirs(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<()> {
    root.root().create_dir_all(&path("blobs")?)?;
    root.root().create_dir_all(&path("tickets")?)
}

pub(super) fn ensure_dirs(root: &crate::blocks::CapabilityChunkRoot) -> crate::error::Result<()> {
    root.root().create_dir_all(&path("chunks")?)?;
    root.root().create_dir_all(&path("manifests")?)?;
    root.root().create_dir_all(&path("metadata")?)?;
    root.root().create_dir_all(&path("pins/manifests")?)?;
    root.root().create_dir_all(&path("pins/chunks")?)
}

pub(super) fn chunk_path(chunk_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("chunks")?.join(&filename_for_ref(chunk_ref)?)
}

pub(super) fn iroh_blob_path(blob_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("blobs")?.join(&filename_for_ref(blob_ref)?)
}

pub(super) fn iroh_ticket_path(manifest_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("tickets")?.join(&filename_for_ref(manifest_ref)?)
}

pub(super) fn manifest_path(manifest_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("manifests")?.join(&filename_for_ref(manifest_ref)?)
}

pub(super) fn metadata_path(metadata_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("metadata")?.join(&filename_for_ref(metadata_ref)?)
}

pub(super) fn manifest_pin_path(manifest_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("pins/manifests")?.join(&filename_for_ref(manifest_ref)?)
}

pub(super) fn chunk_pin_path(chunk_ref: &str) -> crate::error::Result<crate::local_store::Locator> {
    path("pins/chunks")?.join(&filename_for_ref(chunk_ref)?)
}

pub(super) fn filename_for_ref(reference: &str) -> crate::error::Result<String> {
    let hex = crate::preserves_rail::content_ref_hex(reference).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("unsupported chunk-store ref {reference}: {error}"))
    })?;
    Ok(format!("blake3_{hex}.bin"))
}

// r[impl molten.runtime_spine.canonical_content_refs.filename_readback]
fn ref_from_filename(filename: &str) -> Option<String> {
    let hex = filename.strip_prefix("blake3_").and_then(|value| value.strip_suffix(".bin"))?;
    crate::preserves_rail::content_ref_from_hex(hex).ok()
}

pub(super) fn refs_from_dir(
    root: &crate::blocks::CapabilityChunkRoot,
    dir: &crate::local_store::Locator,
) -> crate::error::Result<Vec<String>> {
    if !root.root().try_exists(dir)? {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in root.root().list_entries(dir)? {
        if entry.kind != crate::local_store::MemberKind::File {
            return Err(crate::error::Failure::invalid_harness(format!(
                "chunk store directory entry {} must be a regular file, got {:?}",
                entry.path.display(),
                entry.kind
            )));
        }
        if let Some(reference) = ref_from_filename(&entry.name) {
            crate::bounded::push(
                &mut refs,
                reference,
                crate::blocks::limits::MAX_CHUNK_STORE_REFS,
                "chunk store refs from directory",
            )?;
        }
    }
    Ok(refs)
}

pub(super) fn pinned_refs(
    root: &crate::blocks::CapabilityChunkRoot,
    dir: &crate::local_store::Locator,
) -> crate::error::Result<Vec<String>> {
    if !root.root().try_exists(dir)? {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in root.root().list_entries(dir)? {
        if entry.kind != crate::local_store::MemberKind::File {
            return Err(crate::error::Failure::invalid_harness(format!(
                "chunk pin entry {} must be a regular file, got {:?}",
                entry.path.display(),
                entry.kind
            )));
        }
        crate::bounded::push(
            &mut refs,
            root.root().read_to_string(&entry.path)?,
            crate::blocks::limits::MAX_CHUNK_STORE_REFS,
            "chunk store pinned refs",
        )?;
    }
    refs.sort();
    Ok(refs)
}

pub(super) fn write_immutable_bytes(
    root: &crate::blocks::CapabilityChunkRoot,
    path: &crate::local_store::Locator,
    bytes: &[u8],
    expected_ref: &str,
    parser: fn(&[u8]) -> crate::error::Result<preserves::IOValue>,
) -> crate::error::Result<()> {
    if root.root().try_exists(path)? {
        let existing = root.root().read(path)?;
        let existing_value = parser(&existing)?;
        let existing_ref = crate::preserves_rail::canonical_hash(&existing_value)?;
        if existing_ref != expected_ref {
            return Err(crate::error::Failure::invalid_harness(format!(
                "immutable content path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        root.root().write(path, bytes)?;
    }
    Ok(())
}

pub(super) fn write_immutable_blob(
    root: &crate::blocks::CapabilityChunkRoot,
    path: &crate::local_store::Locator,
    bytes: &[u8],
    expected_ref: &str,
) -> crate::error::Result<()> {
    if root.root().try_exists(path)? {
        let existing = root.root().read(path)?;
        let existing_ref = crate::blocks::integrity::hash_blob_bytes(&existing);
        if existing_ref != expected_ref {
            return Err(crate::error::Failure::invalid_harness(format!(
                "immutable blob path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        root.root().write(path, bytes)?;
    }
    Ok(())
}
