pub struct NodeStateNamespace {
    pub(super) root: std::sync::Arc<super::authority::NodeStateInner>,
    pub(super) kind: super::authority::NodeStateNamespaceKind,
    pub(super) scope: std::path::PathBuf,
    pub(super) dir: cap_std::fs::Dir,
}

impl std::fmt::Debug for NodeStateNamespace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NodeStateNamespace").field("kind", &self.kind).finish_non_exhaustive()
    }
}

impl NodeStateNamespace {
    pub fn open(kind: super::authority::NodeStateNamespaceKind, path: &std::path::Path) -> crate::error::Result<Self> {
        let root = super::authority::NodeStateRoot::open(path)?;
        Self::from_dir(kind, root.try_clone_dir()?)
    }

    pub fn open_existing(
        kind: super::authority::NodeStateNamespaceKind,
        path: &std::path::Path,
    ) -> crate::error::Result<Self> {
        let root = super::authority::NodeStateRoot::open_existing(path)?;
        Self::from_dir(kind, root.try_clone_dir()?)
    }

    pub fn from_dir(
        kind: super::authority::NodeStateNamespaceKind,
        dir: cap_std::fs::Dir,
    ) -> crate::error::Result<Self> {
        let root_dir = dir.try_clone().map_err(crate::error::MoltenError::from)?;
        Ok(Self {
            root: std::sync::Arc::new(super::authority::NodeStateInner { dir: root_dir }),
            kind,
            scope: std::path::PathBuf::new(),
            dir,
        })
    }

    pub fn kind(&self) -> super::authority::NodeStateNamespaceKind {
        self.kind
    }

    pub fn read(&self, path: &super::locator::NodeStatePath, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        super::filesystem::read_regular_file_bounded(&self.dir, path.as_path(), max_bytes)
    }

    pub fn read_to_string(&self, path: &super::locator::NodeStatePath, max_bytes: u64) -> crate::error::Result<String> {
        String::from_utf8(self.read(path, max_bytes)?)
            .map_err(|error| super::invalid(format!("node state file {} is not UTF-8: {error}", path.display())))
    }

    pub fn write(&self, path: &super::locator::NodeStatePath, bytes: &[u8]) -> crate::error::Result<()> {
        super::filesystem::validate_write_size(bytes, super::MAX_NODE_STATE_FILE_BYTES)?;
        super::filesystem::write_regular_file(&self.dir, path.as_path(), bytes, None)
    }

    /// Replace one bounded regular leaf atomically without exporting its directory capability.
    /// This is file visibility, not a power-loss durability or multi-writer transaction guarantee.
    pub fn write_atomic_leaf(&self, path: &super::locator::NodeStatePath, bytes: &[u8]) -> crate::error::Result<()> {
        use std::io::Write;
        super::filesystem::validate_write_size(bytes, super::MAX_NODE_STATE_FILE_BYTES)?;
        if path.as_path().components().count() != 1 {
            return Err(super::invalid("atomic namespace write requires one leaf"));
        }
        if !matches!(self.entry_kind(path)?, None | Some(super::authority::NodeStateEntryKind::RegularFile)) {
            return Err(super::invalid("atomic namespace destination must be a regular file"));
        }
        let mut file = cap_tempfile::TempFile::new(&self.dir).map_err(crate::error::MoltenError::from)?;
        file.write_all(bytes).map_err(crate::error::MoltenError::from)?;
        file.as_file().sync_all().map_err(crate::error::MoltenError::from)?;
        file.replace(path.as_path().as_os_str()).map_err(crate::error::MoltenError::from)
    }

    pub fn write_restricted(
        &self,
        path: &super::locator::NodeStatePath,
        bytes: &[u8],
        unix_mode: u32,
    ) -> crate::error::Result<()> {
        super::filesystem::validate_write_size(bytes, super::MAX_NODE_SECRET_BYTES)?;
        super::filesystem::write_regular_file(&self.dir, path.as_path(), bytes, Some(unix_mode))
    }

    pub fn try_exists(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<bool> {
        super::filesystem::entry_kind_optional(&self.dir, path.as_path()).map(|kind| kind.is_some())
    }

    pub fn entry_kind(
        &self,
        path: &super::locator::NodeStatePath,
    ) -> crate::error::Result<Option<super::authority::NodeStateEntryKind>> {
        super::filesystem::entry_kind_optional(&self.dir, path.as_path())
    }

    pub fn observe_file(
        &self,
        path: &super::locator::NodeStatePath,
    ) -> crate::error::Result<super::authority::NodeStateFileObservation> {
        super::filesystem::observe_file(&self.dir, path.as_path())
    }

    pub fn unix_mode(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<Option<u32>> {
        match self.observe_file(path)? {
            super::authority::NodeStateFileObservation::Missing => Ok(None),
            super::authority::NodeStateFileObservation::NonRegular(_) => {
                Err(super::invalid(format!("node state leaf {} must be a regular file", path.display())))
            }
            super::authority::NodeStateFileObservation::Regular(file) => Ok(file.unix_mode()),
        }
    }

    pub fn remove_regular_file(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<()> {
        super::filesystem::remove_regular_file(&self.dir, path.as_path())
    }

    pub fn create_dir_all(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<()> {
        super::filesystem::create_dir_components(&self.dir, path.as_path())
    }

    pub fn list_entries(&self) -> crate::error::Result<Vec<NodeStateEntry>> {
        super::enumeration::list_entries(&self.dir, &self.root, self.kind, &self.scope)
    }

    pub fn open_subdir(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<Self> {
        let scope = super::locator::join_scope(&self.scope, path)?;
        super::filesystem::create_dir_components(&self.dir, path.as_path())?;
        let dir = super::filesystem::open_dir_components(&self.dir, path.as_path())?;
        Ok(Self {
            root: std::sync::Arc::clone(&self.root),
            kind: self.kind,
            scope,
            dir,
        })
    }

    pub fn read_entry(&self, entry: &NodeStateEntry, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        self.validate_entry(entry)?;
        if entry.kind != super::authority::NodeStateEntryKind::RegularFile {
            return Err(super::invalid(format!("node state entry {} must be a regular file", entry.name)));
        }
        self.read(&entry.path, max_bytes)
    }

    pub fn remove_entry(&self, entry: &NodeStateEntry) -> crate::error::Result<()> {
        self.validate_entry(entry)?;
        if entry.kind != super::authority::NodeStateEntryKind::RegularFile {
            return Err(super::invalid(format!("node state entry {} must be a regular file", entry.name)));
        }
        self.remove_regular_file(&entry.path)
    }

    pub fn open_database_file(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<std::fs::File> {
        super::filesystem::open_database_file(&self.dir, path.as_path())
    }

    pub(crate) fn try_clone_dir(&self) -> crate::error::Result<cap_std::fs::Dir> {
        self.dir.try_clone().map_err(crate::error::MoltenError::from)
    }

    fn validate_entry(&self, entry: &NodeStateEntry) -> crate::error::Result<()> {
        if !std::sync::Arc::ptr_eq(&self.root, &entry.root) || self.kind != entry.namespace || self.scope != entry.scope
        {
            return Err(super::invalid(format!(
                "node state entry {} belongs to a different root or namespace view",
                entry.name
            )));
        }
        Ok(())
    }
}

pub struct NodeStateEntry {
    pub(super) root: std::sync::Arc<super::authority::NodeStateInner>,
    pub(super) namespace: super::authority::NodeStateNamespaceKind,
    pub(super) scope: std::path::PathBuf,
    pub name: String,
    pub path: super::locator::NodeStatePath,
    pub kind: super::authority::NodeStateEntryKind,
}

impl std::fmt::Debug for NodeStateEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NodeStateEntry")
            .field("namespace", &self.namespace)
            .field("scope", &self.scope)
            .field("name", &self.name)
            .field("path", &self.path)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
