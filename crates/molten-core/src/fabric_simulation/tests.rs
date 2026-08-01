use super::*;
use crate::fabric::FabricPortClass;
use crate::fabric::ReferenceSystemKind;

const TEST_DIGEST_HEX_LENGTH: usize = 64;
const TEST_MAX_CHOICES: u64 = 64;
const TEST_MAX_EVENTS: u64 = 64;
const TEST_MAX_TICKS: u64 = 1_024;
const TEST_MAX_TRACE_BYTES: u64 = 65_536;
const TEST_MAX_RESOURCE_UNITS: u64 = 1_024;
const TEST_MAX_SHRINK_ATTEMPTS: u64 = 64;
const TEST_GENERATION: u64 = 1;
const NEXT_GENERATION: u64 = 2;
const FIRST_READY_TICK: u64 = 3;
const SECOND_READY_TICK: u64 = 5;
const EXPECTED_NODE_COUNT: usize = 3;
const EXPECTED_PORT_COUNT: usize = REQUIRED_SIMULATION_PORT_CLASS_COUNT;
const EXPECTED_WORKLOAD_COUNT: usize = 3;
const EXPECTED_FAULT_KIND_COUNT: usize = 17;
const FIRST_STATE_VERSION: u64 = 1;
const FIRST_LOG_OFFSET: u64 = 0;
const FIRST_REPLICATED_OFFSET: u64 = 0;

fn content_ref(character: char) -> String {
    format!("blake3:{}", character.to_string().repeat(TEST_DIGEST_HEX_LENGTH))
}

fn core_identity() -> ExtensionCoreIdentity {
    ExtensionCoreIdentity {
        implementation_ref: content_ref('a'),
        manifest_ref: content_ref('b'),
        callback_dispatcher_ref: content_ref('c'),
        protocol_core_ref: content_ref('d'),
        state_machine_ref: content_ref('e'),
        schema_set_ref: content_ref('f'),
        port_contract_set_ref: content_ref('1'),
    }
}

fn node(node_id: &str, service_id: &str) -> SimulatedNode {
    let identity = core_identity();
    SimulatedNode {
        node_id: node_id.to_string(),
        extension_id: format!("molten.reference.{node_id}"),
        service_id: service_id.to_string(),
        generation: TEST_GENERATION,
        initial_state_ref: content_ref('2'),
        membership_view_ref: content_ref('3'),
        placement_ref: content_ref('4'),
        consistency_profile_ref: content_ref('5'),
        same_core: SameCoreWitness {
            simulation: identity.clone(),
            live: identity,
        },
        required_port_classes: vec![
            FabricPortClass::DurableState,
            FabricPortClass::Transport,
            FabricPortClass::Scheduling,
        ],
    }
}

fn port_profile(class: FabricPortClass) -> SimulatedPortProfile {
    SimulatedPortProfile {
        class,
        port_id: format!("molten.fabric.simulation.{}", class.as_str()),
        version: FABRIC_SIMULATION_PORT_VERSION.to_string(),
        implementation_profile: FABRIC_SIMULATION_PROFILE_ID.to_string(),
        descriptor_ref: content_ref('6'),
        command_schema_ref: format!("molten.fabric.simulation.{}.command.v1", class.as_str()),
        event_schema_ref: format!("molten.fabric.simulation.{}.event.v1", class.as_str()),
        deterministic: true,
        declared_faults: vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::Drop,
            SimulationFaultKind::Crash,
            SimulationFaultKind::Restart,
            SimulationFaultKind::CapacityExhaustion,
        ],
    }
}

fn valid_world() -> SimulatedWorldManifest {
    let nodes = vec![
        node("node-a", "molten.reference.transactional-key-value"),
        node("node-b", "molten.reference.replicated-log"),
        node("node-c", "molten.reference.distributed-scheduler"),
    ];
    let workload = vec![
        SimulationWorkloadStep {
            sequence: 0,
            node_id: "node-a".to_string(),
            request_ref: content_ref('7'),
            service: ReferenceSystemKind::TransactionalKeyValue,
            expected_failure_class: None,
        },
        SimulationWorkloadStep {
            sequence: 1,
            node_id: "node-b".to_string(),
            request_ref: content_ref('8'),
            service: ReferenceSystemKind::ReplicatedLog,
            expected_failure_class: None,
        },
        SimulationWorkloadStep {
            sequence: 2,
            node_id: "node-c".to_string(),
            request_ref: content_ref('9'),
            service: ReferenceSystemKind::DistributedScheduler,
            expected_failure_class: None,
        },
    ];
    let mut invariants =
        REQUIRED_UNIVERSAL_INVARIANTS.into_iter().map(SimulationInvariant::Universal).collect::<Vec<_>>();
    invariants.extend([
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "transaction-version-monotonic".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::ReplicatedLog,
            invariant_id: "log-offsets-contiguous".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::DistributedScheduler,
            invariant_id: "single-authoritative-completion".to_string(),
        },
    ]);
    SimulatedWorldManifest {
        schema: FABRIC_SIMULATION_WORLD_SCHEMA.to_string(),
        runtime_ref: content_ref('a'),
        scheduler_input_ref: content_ref('b'),
        entropy_input_ref: content_ref('c'),
        authority_ref: content_ref('d'),
        policy_ref: content_ref('e'),
        initial_durable_state_ref: content_ref('f'),
        resource_profile_ref: content_ref('1'),
        workload_ref: content_ref('2'),
        fault_plan_ref: content_ref('3'),
        invariant_set_ref: content_ref('4'),
        nodes,
        port_profiles: REQUIRED_SIMULATION_PORT_CLASSES.into_iter().map(port_profile).collect(),
        workload,
        faults: vec![SimulationFaultAction {
            fault_id: "delay-node-a".to_string(),
            kind: SimulationFaultKind::Delay,
            target: "node-a".to_string(),
            boundary: FabricPortClass::Transport,
            activate_at_choice: 0,
            duration_choices: Some(1),
            resource_cost: UNIT_RESOURCE_COST,
            expected_observation: "transport-delay".to_string(),
            direct_extension_state_mutation: false,
        }],
        invariants,
        bounds: SimulationBounds {
            max_choices: TEST_MAX_CHOICES,
            max_events: TEST_MAX_EVENTS,
            max_virtual_ticks: TEST_MAX_TICKS,
            max_trace_bytes: TEST_MAX_TRACE_BYTES,
            max_resource_units: TEST_MAX_RESOURCE_UNITS,
            max_shrink_attempts: TEST_MAX_SHRINK_ATTEMPTS,
        },
        claim_profile: SimulationClaimProfile::DeterministicWholeSystem,
        non_claims: REQUIRED_SIMULATION_NON_CLAIMS.to_vec(),
        ambient_inputs: Vec::new(),
    }
}

#[test]
fn complete_world_admits_and_normalizes_every_required_port() {
    let mut world = valid_world();
    world.nodes.reverse();
    world.port_profiles.reverse();

    let admitted = admit_simulated_world(&world).expect("complete deterministic world");

    assert_eq!(admitted.manifest.nodes.len(), EXPECTED_NODE_COUNT);
    assert_eq!(admitted.manifest.port_profiles.len(), EXPECTED_PORT_COUNT);
    assert_eq!(admitted.manifest.workload.len(), EXPECTED_WORKLOAD_COUNT);
    assert_eq!(admitted.manifest.nodes[0].node_id, "node-a");
    assert_eq!(admitted.manifest.port_profiles[0].class, FabricPortClass::Authority);
    assert!(admitted.manifest.port_profiles.iter().all(|profile| profile.deterministic));
    assert!(admitted.manifest.ambient_inputs.is_empty());
}

#[test]
fn world_denies_ambient_input_missing_port_direct_mutation_and_claim_overreach() {
    let mut world = valid_world();
    world.ambient_inputs.push("std.env.HOME".to_string());
    world.port_profiles.retain(|profile| profile.class != FabricPortClass::Consistency);
    world.faults[0].direct_extension_state_mutation = true;
    world.claim_profile = SimulationClaimProfile::MultiProcessLive;

    let issues = admit_simulated_world(&world).expect_err("unsafe world must deny");

    assert!(issues.contains(&WorldIssue::AmbientInput("std.env.HOME".to_string())));
    assert!(issues.contains(&WorldIssue::MissingPortClass(FabricPortClass::Consistency)));
    assert!(issues.contains(&WorldIssue::DirectExtensionStateMutation("delay-node-a".to_string())));
    assert!(issues.contains(&WorldIssue::ClaimOverreach(SimulationClaimProfile::MultiProcessLive)));
}

#[test]
fn every_named_fault_admits_only_through_a_declared_port_boundary() {
    let mut world = valid_world();
    let faults = [
        SimulationFaultKind::Delay,
        SimulationFaultKind::Drop,
        SimulationFaultKind::Duplicate,
        SimulationFaultKind::Reorder,
        SimulationFaultKind::Partition,
        SimulationFaultKind::Reset,
        SimulationFaultKind::BoundedCorruption,
        SimulationFaultKind::CapacityExhaustion,
        SimulationFaultKind::Pause,
        SimulationFaultKind::Crash,
        SimulationFaultKind::Restart,
        SimulationFaultKind::ClockSkew,
        SimulationFaultKind::ClockJump,
        SimulationFaultKind::AuthorityRevocation,
        SimulationFaultKind::MembershipChange,
        SimulationFaultKind::PlacementReplacement,
        SimulationFaultKind::ConsistencyQuorumLoss,
    ];
    assert_eq!(faults.len(), EXPECTED_FAULT_KIND_COUNT);
    for profile in &mut world.port_profiles {
        profile.declared_faults = faults.to_vec();
    }
    world.faults = faults
        .into_iter()
        .enumerate()
        .map(|(index, kind)| {
            let boundary = fault_boundary(kind);
            let target = world
                .port_profiles
                .iter()
                .find(|profile| profile.class == boundary)
                .expect("fault boundary profile")
                .port_id
                .clone();
            SimulationFaultAction {
                fault_id: format!("fault-{}", kind.as_str()),
                kind,
                target,
                boundary,
                activate_at_choice: u64::try_from(index).expect("fault index"),
                duration_choices: Some(1),
                resource_cost: UNIT_RESOURCE_COST,
                expected_observation: format!("{}-observed", kind.as_str()),
                direct_extension_state_mutation: false,
            }
        })
        .collect();

    let admitted = admit_simulated_world(&world).expect("all named faults at declared boundaries");

    assert_eq!(admitted.manifest.faults.len(), EXPECTED_FAULT_KIND_COUNT);
    assert!(admitted.manifest.faults.iter().all(|fault| {
        admitted
            .manifest
            .port_profiles
            .iter()
            .any(|profile| profile.class == fault.boundary && profile.declared_faults.contains(&fault.kind))
    }));
}

#[test]
fn world_denies_same_core_drift_duplicate_node_unbounded_run_and_stale_generation() {
    let mut world = valid_world();
    world.nodes[0].same_core.live.protocol_core_ref = content_ref('0');
    world.nodes.push(world.nodes[0].clone());
    world.nodes[0].generation = 0;
    world.bounds.max_events = 0;

    let issues = admit_simulated_world(&world).expect_err("identity and bound failures must deny");

    assert!(issues.contains(&WorldIssue::SameCoreMismatch {
        node_id: "node-a".to_string(),
        field: "protocol-core-ref",
    }));
    assert!(issues.contains(&WorldIssue::DuplicateNode("node-a".to_string())));
    assert!(issues.contains(&WorldIssue::ZeroGeneration("node-a".to_string())));
    assert!(issues.contains(&WorldIssue::Unbounded("max-events")));
}

#[test]
fn scheduler_repeats_canonical_choice_and_replay_rejects_ineligible_choice() {
    let world = admit_simulated_world(&valid_world()).expect("world");
    let state = SimulationSchedulerState::default();
    let eligible = vec![
        EligibleChoice {
            kind: SchedulerChoiceKind::TimerFire,
            choice_id: "timer-b".to_string(),
            node_id: "node-b".to_string(),
            generation: TEST_GENERATION,
            ready_at_tick: SECOND_READY_TICK,
        },
        EligibleChoice {
            kind: SchedulerChoiceKind::Runnable,
            choice_id: "run-a".to_string(),
            node_id: "node-a".to_string(),
            generation: TEST_GENERATION,
            ready_at_tick: FIRST_READY_TICK,
        },
    ];

    let first = select_simulation_choice(&world, &state, &eligible, None).expect("first choice");
    let replay = select_simulation_choice(&world, &state, &eligible, Some(&first.record.selected.choice_id))
        .expect("recorded replay choice");
    let divergence = select_simulation_choice(&world, &state, &eligible, Some("missing-choice"))
        .expect_err("ineligible replay choice must stop");

    assert_eq!(first.record, replay.record);
    assert_eq!(first.record.selected.choice_id, "run-a");
    assert_eq!(first.next.virtual_tick, FIRST_READY_TICK);
    assert!(matches!(
        divergence,
        SimulationSchedulerIssue::RecordedChoiceNotEligible(ReplayDivergence { position: 0, .. })
    ));
}

#[test]
fn scheduler_denies_stale_generation_and_explicit_bounds() {
    let mut manifest = valid_world();
    manifest.bounds.max_choices = 1;
    let world = admit_simulated_world(&manifest).expect("bounded world");
    let eligible = [EligibleChoice {
        kind: SchedulerChoiceKind::Runnable,
        choice_id: "run-a".to_string(),
        node_id: "node-a".to_string(),
        generation: NEXT_GENERATION,
        ready_at_tick: FIRST_READY_TICK,
    }];

    let stale = select_simulation_choice(&world, &SimulationSchedulerState::default(), &eligible, None)
        .expect_err("stale generation must deny");

    assert!(matches!(
        stale,
        SimulationSchedulerIssue::StaleGeneration {
            node_id,
            expected: TEST_GENERATION,
            actual: NEXT_GENERATION,
        } if node_id == "node-a"
    ));
}

#[test]
fn invariant_evaluation_reports_the_first_exact_failure_boundary() {
    let invariants = vec![
        SimulationInvariant::Universal(UniversalInvariantKind::NoAmbientEffect),
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "transaction-version-monotonic".to_string(),
        },
    ];
    let observations = vec![
        observation(0, false, &["transaction-version-monotonic"]),
        observation(1, true, &["transaction-version-monotonic"]),
    ];

    let results = evaluate_invariants(&invariants, &observations);

    assert_eq!(results.len(), invariants.len());
    assert!(!results[0].passed);
    assert_eq!(results[0].first_failure_sequence, Some(1));
    assert!(results[1].passed);
    assert_eq!(results[1].first_failure_sequence, None);
}

#[test]
fn claim_ladder_denies_live_promotion_with_only_simulation_evidence() {
    let evidence = ClaimEvidence {
        profile: SimulationClaimProfile::DeterministicWholeSystem,
        implementation_ref: content_ref('a'),
        environment_ref: None,
        adapter_refs: vec![content_ref('b')],
        lifecycle_ref: None,
        fault_ref: None,
        operator_ref: None,
    };

    let denied = evaluate_claim_promotion(
        SimulationClaimProfile::DeterministicWholeSystem,
        SimulationClaimProfile::MultiProcessLive,
        &evidence,
    );

    assert!(!denied.admitted);
    assert!(denied.missing_evidence.contains(&"profile-specific-evidence"));
    assert!(denied.missing_evidence.contains(&"environment-evidence"));
    assert!(denied.missing_evidence.contains(&"lifecycle-evidence"));
    assert!(denied.missing_evidence.contains(&"operator-evidence"));
}

#[test]
fn shrinker_removes_only_valid_failure_preserving_suffixes() {
    let world = valid_world();

    let shrunk = shrink_simulation_failure(&world, |candidate| {
        candidate.manifest.workload.iter().any(|step| step.node_id == "node-a")
    })
    .expect("causal suffix shrink");

    assert!(shrunk.failure_preserved);
    assert!(shrunk.removed_workload_steps > 0);
    assert_eq!(shrunk.world.workload.len(), 1);
    assert_eq!(shrunk.world.workload[0].node_id, "node-a");
    assert!(admit_simulated_world(&shrunk.world).is_ok());
}

#[test]
fn transactional_reference_core_applies_commit_and_preserves_state_on_conflict() {
    let state = initial_reference_state(ReferenceSystemKind::TransactionalKeyValue);
    let commit = ReferenceServiceOperation::TransactionalKeyValue(TransactionalKeyValueOperation::Commit {
        expected_version: initial_transaction_version(),
        writes: vec![("key-a".to_string(), content_ref('a'))],
    });
    let applied = apply_reference_operation(&state, &commit).expect("first commit");
    let conflict = ReferenceServiceOperation::TransactionalKeyValue(TransactionalKeyValueOperation::Commit {
        expected_version: initial_transaction_version(),
        writes: vec![("key-b".to_string(), content_ref('b'))],
    });
    let denied = apply_reference_operation(&applied.next, &conflict).expect("conflict is an explicit outcome");

    assert_eq!(applied.decision, ReferenceTransitionDecision::Applied);
    assert_eq!(denied.decision, ReferenceTransitionDecision::Conflict);
    assert_eq!(denied.next, applied.next);
    let ReferenceServiceState::TransactionalKeyValue(applied_state) = applied.next else {
        panic!("transaction state expected");
    };
    assert_eq!(applied_state.version, FIRST_STATE_VERSION);
    assert_eq!(applied_state.values.get("key-a"), Some(&content_ref('a')));
}

#[test]
fn replicated_log_reference_core_enforces_offsets_and_retention_boundary() {
    let state = initial_reference_state(ReferenceSystemKind::ReplicatedLog);
    let append = ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::Append {
        payload_ref: content_ref('a'),
    });
    let appended = apply_reference_operation(&state, &append).expect("append");
    let invalid_retention = ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::RetainFrom {
        offset: FIRST_LOG_OFFSET,
    });
    let denied = apply_reference_operation(&appended.next, &invalid_retention)
        .expect_err("retention before replication must deny");
    let replicated = apply_reference_operation(
        &appended.next,
        &ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::ReplicateThrough {
            offset: FIRST_REPLICATED_OFFSET,
        }),
    )
    .expect("replication");
    let retained =
        apply_reference_operation(&replicated.next, &invalid_retention).expect("retention after replication");

    assert!(matches!(denied, ReferenceServiceIssue::RetentionBeyondReplication { .. }));
    assert_eq!(retained.decision, ReferenceTransitionDecision::Applied);
}

#[test]
fn distributed_scheduler_reference_core_denies_wrong_owner_and_double_completion() {
    let initial = initial_reference_state(ReferenceSystemKind::DistributedScheduler);
    let submitted = apply_reference_operation(
        &initial,
        &ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Submit {
            job_id: "job-a".to_string(),
        }),
    )
    .expect("submit");
    let leased = apply_reference_operation(
        &submitted.next,
        &ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Lease {
            job_id: "job-a".to_string(),
            owner: "worker-a".to_string(),
        }),
    )
    .expect("lease");
    let wrong_owner = apply_reference_operation(
        &leased.next,
        &ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Complete {
            job_id: "job-a".to_string(),
            owner: "worker-b".to_string(),
            completion_ref: content_ref('a'),
        }),
    )
    .expect_err("wrong owner must deny");
    let completed = apply_reference_operation(
        &leased.next,
        &ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Complete {
            job_id: "job-a".to_string(),
            owner: "worker-a".to_string(),
            completion_ref: content_ref('a'),
        }),
    )
    .expect("authoritative completion");
    let duplicate = apply_reference_operation(
        &completed.next,
        &ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Complete {
            job_id: "job-a".to_string(),
            owner: "worker-a".to_string(),
            completion_ref: content_ref('b'),
        }),
    )
    .expect_err("double authoritative completion must deny");

    assert!(matches!(wrong_owner, ReferenceServiceIssue::LeaseOwnerMismatch { .. }));
    assert!(matches!(duplicate, ReferenceServiceIssue::DuplicateAuthoritativeCompletion(job) if job == "job-a"));
}

#[test]
fn replay_comparison_identifies_choice_and_length_divergence() {
    let choice = EligibleChoice {
        kind: SchedulerChoiceKind::Runnable,
        choice_id: "run-a".to_string(),
        node_id: "node-a".to_string(),
        generation: TEST_GENERATION,
        ready_at_tick: FIRST_READY_TICK,
    };
    let expected = [SchedulerChoiceRecord {
        position: 0,
        virtual_tick: FIRST_READY_TICK,
        eligible: vec![choice.clone()],
        selected: choice.clone(),
    }];
    let matching = compare_replay(&expected, &expected);
    let shorter = compare_replay(&expected, &[]);

    assert!(matching.matches);
    assert!(matching.first_divergence.is_none());
    assert!(!shorter.matches);
    assert_eq!(shorter.first_divergence.as_ref().map(|item| item.position), Some(0));
}

fn fault_boundary(kind: SimulationFaultKind) -> FabricPortClass {
    match kind {
        SimulationFaultKind::Delay
        | SimulationFaultKind::Drop
        | SimulationFaultKind::Duplicate
        | SimulationFaultKind::Reorder
        | SimulationFaultKind::Partition
        | SimulationFaultKind::Reset => FabricPortClass::Transport,
        SimulationFaultKind::BoundedCorruption
        | SimulationFaultKind::CapacityExhaustion
        | SimulationFaultKind::Crash => FabricPortClass::DurableState,
        SimulationFaultKind::Pause | SimulationFaultKind::Restart => FabricPortClass::Supervision,
        SimulationFaultKind::ClockSkew | SimulationFaultKind::ClockJump => FabricPortClass::Time,
        SimulationFaultKind::AuthorityRevocation => FabricPortClass::Authority,
        SimulationFaultKind::MembershipChange => FabricPortClass::Membership,
        SimulationFaultKind::PlacementReplacement => FabricPortClass::Placement,
        SimulationFaultKind::ConsistencyQuorumLoss => FabricPortClass::Consistency,
    }
}

fn observation(sequence: u64, ambient_effect: bool, semantic_invariants: &[&str]) -> SimulationObservation {
    SimulationObservation {
        sequence,
        node_id: "node-a".to_string(),
        service: Some(ReferenceSystemKind::TransactionalKeyValue),
        generation: TEST_GENERATION,
        state_ref: content_ref('a'),
        history_ref: content_ref('b'),
        port_event_ref: content_ref('c'),
        ambient_effect,
        stale_generation_mutation: false,
        resource_bound_bypass: false,
        port_state_machine_violation: false,
        terminal_cleanup_complete: true,
        semantic_invariants_passed: semantic_invariants.iter().map(|value| (*value).to_string()).collect(),
    }
}
