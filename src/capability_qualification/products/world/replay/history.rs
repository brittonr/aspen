//! The recorded source world: its committed turns captured commit by commit,
//! the transition trace over them and the observation boundary they reached.

const STEP_SCHEMA_LABEL: &str = "molten.runtime-step.v1";

/// The recorded world: its commits in the source node store and its trace.
pub(super) struct Recording {
    pub(super) store: crate::world_commit::LocalWorldCommitStore,
    pub(super) initial: super::super::runtime::CommittedState,
    pub(super) commits: Vec<crate::world_commit::CanonicalWorldCommit>,
    pub(super) trace: molten_core::world_replay::WorldTransitionTrace,
    pub(super) inputs: std::collections::BTreeMap<String, crate::runtime::RuntimeStep>,
}

pub(super) fn step_schema_ref() -> String {
    super::super::runtime::reference(STEP_SCHEMA_LABEL)
}

pub(super) fn record(source: &crate::node_state::DirectoryRoot) -> crate::error::Result<Recording> {
    let mut store = super::super::runtime::commit_store(source)?;
    let initial = super::super::runtime::CommittedState::committed()?;
    let mut world = initial.clone();
    let (initial_commit, _) = super::super::runtime::capture(&mut store, &world, Vec::new())?;
    let profile = super::super::runtime::logical_profile()?;
    let replay_profile = molten_core::world_replay::WorldReplayProfile {
        kind: molten_core::world_replay::WorldReplayProfileKind::Logical,
        profile_ref: profile.profile_ref.clone(),
        cohort_ref: None,
        snapshot_descriptor_ref: None,
    };
    let mut commits = vec![initial_commit];
    let recorded_inputs = super::super::runtime::replay_inputs()?;
    let mut steps = Vec::with_capacity(recorded_inputs.len());
    let mut inputs = Vec::with_capacity(recorded_inputs.len());
    for (position, input) in (0_u64..).zip(recorded_inputs) {
        let parent = commits
            .last()
            .map(|commit| commit.commit_ref.clone())
            .ok_or_else(|| super::super::invalid("recording lost its parent commit"))?;
        world.apply(&input);
        let (successor, _) = super::super::runtime::capture(&mut store, &world, vec![parent.clone()])?;
        let input_bytes = crate::preserves_rail::canonical_bytes(&input.to_value())?;
        let input_ref = input.step_ref()?;
        steps.push(molten_core::world_replay::WorldTransitionStep {
            position,
            expected_parent: parent,
            input: molten_core::world_replay::WorldTransitionInput {
                kind: molten_core::world_replay::WorldTransitionInputKind::Command,
                input_ref: input_ref.clone(),
                schema_ref: step_schema_ref(),
                byte_length: u64::try_from(input_bytes.len())
                    .map_err(|_| super::super::invalid("replay input length exceeds u64"))?,
            },
            profile_ref: profile.profile_ref.clone(),
            expected_successor: successor.commit_ref.clone(),
        });
        inputs.push((input_ref, input));
        commits.push(successor);
    }
    let mut trace = molten_core::world_replay::WorldTransitionTrace {
        schema: molten_core::world_replay::WORLD_TRANSITION_TRACE_SCHEMA.to_string(),
        trace_ref: String::new(),
        initial_commit: commits[0].commit_ref.clone(),
        profile: replay_profile,
        steps,
    };
    trace.trace_ref = molten_core::world_replay::identify_world_transition_trace(&trace)
        .map_err(|issue| super::super::invalid(format!("world transition trace identity: {issue:?}")))?;
    Ok(Recording {
        store,
        initial,
        commits,
        trace,
        inputs: inputs.into_iter().collect(),
    })
}

/// The runtime catalog boundary: the recorded turns reach only the simulated
/// virtual clock and entropy stream and the deterministic scheduler, so no
/// recorded observation slot exists.
pub(super) fn boundary(
    trace: &molten_core::world_replay::WorldTransitionTrace,
) -> crate::error::Result<molten_core::world_replay::WorldReplayObservationBoundary> {
    let adapter_cohort_ref =
        super::super::runtime::reference("aspen qualification runtime-spine replay adapter cohort");
    let protection_profile_ref =
        super::super::runtime::reference("aspen qualification runtime-spine replay protection profile");
    let inventory = crate::world_replay::runtime_replay_inventory(&crate::world_replay::RuntimeInventoryInput {
        profile_ref: trace.profile.profile_ref.as_str(),
        adapter_cohort_ref: &adapter_cohort_ref,
        protection_profile_ref: &protection_profile_ref,
        opaque_diagnostic_cohort_ref: None,
        scheduler_policy: molten_core::fabric_time::SchedulerReplayPolicy::Deterministic,
        sensitive_source_ids: &[],
    })?;
    let mut reached_sources = [
        crate::world_replay::CLOCK_VIRTUAL_TIME_SOURCE,
        crate::world_replay::ENTROPY_SEEDED_STREAM_SOURCE,
        crate::world_replay::SCHEDULER_DECISION_SOURCE,
    ]
    .map(ToString::to_string)
    .to_vec();
    reached_sources.sort();
    let mut ledger = molten_core::world_replay::ReplayObservationLedger {
        schema: molten_core::world_replay::REPLAY_OBSERVATION_LEDGER_SCHEMA.to_string(),
        ledger_ref: String::new(),
        inventory_ref: inventory.inventory_ref.clone(),
        trace_ref: trace.trace_ref.clone(),
        reached_sources,
        slots: Vec::new(),
        observations: Vec::new(),
    };
    ledger.ledger_ref = molten_core::world_replay::identify_replay_observation_ledger(&ledger)
        .map_err(|issue| super::super::invalid(format!("replay observation ledger identity: {issue:?}")))?;
    Ok(molten_core::world_replay::WorldReplayObservationBoundary {
        inventory,
        ledger,
        opaque_diagnostics: Vec::new(),
    })
}

pub(super) fn inventory_bytes(
    inventory: &molten_core::world_replay::ReplaySourceInventory,
) -> crate::error::Result<Vec<u8>> {
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

pub(super) fn ledger_bytes(
    ledger: &molten_core::world_replay::ReplayObservationLedger,
) -> crate::error::Result<Vec<u8>> {
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
