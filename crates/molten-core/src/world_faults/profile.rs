use super::*;

const FIRST_CONTENDER_SUFFIX: &str = "left";
const SECOND_CONTENDER_SUFFIX: &str = "right";
const FIRST_NODE_ID: &str = "node-a";
const SECOND_NODE_ID: &str = "node-b";
const PROFILE_NAME: &str = "local-deterministic-process-restart-v1";
const ADAPTER_PROFILE: &str = "deterministic-semantic-phase-v1";

// r[impl molten.world_faults.profile]
pub fn standard_world_fault_profile(source_revision: &str) -> Result<WorldFaultProfile, Vec<WorldFaultIssue>> {
    let inventory = standard_world_mutation_inventory();
    let inventory_ref = identify_world_mutation_inventory(&inventory)?;
    let adapters = inventory.rows.iter().map(adapter_binding).collect::<Vec<_>>();
    let cases = inventory
        .rows
        .iter()
        .flat_map(|row| row.required_phases.iter().map(|phase| fault_case(row, *phase)))
        .collect::<Vec<_>>();
    let schedules = WorldMutationKind::CONCURRENT.into_iter().map(concurrent_schedule).collect::<Vec<_>>();
    let profile = WorldFaultProfile {
        schema: WORLD_FAULT_PROFILE_SCHEMA,
        profile_name: PROFILE_NAME.to_string(),
        source_revision: source_revision.to_string(),
        inventory_ref,
        adapters,
        limits: WorldFaultLimits::standard(),
        cases,
        schedules,
    };
    let issues = validate_world_fault_profile(&profile);
    if issues.is_empty() { Ok(profile) } else { Err(issues) }
}

pub const fn expected_recovery_for_phase(phase: FaultPhase) -> RecoveryClass {
    match phase {
        FaultPhase::Uninterrupted
        | FaultPhase::AfterDurableWrite
        | FaultPhase::BeforeResponse
        | FaultPhase::RecoveryReadBack => RecoveryClass::AlreadyComplete,
        FaultPhase::BeforeSubmit => RecoveryClass::SafeToRetry,
        FaultPhase::AfterPossibleSubmit | FaultPhase::LostResponse | FaultPhase::ProcessRestart => {
            RecoveryClass::Uncertain
        }
    }
}

fn adapter_binding(row: &WorldMutationContract) -> WorldFaultAdapterBinding {
    let owner = row.owner;
    WorldFaultAdapterBinding {
        adapter_id: format!("{}-fault-adapter", owner.as_str()),
        owner,
        profile: if row.support == MutationSupport::Supported {
            ADAPTER_PROFILE.to_string()
        } else {
            "unsupported-independent-witness".to_string()
        },
        implementation_ref: reference(&format!("{}:implementation", owner.as_str())),
        semantic_phase_map_ref: reference(&format!("{}:semantic-phase-map", owner.as_str())),
    }
}

fn fault_case(row: &WorldMutationContract, phase: FaultPhase) -> WorldFaultCase {
    let mutation = row.mutation;
    let expected_decision = if row.support == MutationSupport::UnsupportedIndependentWitness {
        RecoveryClass::ManualReview
    } else {
        expected_recovery_for_phase(phase)
    };
    WorldFaultCase {
        case_id: format!("{}:{}", mutation.as_str(), phase.as_str()),
        mutation,
        operation_id: reference(&format!("{}:operation", mutation.as_str())),
        phase,
        adapter_id: format!("{}-fault-adapter", row.owner.as_str()),
        expected_generation: WORLD_FAULT_OPERATION_GENERATION,
        pre_state_ref: reference(&format!("{}:pre-state", mutation.as_str())),
        expected_decision,
    }
}

fn concurrent_schedule(mutation: WorldMutationKind) -> ConcurrentSchedule {
    let first_operation = reference(&format!("{}:{FIRST_CONTENDER_SUFFIX}", mutation.as_str()));
    let second_operation = reference(&format!("{}:{SECOND_CONTENDER_SUFFIX}", mutation.as_str()));
    let pre_state_ref = reference(&format!("{}:concurrent-pre-state", mutation.as_str()));
    let mut steps = Vec::with_capacity(WORLD_FAULT_STEPS_PER_SCHEDULE);
    push_pair(
        &mut steps,
        mutation,
        &first_operation,
        &second_operation,
        &pre_state_ref,
        WORLD_FAULT_SCHEDULE_FIRST_POSITION,
        InterleavingPoint::Prepare,
    );
    push_pair(
        &mut steps,
        mutation,
        &first_operation,
        &second_operation,
        &pre_state_ref,
        WORLD_FAULT_SCHEDULE_SECOND_POSITION,
        InterleavingPoint::CurrentFactRecheck,
    );
    push_pair(
        &mut steps,
        mutation,
        &first_operation,
        &second_operation,
        &pre_state_ref,
        WORLD_FAULT_SCHEDULE_THIRD_POSITION,
        InterleavingPoint::BeforeLinearization,
    );
    push_pair(
        &mut steps,
        mutation,
        &first_operation,
        &second_operation,
        &pre_state_ref,
        WORLD_FAULT_SCHEDULE_FOURTH_POSITION,
        InterleavingPoint::DurableReadBack,
    );
    ConcurrentSchedule {
        schedule_id: format!("{}-generation-race-v1", mutation.as_str()),
        mutation,
        steps,
    }
}

#[allow(
    tigerstyle::too_many_parameters,
    reason = "the helper keeps both contenders bound to one mutation, pre-state, position, and interleaving point"
)]
fn push_pair(
    steps: &mut Vec<ConcurrentScheduleStep>,
    mutation: WorldMutationKind,
    first_operation: &str,
    second_operation: &str,
    pre_state_ref: &str,
    position: u32,
    interleaving: InterleavingPoint,
) {
    steps.push(schedule_step(position, first_operation, mutation, pre_state_ref, interleaving, FIRST_NODE_ID));
    steps.push(schedule_step(position, second_operation, mutation, pre_state_ref, interleaving, SECOND_NODE_ID));
}

#[allow(
    tigerstyle::too_many_parameters,
    reason = "the constructor makes every fenced schedule binding explicit at the call site"
)]
fn schedule_step(
    position: u32,
    operation_id: &str,
    mutation: WorldMutationKind,
    pre_state_ref: &str,
    interleaving: InterleavingPoint,
    node_id: &str,
) -> ConcurrentScheduleStep {
    ConcurrentScheduleStep {
        position,
        operation_id: operation_id.to_string(),
        mutation,
        expected_generation: WORLD_FAULT_OPERATION_GENERATION,
        pre_state_ref: pre_state_ref.to_string(),
        interleaving,
        node_id: node_id.to_string(),
        node_generation: WORLD_FAULT_NODE_GENERATION,
    }
}
