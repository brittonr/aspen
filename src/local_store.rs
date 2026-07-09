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

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn as_path(&self) -> &Path {
        &self.relative
    }
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

    pub fn list_file_names(&self, path: &LocalStorePath) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in self.dir.read_dir(path.as_path()).map_err(MoltenError::from)? {
            let entry = entry.map_err(MoltenError::from)?;
            if entry.file_type().map_err(MoltenError::from)?.is_file() {
                push_bounded_name(&mut names, entry.file_name().to_string_lossy().into_owned())?;
            }
        }
        names.sort();
        Ok(names)
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

fn validate_local_locator(input: &str) -> Result<()> {
    if input.is_empty() {
        return Err(MoltenError::invalid_harness("local store path cannot be empty"));
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

fn push_bounded_name(names: &mut Vec<String>, name: String) -> Result<()> {
    let next = names
        .len()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("local store entry count overflow"))?;
    if next > MAX_LOCAL_STORE_ENTRIES {
        return Err(MoltenError::invalid_harness(format!(
            "local store entry count {next} exceeds maximum {MAX_LOCAL_STORE_ENTRIES}"
        )));
    }
    names.push(name);
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
        let names = root
            .root()
            .list_file_names(&LocalStorePath::parse("chunks").expect("chunks path"))
            .expect("list chunks");
        assert_eq!(names, vec!["ok.bin".to_string()]);
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
        let missing_root = std::env::temp_dir().join("molten-cap-std-missing-root").join(unique_suffix());
        let error = RetentionStoreRoot::open_existing(&missing_root).expect_err("missing root denied");
        assert!(error.to_string().contains("No such") || error.to_string().contains("not found"));
    }

    #[cfg(unix)]
    #[test]
    fn capability_root_denies_symlink_escape() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.negative]
        let root_path = temp_dir("cap-std-symlink");
        let outside_path = temp_dir("cap-std-outside");
        std::fs::write(outside_path.join("secret.txt"), b"secret").expect("outside secret");
        std::os::unix::fs::symlink(outside_path.join("secret.txt"), root_path.join("escape-link"))
            .expect("create symlink");
        let root = ExchangeStoreRoot::open(&root_path).expect("open exchange root");
        let error = root
            .root()
            .read(&LocalStorePath::parse("escape-link").expect("symlink path"))
            .expect_err("symlink escape denied");
        assert!(
            error.to_string().contains("outside")
                || error.to_string().contains("symlink")
                || error.to_string().contains("permission")
        );
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}", unique_suffix()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn unique_suffix() -> String {
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("{}-{nonce}", std::process::id())
    }
}
