
impl RetentionStoreRoot {
    #[doc(hidden)]
    pub fn share_chunk_state(parent: &ChunkStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().share_authority_as(LocalStoreKind::Retention)?,
        })
    }

    #[doc(hidden)]
    pub fn open_bundle_state(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(LocalStoreKind::Retention, &LocalStorePath::parse("state")?)?,
        })
    }
}

impl ExchangeStoreRoot {
    #[doc(hidden)]
    pub fn open_chunk_subdir(parent: &ChunkStoreRoot, path: &LocalStorePath) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(LocalStoreKind::Exchange, path)?,
        })
    }
}

fn validate_local_locator(input: &str) -> Result<()> {
    if input.is_empty() {
        return Err(MoltenError::invalid_harness("local store path cannot be empty"));
    }
    if has_platform_prefix(input) {
        return Err(MoltenError::invalid_harness(format!(
            "platform-prefixed local store path {input} is not portable relative authority"
        )));
    }
    if input.contains("://")
        || input.starts_with("iroh:")
        || input.starts_with("http:")
        || input.starts_with("https:")
        || input.starts_with("blake3:")
    {
        return Err(MoltenError::invalid_harness(format!(
            "remote or content locator {input} cannot be used as a local filesystem path"
        )));
    }
    Ok(())
}

fn has_platform_prefix(input: &str) -> bool {
    let bytes = input.as_bytes();
    let has_drive_prefix = bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
    has_drive_prefix || input.starts_with("\\\\") || input.contains('\\')
}

fn checked_component_count(count: usize) -> Result<usize> {
    let next = count
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("local store path component count overflow"))?;
    if next > MAX_LOCAL_STORE_COMPONENTS {
        Err(MoltenError::invalid_harness(format!(
            "local store path component count {next} exceeds maximum {MAX_LOCAL_STORE_COMPONENTS}"
        )))
    } else {
        Ok(next)
    }
}

fn local_store_entry_kind(file_type: &cap_std::fs::FileType) -> LocalStoreEntryKind {
    if file_type.is_file() {
        LocalStoreEntryKind::File
    } else if file_type.is_dir() {
        LocalStoreEntryKind::Directory
    } else if file_type.is_symlink() {
        LocalStoreEntryKind::Symlink
    } else {
        LocalStoreEntryKind::Other
    }
}

/// The error for one entry past the bound. Listing loops deny the next push once the collection
/// holds `MAX_LOCAL_STORE_ENTRIES`, so the denied count is always one past the maximum.
fn entry_limit_error() -> MoltenError {
    MoltenError::invalid_harness(format!(
        "local store entry count {} exceeds maximum {MAX_LOCAL_STORE_ENTRIES}",
        MAX_LOCAL_STORE_ENTRIES + 1
    ))
}
