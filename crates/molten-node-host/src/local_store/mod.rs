use std::io::Read;
use std::io::Write;

use cap_fs_ext::FollowSymlinks;
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

impl LocalStoreRoot {
    pub fn open(kind: LocalStoreKind, root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root).map_err(MoltenError::from)?;
        Self::open_existing(kind, root)
    }

    pub fn open_existing(kind: LocalStoreKind, root: &Path) -> Result<Self> {
        let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(MoltenError::from)?;
        Ok(Self { kind, dir })
    }

    pub fn kind(&self) -> LocalStoreKind {
        self.kind
    }

    #[doc(hidden)]
    pub fn try_clone_dir(&self) -> Result<cap_std::fs::Dir> {
        self.dir.try_clone().map_err(MoltenError::from)
    }

    pub(crate) fn from_dir(kind: LocalStoreKind, dir: cap_std::fs::Dir) -> Self {
        Self { kind, dir }
    }

    fn open_subdir(&self, kind: LocalStoreKind, path: &LocalStorePath) -> Result<Self> {
        self.create_dir_all(path)?;
        let dir = self.dir.open_dir(path.as_path()).map_err(MoltenError::from)?;
        Ok(Self { kind, dir })
    }

    fn share_authority_as(&self, kind: LocalStoreKind) -> Result<Self> {
        let dir = self.dir.try_clone().map_err(MoltenError::from)?;
        Ok(Self { kind, dir })
    }

    pub fn create_dir_all(&self, path: &LocalStorePath) -> Result<()> {
        self.dir.create_dir_all(path.as_path()).map_err(MoltenError::from)
    }

    pub fn read(&self, path: &LocalStorePath) -> Result<Vec<u8>> {
        if self.entry_kind(path)? != LocalStoreEntryKind::File {
            return Err(MoltenError::invalid_harness(format!(
                "local store read leaf {} must be a regular file",
                path.display()
            )));
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let mut file = self.dir.open_with(path.as_path(), &options).map_err(MoltenError::from)?;
        if !file.metadata().map_err(MoltenError::from)?.is_file() {
            return Err(MoltenError::invalid_harness(format!(
                "local store read leaf {} changed away from a regular file",
                path.display()
            )));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(MoltenError::from)?;
        Ok(bytes)
    }

    pub fn read_to_string(&self, path: &LocalStorePath) -> Result<String> {
        String::from_utf8(self.read(path)?).map_err(|error| {
            MoltenError::invalid_harness(format!("local store file {} is not UTF-8: {error}", path.display()))
        })
    }

    pub fn write(&self, path: &LocalStorePath, contents: &[u8]) -> Result<()> {
        if let Some(parent) = path.as_path().parent()
            && !parent.as_os_str().is_empty()
        {
            let parent_path = LocalStorePath {
                relative: parent.to_path_buf(),
            };
            self.create_dir_all(&parent_path)?;
        }
        match self.entry_kind_optional(path)? {
            Some(LocalStoreEntryKind::File) | None => {}
            Some(kind) => {
                return Err(MoltenError::invalid_harness(format!(
                    "local store write leaf {} must be a regular file, got {kind:?}",
                    path.display()
                )));
            }
        }
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).follow(FollowSymlinks::No);
        let mut file = self.dir.open_with(path.as_path(), &options).map_err(MoltenError::from)?;
        if !file.metadata().map_err(MoltenError::from)?.is_file() {
            return Err(MoltenError::invalid_harness(format!(
                "local store write leaf {} changed away from a regular file",
                path.display()
            )));
        }
        file.write_all(contents).map_err(MoltenError::from)?;
        file.flush().map_err(MoltenError::from)
    }

    pub fn remove_file(&self, path: &LocalStorePath) -> Result<()> {
        self.dir.remove_file(path.as_path()).map_err(MoltenError::from)
    }

    pub fn remove_dir_all(&self, path: &LocalStorePath) -> Result<()> {
        self.dir.remove_dir_all(path.as_path()).map_err(MoltenError::from)
    }

    pub fn try_exists(&self, path: &LocalStorePath) -> Result<bool> {
        self.entry_kind_optional(path).map(|kind| kind.is_some())
    }

    pub fn entry_kind(&self, path: &LocalStorePath) -> Result<LocalStoreEntryKind> {
        self.entry_kind_optional(path)?
            .ok_or_else(|| MoltenError::invalid_harness(format!("local store path {} does not exist", path.display())))
    }

    pub fn entry_kind_optional(&self, path: &LocalStorePath) -> Result<Option<LocalStoreEntryKind>> {
        match self.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) => Ok(Some(local_store_entry_kind(&metadata.file_type()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(MoltenError::from(error)),
        }
    }

    pub fn list_entries(&self, path: &LocalStorePath) -> Result<Vec<LocalStoreEntry>> {
        let mut entries = Vec::new();
        for entry_result in self.dir.read_dir(path.as_path()).map_err(MoltenError::from)? {
            let entry = entry_result.map_err(MoltenError::from)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let entry_path = path.join(&name)?;
            let kind = local_store_entry_kind(&entry.file_type().map_err(MoltenError::from)?);
            push_bounded_entry(&mut entries, LocalStoreEntry {
                name,
                path: entry_path,
                kind,
            })?;
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    pub fn list_file_names(&self, path: &LocalStorePath) -> Result<Vec<String>> {
        let entries = self.list_entries(path)?;
        let mut names = Vec::new();
        for entry in entries {
            if entry.kind == LocalStoreEntryKind::File {
                push_bounded_name(&mut names, entry.name)?;
            }
        }
        Ok(names)
    }

    pub fn open_database_file(&self, path: &LocalStorePath) -> Result<std::fs::File> {
        match self.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) => {
                let kind = local_store_entry_kind(&metadata.file_type());
                if kind != LocalStoreEntryKind::File {
                    return Err(MoltenError::invalid_harness(format!(
                        "database leaf {} must be a regular file, got {kind:?}",
                        path.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(MoltenError::from(error)),
        }

        if let Some(parent) = path.as_path().parent()
            && !parent.as_os_str().is_empty()
        {
            let parent_path = LocalStorePath {
                relative: parent.to_path_buf(),
            };
            self.create_dir_all(&parent_path)?;
        }

        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).follow(FollowSymlinks::No);
        let file = self.dir.open_with(path.as_path(), &options).map_err(MoltenError::from)?;
        if !file.metadata().map_err(MoltenError::from)?.is_file() {
            return Err(MoltenError::invalid_harness(format!(
                "database leaf {} must remain a regular file after open",
                path.display()
            )));
        }
        Ok(file.into_std())
    }
}

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

fn push_bounded_entry(entries: &mut Vec<LocalStoreEntry>, entry: LocalStoreEntry) -> Result<()> {
    ensure_entry_capacity(entries.len())?;
    entries.push(entry);
    Ok(())
}

fn push_bounded_name(names: &mut Vec<String>, name: String) -> Result<()> {
    ensure_entry_capacity(names.len())?;
    names.push(name);
    Ok(())
}

fn ensure_entry_capacity(current: usize) -> Result<()> {
    let next = current
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("local store entry count overflow"))?;
    if next > MAX_LOCAL_STORE_ENTRIES {
        return Err(MoltenError::invalid_harness(format!(
            "local store entry count {next} exceeds maximum {MAX_LOCAL_STORE_ENTRIES}"
        )));
    }
    Ok(())
}
