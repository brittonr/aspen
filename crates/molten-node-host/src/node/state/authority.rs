const NODE_STATE_NAMESPACE_COUNT: usize = 14;

/// Closed namespace inventory; additions require registry and mapping review.
/// Directory aliases retain distinct kind identities and entry authority.
#[cfg_attr(dylint_lib = "octet", octet::sealed_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeStateNamespaceKind {
    Identity,
    Secrets,
    Ledger,
    ControlInbox,
    ControlOutbox,
    ControlIngress,
    ControlIdempotency,
    ControlService,
    Services,
    Receipts,
    Registry,
    Chunks,
    Storage,
    Ingress,
}

impl NodeStateNamespaceKind {
    pub const ALL: [Self; NODE_STATE_NAMESPACE_COUNT] = [
        Self::Identity,
        Self::Secrets,
        Self::Ledger,
        Self::ControlInbox,
        Self::ControlOutbox,
        Self::ControlIngress,
        Self::ControlIdempotency,
        Self::ControlService,
        Self::Services,
        Self::Receipts,
        Self::Registry,
        Self::Chunks,
        Self::Storage,
        Self::Ingress,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity | Self::Secrets => "identity",
            Self::Ledger => "ledger",
            Self::ControlInbox => "control/inbox",
            Self::ControlOutbox => "control/outbox",
            Self::ControlIngress => "control/iroh-ingress",
            Self::ControlIdempotency => "control/idempotency",
            Self::ControlService => "control/service",
            Self::Services => "services",
            Self::Receipts => "receipts",
            Self::Registry => "registry",
            Self::Chunks => "chunks",
            Self::Storage => "storage",
            Self::Ingress => "control/iroh-ingress",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStateEntryKind {
    RegularFile,
    Directory,
    Symlink,
    Other,
}

/// Closed acquired-file classification; each consumer must handle new outcomes.
/// Missing and non-regular leaves never inherit regular-file read authority.
#[cfg_attr(dylint_lib = "octet", octet::sealed_enum)]
#[derive(Debug)]
pub enum NodeStateFileObservation {
    Missing,
    NonRegular(NodeStateEntryKind),
    Regular(NodeStateFile),
}

pub struct NodeStateFile {
    pub(super) file: cap_std::fs::File,
    pub(super) size: u64,
    pub(super) unix_mode: Option<u32>,
}

impl std::fmt::Debug for NodeStateFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NodeStateFile")
            .field("size", &self.size)
            .field("unix_mode", &self.unix_mode)
            .finish_non_exhaustive()
    }
}

impl NodeStateFile {
    pub fn unix_mode(&self) -> Option<u32> {
        self.unix_mode
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn read_bounded(self, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        super::filesystem::read::consume(self, max_bytes, "observed node state file")
    }
}

pub(super) struct NodeStateInner {
    pub(super) dir: cap_std::fs::Dir,
}

#[derive(Clone)]
pub struct NodeStateRoot {
    inner: std::sync::Arc<NodeStateInner>,
}

impl std::fmt::Debug for NodeStateRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NodeStateRoot").finish_non_exhaustive()
    }
}

impl NodeStateRoot {
    pub fn open(path: &std::path::Path) -> crate::error::Result<Self> {
        super::filesystem::validate_bootstrap_path(path)?;
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => super::filesystem::validate_bootstrap_metadata(&metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(path).map_err(crate::error::MoltenError::from)?;
            }
            Err(error) => return Err(crate::error::MoltenError::from(error)),
        }
        Self::open_existing(path)
    }

    pub fn open_existing(path: &std::path::Path) -> crate::error::Result<Self> {
        super::filesystem::validate_bootstrap_path(path)?;
        let metadata = std::fs::symlink_metadata(path).map_err(crate::error::MoltenError::from)?;
        super::filesystem::validate_bootstrap_metadata(&metadata)?;
        let dir = cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())
            .map_err(crate::error::MoltenError::from)?;
        Ok(Self::from_dir(dir))
    }

    pub fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            inner: std::sync::Arc::new(NodeStateInner { dir }),
        }
    }

    pub fn create_layout(&self) -> crate::error::Result<()> {
        for kind in NodeStateNamespaceKind::ALL {
            self.create_dir_all(&super::locator::NodeStatePath::parse(kind.as_str())?)?;
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
            self.create_dir_all(&super::locator::NodeStatePath::parse(path)?)?;
        }
        Ok(())
    }

    pub fn namespace(
        &self,
        kind: NodeStateNamespaceKind,
    ) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        let path = super::locator::NodeStatePath::parse(kind.as_str())?;
        self.create_dir_all(&path)?;
        let dir = super::filesystem::open_dir_components(&self.inner.dir, path.as_path())?;
        Ok(super::namespace::NodeStateNamespace {
            root: std::sync::Arc::clone(&self.inner),
            kind,
            scope: path.into_path_buf(),
            dir,
        })
    }

    pub fn identity(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::Identity)
    }

    pub fn secrets(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::Secrets)
    }

    pub fn ledger(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::Ledger)
    }

    pub fn control_inbox(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::ControlInbox)
    }

    pub fn control_outbox(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::ControlOutbox)
    }

    pub fn control_ingress(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::ControlIngress)
    }

    pub fn control_idempotency(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::ControlIdempotency)
    }

    pub fn control_service(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::ControlService)
    }

    pub fn receipts(&self) -> crate::error::Result<super::namespace::NodeStateNamespace> {
        self.namespace(NodeStateNamespaceKind::Receipts)
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
        let namespace = self.namespace(NodeStateNamespaceKind::Registry)?;
        Ok(crate::local_store::ArtifactStoreRoot::from_dir(namespace.try_clone_dir()?))
    }

    pub fn chunk_store(&self) -> crate::error::Result<crate::local_store::ChunkStoreRoot> {
        let namespace = self.namespace(NodeStateNamespaceKind::Chunks)?;
        Ok(crate::local_store::ChunkStoreRoot::from_dir(namespace.try_clone_dir()?))
    }

    pub fn read(&self, path: &super::locator::NodeStatePath, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
        super::filesystem::read_regular_file_bounded(&self.inner.dir, path.as_path(), max_bytes)
    }

    pub fn read_to_string(&self, path: &super::locator::NodeStatePath, max_bytes: u64) -> crate::error::Result<String> {
        String::from_utf8(self.read(path, max_bytes)?)
            .map_err(|error| super::invalid(format!("node state file {} is not UTF-8: {error}", path.display())))
    }

    pub fn write(&self, path: &super::locator::NodeStatePath, bytes: &[u8]) -> crate::error::Result<()> {
        super::filesystem::validate_write_size(bytes, super::MAX_NODE_STATE_FILE_BYTES)?;
        super::filesystem::write_regular_file(&self.inner.dir, path.as_path(), bytes, None)
    }

    pub fn try_exists(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<bool> {
        super::filesystem::entry_kind_optional(&self.inner.dir, path.as_path()).map(|kind| kind.is_some())
    }

    pub fn entry_kind(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<Option<NodeStateEntryKind>> {
        super::filesystem::entry_kind_optional(&self.inner.dir, path.as_path())
    }

    pub fn remove_regular_file(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<()> {
        super::filesystem::remove_regular_file(&self.inner.dir, path.as_path())
    }

    pub fn create_dir_all(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<()> {
        super::filesystem::create_dir_components(&self.inner.dir, path.as_path())
    }

    pub fn open_database_file(&self, path: &super::locator::NodeStatePath) -> crate::error::Result<std::fs::File> {
        super::filesystem::open_database_file(&self.inner.dir, path.as_path())
    }

    pub(crate) fn try_clone_dir(&self) -> crate::error::Result<cap_std::fs::Dir> {
        self.inner.dir.try_clone().map_err(crate::error::MoltenError::from)
    }
}
