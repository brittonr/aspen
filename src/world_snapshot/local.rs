use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;

use molten_core::world_commit::WorldCommitRef;
use molten_core::world_snapshot::*;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use molten_node_host::node_state::RelativePath;

use super::localprofile::LogicalRuntimeImage;
use super::localprofile::image_descriptor;
use super::localruntime::RuntimeStore;
use super::*;
use crate::error::Failure;
use crate::error::Result;

/// One rooted, explicitly admitted runtime-spine composition. Clones share the
/// same recreated live store handle and reconstruction, not ambient root paths.
#[derive(Clone)]
pub struct LocalLogicalSnapshot {
    pub(super) root: DirectoryRoot,
    pub(super) trusted_signer: ed25519_dalek::VerifyingKey,
    pub(super) destination_ref: String,
    pub(super) cohort: SnapshotCohort,
    pub(super) state: Rc<RefCell<RestoreState>>,
}

#[derive(Default)]
pub(super) struct RestoreState {
    pub descriptor: Option<SnapshotDescriptor>,
    pub descriptor_ref: String,
    pub verified: BTreeMap<SnapshotComponentKind, Vec<u8>>,
    pub restored: BTreeSet<SnapshotComponentKind>,
    pub store: Option<RuntimeStore>,
    pub admission_ref: Option<String>,
    pub activation_attempted: bool,
    pub activated: bool,
    pub receipt_attempted: bool,
    pub runtime: Option<crate::runtime::RuntimeState>,
}

impl LocalLogicalSnapshot {
    pub fn open(state_root: &Path, trusted_signer: ed25519_dalek::VerifyingKey) -> Result<Self> {
        Self::from_root(DirectoryRoot::open_existing(state_root)?, trusted_signer)
    }

    pub fn from_root(root: DirectoryRoot, trusted_signer: ed25519_dalek::VerifyingKey) -> Result<Self> {
        let destination_ref = destination_identity(&root)?;
        Ok(Self {
            root,
            trusted_signer,
            destination_ref,
            cohort: observe_local_logical_cohort()?,
            state: Rc::new(RefCell::new(RestoreState::default())),
        })
    }

    pub fn destination_ref(&self) -> &str {
        &self.destination_ref
    }
    pub fn observed_cohort(&self) -> &SnapshotCohort {
        &self.cohort
    }

    /// Capture an already committed runtime state; this grants no restore authority.
    pub fn prepare_bundle(&self, image: &LogicalRuntimeImage, commit: WorldCommitRef) -> Result<SnapshotDescriptor> {
        let descriptor = image_descriptor(image, self.cohort.clone(), commit)?;
        for (kind, bytes) in image.projections(&self.cohort)? {
            let directory = RelativePath::parse(&format!("storage/world-snapshot/bundle/{}", kind.as_str()))?;
            self.root.create_dir_all(&directory)?;
            publish_exact(&self.root, &directory.join("state.preserves")?, &bytes)?;
        }
        Ok(descriptor)
    }

    pub fn component_identity(&self, kind: SnapshotComponentKind) -> Result<String> {
        Ok(crate::preserves_rail::content_ref_from_bytes(&self.component_payload(kind)?))
    }

    fn component_payload(&self, kind: SnapshotComponentKind) -> Result<Vec<u8>> {
        let storage = self.root.namespace(NamespaceKind::Storage)?;
        let tree = storage
            .existing_subdir(&RelativePath::parse(&format!("world-snapshot/bundle/{}", kind.as_str()))?)?
            .ok_or_else(|| Failure::invalid_harness(format!("missing snapshot component {}", kind.as_str())))?;
        let entries = tree.list_entries()?;
        if entries.len() != 1
            || entries[0].name != "state.preserves"
            || entries[0].kind != crate::node_state::EntryKind::RegularFile
        {
            return Err(Failure::invalid_harness("unsupported local snapshot component inventory"));
        }
        tree.read_entry(&entries[0], MAX_LOCAL_LOGICAL_IMAGE_BYTES)
    }

    pub(super) fn verified_image(&self) -> Result<LogicalRuntimeImage> {
        let state = self.state.borrow();
        let bytes = state
            .verified
            .get(&SnapshotComponentKind::DurableState)
            .ok_or_else(|| Failure::invalid_harness("runtime image missing"))?;
        let image = LogicalRuntimeImage::from_bytes(bytes)?;
        if image.projections(&self.cohort)? != state.verified || image.policy_ref != self.current_policy_ref()? {
            return Err(Failure::invalid_harness("runtime component semantics/profile/current policy do not match"));
        }
        Ok(image)
    }
}

#[derive(Debug)]
pub struct LocalLogicalRestoreOutcome {
    pub restore: LogicalSnapshotRestoreOutcome,
    /// The actual reconstructed owner. No unresolved effect has been dispatched.
    pub runtime: crate::runtime::RuntimeState,
}

/// Public execution composition. The expected descriptor can only further
/// constrain the observed process cohort, never establish compatibility.
// r[impl molten.world_snapshot.runtime_spine_restore]
// r[impl molten.world_snapshot.runtime_spine_activation]
pub fn restore_local_logical_snapshot(
    adapter: LocalLogicalSnapshot,
    descriptor: &SnapshotDescriptor,
    expected_destination: &SnapshotCohort,
) -> Result<LocalLogicalRestoreOutcome> {
    if descriptor.class != SnapshotClass::Logical
        || descriptor.profile_ref.as_str() != local_logical_profile_ref()?
        || expected_destination != &adapter.cohort
    {
        return Err(Failure::invalid_harness(
            "unsupported logical profile or expected destination differs from observed runtime cohort; opaque/VM restore denied",
        ));
    }
    {
        let mut state = adapter.state.borrow_mut();
        state.descriptor_ref = canonical_snapshot_descriptor(descriptor)?.artifact_ref;
        state.descriptor = Some(descriptor.clone());
    }
    let mut materialization = adapter.clone();
    let mut admission = adapter.clone();
    let mut handles = adapter.clone();
    let mut runtime = adapter.clone();
    let mut receipts = adapter.clone();
    let restore = restore_logical_snapshot(descriptor, &adapter.cohort, LogicalSnapshotPorts {
        materialization: &mut materialization,
        admission: &mut admission,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    })
    .map_err(|error| {
        let state = adapter.state.borrow();
        let outcome = if state.activated && state.receipt_attempted {
            "activation=completed receipt-publication=failed"
        } else if state.activated {
            "activation=completed post-activation-readback=failed"
        } else if state.activation_attempted {
            "activation=uncertain automatic-retry=denied"
        } else {
            "activation=inactive reconstruction=partial-or-absent"
        };
        Failure::invalid_harness(format!("{outcome} cleanup=retained-for-reconciliation: {error}"))
    })?;
    let runtime = adapter
        .state
        .borrow_mut()
        .runtime
        .take()
        .ok_or_else(|| Failure::invalid_harness("activation=completed reconstructed-runtime-owner=missing"))?;
    Ok(LocalLogicalRestoreOutcome { restore, runtime })
}

impl SnapshotMaterializationPort for LocalLogicalSnapshot {
    fn observe_component(&mut self, component: &SnapshotComponent) -> Result<SnapshotMaterializationObservation> {
        let bytes = self.component_payload(component.kind)?;
        let identity = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let is_verified = identity == component.identity;
        if is_verified {
            crate::preserves_rail::strict_canonical_decode(&bytes)?;
            self.state.borrow_mut().verified.insert(component.kind, bytes);
        }
        Ok(SnapshotMaterializationObservation {
            component_identity: component.identity.clone(),
            observation_ref: identity,
            available: true,
            identity_verified: is_verified,
        })
    }
}

impl CurrentSnapshotAdmissionPort for LocalLogicalSnapshot {
    fn observe_current(
        &mut self,
        descriptor: &SnapshotDescriptor,
        descriptor_ref: &str,
        destination: &SnapshotCohort,
    ) -> Result<SnapshotAdmissionObservation> {
        if destination != &self.cohort {
            return Err(Failure::invalid_harness("destination cohort is not observed"));
        }
        let admission = self.admitted(descriptor, descriptor_ref)?;
        let mut state = self.state.borrow_mut();
        if state.admission_ref.as_ref().is_some_and(|prior| prior != &admission.admission_ref) {
            return Err(Failure::invalid_harness("current admission changed during reconstruction"));
        }
        state.admission_ref = Some(admission.admission_ref.clone());
        Ok(admission)
    }
}

impl SnapshotHostHandlePort for LocalLogicalSnapshot {
    fn recreate_handles(&mut self, descriptor_ref: &str) -> Result<String> {
        self.verified_image()?;
        self.state
            .borrow()
            .store
            .as_ref()
            .ok_or_else(|| Failure::invalid_harness("staged live runtime store handle missing"))?
            .verify_handle()?;
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "runtime-spine-recreated-handles-v1",
            vec![
                crate::preserves_rail::string(descriptor_ref),
                crate::preserves_rail::string(&self.destination_ref),
                crate::preserves_rail::string("capability-derived-ledger/redb"),
            ],
        ))
    }
}

impl SnapshotReceiptPort for LocalLogicalSnapshot {
    fn publish_receipt(&mut self, receipt_ref: &str, bytes: &[u8]) -> Result<()> {
        self.state.borrow_mut().receipt_attempted = true;
        crate::preserves_rail::validate_content_ref(receipt_ref)?;
        crate::preserves_rail::strict_canonical_decode(bytes)?;
        let directory = RelativePath::parse("storage/world-snapshot/receipts")?;
        self.root.create_dir_all(&directory)?;
        publish_exact(
            &self.root,
            &directory.join(&format!("{}.preserves", receipt_ref.trim_start_matches("blake3:")))?,
            bytes,
        )
    }
}

pub(super) fn publish_exact(root: &DirectoryRoot, path: &RelativePath, bytes: &[u8]) -> Result<()> {
    if root.try_exists(path)? {
        if root.read(path, MAX_LOCAL_LOGICAL_IMAGE_BYTES)? != bytes {
            return Err(Failure::invalid_harness("snapshot publication conflicts with retained bytes"));
        }
        Ok(())
    } else {
        root.write_new(path, bytes)
    }
}

pub(super) fn destination_identity(root: &DirectoryRoot) -> Result<String> {
    let mut values = Vec::with_capacity(2);
    for name in ["config.preserves", "identity.preserves"] {
        let bytes = root.read(&RelativePath::parse(name)?, 1_048_576)?;
        crate::preserves_rail::strict_canonical_decode(&bytes)?;
        values.push(crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&bytes)));
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("runtime-spine-destination-v2", values))
}
