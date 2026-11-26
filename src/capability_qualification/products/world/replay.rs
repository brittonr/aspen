//! World replay through the production replay service. A recorded runtime
//! world (source node) is captured commit by commit; replay restores its initial
//! commit into a destination node through the runtime-spine snapshot restore,
//! re-executes the same committed turns, captures each successor through the
//! world-commit capture shell, and lets `run_world_replay` compare and receipt.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use molten_core::fabric_time::SchedulerReplayPolicy;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_replay::REPLAY_OBSERVATION_LEDGER_SCHEMA;
use molten_core::world_replay::ReplayObservationLedger;
use molten_core::world_replay::ReplaySourceInventory;
use molten_core::world_replay::WORLD_REPLAY_CAPSULE_SCHEMA;
use molten_core::world_replay::WORLD_TRANSITION_TRACE_SCHEMA;
use molten_core::world_replay::WorldReplayBounds;
use molten_core::world_replay::WorldReplayCapsule;
use molten_core::world_replay::WorldReplayCapsuleMember;
use molten_core::world_replay::WorldReplayCapsuleMemberRole;
use molten_core::world_replay::WorldReplayCommitClosure;
use molten_core::world_replay::WorldReplayMemberCodec;
use molten_core::world_replay::WorldReplayMemberProtection;
use molten_core::world_replay::WorldReplayObservationBoundary;
use molten_core::world_replay::WorldReplayObservedCommit;
use molten_core::world_replay::WorldReplayPlanRequest;
use molten_core::world_replay::WorldReplayProfile;
use molten_core::world_replay::WorldReplayProfileKind;
use molten_core::world_replay::WorldReplayReceiptDecision;
use molten_core::world_replay::WorldReplayTransitionObservation;
use molten_core::world_replay::WorldTransitionInput;
use molten_core::world_replay::WorldTransitionInputKind;
use molten_core::world_replay::WorldTransitionStep;
use molten_core::world_replay::WorldTransitionTrace;
use molten_core::world_replay::identify_replay_observation_ledger;
use molten_core::world_replay::identify_replay_source_inventory;
use molten_core::world_replay::identify_world_replay_capsule;
use molten_core::world_replay::identify_world_transition_trace;
use molten_core::world_replay::required_world_replay_boundary_closure;
use molten_core::world_replay::required_world_replay_closure;
use molten_core::world_replay::world_replay_non_claims;
use molten_core::world_snapshot::SnapshotDescriptor;
use preserves::IOValue;

use super::runtime::NodeRoot;
use super::runtime::SnapshotGrant;
use super::runtime::WorldState;
use super::runtime::admission_document;
use super::runtime::capture;
use super::runtime::commit_store;
use super::runtime::descriptor_ref;
use super::runtime::fixture_key;
use super::runtime::key_ref;
use super::runtime::logical_profile;
use super::runtime::reference;
use super::runtime::replay_inputs;
use super::runtime::store_error;
use crate::capability_tokens::CapabilityProofset;
use crate::capability_tokens::CapabilityRequest;
use crate::capability_tokens::CapabilityToken;
use crate::capability_tokens::admit_capability;
use crate::capability_tokens::capability_token_value;
use crate::error::Result;
use crate::node_state::DirectoryRoot;
use crate::node_state::RelativePath;
use crate::runtime::RuntimeEvent;
use crate::runtime::RuntimeStep;
use crate::world_commit::CanonicalWorldCommit;
use crate::world_commit::LocalWorldCommitStore;
use crate::world_commit::WorldCommitPublicationPort;
use crate::world_commit::WorldImmutableObjectPort;
use crate::world_commit::parse_canonical_world_commit_with_ref;
use crate::world_replay::CLOCK_VIRTUAL_TIME_SOURCE;
use crate::world_replay::CanonicalWorldReplayRecord;
use crate::world_replay::ENTROPY_SEEDED_STREAM_SOURCE;
use crate::world_replay::RuntimeInventoryInput;
use crate::world_replay::SCHEDULER_DECISION_SOURCE;
use crate::world_replay::SealedReplayObservations;
use crate::world_replay::WORLD_REPLAY_RECEIPT_RECORD;
use crate::world_replay::WorldReplayAdmissionObservation;
use crate::world_replay::WorldReplayAdmissionPort;
use crate::world_replay::WorldReplayCaptureObservation;
use crate::world_replay::WorldReplayCapturePort;
use crate::world_replay::WorldReplayExecutionObservation;
use crate::world_replay::WorldReplayMaterializationObservation;
use crate::world_replay::WorldReplayMaterializationPort;
use crate::world_replay::WorldReplayPorts;
use crate::world_replay::WorldReplayReceiptPort;
use crate::world_replay::WorldReplayRestoreObservation;
use crate::world_replay::WorldReplayRestorePort;
use crate::world_replay::WorldReplayRunOutcome;
use crate::world_replay::WorldReplayTransitionPort;
use crate::world_replay::canonical_world_transition_trace;
use crate::world_replay::run_world_replay;
use crate::world_replay::runtime_replay_inventory;
use crate::world_snapshot::LogicalRuntimeImage;
use crate::world_snapshot::local_logical_profile_ref;
use crate::world_snapshot::read_local_logical_runtime;
use crate::world_snapshot::restore_local_logical_snapshot;

const AUTHORITY_GENERATION: u64 = 1;
const SNAPSHOT_REVOCATION: &str = "aspen qualification replay snapshot revocation epoch 1";
const AUTHORITY_PATH: &str = "storage/world-replay/current-authority.preserves";
const RECEIPT_DIRECTORY: &str = "storage/world-replay/receipts";
const AUTHORITY_RECORD: &str = "aspen-qualification-replay-authority-v1";
const TOKEN_KIND: &str = "world-replay-execution";
const REPLAY_ABILITY: &str = "world/replay/execute";
const REPLAY_SCOPE: &str = "runtime-spine-committed-turn-v1";
const MAX_REPLAY_TURNS: usize = 16;
const MAX_AUTHORITY_BYTES: u64 = 65_536;
const STEP_SCHEMA_LABEL: &str = "molten.runtime-step.v1";

/// Everything one replay run observed.
struct ReplayRun {
    request: WorldReplayPlanRequest,
    outcome: WorldReplayRunOutcome,
    materialized: usize,
    restores: usize,
    executed: Vec<u64>,
    captured: usize,
    receipt_kinds: Vec<&'static str>,
    authority_receipts: Vec<IOValue>,
}

// r[impl molten.world_replay.execution_boundary]
pub(super) fn admitted() -> Result<super::super::Evidence> {
    let run = replay(false)?;
    let mut evidence = super::super::Evidence::default();
    let outcome = &run.outcome;
    let steps = run.request.trace.steps.iter().map(|step| step.position).collect::<Vec<_>>();
    evidence.check("capsule-members-materialized-and-verified", run.materialized == run.request.capsule.members.len());
    evidence.check("runtime-spine-snapshot-restored-once", run.restores == 1);
    evidence.check("current-admission-rerun-and-admitted", outcome.admission.admitted());
    evidence.check("every-transition-executed-in-order", run.executed == steps);
    evidence.check("every-successor-captured", run.captured == steps.len() && outcome.captures.len() == steps.len());
    evidence.check("replay-receipt-replayed", outcome.receipt.decision == WorldReplayReceiptDecision::Replayed);
    evidence.check(
        "successors-match-recorded-commits",
        outcome.receipt.horizon == steps.len() && outcome.divergence_record.is_none(),
    );
    evidence.check(
        "receipt-binds-current-admission",
        outcome.receipt.current_admission_ref.as_deref() == Some(outcome.admission.observation_ref.as_str()),
    );
    evidence.check("replay-receipt-published-last", run.receipt_kinds.last() == Some(&WORLD_REPLAY_RECEIPT_RECORD));
    evidence.runtime(outcome.receipt_record.value.clone());
    evidence.runtime(outcome.trace_record.value.clone());
    for receipt in run.authority_receipts {
        evidence.runtime(receipt);
    }
    Ok(evidence)
}

/// A complete, valid capsule replays nothing when current authority is revoked.
/// `revoke = false` is the control that must replay instead.
// r[impl molten.world_replay.execution_boundary]
pub(super) fn authority_denied(revoke: bool) -> Result<super::super::Evidence> {
    let run = replay(revoke)?;
    let mut evidence = super::super::Evidence::default();
    let outcome = &run.outcome;
    let admission = &outcome.admission;
    evidence.check("capsule-complete-and-materialized", run.materialized == run.request.capsule.members.len());
    evidence.check("runtime-spine-snapshot-restored-once", run.restores == 1);
    evidence.check("current-authority-denied", !admission.authority_admitted);
    evidence.check(
        "other-current-admissions-held",
        admission.artifact_admitted
            && admission.schema_admitted
            && admission.resource_admitted
            && admission.runtime_admitted
            && admission.effect_admitted,
    );
    evidence.check("replay-receipt-denied", outcome.receipt.decision == WorldReplayReceiptDecision::Denied);
    evidence.check(
        "denied-before-any-transition",
        run.executed.is_empty() && outcome.executions.is_empty() && outcome.receipt.horizon == 0,
    );
    evidence.check("no-successor-captured", run.captured == 0 && outcome.captures.is_empty());
    evidence.check(
        "denial-receipt-published-last",
        run.receipt_kinds.last() == Some(&WORLD_REPLAY_RECEIPT_RECORD)
            && outcome.receipt.diagnostics == ["current replay admission denied"],
    );
    evidence.runtime(outcome.receipt_record.value.clone());
    for receipt in run.authority_receipts {
        evidence.runtime(receipt);
    }
    Ok(evidence)
}

/// The recorded world: its commits in the source node store and its trace.
struct Recording {
    store: LocalWorldCommitStore,
    initial: WorldState,
    commits: Vec<CanonicalWorldCommit>,
    trace: WorldTransitionTrace,
    inputs: BTreeMap<String, RuntimeStep>,
}

fn step_schema_ref() -> String {
    reference(STEP_SCHEMA_LABEL)
}

fn record(source: &DirectoryRoot) -> Result<Recording> {
    let mut store = commit_store(source)?;
    let initial = WorldState::committed()?;
    let mut world = initial.clone();
    let (initial_commit, _) = capture(&mut store, &world, Vec::new())?;
    let profile = logical_profile()?;
    let replay_profile = WorldReplayProfile {
        kind: WorldReplayProfileKind::Logical,
        profile_ref: profile.profile_ref.clone(),
        cohort_ref: None,
        snapshot_descriptor_ref: None,
    };
    let mut commits = vec![initial_commit];
    let mut steps = Vec::new();
    let mut inputs = BTreeMap::new();
    for (position, input) in (0_u64..).zip(replay_inputs()?) {
        let parent = commits
            .last()
            .map(|commit| commit.commit_ref.clone())
            .ok_or_else(|| super::invalid("recording lost its parent commit"))?;
        world.apply(&input);
        let (successor, _) = capture(&mut store, &world, vec![parent.clone()])?;
        let input_bytes = crate::preserves_rail::canonical_bytes(&input.to_value())?;
        let input_ref = input.step_ref()?;
        steps.push(WorldTransitionStep {
            position,
            expected_parent: parent,
            input: WorldTransitionInput {
                kind: WorldTransitionInputKind::Command,
                input_ref: input_ref.clone(),
                schema_ref: step_schema_ref(),
                byte_length: u64::try_from(input_bytes.len())
                    .map_err(|_| super::invalid("replay input length exceeds u64"))?,
            },
            profile_ref: profile.profile_ref.clone(),
            expected_successor: successor.commit_ref.clone(),
        });
        inputs.insert(input_ref, input);
        commits.push(successor);
    }
    let mut trace = WorldTransitionTrace {
        schema: WORLD_TRANSITION_TRACE_SCHEMA.to_string(),
        trace_ref: String::new(),
        initial_commit: commits[0].commit_ref.clone(),
        profile: replay_profile,
        steps,
    };
    trace.trace_ref = identify_world_transition_trace(&trace)
        .map_err(|issue| super::invalid(format!("world transition trace identity: {issue:?}")))?;
    Ok(Recording {
        store,
        initial,
        commits,
        trace,
        inputs,
    })
}

/// The runtime catalog boundary: the recorded turns reach only the simulated
/// virtual clock and entropy stream and the deterministic scheduler, so no
/// recorded observation slot exists.
fn boundary(trace: &WorldTransitionTrace) -> Result<WorldReplayObservationBoundary> {
    let adapter_cohort_ref = reference("aspen qualification runtime-spine replay adapter cohort");
    let protection_profile_ref = reference("aspen qualification runtime-spine replay protection profile");
    let inventory = runtime_replay_inventory(&RuntimeInventoryInput {
        profile_ref: trace.profile.profile_ref.as_str(),
        adapter_cohort_ref: &adapter_cohort_ref,
        protection_profile_ref: &protection_profile_ref,
        opaque_diagnostic_cohort_ref: None,
        scheduler_policy: SchedulerReplayPolicy::Deterministic,
        sensitive_source_ids: &[],
    })?;
    let mut reached_sources = [
        CLOCK_VIRTUAL_TIME_SOURCE,
        ENTROPY_SEEDED_STREAM_SOURCE,
        SCHEDULER_DECISION_SOURCE,
    ]
    .map(ToString::to_string)
    .to_vec();
    reached_sources.sort();
    let mut ledger = ReplayObservationLedger {
        schema: REPLAY_OBSERVATION_LEDGER_SCHEMA.to_string(),
        ledger_ref: String::new(),
        inventory_ref: inventory.inventory_ref.clone(),
        trace_ref: trace.trace_ref.clone(),
        reached_sources,
        slots: Vec::new(),
        observations: Vec::new(),
    };
    ledger.ledger_ref = identify_replay_observation_ledger(&ledger)
        .map_err(|issue| super::invalid(format!("replay observation ledger identity: {issue:?}")))?;
    Ok(WorldReplayObservationBoundary {
        inventory,
        ledger,
        opaque_diagnostics: Vec::new(),
    })
}

fn inventory_bytes(inventory: &ReplaySourceInventory) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("aspen-replay-source-inventory-v1", vec![
        crate::preserves_rail::string(&inventory.schema),
        crate::preserves_rail::string(&inventory.inventory_ref),
        crate::preserves_rail::string(&inventory.profile_ref),
        crate::preserves_rail::string(&inventory.adapter_cohort_ref),
        crate::preserves_rail::string(&inventory.protection_profile_ref),
        crate::preserves_rail::sequence(
            inventory.opaque_diagnostic_cohort_ref.iter().map(crate::preserves_rail::string).collect(),
        ),
        crate::preserves_rail::sequence(
            inventory
                .sources
                .iter()
                .map(|source| {
                    crate::preserves_rail::record("source", vec![
                        crate::preserves_rail::string(&source.source_id),
                        crate::preserves_rail::string(source.kind.as_str()),
                        crate::preserves_rail::string(source.handling.as_str()),
                        crate::preserves_rail::bool_value(source.is_external_mutation),
                        crate::preserves_rail::bool_value(source.is_sensitive),
                    ])
                })
                .collect(),
        ),
    ]))
}

fn ledger_bytes(ledger: &ReplayObservationLedger) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("aspen-replay-observation-ledger-v1", vec![
        crate::preserves_rail::string(&ledger.schema),
        crate::preserves_rail::string(&ledger.ledger_ref),
        crate::preserves_rail::string(&ledger.inventory_ref),
        crate::preserves_rail::string(&ledger.trace_ref),
        crate::preserves_rail::sequence(ledger.reached_sources.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::sequence(
            ledger
                .slots
                .iter()
                .map(|slot| {
                    crate::preserves_rail::record("slot", vec![
                        crate::preserves_rail::u64_value(slot.step_position),
                        crate::preserves_rail::u64_value(slot.logical_position),
                        crate::preserves_rail::string(&slot.source_id),
                        crate::preserves_rail::string(&slot.operation_id),
                        crate::preserves_rail::string(&slot.request_ref),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::sequence(
            ledger
                .observations
                .iter()
                .map(|observation| crate::preserves_rail::string(&observation.observation_ref))
                .collect(),
        ),
    ]))
}

/// The capsule closure, served from the recorded world-commit store, the
/// recorded inputs and the boundary. Every member is re-identified from its
/// bytes; possession of these bytes grants no admission.
struct CapsuleStore<'a> {
    trace: &'a WorldTransitionTrace,
    boundary: &'a WorldReplayObservationBoundary,
    source: &'a LocalWorldCommitStore,
    objects: BTreeMap<String, Vec<u8>>,
    materialized: usize,
}

impl CapsuleStore<'_> {
    /// The member bytes and the identity re-derived from them.
    fn payload(&self, object_ref: &str, roles: &[WorldReplayCapsuleMemberRole]) -> Result<(Vec<u8>, String)> {
        if roles.contains(&WorldReplayCapsuleMemberRole::Trace) {
            let record = canonical_world_transition_trace(self.trace)?;
            let identity = identify_world_transition_trace(self.trace)
                .map_err(|issue| super::invalid(format!("trace identity: {issue:?}")))?;
            return Ok((record.bytes, identity));
        }
        if roles.contains(&WorldReplayCapsuleMemberRole::WorldCommit) {
            let commit_ref = WorldCommitRef::new(object_ref.to_string())
                .map_err(|error| super::invalid(format!("capsule commit ref: {error:?}")))?;
            let bytes = self.source.read_commit(&commit_ref).map_err(store_error)?;
            let commit = parse_canonical_world_commit_with_ref(&bytes, &commit_ref, &WorldCommitBounds::default())?;
            return Ok((bytes, commit.commit_ref.as_str().to_string()));
        }
        if roles.contains(&WorldReplayCapsuleMemberRole::SourceInventory) {
            let identity = identify_replay_source_inventory(&self.boundary.inventory)
                .map_err(|issue| super::invalid(format!("inventory identity: {issue:?}")))?;
            return Ok((inventory_bytes(&self.boundary.inventory)?, identity));
        }
        if roles.contains(&WorldReplayCapsuleMemberRole::ObservationLedger) {
            let identity = identify_replay_observation_ledger(&self.boundary.ledger)
                .map_err(|issue| super::invalid(format!("ledger identity: {issue:?}")))?;
            return Ok((ledger_bytes(&self.boundary.ledger)?, identity));
        }
        let root_kind = roles.iter().find_map(|role| match role {
            WorldReplayCapsuleMemberRole::TypedRoot(kind) => Some(*kind),
            _ => None,
        });
        let bytes = match root_kind {
            Some(kind) => self.root(kind, object_ref)?,
            None => self
                .objects
                .get(object_ref)
                .cloned()
                .ok_or_else(|| super::invalid("capsule member is absent from the recorded closure"))?,
        };
        let identity = crate::preserves_rail::content_ref_from_bytes(&bytes);
        Ok((bytes, identity))
    }

    fn root(&self, kind: RootKind, object_ref: &str) -> Result<Vec<u8>> {
        let root = WorldRootRef::parse(kind, object_ref.to_string())
            .map_err(|error| super::invalid(format!("capsule root ref: {error:?}")))?;
        self.source.read_root(&root).map_err(store_error)
    }

    fn capsule(
        &self,
        requirements: Vec<molten_core::world_replay::WorldReplayClosureRequirement>,
    ) -> Result<WorldReplayCapsule> {
        let mut roles = BTreeMap::<String, Vec<WorldReplayCapsuleMemberRole>>::new();
        for requirement in requirements {
            roles.entry(requirement.object_ref).or_default().push(requirement.role);
        }
        let mut members = Vec::with_capacity(roles.len());
        for (object_ref, mut member_roles) in roles {
            member_roles.sort();
            member_roles.dedup();
            let (bytes, _) = self.payload(&object_ref, &member_roles)?;
            members.push(WorldReplayCapsuleMember {
                object_ref,
                roles: member_roles,
                codec: WorldReplayMemberCodec::CanonicalPreservesV1,
                byte_length: u64::try_from(bytes.len()).map_err(|_| super::invalid("capsule member exceeds u64"))?,
                protection: WorldReplayMemberProtection::Public,
            });
        }
        let mut capsule = WorldReplayCapsule {
            schema: WORLD_REPLAY_CAPSULE_SCHEMA.to_string(),
            capsule_ref: String::new(),
            trace_ref: self.trace.trace_ref.clone(),
            profile: self.trace.profile.clone(),
            members,
            non_claims: world_replay_non_claims(),
        };
        capsule.capsule_ref = identify_world_replay_capsule(&capsule)
            .map_err(|issue| super::invalid(format!("capsule identity: {issue:?}")))?;
        Ok(capsule)
    }
}

impl WorldReplayMaterializationPort for CapsuleStore<'_> {
    fn materialize(&mut self, member: &WorldReplayCapsuleMember) -> Result<WorldReplayMaterializationObservation> {
        let (bytes, identity) = self.payload(&member.object_ref, &member.roles)?;
        let is_verified = identity == member.object_ref && u64::try_from(bytes.len()).ok() == Some(member.byte_length);
        if is_verified {
            self.materialized = self
                .materialized
                .checked_add(1)
                .ok_or_else(|| super::invalid("materialized member count overflowed"))?;
        }
        Ok(WorldReplayMaterializationObservation {
            object_ref: member.object_ref.clone(),
            observation_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "aspen-replay-member-materialized-v1",
                vec![
                    crate::preserves_rail::string(&member.object_ref),
                    crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&bytes)),
                ],
            ))?,
            available: true,
            identity_verified: is_verified,
        })
    }
}

/// The destination runtime-spine world shared by the restore, transition and
/// capture ports of one replay.
struct Destination {
    node: NodeRoot,
    descriptor: SnapshotDescriptor,
    store: LocalWorldCommitStore,
    world: Option<WorldState>,
    parent: Option<WorldCommitRef>,
    restores: usize,
    executed: Vec<u64>,
    captured: usize,
}

type Shared = Rc<RefCell<Destination>>;

/// Restore the trace's initial commit through the runtime-spine logical
/// snapshot restore; the restored owner must reproduce that commit.
struct SnapshotRestore(Shared);

impl WorldReplayRestorePort for SnapshotRestore {
    fn restore_logical(
        &mut self,
        profile: &molten_core::world_replay::WorldReplayProfile,
        initial_commit: &CanonicalWorldCommit,
    ) -> Result<WorldReplayRestoreObservation> {
        let mut guard = self.0.borrow_mut();
        let destination = &mut *guard;
        if profile.profile_ref.as_str() != local_logical_profile_ref()? || profile.cohort_ref.is_some() {
            return Err(super::invalid("replay profile is not the local logical runtime-spine profile"));
        }
        if destination.descriptor.commit_ref != initial_commit.commit_ref {
            return Err(super::invalid("snapshot descriptor binds another initial world commit"));
        }
        let adapter = destination.node.adapter()?;
        let observed = adapter.observed_cohort().clone();
        let restored = restore_local_logical_snapshot(adapter, &destination.descriptor, &observed)?;
        let readback = read_local_logical_runtime(&destination.node.root)?;
        let world =
            WorldState::from_image(LogicalRuntimeImage::from_bytes(&readback.canonical_image)?, restored.runtime)?;
        let (commit, _) = capture(&mut destination.store, &world, initial_commit.core.parents.clone())?;
        if commit.commit_ref != initial_commit.commit_ref {
            return Err(super::invalid("restored runtime does not reproduce the initial world commit"));
        }
        destination.world = Some(world);
        destination.parent = Some(commit.commit_ref);
        destination.restores =
            destination.restores.checked_add(1).ok_or_else(|| super::invalid("restore count overflowed"))?;
        Ok(WorldReplayRestoreObservation {
            observation_ref: restored.restore.receipt.artifact_ref,
            profile_ref: profile.profile_ref.as_str().to_string(),
            cohort_ref: None,
            logical_fallback_used: false,
        })
    }

    fn restore_opaque_exact(
        &mut self,
        _profile: &molten_core::world_replay::WorldReplayProfile,
        _initial_commit: &CanonicalWorldCommit,
    ) -> Result<WorldReplayRestoreObservation> {
        Err(super::invalid("the runtime-spine destination restores only the logical profile"))
    }
}

/// Execute each recorded committed turn on the restored runtime owner.
struct RuntimeTransitions {
    destination: Shared,
    inputs: BTreeMap<String, RuntimeStep>,
}

impl WorldReplayTransitionPort for RuntimeTransitions {
    fn execute_transition(
        &mut self,
        step: &WorldTransitionStep,
        _observations: &mut SealedReplayObservations,
    ) -> Result<WorldReplayExecutionObservation> {
        let input = self
            .inputs
            .get(&step.input.input_ref)
            .ok_or_else(|| super::invalid("replay input is absent from the recorded inputs"))?;
        let bytes = crate::preserves_rail::canonical_bytes(&input.to_value())?;
        if input.step_ref()? != step.input.input_ref
            || step.input.schema_ref != step_schema_ref()
            || u64::try_from(bytes.len()).ok() != Some(step.input.byte_length)
        {
            return Err(super::invalid("replay input identity, schema or length differs from its trace step"));
        }
        let mut guard = self.destination.borrow_mut();
        let destination = &mut *guard;
        let world = destination
            .world
            .as_mut()
            .ok_or_else(|| super::invalid("transition executed before the runtime owner was restored"))?;
        let events = world.apply(input);
        destination.executed.push(step.position);
        Ok(WorldReplayExecutionObservation {
            observation_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "aspen-runtime-turn-observation-v1",
                vec![
                    crate::preserves_rail::u64_value(step.position),
                    crate::preserves_rail::string(&step.input.input_ref),
                    crate::preserves_rail::sequence(events.iter().map(RuntimeEvent::to_value).collect()),
                ],
            ))?,
            position: step.position,
            input_ref: step.input.input_ref.clone(),
        })
    }
}

/// Capture each successor through the production capture shell.
struct RuntimeCapture(Shared);

impl WorldReplayCapturePort for RuntimeCapture {
    fn capture_successor(
        &mut self,
        step: &WorldTransitionStep,
        _execution: &WorldReplayExecutionObservation,
    ) -> Result<WorldReplayCaptureObservation> {
        let mut guard = self.0.borrow_mut();
        let destination = &mut *guard;
        let parent = destination
            .parent
            .clone()
            .ok_or_else(|| super::invalid("successor captured before the initial commit was restored"))?;
        let world = destination
            .world
            .clone()
            .ok_or_else(|| super::invalid("successor captured without a runtime owner"))?;
        let (commit, receipt) = capture(&mut destination.store, &world, vec![parent.clone()])?;
        destination.parent = Some(commit.commit_ref.clone());
        destination.captured =
            destination.captured.checked_add(1).ok_or_else(|| super::invalid("capture count overflowed"))?;
        Ok(WorldReplayCaptureObservation {
            observation_ref: receipt.receipt_ref,
            transition: WorldReplayTransitionObservation {
                position: step.position,
                observed_parent: parent,
                actual: WorldReplayObservedCommit {
                    commit_ref: commit.commit_ref,
                    roots: commit.core.roots,
                },
                field_differences: Vec::new(),
            },
        })
    }
}

/// Durable replay receipts in the destination node.
struct Receipts {
    root: DirectoryRoot,
    kinds: Vec<&'static str>,
}

impl WorldReplayReceiptPort for Receipts {
    fn publish(&mut self, record: &CanonicalWorldReplayRecord) -> Result<String> {
        crate::preserves_rail::strict_canonical_decode(&record.bytes)?;
        let directory = RelativePath::parse(RECEIPT_DIRECTORY)?;
        self.root.create_dir_all(&directory)?;
        let path =
            directory.join(&format!("{}.preserves", crate::preserves_rail::content_ref_hex(&record.record_ref)?))?;
        if self.root.try_exists(&path)? {
            if self.root.read(&path, crate::world_snapshot::MAX_LOCAL_LOGICAL_IMAGE_BYTES)? != record.bytes {
                return Err(super::invalid("replay receipt publication conflicts with retained bytes"));
            }
        } else {
            self.root.write_new(&path, &record.bytes)?;
        }
        self.kinds.push(record.kind);
        Ok(record.record_ref.clone())
    }
}

/// Current replay admission, observed when the replay service asks for it:
/// authority from capability admission of an Ed25519-signed token against the
/// current revocation document, plus artifact, schema, resource, runtime and
/// effect facts of this process and this capsule.
struct CurrentAuthority {
    destination: Shared,
    issuer: ed25519_dalek::VerifyingKey,
    token: CapabilityToken,
    signature: ed25519_dalek::Signature,
    boundary: WorldReplayObservationBoundary,
    receipts: Vec<IOValue>,
}

impl CurrentAuthority {
    fn revocations(root: &DirectoryRoot) -> Result<(u64, Vec<String>)> {
        use preserves::ValueImpl as _;
        let bytes = root.read(&RelativePath::parse(AUTHORITY_PATH)?, MAX_AUTHORITY_BYTES)?;
        let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
        let fields = crate::preserves_rail::simple_record_fields(&value, AUTHORITY_RECORD, 2)?;
        let generation = fields[0]
            .as_u64()
            .ok_or_else(|| super::invalid("current authority generation missing"))?
            .map_err(|error| super::invalid(format!("current authority generation: {error}")))?;
        let revoked = fields[1]
            .collect_sequence()
            .ok_or_else(|| super::invalid("current revocations must be a sequence"))?
            .as_slice()
            .iter()
            .map(|reference| crate::preserves_rail::required_string_field(reference, "revocation ref"))
            .collect::<Result<Vec<_>>>()?;
        Ok((generation, revoked))
    }

    fn publish(root: &DirectoryRoot, generation: u64, revoked: &[String]) -> Result<()> {
        let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(AUTHORITY_RECORD, vec![
            crate::preserves_rail::u64_value(generation),
            crate::preserves_rail::sequence(revoked.iter().map(crate::preserves_rail::string).collect()),
        ]))?;
        root.create_dir_all(&RelativePath::parse("storage/world-replay")?)?;
        root.write(&RelativePath::parse(AUTHORITY_PATH)?, &bytes)
    }

    fn signature_verified(&self) -> Result<bool> {
        let bytes = crate::preserves_rail::canonical_bytes(&capability_token_value(&self.token)?)?;
        Ok(self.token.issuer_ref == key_ref(&self.issuer) && self.issuer.verify(&bytes, &self.signature).is_ok())
    }
}

impl WorldReplayAdmissionPort for CurrentAuthority {
    fn observe_current(
        &mut self,
        trace: &WorldTransitionTrace,
        capsule: &WorldReplayCapsule,
    ) -> Result<WorldReplayAdmissionObservation> {
        let destination = self.destination.borrow();
        let (generation, revoked) = Self::revocations(&destination.node.root)?;
        let proofset = CapabilityProofset {
            holder_ref: self.token.holder_ref.clone(),
            session_ref: self.token.session_ref.clone(),
            context_ref: self.token.context_ref.clone(),
            tokens: vec![self.token.clone()],
            policy_refs: vec![super::runtime::policy_ref()],
            resource_refs: Vec::new(),
            revocation_refs: revoked,
            evidence_refs: Vec::new(),
        };
        let request = CapabilityRequest {
            holder_ref: self.token.holder_ref.clone(),
            session_ref: self.token.session_ref.clone(),
            context_ref: capsule.capsule_ref.clone(),
            resource_ref: trace.trace_ref.clone(),
            ability: REPLAY_ABILITY.to_string(),
            scope: REPLAY_SCOPE.to_string(),
            at_tick: generation,
            required_policy_refs: vec![super::runtime::policy_ref()],
            required_resource_refs: Vec::new(),
            required_token_kind: Some(TOKEN_KIND.to_string()),
            caveat_context: Vec::new(),
        };
        let admission = admit_capability(&proofset, &request)?;
        let authority_admitted = admission.decision == "pass" && self.signature_verified()?;
        let observed_cohort = destination.node.adapter()?.observed_cohort().cohort_ref.clone();
        let artifact_admitted = observed_cohort == destination.descriptor.cohort.cohort_ref;
        let schema_admitted = trace.steps.iter().all(|step| step.input.schema_ref == step_schema_ref());
        let resource_admitted = trace.steps.len() <= MAX_REPLAY_TURNS;
        let runtime_admitted =
            trace.profile.profile_ref.as_str() == local_logical_profile_ref()? && trace.profile.cohort_ref.is_none();
        let effect_admitted = self.boundary.ledger.reached_sources.iter().all(|source_id: &String| {
            self.boundary.inventory.source(source_id).is_some_and(|source| !source.is_external_mutation)
        });
        let facts = [
            authority_admitted,
            artifact_admitted,
            schema_admitted,
            resource_admitted,
            runtime_admitted,
            effect_admitted,
        ];
        let observation_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "aspen-qualification-replay-admission-v1",
            vec![
                crate::preserves_rail::string(&trace.trace_ref),
                crate::preserves_rail::string(&capsule.capsule_ref),
                crate::preserves_rail::string(&admission.receipt_ref),
                crate::preserves_rail::u64_value(generation),
                crate::preserves_rail::sequence(
                    facts.iter().map(|held| crate::preserves_rail::bool_value(*held)).collect(),
                ),
            ],
        ))?;
        self.receipts.push(admission.value);
        Ok(WorldReplayAdmissionObservation {
            observation_ref,
            trace_ref: trace.trace_ref.clone(),
            capsule_ref: capsule.capsule_ref.clone(),
            profile_ref: trace.profile.profile_ref.as_str().to_string(),
            generation,
            authority_admitted,
            artifact_admitted,
            schema_admitted,
            resource_admitted,
            runtime_admitted,
            effect_admitted,
        })
    }
}

fn replay_token(
    issuer: &SigningKey,
    request: &WorldReplayPlanRequest,
) -> Result<(CapabilityToken, ed25519_dalek::Signature)> {
    let token = CapabilityToken {
        token_kind: TOKEN_KIND.to_string(),
        issuer_ref: key_ref(&issuer.verifying_key()),
        holder_ref: reference("aspen qualification replay executor"),
        session_ref: reference("aspen qualification replay session"),
        context_ref: request.capsule.capsule_ref.clone(),
        resource_ref: request.trace.trace_ref.clone(),
        ability: REPLAY_ABILITY.to_string(),
        scope: REPLAY_SCOPE.to_string(),
        attenuation: "exact".to_string(),
        caveats: Vec::new(),
        expires_at_tick: AUTHORITY_GENERATION,
        revoked_refs: Vec::new(),
        policy_refs: vec![super::runtime::policy_ref()],
        resource_refs: Vec::new(),
        delegation_refs: Vec::new(),
        evidence_refs: Vec::new(),
    };
    let bytes = crate::preserves_rail::canonical_bytes(&capability_token_value(&token)?)?;
    let signature = issuer.sign(&bytes);
    Ok((token, signature))
}

fn dependency_refs() -> Vec<String> {
    [
        "molten runtime-spine committed turn",
        "molten world-commit capture shell",
        "molten world replay service",
    ]
    .map(reference)
    .to_vec()
}

fn replay(revoke: bool) -> Result<ReplayRun> {
    let scratch = super::super::scratch("world-replay")?;
    let source = NodeRoot::open(&scratch.path().join("source"), "replay-source")?;
    let recording = record(&source.root)?;
    let boundary = boundary(&recording.trace)?;
    let mut objects = BTreeMap::new();
    for input in recording.inputs.values() {
        objects.insert(input.step_ref()?, crate::preserves_rail::canonical_bytes(&input.to_value())?);
    }
    let profile_bytes =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("logical-runtime-profile-v1", vec![
            crate::preserves_rail::string(crate::world_snapshot::LOCAL_LOGICAL_RUNTIME_PROFILE),
        ]))?;
    objects.insert(crate::preserves_rail::content_ref_from_bytes(&profile_bytes), profile_bytes);

    let mut closures = Vec::with_capacity(recording.commits.len());
    for commit in &recording.commits {
        let bytes = recording.store.read_commit(&commit.commit_ref).map_err(store_error)?;
        let verified = parse_canonical_world_commit_with_ref(&bytes, &commit.commit_ref, &WorldCommitBounds::default())
            .is_ok_and(|parsed| parsed == *commit);
        closures.push(WorldReplayCommitClosure {
            commit_ref: commit.commit_ref.clone(),
            parents: commit.core.parents.clone(),
            roots: commit.core.roots.clone(),
            canonical_identity_verified: verified,
        });
    }
    let mut requirements = required_world_replay_closure(&recording.trace, &closures, &[]);
    requirements.extend(required_world_replay_boundary_closure(&boundary));
    requirements.sort();
    requirements.dedup();
    let mut capsule_store = CapsuleStore {
        trace: &recording.trace,
        boundary: &boundary,
        source: &recording.store,
        objects,
        materialized: 0,
    };
    let capsule = capsule_store.capsule(requirements)?;
    let request = WorldReplayPlanRequest {
        supported_profile_refs: vec![recording.trace.profile.profile_ref.as_str().to_string()],
        trace: recording.trace.clone(),
        capsule,
        commits: closures,
        additional_requirements: Vec::new(),
        bounds: WorldReplayBounds::default(),
        boundary: boundary.clone(),
    };

    let node = NodeRoot::open(&scratch.path().join("destination"), "replay-destination")?;
    let descriptor =
        node.adapter()?.prepare_bundle(&recording.initial.image(), recording.trace.initial_commit.clone())?;
    let snapshot_ref = descriptor_ref(&descriptor)?;
    let current = admission_document(AUTHORITY_GENERATION, SNAPSHOT_REVOCATION, true);
    node.admit(SnapshotGrant {
        signer: &node.signer,
        descriptor: &descriptor,
        descriptor_ref: &snapshot_ref,
        granted: &current,
        current: &current,
    })?;
    let issuer = fixture_key("world replay execution authority");
    let (token, signature) = replay_token(&issuer, &request)?;
    let revoked = if revoke {
        vec![token.issuer_ref.clone()]
    } else {
        Vec::new()
    };
    CurrentAuthority::publish(&node.root, AUTHORITY_GENERATION, &revoked)?;

    let receipt_root = node.root.clone();
    let store = commit_store(&node.root)?;
    let shared = Rc::new(RefCell::new(Destination {
        node,
        descriptor,
        store,
        world: None,
        parent: None,
        restores: 0,
        executed: Vec::new(),
        captured: 0,
    }));
    let mut restore = SnapshotRestore(Rc::clone(&shared));
    let mut admission = CurrentAuthority {
        destination: Rc::clone(&shared),
        issuer: issuer.verifying_key(),
        token,
        signature,
        boundary: boundary.clone(),
        receipts: Vec::new(),
    };
    let mut transitions = RuntimeTransitions {
        destination: Rc::clone(&shared),
        inputs: recording.inputs.clone(),
    };
    let mut capture_port = RuntimeCapture(Rc::clone(&shared));
    let mut receipts = Receipts {
        root: receipt_root,
        kinds: Vec::new(),
    };
    let outcome = run_world_replay(&request, &recording.commits[0], &dependency_refs(), WorldReplayPorts {
        materialization: &mut capsule_store,
        restore: &mut restore,
        admission: &mut admission,
        transitions: &mut transitions,
        capture: &mut capture_port,
        receipts: &mut receipts,
    })?;
    let materialized = capsule_store.materialized;
    let destination = shared.borrow();
    Ok(ReplayRun {
        request,
        outcome,
        materialized,
        restores: destination.restores,
        executed: destination.executed.clone(),
        captured: destination.captured,
        receipt_kinds: receipts.kinds,
        authority_receipts: admission.receipts,
    })
}
