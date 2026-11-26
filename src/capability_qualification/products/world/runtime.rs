//! The committed runtime-spine world the world producers capture, restore and
//! replay, and its world-commit roots.

use crate::world_commit::WorldRevisionRecheckPort;
use crate::world_commit::WorldRootObservationPort;

/// The runtime seed of the qualification world (any nonzero xorshift state).
const RUNTIME_SEED: u64 = 0x5eed_a5be_0001;
const RANDOM_UPPER: u64 = 1_000;
const KEY_CONTEXT: &str = "aspen 2026-11 capability qualification world fixture key v1";
/// Declared runtime artifact and schema roots: the committed-turn handler set
/// and snapshot ABI, not the executing binary (bound by the qualification subject).
const RUNTIME_ARTIFACT: &str = "molten-runtime-state/committed-turn";
const RUNTIME_SCHEMA: &str = "runtime-snapshot-v1/runtime-spine-image-v1";
const ROOT_RECORD: &str = "aspen-qualification-runtime-world-root-v1";

pub(super) fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

/// Fixed fixture signing key derived from a label; identical on every run.
pub(super) fn fixture_key(label: &str) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&blake3::derive_key(KEY_CONTEXT, label.as_bytes()))
}

pub(super) fn key_ref(key: &ed25519_dalek::VerifyingKey) -> String {
    crate::preserves_rail::content_ref_from_bytes(key.as_bytes())
}

pub(super) fn policy_ref() -> String {
    reference("aspen qualification runtime world policy")
}

/// A committed runtime world: owner state, committed turn history, unresolved
/// effect requests and the policy it was committed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommittedState {
    pub runtime: crate::runtime::RuntimeState,
    pub history: Vec<preserves::IOValue>,
    pub unresolved_effects: Vec<crate::runtime::RuntimeEvent>,
    pub policy_ref: String,
}

impl CommittedState {
    /// The committed initial world every scenario starts from.
    pub fn committed() -> crate::error::Result<Self> {
        let mut world = Self {
            runtime: crate::runtime::RuntimeState::new(RUNTIME_SEED),
            history: Vec::new(),
            unresolved_effects: Vec::new(),
            policy_ref: policy_ref(),
        };
        let ready = crate::runtime::RuntimeValue::string("ready")?;
        for step in [
            crate::runtime::RuntimeStep::Assert {
                actor: "producer".into(),
                value: ready.clone(),
            },
            crate::runtime::RuntimeStep::Observe {
                actor: "consumer".into(),
                pattern: ready,
            },
            crate::runtime::RuntimeStep::Clock {
                actor: "producer".into(),
            },
            crate::runtime::RuntimeStep::Random {
                actor: "producer".into(),
                upper: RANDOM_UPPER,
            },
        ] {
            world.apply(&step);
        }
        Ok(world)
    }

    pub fn from_image(
        image: crate::world_snapshot::LogicalRuntimeImage,
        runtime: crate::runtime::RuntimeState,
    ) -> crate::error::Result<Self> {
        if runtime.snapshot() != image.snapshot {
            return Err(super::invalid("restored runtime owner differs from its activated image"));
        }
        Ok(Self {
            runtime,
            history: image.history,
            unresolved_effects: image.unresolved_effects,
            policy_ref: image.policy_ref,
        })
    }

    /// Apply one committed turn and append its events to the turn history.
    pub fn apply(&mut self, step: &crate::runtime::RuntimeStep) -> Vec<crate::runtime::RuntimeEvent> {
        let events = self.runtime.apply_step(step);
        self.history.extend(events.iter().map(crate::runtime::RuntimeEvent::to_value));
        events
    }

    pub fn image(&self) -> crate::world_snapshot::LogicalRuntimeImage {
        crate::world_snapshot::LogicalRuntimeImage {
            snapshot: self.runtime.snapshot(),
            history: self.history.clone(),
            unresolved_effects: self.unresolved_effects.clone(),
            policy_ref: self.policy_ref.clone(),
        }
    }

    fn root_materials(
        &self,
    ) -> crate::error::Result<std::collections::BTreeMap<molten_core::world_commit::RootKind, Vec<u8>>> {
        let snapshot = self.runtime.snapshot();
        molten_core::world_commit::SnapshotProfileKind::Logical
            .required_roots()
            .iter()
            .map(|kind| -> crate::error::Result<(molten_core::world_commit::RootKind, Vec<u8>)> {
                let value = match kind {
                    molten_core::world_commit::RootKind::Artifact => crate::preserves_rail::string(RUNTIME_ARTIFACT),
                    molten_core::world_commit::RootKind::Schema => crate::preserves_rail::string(RUNTIME_SCHEMA),
                    molten_core::world_commit::RootKind::DurableState => snapshot.to_value(),
                    molten_core::world_commit::RootKind::Tasks => crate::preserves_rail::sequence(
                        snapshot.facets.values().map(crate::runtime::RuntimeFacet::to_value).collect(),
                    ),
                    molten_core::world_commit::RootKind::History => {
                        crate::preserves_rail::sequence(self.history.clone())
                    }
                    molten_core::world_commit::RootKind::Effects => crate::preserves_rail::record("effects", vec![
                        crate::preserves_rail::u64_value(snapshot.effect_sequence),
                        crate::preserves_rail::sequence(
                            self.unresolved_effects.iter().map(crate::runtime::RuntimeEvent::to_value).collect(),
                        ),
                    ]),
                    molten_core::world_commit::RootKind::Scheduler => crate::preserves_rail::sequence(
                        snapshot.messages.iter().map(crate::runtime::RuntimeMessage::to_value).collect(),
                    ),
                    molten_core::world_commit::RootKind::Time => {
                        crate::preserves_rail::u64_value(snapshot.logical_time)
                    }
                    molten_core::world_commit::RootKind::Entropy => {
                        crate::preserves_rail::u64_value(snapshot.rng_state)
                    }
                    molten_core::world_commit::RootKind::RuntimeProfile => {
                        crate::preserves_rail::string(crate::world_snapshot::local_logical_profile_ref()?)
                    }
                    molten_core::world_commit::RootKind::Policy => crate::preserves_rail::string(&self.policy_ref),
                    _ => return Err(super::invalid("logical runtime world has no such root")),
                };
                let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(ROOT_RECORD, vec![
                    crate::preserves_rail::string(kind.as_str()),
                    value,
                ]))?;
                Ok((*kind, bytes))
            })
            .collect()
    }
}

/// The replay inputs: committed turns that reach only the simulated virtual
/// clock, seeded entropy stream and deterministic scheduler.
pub(super) fn replay_inputs() -> crate::error::Result<Vec<crate::runtime::RuntimeStep>> {
    Ok(vec![
        crate::runtime::RuntimeStep::Clock {
            actor: "consumer".into(),
        },
        crate::runtime::RuntimeStep::Assert {
            actor: "consumer".into(),
            value: crate::runtime::RuntimeValue::string("acknowledged")?,
        },
        crate::runtime::RuntimeStep::Random {
            actor: "consumer".into(),
            upper: RANDOM_UPPER,
        },
    ])
}

pub(super) fn logical_profile() -> crate::error::Result<molten_core::world_commit::SnapshotProfile> {
    Ok(molten_core::world_commit::SnapshotProfile {
        kind: molten_core::world_commit::SnapshotProfileKind::Logical,
        profile_ref: molten_core::world_commit::SnapshotProfileRef::new(
            crate::world_snapshot::local_logical_profile_ref()?,
        )
        .map_err(|error| super::invalid(format!("logical runtime profile ref: {error:?}")))?,
        cohort_ref: None,
    })
}

/// Capture one world commit through the production capture shell into the
/// durable local world-commit store.
pub(super) fn capture(
    store: &mut crate::world_commit::LocalWorldCommitStore,
    world: &CommittedState,
    parents: Vec<molten_core::world_commit::WorldCommitRef>,
) -> crate::error::Result<(crate::world_commit::CanonicalWorldCommit, crate::world_commit::CanonicalCaptureReceipt)> {
    let input = crate::world_commit::CaptureShellInput {
        version: molten_core::world_commit::WorldCommitVersion::V1,
        profile: logical_profile()?,
        parents,
        root_kinds: molten_core::world_commit::SnapshotProfileKind::Logical.required_roots().to_vec(),
        bounds: commit_bounds(),
    };
    let mut roots = CommittedRoots {
        materials: world.root_materials()?,
    };
    let execution = crate::world_commit::capture_world_commit(&input, &mut roots, store, &mut ImmutableRoots)?;
    store.write_capture_receipt(&execution.receipt)?;
    let commit = execution
        .commit
        .ok_or_else(|| super::invalid("runtime world capture was denied by the capture shell"))?;
    Ok((commit, execution.receipt))
}

pub(super) fn commit_store(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<crate::world_commit::LocalWorldCommitStore> {
    crate::world_commit::LocalWorldCommitStore::open(&root.namespace(crate::node_state::NamespaceKind::Storage)?)
}

/// The world-commit bounds the producers capture and parse under: the
/// molten-core defaults, spelled out.
pub(super) fn commit_bounds() -> molten_core::world_commit::WorldCommitBounds {
    molten_core::world_commit::WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: molten_core::world_commit::MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

/// Root observations of one committed runtime state. A committed turn image is
/// immutable once captured, so no revision fence applies.
struct CommittedRoots {
    materials: std::collections::BTreeMap<molten_core::world_commit::RootKind, Vec<u8>>,
}

impl WorldRootObservationPort for CommittedRoots {
    fn observe_root(
        &mut self,
        kind: molten_core::world_commit::RootKind,
    ) -> std::result::Result<crate::world_commit::ObservedRootMaterial, crate::world_commit::WorldCommitPortError> {
        let bytes = self
            .materials
            .get(&kind)
            .cloned()
            .ok_or_else(|| crate::world_commit::WorldCommitPortError::new("missing-runtime-root", kind.as_str()))?;
        let is_schema_validated = crate::preserves_rail::strict_canonical_decode(&bytes).is_ok();
        let root =
            molten_core::world_commit::WorldRootRef::parse(kind, crate::preserves_rail::content_ref_from_bytes(&bytes))
                .map_err(|error| {
                    crate::world_commit::WorldCommitPortError::new("runtime-root-ref", format!("{error:?}"))
                })?;
        Ok(crate::world_commit::ObservedRootMaterial {
            observation: molten_core::world_commit::RootObservation {
                root,
                source_kind: kind,
                schema_validated: is_schema_validated,
                stability: molten_core::world_commit::ObservationStability::Immutable,
                durable: false,
                inventory_complete: true,
            },
            canonical_bytes: bytes,
        })
    }
}

struct ImmutableRoots;

impl WorldRevisionRecheckPort for ImmutableRoots {
    fn recheck_revision(
        &mut self,
        fence: &molten_core::world_commit::RevisionFence,
    ) -> std::result::Result<molten_core::world_commit::RevisionRecheck, crate::world_commit::WorldCommitPortError>
    {
        Err(crate::world_commit::WorldCommitPortError::new("unexpected-revision-fence", fence.source_id.clone()))
    }
}

pub(super) fn store_error(error: crate::world_commit::WorldCommitPortError) -> crate::error::Failure {
    super::invalid(format!("world-commit store: {error}"))
}
