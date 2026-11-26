//! Runtime-spine restore with current admission (`runtime-spine-committed-turn-v1`).

use molten_core::world_snapshot::SnapshotDescriptor;
use molten_core::world_snapshot::SnapshotRestoreStep;

use super::runtime::NodeRoot;
use super::runtime::SnapshotGrant;
use super::runtime::WorldState;
use super::runtime::admission_document;
use super::runtime::capture;
use super::runtime::commit_store;
use super::runtime::descriptor_ref;
use super::runtime::fixture_key;
use super::runtime::reference;
use crate::error::Result;
use crate::runtime::RuntimeEvent;
use crate::runtime::RuntimeState;
use crate::runtime::RuntimeStep;
use crate::world_snapshot::LocalCurrentAdmission;
use crate::world_snapshot::LogicalRuntimeImage;
use crate::world_snapshot::read_local_logical_runtime;
use crate::world_snapshot::restore_local_logical_snapshot;

const GENERATION: u64 = 1;
const CURRENT_REVOCATION: &str = "aspen qualification restore revocation epoch 1";
const RANDOM_UPPER: u64 = 100;

/// A captured, bundled and admitted runtime world in one destination node root.
struct Prepared {
    node: NodeRoot,
    world: WorldState,
    image: LogicalRuntimeImage,
    commit_ref: molten_core::world_commit::WorldCommitRef,
    descriptor: SnapshotDescriptor,
    descriptor_ref: String,
}

impl Prepared {
    fn new(path: &std::path::Path, label: &str) -> Result<Self> {
        let node = NodeRoot::open(path, label)?;
        let world = restorable_world()?;
        let image = world.image();
        let mut store = commit_store(&node.root)?;
        let (commit, _) = capture(&mut store, &world, Vec::new())?;
        let descriptor = node.adapter()?.prepare_bundle(&image, commit.commit_ref.clone())?;
        let descriptor_ref = descriptor_ref(&descriptor)?;
        Ok(Self {
            node,
            world,
            image,
            commit_ref: commit.commit_ref,
            descriptor,
            descriptor_ref,
        })
    }

    fn admit(&self, granted: &LocalCurrentAdmission, current: &LocalCurrentAdmission) -> Result<()> {
        self.node.admit(SnapshotGrant {
            signer: &self.node.signer,
            descriptor: &self.descriptor,
            descriptor_ref: &self.descriptor_ref,
            granted,
            current,
        })
    }

    fn restore(&self) -> Result<crate::world_snapshot::LocalLogicalRestoreOutcome> {
        let adapter = self.node.adapter()?;
        let observed = adapter.observed_cohort().clone();
        restore_local_logical_snapshot(adapter, &self.descriptor, &observed)
    }
}

/// The committed world plus an open facet and one unresolved clock request that
/// restore must retain and never dispatch.
fn restorable_world() -> Result<WorldState> {
    let mut world = WorldState::committed()?;
    let ready = crate::runtime::RuntimeValue::string("ready")?;
    let (events, _) = world.runtime.apply_facet_create("producer", "facet:root", None, None)?;
    world.history.extend(events.iter().map(RuntimeEvent::to_value));
    let (events, _) = world.runtime.apply_facet_assert("producer", "facet:root", &ready)?;
    world.history.extend(events.iter().map(RuntimeEvent::to_value));
    let unresolved = world
        .runtime
        .begin_effect_for_step(&RuntimeStep::Clock {
            actor: "producer".into(),
        })
        .ok_or_else(|| super::invalid("clock step did not open an effect request"))?;
    world.history.push(unresolved.to_value());
    world.unresolved_effects.push(unresolved);
    Ok(world)
}

fn continuation() -> [RuntimeStep; 2] {
    [
        RuntimeStep::Clock {
            actor: "producer".into(),
        },
        RuntimeStep::Random {
            actor: "producer".into(),
            upper: RANDOM_UPPER,
        },
    ]
}

fn step_position(steps: &[SnapshotRestoreStep], step: SnapshotRestoreStep) -> Option<usize> {
    steps.iter().position(|observed| *observed == step)
}

// r[impl molten.world_snapshot.restore]
pub(super) fn current_admission() -> Result<super::super::Evidence> {
    let scratch = super::super::scratch("world-restore")?;
    let mut evidence = super::super::Evidence::default();
    let prepared = Prepared::new(scratch.path(), "restore-destination")?;
    let current = admission_document(GENERATION, CURRENT_REVOCATION, true);
    prepared.admit(&current, &current)?;

    let restored = prepared.restore()?;
    let steps = restored.restore.observations.iter().map(|observation| observation.step).collect::<Vec<_>>();
    let activation = step_position(&steps, SnapshotRestoreStep::ActivateRuntime);
    let handles = step_position(&steps, SnapshotRestoreStep::RecreateHostHandles);
    let recheck = step_position(&steps, SnapshotRestoreStep::RecheckCurrentAdmission);
    evidence.check("runtime-activated", activation.is_some());
    evidence.check("new-host-handles-before-activation", handles.zip(activation).is_some_and(|(h, a)| h < a));
    evidence
        .check("current-admission-rechecked-before-activation", recheck.zip(activation).is_some_and(|(r, a)| r < a));
    evidence.runtime(crate::preserves_rail::record(
        "world-restore-steps-v1",
        steps.iter().map(|step| crate::preserves_rail::string(step.as_str())).collect(),
    ));

    let mut readback = read_local_logical_runtime(&prepared.node.root)?;
    evidence.check("activated-image-reads-back", readback.canonical_image == prepared.image.canonical_bytes()?);
    evidence.check(
        "restored-owner-matches-committed-state",
        restored.runtime.snapshot() == prepared.image.snapshot
            && readback.runtime.snapshot() == prepared.image.snapshot,
    );
    evidence.check(
        "unresolved-request-retained-not-dispatched",
        !prepared.world.unresolved_effects.is_empty()
            && readback.unresolved_effects == prepared.world.unresolved_effects
            && restored.runtime.snapshot().effect_sequence == prepared.image.snapshot.effect_sequence,
    );
    let image = LogicalRuntimeImage::from_bytes(&readback.canonical_image)?;
    let restored_world = WorldState::from_image(image, RuntimeState::from_snapshot(readback.runtime.snapshot())?)?;
    let mut store = commit_store(&prepared.node.root)?;
    let (recaptured, _) = capture(&mut store, &restored_world, Vec::new())?;
    evidence.check("restored-state-reproduces-world-commit", recaptured.commit_ref == prepared.commit_ref);
    evidence.runtime(prepared.image.snapshot.to_value());
    evidence.runtime(crate::preserves_rail::sequence(
        readback.unresolved_effects.iter().map(RuntimeEvent::to_value).collect(),
    ));

    let mut uninterrupted = RuntimeState::from_snapshot(prepared.image.snapshot.clone())?;
    let mut continued = Vec::new();
    let mut is_deterministic = true;
    for step in continuation() {
        let resumed = readback.runtime.apply_step(&step);
        is_deterministic &= resumed == uninterrupted.apply_step(&step);
        continued.extend(resumed.iter().map(RuntimeEvent::to_value));
    }
    evidence.check(
        "restored-owner-continues-deterministically",
        is_deterministic && readback.runtime.snapshot() == uninterrupted.snapshot(),
    );
    evidence.runtime(crate::preserves_rail::sequence(continued));

    let repeated = prepared.restore()?;
    evidence.check(
        "same-activation-idempotent",
        repeated.restore.receipt.artifact_ref == restored.restore.receipt.artifact_ref,
    );
    Ok(evidence)
}

#[derive(Debug, Clone, Copy)]
enum Tamper {
    Revoked,
    StaleGeneration,
    CrossedDescriptor,
    UntrustedSigner,
}

impl Tamper {
    const ALL: [Self; 4] = [
        Self::Revoked,
        Self::StaleGeneration,
        Self::CrossedDescriptor,
        Self::UntrustedSigner,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Revoked => "revoked",
            Self::StaleGeneration => "stale-generation",
            Self::CrossedDescriptor => "crossed-descriptor",
            Self::UntrustedSigner => "untrusted-signer",
        }
    }

    const fn checks(self) -> (&'static str, &'static str) {
        match self {
            Self::Revoked => ("revoked-admission-denied", "revoked-admission-left-inactive"),
            Self::StaleGeneration => ("stale-generation-denied", "stale-generation-left-inactive"),
            Self::CrossedDescriptor => ("crossed-descriptor-denied", "crossed-descriptor-left-inactive"),
            Self::UntrustedSigner => ("untrusted-signer-denied", "untrusted-signer-left-inactive"),
        }
    }

    /// Publish the tampered (or, for a control, the current) admission.
    fn admit(self, prepared: &Prepared, tampered: bool) -> Result<()> {
        let current = admission_document(GENERATION, CURRENT_REVOCATION, true);
        if !tampered {
            return prepared.admit(&current, &current);
        }
        match self {
            Self::Revoked => {
                let revoked = admission_document(GENERATION, "aspen qualification restore revocation epoch 2", false);
                prepared.admit(&revoked, &revoked)
            }
            Self::StaleGeneration => {
                let advanced =
                    admission_document(GENERATION + 1, "aspen qualification restore revocation epoch 2", true);
                prepared.admit(&current, &advanced)
            }
            Self::CrossedDescriptor => {
                let crossed = reference("aspen qualification another snapshot descriptor");
                prepared.node.admit(SnapshotGrant {
                    signer: &prepared.node.signer,
                    descriptor: &prepared.descriptor,
                    descriptor_ref: &crossed,
                    granted: &current,
                    current: &current,
                })
            }
            Self::UntrustedSigner => prepared.node.admit(SnapshotGrant {
                signer: &fixture_key("untrusted restore authority"),
                descriptor: &prepared.descriptor,
                descriptor_ref: &prepared.descriptor_ref,
                granted: &current,
                current: &current,
            }),
        }
    }
}

/// Every stale, revoked, crossed or untrusted admission denies before
/// activation; `tampered = false` is the control that must activate instead.
// r[impl molten.world_snapshot.restore]
pub(super) fn stale_admission_denied(tampered: bool) -> Result<super::super::Evidence> {
    let scratch = super::super::scratch("world-restore-denied")?;
    let mut evidence = super::super::Evidence::default();
    for tamper in Tamper::ALL {
        let prepared = Prepared::new(&scratch.path().join(tamper.label()), tamper.label())?;
        tamper.admit(&prepared, tampered)?;
        let outcome = prepared.restore();
        let (denied, inactive) = tamper.checks();
        evidence.check(denied, outcome.is_err());
        let is_inactive =
            read_local_logical_runtime(&prepared.node.root).is_err() && !prepared.node.runtime_store_exists()?;
        evidence.check(inactive, is_inactive);
        let class = match &outcome {
            Ok(_) => crate::preserves_rail::string("activated"),
            Err(error) => super::super::error_class(error),
        };
        evidence.runtime(crate::preserves_rail::record("world-restore-denial-v1", vec![
            crate::preserves_rail::string(tamper.label()),
            class,
        ]));
    }
    Ok(evidence)
}
