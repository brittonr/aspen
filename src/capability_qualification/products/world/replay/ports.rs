//! The destination-side restore, transition, capture and receipt ports of one
//! replay over the shared destination runtime-spine world.

use crate::world_replay::WorldReplayCapturePort;
use crate::world_replay::WorldReplayReceiptPort;
use crate::world_replay::WorldReplayRestorePort;
use crate::world_replay::WorldReplayTransitionPort;

const RECEIPT_DIRECTORY: &str = "storage/world-replay/receipts";

/// The destination runtime-spine world shared by the restore, transition and
/// capture ports of one replay.
pub(super) struct Destination {
    pub(super) node: super::super::fixture::NodeRoot,
    pub(super) descriptor: molten_core::world_snapshot::SnapshotDescriptor,
    pub(super) store: crate::world_commit::LocalWorldCommitStore,
    pub(super) world: Option<super::super::runtime::CommittedState>,
    pub(super) parent: Option<molten_core::world_commit::WorldCommitRef>,
    pub(super) restores: usize,
    pub(super) executed: Vec<u64>,
    pub(super) captured: usize,
}

impl Destination {
    /// The not-yet-restored destination, shared by the ports of one replay.
    pub(super) fn shared(
        node: super::super::fixture::NodeRoot,
        descriptor: molten_core::world_snapshot::SnapshotDescriptor,
        store: crate::world_commit::LocalWorldCommitStore,
    ) -> Shared {
        std::rc::Rc::new(std::cell::RefCell::new(Self {
            node,
            descriptor,
            store,
            world: None,
            parent: None,
            restores: 0,
            executed: Vec::new(),
            captured: 0,
        }))
    }
}

pub(super) type Shared = std::rc::Rc<std::cell::RefCell<Destination>>;

/// Restore the trace's initial commit through the runtime-spine logical
/// snapshot restore; the restored owner must reproduce that commit.
pub(super) struct SnapshotRestore(pub(super) Shared);

impl WorldReplayRestorePort for SnapshotRestore {
    fn restore_logical(
        &mut self,
        profile: &molten_core::world_replay::WorldReplayProfile,
        initial_commit: &crate::world_commit::CanonicalWorldCommit,
    ) -> crate::error::Result<crate::world_replay::WorldReplayRestoreObservation> {
        let mut guard = self.0.borrow_mut();
        let destination = &mut *guard;
        if profile.profile_ref.as_str() != crate::world_snapshot::local_logical_profile_ref()?
            || profile.cohort_ref.is_some()
        {
            return Err(super::super::invalid("replay profile is not the local logical runtime-spine profile"));
        }
        if destination.descriptor.commit_ref != initial_commit.commit_ref {
            return Err(super::super::invalid("snapshot descriptor binds another initial world commit"));
        }
        let adapter = destination.node.adapter()?;
        let observed = adapter.observed_cohort().clone();
        let restored =
            crate::world_snapshot::restore_local_logical_snapshot(adapter, &destination.descriptor, &observed)?;
        let readback = crate::world_snapshot::read_local_logical_runtime(&destination.node.root)?;
        let world = super::super::runtime::CommittedState::from_image(
            crate::world_snapshot::LogicalRuntimeImage::from_bytes(&readback.canonical_image)?,
            restored.runtime,
        )?;
        let (commit, _) =
            super::super::runtime::capture(&mut destination.store, &world, initial_commit.core.parents.clone())?;
        if commit.commit_ref != initial_commit.commit_ref {
            return Err(super::super::invalid("restored runtime does not reproduce the initial world commit"));
        }
        destination.world = Some(world);
        destination.parent = Some(commit.commit_ref);
        destination.restores = destination
            .restores
            .checked_add(1)
            .ok_or_else(|| super::super::invalid("restore count overflowed"))?;
        Ok(crate::world_replay::WorldReplayRestoreObservation {
            observation_ref: restored.restore.receipt.artifact_ref,
            profile_ref: profile.profile_ref.as_str().to_string(),
            cohort_ref: None,
            logical_fallback_used: false,
        })
    }

    fn restore_opaque_exact(
        &mut self,
        _profile: &molten_core::world_replay::WorldReplayProfile,
        _initial_commit: &crate::world_commit::CanonicalWorldCommit,
    ) -> crate::error::Result<crate::world_replay::WorldReplayRestoreObservation> {
        Err(super::super::invalid("the runtime-spine destination restores only the logical profile"))
    }
}

/// Execute each recorded committed turn on the restored runtime owner.
pub(super) struct RuntimeTransitions {
    pub(super) destination: Shared,
    pub(super) inputs: std::collections::BTreeMap<String, crate::runtime::RuntimeStep>,
}

impl WorldReplayTransitionPort for RuntimeTransitions {
    fn execute_transition(
        &mut self,
        step: &molten_core::world_replay::WorldTransitionStep,
        _observations: &mut crate::world_replay::SealedReplayObservations,
    ) -> crate::error::Result<crate::world_replay::WorldReplayExecutionObservation> {
        let input = self
            .inputs
            .get(&step.input.input_ref)
            .ok_or_else(|| super::super::invalid("replay input is absent from the recorded inputs"))?;
        let bytes = crate::preserves_rail::canonical_bytes(&input.to_value())?;
        if input.step_ref()? != step.input.input_ref
            || step.input.schema_ref != super::history::step_schema_ref()
            || u64::try_from(bytes.len()).ok() != Some(step.input.byte_length)
        {
            return Err(super::super::invalid("replay input identity, schema or length differs from its trace step"));
        }
        let mut guard = self.destination.borrow_mut();
        let destination = &mut *guard;
        let world = destination
            .world
            .as_mut()
            .ok_or_else(|| super::super::invalid("transition executed before the runtime owner was restored"))?;
        let events = world.apply(input);
        destination.executed.push(step.position);
        Ok(crate::world_replay::WorldReplayExecutionObservation {
            observation_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "aspen-runtime-turn-observation-v1",
                vec![
                    crate::preserves_rail::u64_value(step.position),
                    crate::preserves_rail::string(&step.input.input_ref),
                    crate::preserves_rail::sequence(
                        events.iter().map(crate::runtime::RuntimeEvent::to_value).collect(),
                    ),
                ],
            ))?,
            position: step.position,
            input_ref: step.input.input_ref.clone(),
        })
    }
}

/// Capture each successor through the production capture shell.
pub(super) struct RuntimeCapture(pub(super) Shared);

impl WorldReplayCapturePort for RuntimeCapture {
    fn capture_successor(
        &mut self,
        step: &molten_core::world_replay::WorldTransitionStep,
        _execution: &crate::world_replay::WorldReplayExecutionObservation,
    ) -> crate::error::Result<crate::world_replay::WorldReplayCaptureObservation> {
        let mut guard = self.0.borrow_mut();
        let destination = &mut *guard;
        let parent = destination
            .parent
            .clone()
            .ok_or_else(|| super::super::invalid("successor captured before the initial commit was restored"))?;
        let world = destination
            .world
            .clone()
            .ok_or_else(|| super::super::invalid("successor captured without a runtime owner"))?;
        let (commit, receipt) = super::super::runtime::capture(&mut destination.store, &world, vec![parent.clone()])?;
        destination.parent = Some(commit.commit_ref.clone());
        destination.captured = destination
            .captured
            .checked_add(1)
            .ok_or_else(|| super::super::invalid("capture count overflowed"))?;
        Ok(crate::world_replay::WorldReplayCaptureObservation {
            observation_ref: receipt.receipt_ref,
            transition: molten_core::world_replay::WorldReplayTransitionObservation {
                position: step.position,
                observed_parent: parent,
                actual: molten_core::world_replay::WorldReplayObservedCommit {
                    commit_ref: commit.commit_ref,
                    roots: commit.core.roots,
                },
                field_differences: Vec::new(),
            },
        })
    }
}

/// Durable replay receipts in the destination node.
pub(super) struct Receipts {
    pub(super) root: crate::node_state::DirectoryRoot,
    pub(super) kinds: Vec<&'static str>,
}

impl WorldReplayReceiptPort for Receipts {
    fn publish(&mut self, record: &crate::world_replay::CanonicalWorldReplayRecord) -> crate::error::Result<String> {
        crate::preserves_rail::strict_canonical_decode(&record.bytes)?;
        let directory = crate::node_state::RelativePath::parse(RECEIPT_DIRECTORY)?;
        self.root.create_dir_all(&directory)?;
        let path =
            directory.join(&format!("{}.preserves", crate::preserves_rail::content_ref_hex(&record.record_ref)?))?;
        if self.root.try_exists(&path)? {
            if self.root.read(&path, crate::world_snapshot::MAX_LOCAL_LOGICAL_IMAGE_BYTES)? != record.bytes {
                return Err(super::super::invalid("replay receipt publication conflicts with retained bytes"));
            }
        } else {
            self.root.write_new(&path, &record.bytes)?;
        }
        self.kinds.push(record.kind);
        Ok(record.record_ref.clone())
    }
}
