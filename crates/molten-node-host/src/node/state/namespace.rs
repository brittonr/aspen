use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use super::MAX_NODE_SECRET_BYTES;
use super::MAX_NODE_STATE_FILE_BYTES;
use super::authority::NodeStateEntryKind;
use super::authority::NodeStateFileObservation;
use super::authority::NodeStateInner;
use super::authority::NodeStateNamespaceKind;
use super::authority::NodeStateRoot;
use super::enumeration::list_entries;
use super::filesystem::create_dir_components;
use super::filesystem::entry_kind_optional;
use super::filesystem::observe_file;
use super::filesystem::open_database_file;
use super::filesystem::open_dir_components;
use super::filesystem::read_regular_file_bounded;
use super::filesystem::remove_regular_file;
use super::filesystem::validate_write_size;
use super::filesystem::write_regular_file;
use super::invalid;
use super::locator::NodeStatePath;
use super::locator::join_scope;

pub struct NodeStateNamespace {
    pub(super) root: Arc<NodeStateInner>,
    pub(super) kind: NodeStateNamespaceKind,
    pub(super) scope: PathBuf,
    pub(super) dir: cap_std::fs::Dir,
}

impl std::fmt::Debug for NodeStateNamespace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NodeStateNamespace").field("kind", &self.kind).finish_non_exhaustive()
    }
}

impl NodeStateNamespace {
    pub fn open(kind: NodeStateNamespaceKind, path: &Path) -> crate::error::Result<Self> {
        let root = NodeStateRoot::open(path)?;
        Self::from_dir(kind, root.try_clone_dir()?)
    }

    pub fn open_existing(kind: NodeStateNamespaceKind, path: &Path) -> crate::error::Result<Self> {
        let root = NodeStateRoot::open_existing(path)?;
        Self::from_dir(kind, root.try_clone_dir()?)
    }

    pub fn from_dir(kind: NodeStateNamespaceKind, dir: cap_std::fs::Dir) -> crate::error::Result<Self> {
        let root_dir = dir.try_clone().map_err(crate::error::MoltenError::from)?;
        Ok(Self {
            root: Arc::new(NodeStateInner { dir: root_dir }),
            kind,
            scope: PathBuf::new(),
            dir,
        })
    }

    pub fn kind(&self) -> NodeStateNamespaceKind {
        self.kind
    }

    pub fn read(&self, path: &NodeStatePath, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        read_regular_file_bounded(&self.dir, path.as_path(), max_bytes)
    }

    pub fn read_to_string(&self, path: &NodeStatePath, max_bytes: u64) -> crate::error::Result<String> {
        String::from_utf8(self.read(path, max_bytes)?)
            .map_err(|error| invalid(format!("node state file {} is not UTF-8: {error}", path.display())))
    }

    pub fn write(&self, path: &NodeStatePath, bytes: &[u8]) -> crate::error::Result<()> {
        validate_write_size(bytes, MAX_NODE_STATE_FILE_BYTES)?;
        write_regular_file(&self.dir, path.as_path(), bytes, None)
    }

    pub fn write_restricted(&self, path: &NodeStatePath, bytes: &[u8], unix_mode: u32) -> crate::error::Result<()> {
        validate_write_size(bytes, MAX_NODE_SECRET_BYTES)?;
        write_regular_file(&self.dir, path.as_path(), bytes, Some(unix_mode))
    }

    pub fn try_exists(&self, path: &NodeStatePath) -> crate::error::Result<bool> {
        entry_kind_optional(&self.dir, path.as_path()).map(|kind| kind.is_some())
    }

    pub fn entry_kind(&self, path: &NodeStatePath) -> crate::error::Result<Option<NodeStateEntryKind>> {
        entry_kind_optional(&self.dir, path.as_path())
    }

    pub fn observe_file(&self, path: &NodeStatePath) -> crate::error::Result<NodeStateFileObservation> {
        observe_file(&self.dir, path.as_path())
    }

    pub fn unix_mode(&self, path: &NodeStatePath) -> crate::error::Result<Option<u32>> {
        match self.observe_file(path)? {
            NodeStateFileObservation::Missing => Ok(None),
            NodeStateFileObservation::NonRegular(_) => {
                Err(invalid(format!("node state leaf {} must be a regular file", path.display())))
            }
            NodeStateFileObservation::Regular(file) => Ok(file.unix_mode()),
        }
    }

    pub fn remove_regular_file(&self, path: &NodeStatePath) -> crate::error::Result<()> {
        remove_regular_file(&self.dir, path.as_path())
    }

    pub fn create_dir_all(&self, path: &NodeStatePath) -> crate::error::Result<()> {
        create_dir_components(&self.dir, path.as_path())
    }

    pub fn list_entries(&self) -> crate::error::Result<Vec<NodeStateEntry>> {
        list_entries(&self.dir, &self.root, self.kind, &self.scope)
    }

    pub fn open_subdir(&self, path: &NodeStatePath) -> crate::error::Result<Self> {
        let scope = join_scope(&self.scope, path)?;
        create_dir_components(&self.dir, path.as_path())?;
        let dir = open_dir_components(&self.dir, path.as_path())?;
        Ok(Self {
            root: Arc::clone(&self.root),
            kind: self.kind,
            scope,
            dir,
        })
    }

    pub fn read_entry(&self, entry: &NodeStateEntry, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        self.validate_entry(entry)?;
        if entry.kind != NodeStateEntryKind::RegularFile {
            return Err(invalid(format!("node state entry {} must be a regular file", entry.name)));
        }
        self.read(&entry.path, max_bytes)
    }

    pub fn remove_entry(&self, entry: &NodeStateEntry) -> crate::error::Result<()> {
        self.validate_entry(entry)?;
        if entry.kind != NodeStateEntryKind::RegularFile {
            return Err(invalid(format!("node state entry {} must be a regular file", entry.name)));
        }
        self.remove_regular_file(&entry.path)
    }

    pub fn open_database_file(&self, path: &NodeStatePath) -> crate::error::Result<std::fs::File> {
        open_database_file(&self.dir, path.as_path())
    }

    pub(crate) fn try_clone_dir(&self) -> crate::error::Result<cap_std::fs::Dir> {
        self.dir.try_clone().map_err(crate::error::MoltenError::from)
    }

    fn validate_entry(&self, entry: &NodeStateEntry) -> crate::error::Result<()> {
        if !Arc::ptr_eq(&self.root, &entry.root) || self.kind != entry.namespace || self.scope != entry.scope {
            return Err(invalid(format!(
                "node state entry {} belongs to a different root or namespace view",
                entry.name
            )));
        }
        Ok(())
    }
}

pub struct NodeStateEntry {
    pub(super) root: Arc<NodeStateInner>,
    pub(super) namespace: NodeStateNamespaceKind,
    pub(super) scope: PathBuf,
    pub name: String,
    pub path: NodeStatePath,
    pub kind: NodeStateEntryKind,
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
