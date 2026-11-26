use cap_fs_ext::DirExt;

#[path = "authority/layout.rs"]
mod layout;

pub use layout::NamespaceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    RegularFile,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug)]
pub enum FileObservation {
    Missing,
    NonRegular(EntryKind),
    Regular(FileHandle),
}

pub struct FileHandle {
    pub(super) file: cap_std::fs::File,
    pub(super) size: u64,
    pub(super) unix_mode: Option<u32>,
}

impl std::fmt::Debug for FileHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NodeStateFile")
            .field("size", &self.size)
            .field("unix_mode", &self.unix_mode)
            .finish_non_exhaustive()
    }
}

impl FileHandle {
    pub fn unix_mode(&self) -> Option<u32> {
        self.unix_mode
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn read_bounded(self, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        super::filesystem::read_open_file_bounded(self.file, self.size, max_bytes, "observed node state file")
    }
}

pub(super) struct Inner {
    pub(super) dir: cap_std::fs::Dir,
}

#[derive(Clone)]
pub struct DirectoryRoot {
    inner: std::sync::Arc<Inner>,
}

impl std::fmt::Debug for DirectoryRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NodeStateRoot").finish_non_exhaustive()
    }
}

impl DirectoryRoot {
    pub fn open(path: &std::path::Path) -> crate::error::Result<Self> {
        super::filesystem::validate_bootstrap_path(path)?;
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => super::filesystem::validate_bootstrap_metadata(&metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(path).map_err(crate::error::Failure::from)?;
            }
            Err(error) => return Err(crate::error::Failure::from(error)),
        }
        Self::open_existing(path)
    }

    /// Opens the root's final component without following a symlink, so the directory validated is
    /// the directory used; a root swapped for a symlink after any earlier check is refused.
    // r[impl molten.runtime_spine.admission_and_capability_hardening]
    pub fn open_existing(path: &std::path::Path) -> crate::error::Result<Self> {
        super::filesystem::validate_bootstrap_path(path)?;
        let leaf = path.file_name().ok_or_else(|| super::invalid("node state root must name a directory leaf"))?;
        let parent_path = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let parent = cap_std::fs::Dir::open_ambient_dir(parent_path, cap_std::ambient_authority())
            .map_err(crate::error::Failure::from)?;
        // Seam immediately before the open: the earlier implementation re-checked metadata here and
        // then opened with symlink following, so a swap at this point redirected it.
        #[cfg(test)]
        open_seam::after_bootstrap_check();
        let dir = parent.open_dir_nofollow(leaf).map_err(|error| match parent.symlink_metadata(leaf) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                super::invalid("node state root must not be a symlink")
            }
            Ok(metadata) if !metadata.is_dir() => super::invalid("node state root must be a directory"),
            _ => crate::error::Failure::from(error),
        })?;
        Ok(Self::from_dir(dir))
    }

    pub fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            inner: std::sync::Arc::new(Inner { dir }),
        }
    }

    pub fn create_layout(&self) -> crate::error::Result<()> {
        for kind in NamespaceKind::ALL {
            self.create_dir_all(&super::locator::RelativePath::parse(kind.as_str())?)?;
        }
        for path in [
            "cache",
            "remote-dataspace",
            "jobs",
            "coordination",
            "plugin-host",
            "catalog-mcp",
            "control",
        ] {
            self.create_dir_all(&super::locator::RelativePath::parse(path)?)?;
        }
        Ok(())
    }

    pub fn namespace(&self, kind: NamespaceKind) -> crate::error::Result<super::namespace::Scope> {
        let path = super::locator::RelativePath::parse(kind.as_str())?;
        self.create_dir_all(&path)?;
        let dir = super::filesystem::open_dir_components(&self.inner.dir, path.as_path())?;
        Ok(super::namespace::Scope {
            root: std::sync::Arc::clone(&self.inner),
            kind,
            scope: path.into_path_buf(),
            dir,
        })
    }

    /// Observe an optional supplied namespace without creating directories.
    pub fn existing_namespace(&self, kind: NamespaceKind) -> crate::error::Result<Option<super::namespace::Scope>> {
        let path = super::locator::RelativePath::parse(kind.as_str())?;
        match self.entry_kind(&path)? {
            None => return Ok(None),
            Some(EntryKind::Directory) => {}
            Some(_) => return Err(super::invalid("supplied namespace must be an owned directory")),
        }
        let dir = super::filesystem::open_dir_components(&self.inner.dir, path.as_path())?;
        Ok(Some(super::namespace::Scope {
            root: std::sync::Arc::clone(&self.inner),
            kind,
            scope: path.into_path_buf(),
            dir,
        }))
    }

    pub fn identity(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::Identity)
    }

    pub fn secrets(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::Secrets)
    }

    pub fn ledger(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::Ledger)
    }

    pub fn control_inbox(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::ControlInbox)
    }

    pub fn control_outbox(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::ControlOutbox)
    }

    pub fn control_ingress(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::ControlIngress)
    }

    pub fn control_idempotency(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::ControlIdempotency)
    }

    pub fn control_service(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::ControlService)
    }

    pub fn receipts(&self) -> crate::error::Result<super::namespace::Scope> {
        self.namespace(NamespaceKind::Receipts)
    }

    pub fn ledger_store(&self) -> crate::error::Result<crate::local_store::LedgerStoreRoot> {
        let namespace = self.ledger()?;
        Ok(crate::local_store::LedgerStoreRoot::from_dir(namespace.try_clone_dir()?))
    }

    pub fn delivery_store(&self) -> crate::error::Result<crate::local_store::DeliveryStoreRoot> {
        let namespace = self.control_idempotency()?;
        Ok(crate::local_store::DeliveryStoreRoot::from_dir(namespace.try_clone_dir()?))
    }

    pub fn artifact_store(&self) -> crate::error::Result<crate::local_store::ArtifactStoreRoot> {
        let namespace = self.namespace(NamespaceKind::Registry)?;
        Ok(crate::local_store::ArtifactStoreRoot::from_dir(namespace.try_clone_dir()?))
    }

    pub fn chunk_store(&self) -> crate::error::Result<crate::local_store::ChunkStoreRoot> {
        let namespace = self.namespace(NamespaceKind::Chunks)?;
        Ok(crate::local_store::ChunkStoreRoot::from_dir(namespace.try_clone_dir()?))
    }

    pub fn read(&self, path: &super::locator::RelativePath, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        super::filesystem::read_regular_file_bounded(&self.inner.dir, path.as_path(), max_bytes)
    }

    pub fn read_to_string(&self, path: &super::locator::RelativePath, max_bytes: u64) -> crate::error::Result<String> {
        String::from_utf8(self.read(path, max_bytes)?)
            .map_err(|error| super::invalid(format!("node state file {} is not UTF-8: {error}", path.display())))
    }

    pub fn write(&self, path: &super::locator::RelativePath, bytes: &[u8]) -> crate::error::Result<()> {
        super::filesystem::validate_write_size(bytes, super::MAX_NODE_STATE_FILE_BYTES)?;
        super::filesystem::write_regular_file(&self.inner.dir, path.as_path(), bytes)
    }

    /// Publish one new regular file in an existing parent, without replacement.
    /// Success observes payload and parent synchronization, not hardware durability.
    pub fn write_new(&self, path: &super::locator::RelativePath, bytes: &[u8]) -> crate::error::Result<()> {
        super::filesystem::validate_write_size(bytes, super::MAX_NODE_STATE_FILE_BYTES)?;
        super::publication::write_new_regular_file(&self.inner.dir, path.as_path(), bytes)
    }

    pub fn try_exists(&self, path: &super::locator::RelativePath) -> crate::error::Result<bool> {
        super::filesystem::entry_kind_optional(&self.inner.dir, path.as_path()).map(|kind| kind.is_some())
    }

    pub fn entry_kind(&self, path: &super::locator::RelativePath) -> crate::error::Result<Option<EntryKind>> {
        super::filesystem::entry_kind_optional(&self.inner.dir, path.as_path())
    }

    pub fn remove_regular_file(&self, path: &super::locator::RelativePath) -> crate::error::Result<()> {
        super::filesystem::remove_regular_file(&self.inner.dir, path.as_path())
    }

    pub fn create_dir_all(&self, path: &super::locator::RelativePath) -> crate::error::Result<()> {
        super::filesystem::create_dir_components(&self.inner.dir, path.as_path())
    }

    pub fn open_database_file(&self, path: &super::locator::RelativePath) -> crate::error::Result<std::fs::File> {
        super::filesystem::open_database_file(&self.inner.dir, path.as_path())
    }

    pub(crate) fn try_clone_dir(&self) -> crate::error::Result<cap_std::fs::Dir> {
        self.inner.dir.try_clone().map_err(crate::error::Failure::from)
    }
}

/// Test-only hook between `open`'s metadata check and the root open, so a regression can swap the
/// root for a symlink at exactly that boundary.
#[cfg(test)]
mod open_seam {
    type Hook = Box<dyn FnMut()>;

    thread_local! {
        static AFTER_CHECK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
    }

    pub(super) fn set_after_bootstrap_check(hook: impl FnMut() + 'static) {
        AFTER_CHECK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }

    pub(super) fn after_bootstrap_check() {
        let hook = AFTER_CHECK.with(|slot| slot.borrow_mut().take());
        if let Some(mut hook) = hook {
            hook();
        }
    }
}

#[cfg(all(test, unix))]
#[path = "authority/tests.rs"]
mod open_existing_tests;
