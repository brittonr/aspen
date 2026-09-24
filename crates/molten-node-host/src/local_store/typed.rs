type Result<T> = crate::error::Result<T>;
type Path = std::path::Path;

macro_rules! typed_root {
    ($name:ident, $kind:expr) => {
        pub struct $name {
            root: super::root::LocalStoreRoot,
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.debug_tuple(stringify!($name)).field(&self.root.kind()).finish()
            }
        }

        impl $name {
            pub fn open(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: super::root::LocalStoreRoot::open($kind, path)?,
                })
            }

            pub fn open_existing(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: super::root::LocalStoreRoot::open_existing($kind, path)?,
                })
            }

            pub fn root(&self) -> &super::root::LocalStoreRoot {
                &self.root
            }
        }
    };
}

typed_root!(ArtifactStoreRoot, super::path::LocalStoreKind::Artifact);
typed_root!(ChunkStoreRoot, super::path::LocalStoreKind::Chunk);
typed_root!(RetentionStoreRoot, super::path::LocalStoreKind::Retention);
typed_root!(DataspaceStoreRoot, super::path::LocalStoreKind::Dataspace);
typed_root!(ExchangeStoreRoot, super::path::LocalStoreKind::Exchange);
typed_root!(LedgerStoreRoot, super::path::LocalStoreKind::Ledger);
typed_root!(DeliveryStoreRoot, super::path::LocalStoreKind::Delivery);
typed_root!(DurableStoreRoot, super::path::LocalStoreKind::Durable);

impl ArtifactStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::LocalStoreRoot::from_dir(super::path::LocalStoreKind::Artifact, dir),
        }
    }
}

impl ChunkStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::LocalStoreRoot::from_dir(super::path::LocalStoreKind::Chunk, dir),
        }
    }
}

impl LedgerStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::LocalStoreRoot::from_dir(super::path::LocalStoreKind::Ledger, dir),
        }
    }
}

impl DeliveryStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: super::root::LocalStoreRoot::from_dir(super::path::LocalStoreKind::Delivery, dir),
        }
    }
}

impl ChunkStoreRoot {
    #[doc(hidden)]
    pub fn open_artifact_chunks(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent
                .root()
                .open_subdir(super::path::LocalStoreKind::Chunk, &super::path::LocalStorePath::parse("chunks")?)?,
        })
    }
}

impl RetentionStoreRoot {
    #[doc(hidden)]
    pub fn share_chunk_state(parent: &ChunkStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().share_authority_as(super::path::LocalStoreKind::Retention)?,
        })
    }

    #[doc(hidden)]
    pub fn open_bundle_state(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent
                .root()
                .open_subdir(super::path::LocalStoreKind::Retention, &super::path::LocalStorePath::parse("state")?)?,
        })
    }
}

impl ExchangeStoreRoot {
    #[doc(hidden)]
    pub fn open_chunk_subdir(parent: &ChunkStoreRoot, path: &super::path::LocalStorePath) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(super::path::LocalStoreKind::Exchange, path)?,
        })
    }
}
