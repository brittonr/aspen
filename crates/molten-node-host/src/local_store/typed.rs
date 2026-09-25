type Result<T> = crate::error::Result<T>;
type Path = std::path::Path;

macro_rules! typed_root {
    ($name:ident, $kind:expr) => {
        pub struct $name {
            root: super::root::DirectoryHandle,
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.debug_tuple(stringify!($name)).field(&self.root.kind()).finish()
            }
        }

        impl $name {
            pub fn open(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: super::root::DirectoryHandle::open($kind, path)?,
                })
            }

            pub fn open_existing(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: super::root::DirectoryHandle::open_existing($kind, path)?,
                })
            }

            pub fn root(&self) -> &super::root::DirectoryHandle {
                &self.root
            }
        }
    };
}

typed_root!(ArtifactStoreRoot, super::path::Category::Artifact);
typed_root!(ChunkStoreRoot, super::path::Category::Chunk);
typed_root!(RetentionStoreRoot, super::path::Category::Retention);
typed_root!(DataspaceStoreRoot, super::path::Category::Dataspace);
typed_root!(ExchangeStoreRoot, super::path::Category::Exchange);
typed_root!(LedgerStoreRoot, super::path::Category::Ledger);
typed_root!(DeliveryStoreRoot, super::path::Category::Delivery);
typed_root!(DurableStoreRoot, super::path::Category::Durable);

impl ArtifactStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::DirectoryHandle::from_dir(super::path::Category::Artifact, dir),
        }
    }
}

impl ChunkStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::DirectoryHandle::from_dir(super::path::Category::Chunk, dir),
        }
    }
}

impl LedgerStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::DirectoryHandle::from_dir(super::path::Category::Ledger, dir),
        }
    }
}

impl DeliveryStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::DirectoryHandle::from_dir(super::path::Category::Delivery, dir),
        }
    }
}

impl ChunkStoreRoot {
    #[doc(hidden)]
    pub fn open_artifact_chunks(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent
                .root()
                .open_subdir(super::path::Category::Chunk, &super::path::RelativeLocator::parse("chunks")?)?,
        })
    }
}

impl RetentionStoreRoot {
    #[doc(hidden)]
    pub fn share_chunk_state(parent: &ChunkStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().share_authority_as(super::path::Category::Retention)?,
        })
    }

    #[doc(hidden)]
    pub fn open_bundle_state(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent
                .root()
                .open_subdir(super::path::Category::Retention, &super::path::RelativeLocator::parse("state")?)?,
        })
    }
}

impl ExchangeStoreRoot {
    #[doc(hidden)]
    pub fn open_chunk_subdir(parent: &ChunkStoreRoot, path: &super::path::RelativeLocator) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(super::path::Category::Exchange, path)?,
        })
    }
}
