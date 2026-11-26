//! The committed runtime-spine world the world producers capture, restore and
//! replay, its world-commit roots, and the node roots it lives in.

use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use molten_core::world_commit::ObservationStability;
use molten_core::world_commit::RevisionFence;
use molten_core::world_commit::RevisionRecheck;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::RootObservation;
use molten_core::world_commit::SnapshotProfile;
use molten_core::world_commit::SnapshotProfileKind;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldCommitVersion;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::SnapshotDescriptor;
use preserves::IOValue;

use crate::error::Failure;
use crate::error::Result;
use crate::node_state::DirectoryRoot;
use crate::node_state::NamespaceKind;
use crate::node_state::RelativePath;
use crate::runtime::RuntimeEvent;
use crate::runtime::RuntimeState;
use crate::runtime::RuntimeStep;
use crate::runtime::RuntimeValue;
use crate::world_commit::CanonicalCaptureReceipt;
use crate::world_commit::CanonicalWorldCommit;
use crate::world_commit::CaptureShellInput;
use crate::world_commit::LocalWorldCommitStore;
use crate::world_commit::ObservedRootMaterial;
use crate::world_commit::WorldCommitPortError;
use crate::world_commit::WorldRevisionRecheckPort;
use crate::world_commit::WorldRootObservationPort;
use crate::world_commit::capture_world_commit;
use crate::world_snapshot::LocalAdmissionInput;
use crate::world_snapshot::LocalCurrentAdmission;
use crate::world_snapshot::LocalLogicalSnapshot;
use crate::world_snapshot::LogicalRuntimeImage;
use crate::world_snapshot::canonical_snapshot_descriptor;
use crate::world_snapshot::local_logical_profile_ref;
use crate::world_snapshot::sign_local_admission;

/// The runtime seed of the qualification world (any nonzero xorshift state).
const RUNTIME_SEED: u64 = 0x5eed_a5be_0001;
const RANDOM_UPPER: u64 = 1_000;
const KEY_CONTEXT: &str = "aspen 2026-11 capability qualification world fixture key v1";
/// Declared runtime artifact and schema roots: the committed-turn handler set
/// and snapshot ABI, not the executing binary (bound by the qualification subject).
const RUNTIME_ARTIFACT: &str = "molten-runtime-state/committed-turn";
const RUNTIME_SCHEMA: &str = "runtime-snapshot-v1/runtime-spine-image-v1";
const ROOT_RECORD: &str = "aspen-qualification-runtime-world-root-v1";
const RUNTIME_STORE: &str = "ledger/world-snapshot/runtime.redb";
const ADMISSION_EXPIRY: u64 = u64::MAX;

pub(super) fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

/// Fixed fixture signing key derived from a label; identical on every run.
pub(super) fn fixture_key(label: &str) -> SigningKey {
    SigningKey::from_bytes(&blake3::derive_key(KEY_CONTEXT, label.as_bytes()))
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
pub(super) struct WorldState {
    pub runtime: RuntimeState,
    pub history: Vec<IOValue>,
    pub unresolved_effects: Vec<RuntimeEvent>,
    pub policy_ref: String,
}

impl WorldState {
    /// The committed initial world every scenario starts from.
    pub fn committed() -> Result<Self> {
        let mut world = Self {
            runtime: RuntimeState::new(RUNTIME_SEED),
            history: Vec::new(),
            unresolved_effects: Vec::new(),
            policy_ref: policy_ref(),
        };
        let ready = RuntimeValue::string("ready")?;
        for step in [
            RuntimeStep::Assert {
                actor: "producer".into(),
                value: ready.clone(),
            },
            RuntimeStep::Observe {
                actor: "consumer".into(),
                pattern: ready,
            },
            RuntimeStep::Clock {
                actor: "producer".into(),
            },
            RuntimeStep::Random {
                actor: "producer".into(),
                upper: RANDOM_UPPER,
            },
        ] {
            world.apply(&step);
        }
        Ok(world)
    }

    pub fn from_image(image: LogicalRuntimeImage, runtime: RuntimeState) -> Result<Self> {
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
    pub fn apply(&mut self, step: &RuntimeStep) -> Vec<RuntimeEvent> {
        let events = self.runtime.apply_step(step);
        self.history.extend(events.iter().map(RuntimeEvent::to_value));
        events
    }

    pub fn image(&self) -> LogicalRuntimeImage {
        LogicalRuntimeImage {
            snapshot: self.runtime.snapshot(),
            history: self.history.clone(),
            unresolved_effects: self.unresolved_effects.clone(),
            policy_ref: self.policy_ref.clone(),
        }
    }

    fn root_materials(&self) -> Result<BTreeMap<RootKind, Vec<u8>>> {
        let snapshot = self.runtime.snapshot();
        let mut materials = BTreeMap::new();
        for kind in SnapshotProfileKind::Logical.required_roots() {
            let value = match kind {
                RootKind::Artifact => crate::preserves_rail::string(RUNTIME_ARTIFACT),
                RootKind::Schema => crate::preserves_rail::string(RUNTIME_SCHEMA),
                RootKind::DurableState => snapshot.to_value(),
                RootKind::Tasks => crate::preserves_rail::sequence(
                    snapshot.facets.values().map(crate::runtime::RuntimeFacet::to_value).collect(),
                ),
                RootKind::History => crate::preserves_rail::sequence(self.history.clone()),
                RootKind::Effects => crate::preserves_rail::record("effects", vec![
                    crate::preserves_rail::u64_value(snapshot.effect_sequence),
                    crate::preserves_rail::sequence(
                        self.unresolved_effects.iter().map(RuntimeEvent::to_value).collect(),
                    ),
                ]),
                RootKind::Scheduler => crate::preserves_rail::sequence(
                    snapshot.messages.iter().map(crate::runtime::RuntimeMessage::to_value).collect(),
                ),
                RootKind::Time => crate::preserves_rail::u64_value(snapshot.logical_time),
                RootKind::Entropy => crate::preserves_rail::u64_value(snapshot.rng_state),
                RootKind::RuntimeProfile => crate::preserves_rail::string(local_logical_profile_ref()?),
                RootKind::Policy => crate::preserves_rail::string(&self.policy_ref),
                _ => return Err(super::invalid("logical runtime world has no such root")),
            };
            let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(ROOT_RECORD, vec![
                crate::preserves_rail::string(kind.as_str()),
                value,
            ]))?;
            materials.insert(*kind, bytes);
        }
        Ok(materials)
    }
}

/// The replay inputs: committed turns that reach only the simulated virtual
/// clock, seeded entropy stream and deterministic scheduler.
pub(super) fn replay_inputs() -> Result<Vec<RuntimeStep>> {
    Ok(vec![
        RuntimeStep::Clock {
            actor: "consumer".into(),
        },
        RuntimeStep::Assert {
            actor: "consumer".into(),
            value: RuntimeValue::string("acknowledged")?,
        },
        RuntimeStep::Random {
            actor: "consumer".into(),
            upper: RANDOM_UPPER,
        },
    ])
}

pub(super) fn logical_profile() -> Result<SnapshotProfile> {
    Ok(SnapshotProfile {
        kind: SnapshotProfileKind::Logical,
        profile_ref: SnapshotProfileRef::new(local_logical_profile_ref()?)
            .map_err(|error| super::invalid(format!("logical runtime profile ref: {error:?}")))?,
        cohort_ref: None,
    })
}

/// Capture one world commit through the production capture shell into the
/// durable local world-commit store.
pub(super) fn capture(
    store: &mut LocalWorldCommitStore,
    world: &WorldState,
    parents: Vec<WorldCommitRef>,
) -> Result<(CanonicalWorldCommit, CanonicalCaptureReceipt)> {
    let input = CaptureShellInput {
        version: WorldCommitVersion::V1,
        profile: logical_profile()?,
        parents,
        root_kinds: SnapshotProfileKind::Logical.required_roots().to_vec(),
        bounds: WorldCommitBounds::default(),
    };
    let mut roots = RuntimeRoots {
        materials: world.root_materials()?,
    };
    let execution = capture_world_commit(&input, &mut roots, store, &mut ImmutableRoots)?;
    store.write_capture_receipt(&execution.receipt)?;
    let commit = execution
        .commit
        .ok_or_else(|| super::invalid("runtime world capture was denied by the capture shell"))?;
    Ok((commit, execution.receipt))
}

pub(super) fn commit_store(root: &DirectoryRoot) -> Result<LocalWorldCommitStore> {
    LocalWorldCommitStore::open(&root.namespace(NamespaceKind::Storage)?)
}

/// Root observations of one committed runtime state. A committed turn image is
/// immutable once captured, so no revision fence applies.
struct RuntimeRoots {
    materials: BTreeMap<RootKind, Vec<u8>>,
}

impl WorldRootObservationPort for RuntimeRoots {
    fn observe_root(&mut self, kind: RootKind) -> std::result::Result<ObservedRootMaterial, WorldCommitPortError> {
        let bytes = self
            .materials
            .get(&kind)
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("missing-runtime-root", kind.as_str()))?;
        let schema_validated = crate::preserves_rail::strict_canonical_decode(&bytes).is_ok();
        let root = WorldRootRef::parse(kind, crate::preserves_rail::content_ref_from_bytes(&bytes))
            .map_err(|error| WorldCommitPortError::new("runtime-root-ref", format!("{error:?}")))?;
        Ok(ObservedRootMaterial {
            observation: RootObservation {
                root,
                source_kind: kind,
                schema_validated,
                stability: ObservationStability::Immutable,
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
        fence: &RevisionFence,
    ) -> std::result::Result<RevisionRecheck, WorldCommitPortError> {
        Err(WorldCommitPortError::new("unexpected-revision-fence", fence.source_id.clone()))
    }
}

/// One node state root holding the runtime-spine destination layout.
pub(super) struct NodeRoot {
    pub root: DirectoryRoot,
    pub signer: SigningKey,
}

impl NodeRoot {
    pub fn open(path: &std::path::Path, label: &str) -> Result<Self> {
        let root = DirectoryRoot::open(path)?;
        root.create_layout()?;
        for name in ["config.preserves", "identity.preserves"] {
            let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
                "aspen-qualification-node-v1",
                vec![
                    crate::preserves_rail::string(label),
                    crate::preserves_rail::string(name),
                ],
            ))?;
            root.write(&RelativePath::parse(name)?, &bytes)?;
        }
        Ok(Self {
            root,
            signer: fixture_key("runtime-spine restore authority"),
        })
    }

    pub fn adapter(&self) -> Result<LocalLogicalSnapshot> {
        LocalLogicalSnapshot::from_root(self.root.clone(), self.signer.verifying_key())
    }

    /// Publish the independently provisioned current admission and its signed
    /// grant. `granted` is what the grant signs; `current` is the live document.
    pub fn admit(&self, grant: SnapshotGrant<'_>) -> Result<()> {
        let bytes = sign_local_admission(LocalAdmissionInput {
            signer: grant.signer,
            descriptor_ref: grant.descriptor_ref,
            profile_ref: grant.descriptor.profile_ref.as_str(),
            cohort_ref: grant.descriptor.cohort.cohort_ref.as_str(),
            destination_ref: self.adapter()?.destination_ref(),
            current: grant.granted.clone(),
        })?;
        let current = serde_json::to_vec(grant.current)
            .map_err(|error| super::invalid(format!("current admission encoding: {error}")))?;
        self.root.write(&RelativePath::parse("storage/world-snapshot/current.json")?, &current)?;
        self.root.write(&RelativePath::parse("storage/world-snapshot/admission.json")?, &bytes)
    }

    pub fn runtime_store_exists(&self) -> Result<bool> {
        self.root.try_exists(&RelativePath::parse(RUNTIME_STORE)?)
    }
}

pub(super) struct SnapshotGrant<'a> {
    pub signer: &'a SigningKey,
    pub descriptor: &'a SnapshotDescriptor,
    pub descriptor_ref: &'a str,
    pub granted: &'a LocalCurrentAdmission,
    pub current: &'a LocalCurrentAdmission,
}

pub(super) fn descriptor_ref(descriptor: &SnapshotDescriptor) -> Result<String> {
    Ok(canonical_snapshot_descriptor(descriptor)?.artifact_ref)
}

/// The current runtime-spine admission at one generation and revocation epoch.
pub(super) fn admission_document(generation: u64, revocation_label: &str, allowed: bool) -> LocalCurrentAdmission {
    LocalCurrentAdmission {
        policy_ref: policy_ref(),
        capability_ref: reference("aspen qualification runtime-spine restore capability"),
        revocation_ref: reference(revocation_label),
        resource_ref: reference("aspen qualification bounded runtime resources"),
        adapter_ref: reference("aspen qualification runtime-spine redb adapter"),
        generation,
        allowed,
        expires_unix_seconds: ADMISSION_EXPIRY,
    }
}

pub(super) fn store_error(error: WorldCommitPortError) -> Failure {
    super::invalid(format!("world-commit store: {error}"))
}
