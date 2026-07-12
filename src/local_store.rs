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
}

impl LocalStoreKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Chunk => "chunk",
            Self::Retention => "retention",
            Self::Dataspace => "dataspace",
            Self::Exchange => "exchange",
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
        self.dir.read(path.as_path()).map_err(MoltenError::from)
    }

    pub fn read_to_string(&self, path: &LocalStorePath) -> Result<String> {
        self.dir.read_to_string(path.as_path()).map_err(MoltenError::from)
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
        self.dir.write(path.as_path(), contents).map_err(MoltenError::from)
    }

    pub fn remove_file(&self, path: &LocalStorePath) -> Result<()> {
        self.dir.remove_file(path.as_path()).map_err(MoltenError::from)
    }

    pub fn remove_dir_all(&self, path: &LocalStorePath) -> Result<()> {
        self.dir.remove_dir_all(path.as_path()).map_err(MoltenError::from)
    }

    pub fn try_exists(&self, path: &LocalStorePath) -> Result<bool> {
        self.dir.try_exists(path.as_path()).map_err(MoltenError::from)
    }

    pub fn entry_kind(&self, path: &LocalStorePath) -> Result<LocalStoreEntryKind> {
        let metadata = self.dir.symlink_metadata(path.as_path()).map_err(MoltenError::from)?;
        Ok(local_store_entry_kind(&metadata.file_type()))
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

impl ChunkStoreRoot {
    pub(crate) fn open_artifact_chunks(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(LocalStoreKind::Chunk, &LocalStorePath::parse("chunks")?)?,
        })
    }
}

impl RetentionStoreRoot {
    pub(crate) fn share_chunk_state(parent: &ChunkStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().share_authority_as(LocalStoreKind::Retention)?,
        })
    }

    pub(crate) fn open_bundle_state(parent: &ArtifactStoreRoot) -> Result<Self> {
        Ok(Self {
            root: parent.root().open_subdir(LocalStoreKind::Retention, &LocalStorePath::parse("state")?)?,
        })
    }
}

#[cfg(test)]
impl ExchangeStoreRoot {
    fn open_chunk_subdir(parent: &ChunkStoreRoot, path: &LocalStorePath) -> Result<Self> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_root_reads_and_writes_valid_relative_paths() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.positive]
        let root_path = temp_dir("cap-std-positive");
        let root = ChunkStoreRoot::open(&root_path).expect("open chunk root");
        let artifact_root = ArtifactStoreRoot::open(&root_path.join("artifact")).expect("open artifact root");
        let retention_root = RetentionStoreRoot::open(&root_path.join("retention")).expect("open retention root");
        let dataspace_root = DataspaceStoreRoot::open(&root_path.join("dataspace")).expect("open dataspace root");
        let path = LocalStorePath::parse("chunks/ok.bin").expect("relative path");
        root.root().write(&path, b"chunk-bytes").expect("write under root");
        assert_eq!(root.root().read(&path).expect("read under root"), b"chunk-bytes");
        artifact_root
            .root()
            .write(&LocalStorePath::parse("objects/value.preserves").expect("artifact path"), b"artifact")
            .expect("write artifact");
        retention_root
            .root()
            .write(&LocalStorePath::parse("pins/pin.preserves").expect("retention path"), b"pin")
            .expect("write retention");
        dataspace_root
            .root()
            .write(&LocalStorePath::parse("envelopes/message.preserves").expect("dataspace path"), b"message")
            .expect("write dataspace");
        let chunks_path = LocalStorePath::parse("chunks").expect("chunks path");
        let names = root.root().list_file_names(&chunks_path).expect("list chunks");
        assert_eq!(names, vec!["ok.bin".to_string()]);
        let derived = ExchangeStoreRoot::open_chunk_subdir(&root, &chunks_path).expect("derive exchange subroot");
        assert_eq!(derived.root().kind(), LocalStoreKind::Exchange);
        assert_eq!(
            derived
                .root()
                .read(&LocalStorePath::parse("ok.bin").expect("derived path"))
                .expect("read through derived root"),
            b"chunk-bytes"
        );
    }

    #[test]
    fn capability_roots_deny_invalid_local_locators_and_missing_authority() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.negative]
        assert!(
            LocalStorePath::parse("../escape")
                .expect_err("parent traversal denied")
                .to_string()
                .contains("parent")
        );
        assert!(LocalStorePath::parse("/tmp/escape").expect_err("absolute denied").to_string().contains("relative"));
        assert!(
            LocalStorePath::parse("iroh://peer/blob")
                .expect_err("remote locator denied")
                .to_string()
                .contains("local filesystem")
        );
        assert!(
            LocalStorePath::parse("blake3:abc")
                .expect_err("content ref denied")
                .to_string()
                .contains("local filesystem")
        );
        assert!(
            LocalStorePath::parse(r"C:\escape")
                .expect_err("drive-prefixed path denied")
                .to_string()
                .contains("platform-prefixed")
        );
        assert!(
            LocalStorePath::parse(r"\\server\share")
                .expect_err("UNC-prefixed path denied")
                .to_string()
                .contains("platform-prefixed")
        );
        let missing_parent =
            crate::test_support::process_workspace("cap_std_missing_root").expect("create missing-root workspace");
        let missing_root = missing_parent.join("missing");
        let error = RetentionStoreRoot::open_existing(&missing_root).expect_err("missing root denied");
        assert!(error.to_string().contains("No such") || error.to_string().contains("not found"));
    }

    #[cfg(unix)]
    #[test]
    fn capability_root_denies_symlink_escape() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.negative]
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("cap-std-symlink");
        let outside_path = temp_dir("cap-std-outside");
        std::fs::write(outside_path.join("secret.txt"), b"secret").expect("outside secret");
        std::fs::create_dir(root_path.join("entries")).expect("create entry directory");
        std::os::unix::fs::symlink(outside_path.join("secret.txt"), root_path.join("entries/escape-link"))
            .expect("create symlink");
        std::os::unix::fs::symlink(&outside_path, root_path.join("entries/escape-directory"))
            .expect("create intermediate symlink");
        std::fs::write(root_path.join("entries/replace-me"), b"inside").expect("replacement fixture");
        let root = ExchangeStoreRoot::open(&root_path).expect("open exchange root");
        let error = root
            .root()
            .read(&LocalStorePath::parse("entries/escape-link").expect("symlink path"))
            .expect_err("symlink escape denied");
        assert!(
            error.to_string().contains("outside")
                || error.to_string().contains("symlink")
                || error.to_string().contains("permission")
        );
        let intermediate_error = root
            .root()
            .read(&LocalStorePath::parse("entries/escape-directory/secret.txt").expect("intermediate path"))
            .expect_err("intermediate symlink escape denied");
        assert!(!intermediate_error.to_string().is_empty());
        let entries = root
            .root()
            .list_entries(&LocalStorePath::parse("entries").expect("entry directory"))
            .expect("list logical entries");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "escape-directory");
        assert_eq!(entries[0].kind, LocalStoreEntryKind::Symlink);
        assert_eq!(entries[1].name, "escape-link");
        assert_eq!(entries[1].kind, LocalStoreEntryKind::Symlink);
        std::fs::remove_file(root_path.join("entries/replace-me")).expect("remove enumerated file");
        std::os::unix::fs::symlink(outside_path.join("secret.txt"), root_path.join("entries/replace-me"))
            .expect("replace enumerated file with symlink");
        let replacement_error = root
            .root()
            .read(&LocalStorePath::parse("entries/replace-me").expect("replacement path"))
            .expect_err("replacement symlink escape denied");
        assert!(!replacement_error.to_string().is_empty());
    }

    #[test]
    fn typed_roots_do_not_substitute_same_relative_handle_across_authorities() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let artifact_path = temp_dir("cap-std-artifact-authority");
        let chunk_path = temp_dir("cap-std-chunk-authority");
        let artifact_root = ArtifactStoreRoot::open(&artifact_path).expect("open artifact root");
        let chunk_root = ChunkStoreRoot::open(&chunk_path).expect("open chunk root");
        let locator = LocalStorePath::parse("shared/value.bin").expect("shared locator");
        artifact_root.root().write(&locator, b"artifact").expect("write artifact authority");
        chunk_root.root().write(&locator, b"chunk").expect("write chunk authority");

        assert_eq!(artifact_root.root().read(&locator).expect("read artifact authority"), b"artifact");
        assert_eq!(chunk_root.root().read(&locator).expect("read chunk authority"), b"chunk");
        assert_ne!(artifact_root.root().kind(), chunk_root.root().kind());
    }

    #[cfg(unix)]
    #[test]
    fn database_handle_rejects_symlink_and_non_regular_leaf() {
        // r[verify molten.chunk_store.cap_std_backend_handles]
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("cap-std-database-leaf");
        let outside_path = temp_dir("cap-std-database-outside");
        let outside_file = outside_path.join("outside.redb");
        std::fs::write(&outside_file, b"outside").expect("outside database fixture");
        std::os::unix::fs::symlink(&outside_file, root_path.join("symlink.redb")).expect("database symlink fixture");
        std::fs::create_dir(root_path.join("directory.redb")).expect("database directory fixture");
        let root = ChunkStoreRoot::open(&root_path).expect("open chunk root");

        let symlink_error = root
            .root()
            .open_database_file(&LocalStorePath::parse("symlink.redb").expect("symlink path"))
            .expect_err("database symlink denied");
        assert!(symlink_error.to_string().contains("regular file"));
        let directory_error = root
            .root()
            .open_database_file(&LocalStorePath::parse("directory.redb").expect("directory path"))
            .expect_err("database directory denied");
        assert!(directory_error.to_string().contains("regular file"));
    }

    #[cfg(unix)]
    #[test]
    fn open_capability_survives_ambient_root_replacement_without_switching_authority() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("cap-std-root-replacement");
        let moved_path = root_path.with_extension("moved");
        let root = RetentionStoreRoot::open(&root_path).expect("open retention root");
        std::fs::rename(&root_path, &moved_path).expect("move opened root");
        std::fs::create_dir(&root_path).expect("replace ambient root path");
        std::fs::write(root_path.join("replacement.txt"), b"replacement").expect("write replacement root");

        let logical_path = LocalStorePath::parse("bound.txt").expect("bound path");
        root.root().write(&logical_path, b"bound").expect("write through opened capability");
        assert_eq!(std::fs::read(moved_path.join("bound.txt")).expect("read moved root"), b"bound");
        assert!(!root_path.join("bound.txt").exists());
        assert_eq!(std::fs::read(root_path.join("replacement.txt")).expect("read replacement"), b"replacement");
    }

    fn temp_dir(label: &str) -> crate::test_support::ProcessWorkspace {
        crate::test_support::process_workspace(label).expect("create isolated local-store workspace")
    }
}
