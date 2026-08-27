use std::fmt;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use molten_core::world_commit::RestoreStep;
use molten_core::world_commit::RevisionFence;
use molten_core::world_commit::RevisionRecheck;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::RootObservation;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_node_host::node_state::MAX_NODE_STATE_FILE_BYTES;
use molten_node_host::node_state::NodeStateNamespace;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStatePath;

use super::CanonicalCaptureReceipt;
use super::PublicationOutcome;
use crate::error::MoltenError;
use crate::error::Result;

const WORLD_COMMIT_DIRECTORY: &str = "world-commits";
const WORLD_COMMIT_OBJECT_DIRECTORY: &str = "objects";
const WORLD_COMMIT_COMMIT_DIRECTORY: &str = "commits";
const WORLD_COMMIT_RECEIPT_DIRECTORY: &str = "receipts";
const ROOT_FILE_SUFFIX: &str = ".root.preserves";
const COMMIT_FILE_SUFFIX: &str = ".commit.preserves";
const RECEIPT_FILE_SUFFIX: &str = ".receipt.preserves";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCommitPortError {
    pub class: &'static str,
    pub message: String,
}

impl WorldCommitPortError {
    pub fn new(class: &'static str, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

impl fmt::Display for WorldCommitPortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.class, self.message)
    }
}

impl std::error::Error for WorldCommitPortError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRootMaterial {
    pub observation: RootObservation,
    pub canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreStepOutcome {
    pub step: RestoreStep,
    pub evidence_ref: String,
}

// r[impl molten.world_commit.capture]
pub trait WorldRootObservationPort {
    fn observe_root(&mut self, kind: RootKind) -> std::result::Result<ObservedRootMaterial, WorldCommitPortError>;
}

// r[impl molten.world_commit.capture]
pub trait WorldImmutableObjectPort {
    fn contains_root(&self, root: &WorldRootRef) -> std::result::Result<bool, WorldCommitPortError>;
    fn persist_root(
        &mut self,
        root: &WorldRootRef,
        canonical_bytes: &[u8],
    ) -> std::result::Result<(), WorldCommitPortError>;
    fn read_root(&self, root: &WorldRootRef) -> std::result::Result<Vec<u8>, WorldCommitPortError>;
}

// r[impl molten.world_commit.capture]
pub trait WorldRevisionRecheckPort {
    fn recheck_revision(&mut self, fence: &RevisionFence)
    -> std::result::Result<RevisionRecheck, WorldCommitPortError>;
}

// r[impl molten.world_commit.capture]
pub trait WorldCommitPublicationPort {
    fn publish_commit(
        &mut self,
        commit_ref: &WorldCommitRef,
        canonical_bytes: &[u8],
    ) -> std::result::Result<PublicationOutcome, WorldCommitPortError>;
    fn read_commit(&self, commit_ref: &WorldCommitRef) -> std::result::Result<Vec<u8>, WorldCommitPortError>;
}

// r[impl molten.world_commit.restore]
pub trait WorldRestorePort {
    fn execute_restore_step(
        &mut self,
        step: &RestoreStep,
    ) -> std::result::Result<RestoreStepOutcome, WorldCommitPortError>;
}

#[derive(Debug)]
pub struct LocalWorldCommitStore {
    objects: NodeStateNamespace,
    commits: NodeStateNamespace,
    receipts: NodeStateNamespace,
}

impl LocalWorldCommitStore {
    pub fn open(storage: &NodeStateNamespace) -> Result<Self> {
        if storage.kind() != NodeStateNamespaceKind::Storage {
            return Err(MoltenError::invalid_harness("local world commit store requires the storage namespace"));
        }
        let root = storage.open_subdir(&NodeStatePath::parse(WORLD_COMMIT_DIRECTORY)?)?;
        let objects = root.open_subdir(&NodeStatePath::parse(WORLD_COMMIT_OBJECT_DIRECTORY)?)?;
        let commits = root.open_subdir(&NodeStatePath::parse(WORLD_COMMIT_COMMIT_DIRECTORY)?)?;
        let receipts = root.open_subdir(&NodeStatePath::parse(WORLD_COMMIT_RECEIPT_DIRECTORY)?)?;
        Ok(Self {
            objects,
            commits,
            receipts,
        })
    }

    pub fn write_capture_receipt(&self, receipt: &CanonicalCaptureReceipt) -> Result<()> {
        let path = digest_path(&receipt.receipt_ref, RECEIPT_FILE_SUFFIX)?;
        sync_write_exact(&self.receipts, &path, &receipt.bytes).map_err(port_to_molten)
    }

    pub fn read_capture_receipt(&self, receipt_ref: &str) -> Result<Vec<u8>> {
        let path = digest_path(receipt_ref, RECEIPT_FILE_SUFFIX)?;
        self.receipts.read(&path, MAX_NODE_STATE_FILE_BYTES)
    }
}

impl WorldImmutableObjectPort for LocalWorldCommitStore {
    fn contains_root(&self, root: &WorldRootRef) -> std::result::Result<bool, WorldCommitPortError> {
        let path = root_path(root)?;
        self.objects
            .try_exists(&path)
            .map_err(|error| WorldCommitPortError::new("root-existence", error.to_string()))
    }

    fn persist_root(
        &mut self,
        root: &WorldRootRef,
        canonical_bytes: &[u8],
    ) -> std::result::Result<(), WorldCommitPortError> {
        crate::preserves_rail::strict_canonical_decode(canonical_bytes)
            .map_err(|error| WorldCommitPortError::new("root-noncanonical", error.to_string()))?;
        let observed = crate::preserves_rail::content_ref_from_bytes(canonical_bytes);
        if observed != root.as_str() {
            return Err(WorldCommitPortError::new(
                "root-identity-mismatch",
                format!("expected {}, got {observed}", root.as_str()),
            ));
        }
        let path = root_path(root)?;
        sync_write_exact(&self.objects, &path, canonical_bytes)
    }

    fn read_root(&self, root: &WorldRootRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        let path = root_path(root)?;
        let bytes = self
            .objects
            .read(&path, MAX_NODE_STATE_FILE_BYTES)
            .map_err(|error| WorldCommitPortError::new("root-read", error.to_string()))?;
        let observed = crate::preserves_rail::content_ref_from_bytes(&bytes);
        if observed != root.as_str() {
            return Err(WorldCommitPortError::new(
                "root-readback-mismatch",
                format!("expected {}, got {observed}", root.as_str()),
            ));
        }
        crate::preserves_rail::strict_canonical_decode(&bytes)
            .map_err(|error| WorldCommitPortError::new("root-readback-noncanonical", error.to_string()))?;
        Ok(bytes)
    }
}

impl WorldCommitPublicationPort for LocalWorldCommitStore {
    fn publish_commit(
        &mut self,
        commit_ref: &WorldCommitRef,
        canonical_bytes: &[u8],
    ) -> std::result::Result<PublicationOutcome, WorldCommitPortError> {
        super::parse_canonical_world_commit_with_ref(canonical_bytes, commit_ref, &store_bounds())
            .map_err(|error| WorldCommitPortError::new("commit-validation", error.to_string()))?;
        let path = digest_path(commit_ref.as_str(), COMMIT_FILE_SUFFIX)
            .map_err(|error| WorldCommitPortError::new("commit-path", error.to_string()))?;
        if self
            .commits
            .try_exists(&path)
            .map_err(|error| WorldCommitPortError::new("commit-existence", error.to_string()))?
        {
            let existing = self
                .commits
                .read(&path, MAX_NODE_STATE_FILE_BYTES)
                .map_err(|error| WorldCommitPortError::new("commit-read", error.to_string()))?;
            if existing == canonical_bytes {
                return Ok(PublicationOutcome::AlreadyPresent);
            }
            return Err(WorldCommitPortError::new(
                "commit-conflict",
                "existing commit bytes differ for the same identity",
            ));
        }
        sync_write_exact(&self.commits, &path, canonical_bytes)?;
        Ok(PublicationOutcome::Published)
    }

    fn read_commit(&self, commit_ref: &WorldCommitRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        let path = digest_path(commit_ref.as_str(), COMMIT_FILE_SUFFIX)
            .map_err(|error| WorldCommitPortError::new("commit-path", error.to_string()))?;
        self.commits
            .read(&path, MAX_NODE_STATE_FILE_BYTES)
            .map_err(|error| WorldCommitPortError::new("commit-read", error.to_string()))
    }
}

fn root_path(root: &WorldRootRef) -> std::result::Result<NodeStatePath, WorldCommitPortError> {
    let digest = crate::preserves_rail::content_ref_hex(root.as_str())
        .map_err(|error| WorldCommitPortError::new("root-path", error.to_string()))?;
    NodeStatePath::parse(&format!("{}-{digest}{ROOT_FILE_SUFFIX}", root.kind().as_str()))
        .map_err(|error| WorldCommitPortError::new("root-path", error.to_string()))
}

fn digest_path(reference: &str, suffix: &str) -> Result<NodeStatePath> {
    let digest = crate::preserves_rail::content_ref_hex(reference)?;
    NodeStatePath::parse(&format!("{digest}{suffix}"))
}

fn sync_write_exact(
    namespace: &NodeStateNamespace,
    path: &NodeStatePath,
    bytes: &[u8],
) -> std::result::Result<(), WorldCommitPortError> {
    let byte_count = u64::try_from(bytes.len())
        .map_err(|_| WorldCommitPortError::new("durable-write-size", "byte count conversion overflow"))?;
    if byte_count > MAX_NODE_STATE_FILE_BYTES {
        return Err(WorldCommitPortError::new(
            "durable-write-size",
            format!("byte count {byte_count} exceeds {MAX_NODE_STATE_FILE_BYTES}"),
        ));
    }
    if namespace
        .try_exists(path)
        .map_err(|error| WorldCommitPortError::new("durable-write-existence", error.to_string()))?
    {
        let existing = namespace
            .read(path, MAX_NODE_STATE_FILE_BYTES)
            .map_err(|error| WorldCommitPortError::new("durable-write-readback", error.to_string()))?;
        if existing == bytes {
            return Ok(());
        }
        return Err(WorldCommitPortError::new("durable-write-conflict", "existing immutable object bytes differ"));
    }
    let mut file = namespace
        .open_database_file(path)
        .map_err(|error| WorldCommitPortError::new("durable-write-open", error.to_string()))?;
    file.set_len(0)
        .map_err(|error| WorldCommitPortError::new("durable-write-truncate", error.to_string()))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| WorldCommitPortError::new("durable-write-seek", error.to_string()))?;
    file.write_all(bytes)
        .map_err(|error| WorldCommitPortError::new("durable-write-bytes", error.to_string()))?;
    file.sync_all()
        .map_err(|error| WorldCommitPortError::new("durable-write-sync", error.to_string()))?;
    let readback = namespace
        .read(path, MAX_NODE_STATE_FILE_BYTES)
        .map_err(|error| WorldCommitPortError::new("durable-write-readback", error.to_string()))?;
    if readback != bytes {
        return Err(WorldCommitPortError::new(
            "durable-write-readback-mismatch",
            "readback bytes differ from supplied immutable object",
        ));
    }
    Ok(())
}

fn store_bounds() -> molten_core::world_commit::WorldCommitBounds {
    molten_core::world_commit::WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: molten_core::world_commit::MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

fn port_to_molten(error: WorldCommitPortError) -> MoltenError {
    MoltenError::invalid_harness(format!("local world commit store failed: {error}"))
}
