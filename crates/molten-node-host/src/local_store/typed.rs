use super::path::{LocalStoreKind, LocalStorePath};
use super::root::LocalStoreRoot;
use crate::error::Result;

type Path = std::path::Path;

macro_rules! typed_root {
    ($name:ident, $kind:expr) => {
        pub struct $name {
            root: LocalStoreRoot,
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.debug_tuple(stringify!($name)).field(&self.root.kind()).finish()
            }
        }

        impl $name {
            pub fn open(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: LocalStoreRoot::open($kind, path)?,
                })
            }

            pub fn open_existing(path: &Path) -> Result<Self> {
                Ok(Self {
                    root: LocalStoreRoot::open_existing($kind, path)?,
                })
            }

            pub fn root(&self) -> &LocalStoreRoot {
                &self.root
            }
        }
    };
}

typed_root!(ArtifactStoreRoot, LocalStoreKind::Artifact);
typed_root!(ChunkStoreRoot, LocalStoreKind::Chunk);
typed_root!(RetentionStoreRoot, LocalStoreKind::Retention);
typed_root!(DataspaceStoreRoot, LocalStoreKind::Dataspace);
typed_root!(ExchangeStoreRoot, LocalStoreKind::Exchange);
typed_root!(LedgerStoreRoot, LocalStoreKind::Ledger);
typed_root!(DeliveryStoreRoot, LocalStoreKind::Delivery);
typed_root!(DurableStoreRoot, LocalStoreKind::Durable);

impl ArtifactStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: LocalStoreRoot::from_dir(LocalStoreKind::Artifact, dir),
        }
    }
}

impl ChunkStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: LocalStoreRoot::from_dir(LocalStoreKind::Chunk, dir),
        }
    }
}

impl LedgerStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: LocalStoreRoot::from_dir(LocalStoreKind::Ledger, dir),
        }
    }
}

impl DeliveryStoreRoot {
    pub(crate) fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            root: LocalStoreRoot::from_dir(LocalStoreKind::Delivery, dir),
        }
    }
}

impl ChunkStoreRoot {
    #[doc(hidden)]
    pub fn open_artifact_chunks(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(LocalStoreKind::Chunk, &LocalStorePath::parse("chunks")?)?,
        })
    }
}

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
