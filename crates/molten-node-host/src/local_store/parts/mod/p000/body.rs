use std::io::Read;
use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Result<T> = crate::error::Result<T>;

const MAX_LOCAL_STORE_COMPONENTS: usize = 32;
const MAX_LOCAL_STORE_ENTRIES: usize = 100_000;

const _: () = assert!(MAX_LOCAL_STORE_COMPONENTS <= 1_000);
const _: () = assert!(MAX_LOCAL_STORE_ENTRIES <= 1_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalStoreKind {
    Artifact,
    Chunk,
    Retention,
    Dataspace,
    Exchange,
    Ledger,
    Delivery,
    Durable,
}

impl LocalStoreKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Chunk => "chunk",
            Self::Retention => "retention",
            Self::Dataspace => "dataspace",
            Self::Exchange => "exchange",
            Self::Ledger => "ledger",
            Self::Delivery => "delivery",
            Self::Durable => "durable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalStorePath {
    relative: PathBuf,
}

impl LocalStorePath {
    pub fn parse(input: &str) -> Result<Self> {
        validate_local_locator(input)?;
        let path = Path::new(input);
        let mut relative = PathBuf::new();
        let mut component_count = 0usize;
        for component in path.components() {
            match component {
                std::path::Component::Normal(value) => {
                    component_count = checked_component_count(component_count)?;
                    relative.push(value);
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    return Err(MoltenError::invalid_harness(format!(
                        "local store path {input} cannot contain parent traversal"
                    )));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(MoltenError::invalid_harness(format!("local store path {input} must be relative")));
                }
            }
        }
        if relative.as_os_str().is_empty() {
            return Err(MoltenError::invalid_harness("local store path cannot be empty"));
        }
        Ok(Self { relative })
    }

    pub fn join(&self, suffix: &str) -> Result<Self> {
        let suffix = Self::parse(suffix)?;
        let base_count = self.relative.components().count();
        let suffix_count = suffix.relative.components().count();
        let component_count = base_count
            .checked_add(suffix_count)
            .ok_or_else(|| MoltenError::invalid_harness("local store path component count overflow"))?;
        if component_count > MAX_LOCAL_STORE_COMPONENTS {
            return Err(MoltenError::invalid_harness(format!(
                "local store path component count {component_count} exceeds maximum {MAX_LOCAL_STORE_COMPONENTS}"
            )));
        }
        Ok(Self {
            relative: self.relative.join(suffix.relative),
        })
    }

    pub fn as_path(&self) -> &Path {
        &self.relative
    }

    pub fn display(&self) -> String {
        self.relative.to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalStoreEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalStoreEntry {
    pub name: String,
    pub path: LocalStorePath,
    pub kind: LocalStoreEntryKind,
}

pub struct LocalStoreRoot {
    kind: LocalStoreKind,
    dir: cap_std::fs::Dir,
}

impl std::fmt::Debug for LocalStoreRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("LocalStoreRoot").field("kind", &self.kind).finish_non_exhaustive()
    }
}
