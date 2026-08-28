use super::*;
use crate::fabric_simulation::AdmittedSimulatedWorld;
use crate::fabric_simulation::ExtensionCoreIdentity;
use crate::fabric_simulation::SameCoreWitness;
use crate::fabric_simulation::SimulatedNode;
use crate::fabric_simulation::SimulatedWorldManifest;
use crate::fabric_simulation::SimulationBounds;
use crate::fabric_simulation::SimulationClaimProfile;
use crate::fabric_simulation::SimulationSchedulerState;
use crate::fabric_simulation::select_simulation_choice;

const TEST_SOURCE_REVISION: &str = "51646db62379c6790f21211630ff648f4a0446d1";
const TEST_MAX_CHOICES: u64 = 128;
const TEST_MAX_EVENTS: u64 = 128;
const TEST_MAX_TICKS: u64 = 128;
const TEST_MAX_TRACE_BYTES: u64 = 65_536;
const TEST_MAX_RESOURCE_UNITS: u64 = 128;
const TEST_MAX_SHRINK_ATTEMPTS: u64 = 16;
const TEST_IDENTITY_BYTE: u8 = 7;
const TEST_SECOND_IDENTITY_BYTE: u8 = 8;
const TEST_THIRD_IDENTITY_BYTE: u8 = 9;
const TEST_FOURTH_IDENTITY_BYTE: u8 = 10;
const TEST_FIFTH_IDENTITY_BYTE: u8 = 11;
const TEST_SIXTH_IDENTITY_BYTE: u8 = 12;
const TEST_DIGEST_BYTES: usize = 32;
const TEST_OPERATION_BOUND: u32 = 2;
const TEST_PREREQUISITE_BOUND: u32 = 2;
const TEST_REVISION: u64 = 4;
const TEST_GENERATION: u64 = 1;

// r[verify molten.world_faults.inventory]
#[test]
fn closed_inventory_is_complete_and_rejects_unregistered_mutation() {
    let inventory = standard_world_mutation_inventory();
    assert!(validate_world_mutation_inventory(&inventory, &registered_world_mutation_names()).is_empty());
    assert_eq!(inventory.rows.len(), REQUIRED_WORLD_MUTATION_COUNT);
    let first = identify_world_mutation_inventory(&inventory).expect("inventory identity");
    let second = identify_world_mutation_inventory(&inventory).expect("stable inventory identity");
    assert_eq!(first, second);

    let mut product = registered_world_mutation_names();
    product.push("unregistered-world-write".to_string());
    assert!(
        validate_world_mutation_inventory(&inventory, &product)
            .contains(&WorldFaultIssue::UnknownProductMutation("unregistered-world-write".to_string()))
    );
}

// r[verify molten.world_faults.profile]
#[test]
fn profile_binds_named_limits_phases_adapters_and_explicit_schedules() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    assert!(validate_world_fault_profile(&profile).is_empty());
    assert_eq!(profile.cases.len(), MAX_WORLD_FAULT_CASES);
    assert_eq!(profile.schedules.len(), MAX_WORLD_FAULT_SCHEDULES);
    assert_eq!(
        profile.schedules.iter().map(|schedule| schedule.steps.len()).sum::<usize>(),
        MAX_WORLD_FAULT_SCHEDULE_STEPS
    );
    assert_eq!(
        identify_world_fault_profile(&profile).expect("profile identity"),
        identify_world_fault_profile(&profile).expect("stable profile identity")
    );

    let mut invalid = profile;
    invalid.limits.max_restarts = 0;
    assert!(validate_world_fault_profile(&invalid).contains(&WorldFaultIssue::InvalidLimit("max-restarts")));
}

// r[verify molten.world_faults.interruption]
// r[verify molten.world_faults.receipt]
#[test]
fn bounded_matrix_passes_supported_rows_and_retains_witness_and_physical_nonclaims() {
    let inventory = standard_world_mutation_inventory();
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let observations = passing_observations(&profile);
    let schedules = passing_schedules(&profile);
    let receipt = build_world_fault_conformance_receipt(&inventory, &profile, &observations, schedules)
        .expect("bounded conformance receipt");

    assert_eq!(receipt.decision, ConformanceDisposition::Passed);
    assert!(!receipt.results.is_empty());
    assert!(receipt.results.iter().all(|result| result.disposition == ConformanceDisposition::Passed));
    assert!(
        receipt
            .unsupported_rows
            .iter()
            .any(|row| row.reason == UnsupportedReason::IndependentWitnessOwnerUnavailable)
    );
    assert!(
        receipt
            .unsupported_rows
            .iter()
            .any(|row| row.reason == UnsupportedReason::PhysicalFailureProfileNotExercised)
    );
    assert!(!receipt.mutation_authorized_by_evidence);
    assert!(!receipt.cleanup_authorized_by_evidence);
    assert!(validate_world_fault_receipt(&receipt, &profile).is_empty());
}

// r[verify molten.world_faults.recovery]
#[test]
fn success_missing_and_corrupt_observations_fail_conservatively() {
    let inventory = standard_world_mutation_inventory();
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let mut observations = passing_observations(&profile);
    let target = observations
        .iter_mut()
        .find(|observation| observation.phase == FaultPhase::Uninterrupted)
        .expect("uninterrupted observation");
    target.read_back.status = DurableReadBackStatus::Missing;
    target.read_back.state_ref = None;
    target.read_back.record_ref = None;
    let receipt =
        build_world_fault_conformance_receipt(&inventory, &profile, &observations, passing_schedules(&profile))
            .expect("failing receipt remains bounded evidence");
    assert_eq!(receipt.decision, ConformanceDisposition::Failed);
    assert!(receipt.results.iter().any(|result| {
        result.diagnostics.contains(&WorldFaultIssue::SuccessWithoutDurableReadBack(result.case_id.clone()))
    }));

    let mut corrupt = passing_observations(&profile);
    let target = corrupt
        .iter_mut()
        .find(|observation| observation.phase == FaultPhase::BeforeSubmit)
        .expect("before-submit observation");
    target.read_back.status = DurableReadBackStatus::Corrupt;
    let receipt = build_world_fault_conformance_receipt(&inventory, &profile, &corrupt, passing_schedules(&profile))
        .expect("corrupt receipt");
    assert_eq!(receipt.decision, ConformanceDisposition::Failed);
}

// r[verify molten.world_faults.recovery]
#[test]
fn lost_response_and_local_rollback_never_create_retry_or_witness_claims() {
    let inventory = standard_world_mutation_inventory();
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let mut observations = passing_observations(&profile);
    let lost = observations
        .iter_mut()
        .find(|observation| observation.phase == FaultPhase::LostResponse)
        .expect("lost-response observation");
    lost.owner_decision = RecoveryClass::SafeToRetry;
    let rollback = observations
        .iter_mut()
        .find(|observation| observation.phase == FaultPhase::ProcessRestart)
        .expect("restart observation");
    rollback.whole_store_rollback = true;
    rollback.owner_decision = RecoveryClass::AlreadyComplete;
    rollback.read_back.status = DurableReadBackStatus::Applied;
    rollback.read_back.state_ref = Some(reference("rolled-back-state"));
    rollback.read_back.record_ref = Some(reference("rolled-back-record"));
    rollback.read_back.independent_witness = false;

    let receipt =
        build_world_fault_conformance_receipt(&inventory, &profile, &observations, passing_schedules(&profile))
            .expect("negative conformance receipt");
    assert_eq!(receipt.decision, ConformanceDisposition::Failed);
    assert!(receipt.results.iter().flat_map(|result| &result.diagnostics).any(|issue| {
        matches!(
            issue,
            WorldFaultIssue::UnsafeRetryAfterPossibleSubmit(_) | WorldFaultIssue::LocalRollbackDetectionOverclaim(_)
        )
    }));
}

// r[verify molten.world_faults.concurrency]
#[test]
fn explicit_schedule_allows_one_linearization_and_rejects_two_promotions() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let schedule = profile
        .schedules
        .iter()
        .find(|schedule| schedule.mutation == WorldMutationKind::Promotion)
        .expect("promotion schedule");
    let operations = schedule_operations(schedule);
    let positive = vec![
        concurrent_observation(schedule, &operations[0], ConcurrentOutcome::Applied, 1),
        concurrent_observation(schedule, &operations[1], ConcurrentOutcome::Stale, 0),
    ];
    let passed = evaluate_concurrent_schedule(schedule, positive, all_schedule_choices(schedule));
    assert_eq!(passed.disposition, ConformanceDisposition::Passed);

    let negative = vec![
        concurrent_observation(schedule, &operations[0], ConcurrentOutcome::Applied, 1),
        concurrent_observation(schedule, &operations[1], ConcurrentOutcome::Applied, 1),
    ];
    let failed = evaluate_concurrent_schedule(schedule, negative, all_schedule_choices(schedule));
    assert_eq!(failed.disposition, ConformanceDisposition::Failed);
    assert!(
        failed
            .diagnostics
            .iter()
            .any(|issue| matches!(issue, WorldFaultIssue::MultipleLinearizations { .. }))
    );
    assert!(
        failed
            .diagnostics
            .iter()
            .any(|issue| matches!(issue, WorldFaultIssue::DuplicateEffectRelease { .. }))
    );
}

// r[verify molten.world_faults.concurrency]
#[test]
fn schedules_reuse_the_existing_fabric_simulation_choice_function() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let schedule = profile.schedules.first().expect("first schedule");
    let choices = eligible_world_fault_choices(schedule, WORLD_FAULT_SCHEDULE_FIRST_POSITION);
    let world = scheduler_world();
    let transition = select_simulation_choice(&world, &SimulationSchedulerState::default(), &choices, None)
        .expect("existing deterministic scheduler selects a world-fault choice");
    assert_eq!(transition.record.eligible, choices);
}

// r[verify molten.world_faults.verification]
#[test]
fn owner_recovery_fixtures_cover_complete_retry_superseded_conflict_and_manual_review() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let template = profile
        .cases
        .iter()
        .find(|case| case.mutation == WorldMutationKind::Head)
        .expect("head case")
        .clone();
    for (decision, status, submission) in [
        (
            RecoveryClass::AlreadyComplete,
            DurableReadBackStatus::Applied,
            SubmissionObservation::DurablySubmitted,
        ),
        (RecoveryClass::SafeToRetry, DurableReadBackStatus::Prior, SubmissionObservation::NotSubmitted),
        (RecoveryClass::Superseded, DurableReadBackStatus::Prior, SubmissionObservation::PossiblySubmitted),
        (
            RecoveryClass::Conflict,
            DurableReadBackStatus::Contradictory,
            SubmissionObservation::PossiblySubmitted,
        ),
        (
            RecoveryClass::ManualReview,
            DurableReadBackStatus::Missing,
            SubmissionObservation::PossiblySubmitted,
        ),
    ] {
        let mut case = template.clone();
        case.expected_decision = decision;
        let complete = decision == RecoveryClass::AlreadyComplete;
        let observation = WorldOperationObservation {
            case_id: case.case_id.clone(),
            operation_id: case.operation_id.clone(),
            phase: case.phase,
            submission,
            response: ResponseObservation::NotExpected,
            read_back: DurableReadBack {
                status,
                state_ref: if status == DurableReadBackStatus::Missing {
                    None
                } else {
                    Some(reference("recovery-state"))
                },
                record_ref: if complete {
                    Some(reference("recovery-record"))
                } else {
                    None
                },
                observed_generation: if status == DurableReadBackStatus::Missing {
                    None
                } else {
                    Some(case.expected_generation)
                },
                independent_witness: false,
            },
            owner_decision: decision,
            whole_store_rollback: false,
            cleanup_authorized: false,
        };
        let result = compare_world_fault_observation(&case, &observation);
        assert_eq!(result.disposition, ConformanceDisposition::Passed, "{decision:?}");
    }
}

// r[verify molten.world_faults.verification]
#[test]
fn torn_duplicate_stale_missing_corrupt_cleanup_and_contradictory_fixtures_fail_closed() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("fault profile");
    let case = profile
        .cases
        .iter()
        .find(|case| case.mutation == WorldMutationKind::GarbageCollection && case.phase == FaultPhase::Uninterrupted)
        .expect("garbage-collection case")
        .clone();
    let mut torn = passing_observation(&case);
    torn.read_back.record_ref = None;
    assert_eq!(compare_world_fault_observation(&case, &torn).disposition, ConformanceDisposition::Failed);

    let mut duplicate_or_uncertain = passing_observation(&case);
    duplicate_or_uncertain.submission = SubmissionObservation::PossiblySubmitted;
    duplicate_or_uncertain.owner_decision = RecoveryClass::SafeToRetry;
    assert_eq!(
        compare_world_fault_observation(&case, &duplicate_or_uncertain).disposition,
        ConformanceDisposition::Failed
    );

    let mut stale = passing_observation(&case);
    stale.owner_decision = RecoveryClass::Superseded;
    assert_eq!(compare_world_fault_observation(&case, &stale).disposition, ConformanceDisposition::Failed);

    let mut missing = passing_observation(&case);
    missing.read_back.status = DurableReadBackStatus::Missing;
    assert_eq!(compare_world_fault_observation(&case, &missing).disposition, ConformanceDisposition::Failed);

    let mut corrupt = passing_observation(&case);
    corrupt.read_back.status = DurableReadBackStatus::Corrupt;
    assert_eq!(compare_world_fault_observation(&case, &corrupt).disposition, ConformanceDisposition::Failed);

    let mut cleanup = passing_observation(&case);
    cleanup.cleanup_authorized = true;
    assert_eq!(compare_world_fault_observation(&case, &cleanup).disposition, ConformanceDisposition::Failed);

    let mut contradictory = passing_observation(&case);
    contradictory.read_back.status = DurableReadBackStatus::Contradictory;
    assert_eq!(compare_world_fault_observation(&case, &contradictory).disposition, ConformanceDisposition::Failed);
}

// r[verify molten.world_faults.recovery]
#[test]
fn transactional_reconciliation_decisions_map_without_a_test_owned_state_machine() {
    let binding = transactional_binding();
    let unknown = transactional_reconciliation_core::classify_commit(
        binding,
        transactional_reconciliation_core::CommitObservation::OutcomeUnknown,
    );
    assert_eq!(classify_transactional_recovery(unknown), RecoveryClass::Uncertain);
    let missing = transactional_reconciliation_core::reconcile_read_back(
        unknown,
        transactional_reconciliation_core::ReadBackObservation::Missing,
    )
    .expect("missing read-back classification");
    assert_eq!(classify_transactional_recovery(missing), RecoveryClass::ManualReview);
    let applied = transactional_reconciliation_core::classify_commit(
        binding,
        transactional_reconciliation_core::CommitObservation::Applied(binding),
    );
    assert_eq!(classify_transactional_recovery(applied), RecoveryClass::AlreadyComplete);
}

fn passing_observations(profile: &WorldFaultProfile) -> Vec<WorldOperationObservation> {
    profile
        .cases
        .iter()
        .filter(|case| case.mutation != WorldMutationKind::Witness)
        .map(passing_observation)
        .collect()
}

fn passing_observation(case: &WorldFaultCase) -> WorldOperationObservation {
    let complete = case.expected_decision == RecoveryClass::AlreadyComplete;
    let before_submit = case.phase == FaultPhase::BeforeSubmit;
    WorldOperationObservation {
        case_id: case.case_id.clone(),
        operation_id: case.operation_id.clone(),
        phase: case.phase,
        submission: if before_submit {
            SubmissionObservation::NotSubmitted
        } else if complete {
            SubmissionObservation::DurablySubmitted
        } else {
            SubmissionObservation::PossiblySubmitted
        },
        response: if case.phase == FaultPhase::LostResponse {
            ResponseObservation::Lost
        } else if complete {
            ResponseObservation::Received
        } else {
            ResponseObservation::NotExpected
        },
        read_back: DurableReadBack {
            status: if complete {
                DurableReadBackStatus::Applied
            } else if before_submit {
                DurableReadBackStatus::Prior
            } else {
                DurableReadBackStatus::Missing
            },
            state_ref: if complete || before_submit {
                Some(reference(&format!("{}:state", case.case_id)))
            } else {
                None
            },
            record_ref: if complete {
                Some(reference(&format!("{}:record", case.case_id)))
            } else {
                None
            },
            observed_generation: if complete || before_submit {
                Some(case.expected_generation)
            } else {
                None
            },
            independent_witness: false,
        },
        owner_decision: case.expected_decision,
        whole_store_rollback: false,
        cleanup_authorized: false,
    }
}

fn passing_schedules(profile: &WorldFaultProfile) -> Vec<ConcurrentScheduleResult> {
    profile
        .schedules
        .iter()
        .map(|schedule| {
            if schedule.mutation == WorldMutationKind::Witness {
                return evaluate_concurrent_schedule(schedule, Vec::new(), all_schedule_choices(schedule));
            }
            let operations = schedule_operations(schedule);
            let release_count =
                u32::from(matches!(schedule.mutation, WorldMutationKind::Promotion | WorldMutationKind::Outbox));
            evaluate_concurrent_schedule(
                schedule,
                vec![
                    concurrent_observation(schedule, &operations[0], ConcurrentOutcome::Applied, release_count),
                    concurrent_observation(schedule, &operations[1], ConcurrentOutcome::Stale, 0),
                ],
                all_schedule_choices(schedule),
            )
        })
        .collect()
}

fn schedule_operations(schedule: &ConcurrentSchedule) -> Vec<String> {
    let mut operations = schedule.steps.iter().map(|step| step.operation_id.clone()).collect::<Vec<_>>();
    operations.sort();
    operations.dedup();
    assert_eq!(operations.len(), WORLD_FAULT_CONTENDER_COUNT);
    operations
}

fn concurrent_observation(
    schedule: &ConcurrentSchedule,
    operation_id: &str,
    outcome: ConcurrentOutcome,
    effect_release_count: u32,
) -> ConcurrentOperationObservation {
    let step = schedule.steps.iter().find(|step| step.operation_id == operation_id).expect("schedule operation");
    ConcurrentOperationObservation {
        operation_id: operation_id.to_string(),
        mutation: schedule.mutation,
        expected_generation: step.expected_generation,
        pre_state_ref: step.pre_state_ref.clone(),
        outcome,
        effect_release_count,
    }
}

fn all_schedule_choices(schedule: &ConcurrentSchedule) -> Vec<crate::fabric_simulation::EligibleChoice> {
    let mut choices = Vec::new();
    for position in [
        WORLD_FAULT_SCHEDULE_FIRST_POSITION,
        WORLD_FAULT_SCHEDULE_SECOND_POSITION,
        WORLD_FAULT_SCHEDULE_THIRD_POSITION,
        WORLD_FAULT_SCHEDULE_FOURTH_POSITION,
    ] {
        choices.extend(eligible_world_fault_choices(schedule, position));
    }
    choices
}

fn scheduler_world() -> AdmittedSimulatedWorld {
    let identity = ExtensionCoreIdentity {
        implementation_ref: reference("implementation"),
        manifest_ref: reference("manifest"),
        callback_dispatcher_ref: reference("dispatcher"),
        protocol_core_ref: reference("protocol"),
        state_machine_ref: reference("state-machine"),
        schema_set_ref: reference("schema-set"),
        port_contract_set_ref: reference("port-contract-set"),
    };
    let node = |node_id: &str| SimulatedNode {
        node_id: node_id.to_string(),
        extension_id: format!("world-fault-{node_id}"),
        service_id: "world-fault-conformance".to_string(),
        generation: WORLD_FAULT_NODE_GENERATION,
        initial_state_ref: reference(&format!("{node_id}:state")),
        membership_view_ref: reference(&format!("{node_id}:membership")),
        placement_ref: reference(&format!("{node_id}:placement")),
        consistency_profile_ref: reference(&format!("{node_id}:consistency")),
        same_core: SameCoreWitness {
            simulation: identity.clone(),
            live: identity.clone(),
        },
        required_port_classes: Vec::new(),
    };
    AdmittedSimulatedWorld {
        manifest: SimulatedWorldManifest {
            schema: crate::fabric_simulation::FABRIC_SIMULATION_WORLD_SCHEMA.to_string(),
            runtime_ref: reference("runtime"),
            scheduler_input_ref: reference("scheduler-input"),
            entropy_input_ref: reference("entropy-input"),
            authority_ref: reference("authority"),
            policy_ref: reference("policy"),
            initial_durable_state_ref: reference("durable-state"),
            resource_profile_ref: reference("resource-profile"),
            workload_ref: reference("workload"),
            fault_plan_ref: reference("fault-plan"),
            invariant_set_ref: reference("invariants"),
            nodes: vec![node("node-a"), node("node-b")],
            port_profiles: Vec::new(),
            workload: Vec::new(),
            faults: Vec::new(),
            invariants: Vec::new(),
            bounds: SimulationBounds {
                max_choices: TEST_MAX_CHOICES,
                max_events: TEST_MAX_EVENTS,
                max_virtual_ticks: TEST_MAX_TICKS,
                max_trace_bytes: TEST_MAX_TRACE_BYTES,
                max_resource_units: TEST_MAX_RESOURCE_UNITS,
                max_shrink_attempts: TEST_MAX_SHRINK_ATTEMPTS,
            },
            claim_profile: SimulationClaimProfile::PureModel,
            non_claims: Vec::new(),
            ambient_inputs: Vec::new(),
        },
    }
}

struct TestBlake3;

impl transactional_reconciliation_core::Blake3IdentityDeriver for TestBlake3 {
    fn derive_identity(
        &self,
        framed_bytes: &[u8],
    ) -> Result<transactional_reconciliation_core::Identity, transactional_reconciliation_core::CoreError> {
        transactional_reconciliation_core::Identity::new(*blake3::hash(framed_bytes).as_bytes())
    }
}

fn transactional_identity(
    byte: u8,
) -> Result<transactional_reconciliation_core::Identity, transactional_reconciliation_core::CoreError> {
    transactional_reconciliation_core::Identity::new([byte; TEST_DIGEST_BYTES])
}

fn transactional_binding() -> transactional_reconciliation_core::PersistenceBinding {
    let desired = transactional_identity(TEST_IDENTITY_BYTE).expect("desired identity");
    let policy = transactional_identity(TEST_SECOND_IDENTITY_BYTE).expect("policy identity");
    let operation = transactional_reconciliation_core::OperationDraft::new(
        transactional_identity(TEST_THIRD_IDENTITY_BYTE).expect("operation identity"),
        transactional_identity(TEST_FOURTH_IDENTITY_BYTE).expect("idempotency identity"),
        transactional_reconciliation_core::Generation::observed(TEST_GENERATION),
        transactional_identity(TEST_FIFTH_IDENTITY_BYTE).expect("operation content identity"),
    );
    let plan = transactional_reconciliation_core::build_plan(
        &TestBlake3,
        transactional_reconciliation_core::Limits::new(
            transactional_reconciliation_core::Bound::new(TEST_OPERATION_BOUND).expect("operation bound"),
            transactional_reconciliation_core::Bound::new(TEST_PREREQUISITE_BOUND).expect("prerequisite bound"),
        ),
        transactional_reconciliation_core::PlanningInput::new(
            transactional_reconciliation_core::Revision::observed(TEST_REVISION),
            desired,
            policy,
            Vec::new(),
            vec![operation],
        ),
    )
    .expect("transactional plan");
    let current = transactional_reconciliation_core::CurrentFacts::new(
        transactional_reconciliation_core::Revision::observed(TEST_REVISION),
        desired,
        policy,
        Vec::new(),
    );
    let operation_identity = plan.operations()[0].idempotency_identity();
    let reservation = transactional_reconciliation_core::reserve_attempt(
        &plan,
        operation_identity,
        transactional_identity(TEST_SIXTH_IDENTITY_BYTE).expect("attempt identity"),
    )
    .expect("attempt reservation");
    let publication =
        transactional_reconciliation_core::plan_publication(&plan, &current, desired).expect("publication intent");
    transactional_reconciliation_core::PersistenceBinding::new(publication, reservation, desired)
        .expect("persistence binding")
}
