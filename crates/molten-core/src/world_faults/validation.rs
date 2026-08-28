use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

const REQUIRED_SCHEDULE_POSITION_COUNT: u32 = 4;
const REQUIRED_CONTENDER_COUNT: usize = 2;

// r[impl molten.world_faults.profile]
pub fn validate_world_fault_profile(profile: &WorldFaultProfile) -> Vec<WorldFaultIssue> {
    let mut issues = Vec::new();
    if profile.schema != WORLD_FAULT_PROFILE_SCHEMA {
        issues.push(WorldFaultIssue::SchemaMismatch("world-fault-profile"));
    }
    validate_non_empty("profile-name", &profile.profile_name, &mut issues);
    validate_non_empty("source-revision", &profile.source_revision, &mut issues);
    validate_ref("inventory-ref", &profile.inventory_ref, &mut issues);
    validate_limits(profile.limits, &mut issues);
    check_bound("adapters", profile.adapters.len(), profile.limits.max_adapters, &mut issues);
    check_bound("cases", profile.cases.len(), profile.limits.max_cases, &mut issues);
    check_bound("schedules", profile.schedules.len(), profile.limits.max_schedules, &mut issues);

    let inventory = standard_world_mutation_inventory();
    let owner_by_mutation = inventory.rows.iter().map(|row| (row.mutation, row.owner)).collect::<BTreeMap<_, _>>();
    let required_phases = inventory
        .rows
        .iter()
        .map(|row| (row.mutation, row.required_phases.iter().copied().collect::<BTreeSet<_>>()))
        .collect::<BTreeMap<_, _>>();

    let adapters = validate_adapters(&profile.adapters, &mut issues);
    validate_cases(profile, &owner_by_mutation, &required_phases, &adapters, &mut issues);
    validate_schedules(profile, &mut issues);
    issues.sort();
    issues.dedup();
    issues
}

fn validate_limits(limits: WorldFaultLimits, issues: &mut Vec<WorldFaultIssue>) {
    for (field, value, maximum) in [
        ("max-cases", limits.max_cases, MAX_WORLD_FAULT_CASES),
        ("max-schedules", limits.max_schedules, MAX_WORLD_FAULT_SCHEDULES),
        ("max-schedule-steps", limits.max_schedule_steps, MAX_WORLD_FAULT_SCHEDULE_STEPS),
        ("max-adapters", limits.max_adapters, MAX_WORLD_FAULT_ADAPTERS),
        ("max-observations", limits.max_observations, MAX_WORLD_FAULT_OBSERVATIONS),
        ("max-unsupported-rows", limits.max_unsupported_rows, MAX_WORLD_FAULT_UNSUPPORTED_ROWS),
    ] {
        if value == 0 || value > maximum {
            issues.push(WorldFaultIssue::InvalidLimit(field));
        }
    }
    if limits.max_restarts == 0 || limits.max_restarts > MAX_WORLD_FAULT_RESTARTS {
        issues.push(WorldFaultIssue::InvalidLimit("max-restarts"));
    }
}

fn validate_adapters(
    adapter_list: &[WorldFaultAdapterBinding],
    issues: &mut Vec<WorldFaultIssue>,
) -> BTreeMap<String, WorldMutationOwner> {
    let mut adapters = BTreeMap::new();
    let mut owners = BTreeSet::new();
    for adapter in adapter_list {
        validate_non_empty("adapter-id", &adapter.adapter_id, issues);
        validate_non_empty("adapter-profile", &adapter.profile, issues);
        validate_ref("adapter-implementation-ref", &adapter.implementation_ref, issues);
        validate_ref("semantic-phase-map-ref", &adapter.semantic_phase_map_ref, issues);
        if adapters.insert(adapter.adapter_id.clone(), adapter.owner).is_some() {
            issues.push(WorldFaultIssue::DuplicateAdapter(adapter.adapter_id.clone()));
        }
        owners.insert(adapter.owner);
    }
    for owner in WorldMutationKind::ALL.into_iter().map(expected_owner).collect::<BTreeSet<_>>() {
        if !owners.contains(&owner) {
            issues.push(WorldFaultIssue::AdapterOwnerMissing(owner));
        }
    }
    adapters
}

fn validate_cases(
    profile: &WorldFaultProfile,
    owner_by_mutation: &BTreeMap<WorldMutationKind, WorldMutationOwner>,
    required_phases: &BTreeMap<WorldMutationKind, BTreeSet<FaultPhase>>,
    adapters: &BTreeMap<String, WorldMutationOwner>,
    issues: &mut Vec<WorldFaultIssue>,
) {
    let mut case_ids = BTreeSet::new();
    let mut covered = BTreeMap::<WorldMutationKind, BTreeSet<FaultPhase>>::new();
    for case in &profile.cases {
        validate_non_empty("case-id", &case.case_id, issues);
        validate_ref("operation-id", &case.operation_id, issues);
        validate_ref("pre-state-ref", &case.pre_state_ref, issues);
        if !case_ids.insert(case.case_id.clone()) {
            issues.push(WorldFaultIssue::DuplicateCase(case.case_id.clone()));
        }
        let Some(owner) = owner_by_mutation.get(&case.mutation) else {
            issues.push(WorldFaultIssue::CaseMutationMissing(case.mutation));
            continue;
        };
        match adapters.get(&case.adapter_id) {
            Some(adapter_owner) if adapter_owner == owner => {}
            _ => issues.push(WorldFaultIssue::CaseAdapterMissing(case.adapter_id.clone())),
        }
        if !required_phases.get(&case.mutation).is_some_and(|phases| phases.contains(&case.phase)) {
            issues.push(WorldFaultIssue::CasePhaseNotRequired {
                mutation: case.mutation,
                phase: case.phase,
            });
        }
        let expected_decision = if case.mutation == WorldMutationKind::Witness {
            RecoveryClass::ManualReview
        } else {
            expected_recovery_for_phase(case.phase)
        };
        if case.expected_decision != expected_decision {
            issues.push(WorldFaultIssue::CaseExpectedDecisionMismatch {
                case_id: case.case_id.clone(),
                expected: expected_decision,
                actual: case.expected_decision,
            });
        }
        covered.entry(case.mutation).or_default().insert(case.phase);
    }
    for (mutation, phases) in required_phases {
        for phase in phases {
            if !covered.get(mutation).is_some_and(|observed| observed.contains(phase)) {
                issues.push(WorldFaultIssue::MissingRequiredPhase {
                    mutation: *mutation,
                    phase: *phase,
                });
            }
        }
    }
}

fn validate_schedules(profile: &WorldFaultProfile, issues: &mut Vec<WorldFaultIssue>) {
    let mut schedule_ids = BTreeSet::new();
    let mut mutations = BTreeSet::new();
    let mut total_steps = 0_usize;
    for schedule in &profile.schedules {
        validate_non_empty("schedule-id", &schedule.schedule_id, issues);
        if !schedule_ids.insert(schedule.schedule_id.clone()) {
            issues.push(WorldFaultIssue::DuplicateSchedule(schedule.schedule_id.clone()));
        }
        if !mutations.insert(schedule.mutation) {
            issues.push(WorldFaultIssue::DuplicateSchedule(schedule.mutation.as_str().to_string()));
        }
        total_steps = total_steps.saturating_add(schedule.steps.len());
        validate_schedule(schedule, issues);
    }
    if total_steps > profile.limits.max_schedule_steps {
        issues.push(WorldFaultIssue::ScheduleStepBoundExceeded {
            actual: total_steps,
            maximum: profile.limits.max_schedule_steps,
        });
    }
    for mutation in WorldMutationKind::CONCURRENT {
        if !mutations.contains(&mutation) {
            issues.push(WorldFaultIssue::ScheduleMutationMismatch(mutation));
        }
    }
}

fn validate_schedule(schedule: &ConcurrentSchedule, issues: &mut Vec<WorldFaultIssue>) {
    let mut positions = BTreeSet::new();
    let mut operations = BTreeSet::new();
    let mut expected_generation = None;
    let mut expected_pre_state = None;
    for step in &schedule.steps {
        if step.mutation != schedule.mutation {
            issues.push(WorldFaultIssue::ScheduleMutationMismatch(step.mutation));
        }
        validate_ref("schedule-operation-id", &step.operation_id, issues);
        validate_ref("schedule-pre-state-ref", &step.pre_state_ref, issues);
        validate_non_empty("schedule-node-id", &step.node_id, issues);
        if step.node_generation == 0 {
            issues.push(WorldFaultIssue::ScheduleNodeGenerationZero(step.node_id.clone()));
        }
        positions.insert(step.position);
        operations.insert(step.operation_id.clone());
        if expected_generation
            .replace(step.expected_generation)
            .is_some_and(|prior| prior != step.expected_generation)
        {
            issues.push(WorldFaultIssue::ConcurrentBindingMismatch(step.operation_id.clone()));
        }
        if expected_pre_state
            .replace(step.pre_state_ref.as_str())
            .is_some_and(|prior| prior != step.pre_state_ref)
        {
            issues.push(WorldFaultIssue::ConcurrentBindingMismatch(step.operation_id.clone()));
        }
    }
    if operations.len() != REQUIRED_CONTENDER_COUNT {
        issues.push(WorldFaultIssue::ScheduleOperationMissing(schedule.schedule_id.clone()));
    }
    for expected in 0..REQUIRED_SCHEDULE_POSITION_COUNT {
        if !positions.contains(&expected) {
            issues.push(WorldFaultIssue::SchedulePositionGap {
                schedule_id: schedule.schedule_id.clone(),
                expected,
                actual: positions.iter().next_back().copied().unwrap_or_default(),
            });
        }
    }
}

fn expected_owner(mutation: WorldMutationKind) -> WorldMutationOwner {
    standard_world_mutation_inventory()
        .rows
        .into_iter()
        .find(|row| row.mutation == mutation)
        .map(|row| row.owner)
        .unwrap_or(WorldMutationOwner::WorldCommit)
}

fn validate_non_empty(field: &'static str, value: &str, issues: &mut Vec<WorldFaultIssue>) {
    if value.is_empty() {
        issues.push(WorldFaultIssue::EmptyIdentifier(field));
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<WorldFaultIssue>) {
    if !is_blake3_ref(value) {
        issues.push(WorldFaultIssue::MalformedReference {
            field,
            value: value.to_string(),
        });
    }
}

fn check_bound(field: &'static str, actual: usize, maximum: usize, issues: &mut Vec<WorldFaultIssue>) {
    if actual == 0 || actual > maximum {
        issues.push(WorldFaultIssue::BoundExceeded { field, actual, maximum });
    }
}
