
impl DeliveryStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: StorageRoot::from_dir(Kind::Delivery, dir),
        }
    }
}

impl ChunkStoreRoot {
    #[doc(hidden)]
    pub fn open_artifact_chunks(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(Kind::Chunk, &Locator::parse("chunks")?)?,
        })
    }
}

impl DurableStoreRoot {
    /// Derive a durable database capability without reopening an ambient path.
    pub fn from_ledger(parent: &LedgerStoreRoot, path: &Locator) -> Result<Self> {
        Ok(Self { root: parent.root().open_subdir(Kind::Durable, path)? })
    }
}
