use super::*;
use crate::fabric::DeterminismClass;
use crate::fabric::ExtensionTier;
use crate::fabric::ExtensionTierAdmission;
use crate::fabric::ExtensionTierRequest;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricPortKey;
use crate::fabric::FabricPortRequirement;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE;
use crate::fabric::ReplayClass;
use crate::fabric::build_fabric_port_registry;
use crate::fabric::validate_extension_tier;
use crate::system_extension::AdmittedSystemExtensionManifest;
use crate::system_extension::ExecutionProfile;
use crate::system_extension::INITIAL_SYSTEM_EXTENSION_GENERATION;
use crate::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS;
use crate::system_extension::ResourceEnvelope;
use crate::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA;
use crate::system_extension::SystemExtensionManifestInput;
use crate::system_extension::admit_system_extension_manifest;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PORT_ID: &str = "molten.fabric.transport.session";
const PORT_VERSION: &str = "v1";
const OPERATION: &str = "send-envelope";
const INPUT_SCHEMA: &str = "molten.transport.input.v1";
const OUTPUT_SCHEMA: &str = "molten.transport.output.v1";
const PROFILE: &str = "fixture-transport-v1";
const STATE_SCHEMA: &str = "molten.fixture.facade.state.v1";

fn descriptor() -> FabricPortDescriptor {
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
        class: FabricPortClass::Transport,
        operation_classes: vec![OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![FabricAuthority::Transport],
        resource_requirements: vec![FabricResource::Concurrency, FabricResource::NetworkBytes],
        determinism: DeterminismClass::ExternalEffect,
        replay: ReplayClass::RecordedEffectRequired,
        implementation_profile: PROFILE.to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn requirement() -> FabricPortRequirement {
    FabricPortRequirement {
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
        class: FabricPortClass::Transport,
        operation_classes: vec![OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        allowed_authorities: vec![FabricAuthority::Transport],
        available_resources: vec![FabricResource::Concurrency, FabricResource::NetworkBytes],
        expected_determinism: DeterminismClass::ExternalEffect,
        expected_replay: ReplayClass::RecordedEffectRequired,
        expected_profile: PROFILE.to_string(),
    }
}

fn extension_manifest() -> AdmittedSystemExtensionManifest {
    let registry = build_fabric_port_registry(&[descriptor()]).expect("valid fixture registry");
    let tier: ExtensionTierAdmission = validate_extension_tier(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![
            FabricAuthority::Transport,
            FabricAuthority::Resources,
            FabricAuthority::Supervision,
            FabricAuthority::Evidence,
        ],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .expect("valid tier admission");
    admit_system_extension_manifest(
        &SystemExtensionManifestInput {
            schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
            extension_id: "molten.fixture.facade".to_string(),
            service_id: "molten.fixture.facade.service".to_string(),
            implementation_ref: HASH_A.to_string(),
            callback_groups: vec![
                "initialize".to_string(),
                "start".to_string(),
                "request".to_string(),
                "timer".to_string(),
                "drain".to_string(),
                "shutdown".to_string(),
            ],
            required_ports: vec![requirement()],
            optional_ports: Vec::new(),
            capability_refs: vec![HASH_B.to_string()],
            policy_refs: vec![HASH_C.to_string()],
            provenance_refs: vec![HASH_D.to_string()],
            resources: ResourceEnvelope {
                max_concurrent_callbacks: 2,
                max_queued_events: 2,
                max_inflight_bytes: 4_096,
                max_open_streams: 1,
                max_timers: 2,
                max_effect_requests: 2,
                callback_deadline_ticks: 16,
                shutdown_grace_ticks: 32,
                max_restart_attempts: 2,
                overload_policy: crate::system_extension::OverloadPolicy::Reject,
            },
            recovery_group: None,
            execution_profile: ExecutionProfile::SandboxedComponent,
            state_schema: STATE_SCHEMA.to_string(),
            compatible_state_schemas: vec![STATE_SCHEMA.to_string()],
            evidence_profile_ref: HASH_A.to_string(),
            initial_generation: INITIAL_SYSTEM_EXTENSION_GENERATION,
            non_claims: REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
        },
        crate::system_extension::SystemExtensionAdmissionContext {
            registry: &registry,
            tier_admission: &tier,
            admitted_execution_profiles: &[ExecutionProfile::SandboxedComponent],
        },
    )
    .expect("valid fixture manifest")
}

fn phase(phase: &str, legal: &[&str], postponable: &[&str]) -> FacadePhaseEntry {
    FacadePhaseEntry {
        phase: phase.to_string(),
        legal_events: legal.iter().map(|event| (*event).to_string()).collect(),
        postponable_events: postponable.iter().map(|event| (*event).to_string()).collect(),
        postponement_bound: if postponable.is_empty() { None } else { Some(2) },
        state_timeout_ticks: None,
        timeout_fencing: None,
    }
}

fn facade_input(profile: FacadeProfile) -> FacadeManifestInput {
    FacadeManifestInput {
        schema: FACADE_MANIFEST_SCHEMA.to_string(),
        extension_manifest_ref: HASH_A.to_string(),
        service_id: "molten.fixture.facade.service".to_string(),
        profile,
        event_schema_ref: HASH_B.to_string(),
        state_schema_ref: HASH_C.to_string(),
        reply_schema_ref: HASH_D.to_string(),
        completion_schema_ref: (profile == FacadeProfile::Task).then(|| HASH_A.to_string()),
        phase_table: Vec::new(),
        concurrency_limit: 2,
    }
}

fn admitted_facade(profile: FacadeProfile) -> AdmittedFacadeManifest {
    let mut input = facade_input(profile);
    if profile == FacadeProfile::StateMachine {
        input.phase_table = vec![phase("idle", &["command"], &[])];
    }
    admit_facade_manifest(&input).expect("admitted facade manifest")
}

fn admitted_state_machine() -> AdmittedFacadeManifest {
    let mut input = facade_input(FacadeProfile::StateMachine);
    let mut idle = phase("idle", &["start"], &["configure"]);
    idle.state_timeout_ticks = Some(30);
    idle.timeout_fencing = Some(TimeoutFencing::IncarnationAndPhase);
    let running = phase("running", &["command"], &["configure"]);
    input.phase_table = vec![idle, running];
    admit_facade_manifest(&input).expect("admitted state machine facade")
}

fn runtime_state(phase: &str) -> FacadeRuntimeState {
    FacadeRuntimeState {
        generation: INITIAL_SYSTEM_EXTENSION_GENERATION,
        incarnation: 0,
        phase: phase.to_string(),
        state_ref: Some(HASH_C.to_string()),
        turn_sequence: 1,
        postponed: Vec::new(),
        completed: false,
    }
}

fn event(kind: &str, sequence: u64) -> FacadeEvent {
    FacadeEvent {
        kind: kind.to_string(),
        payload_ref: Some(HASH_B.to_string()),
        logical_tick: 10,
        expected_turn_sequence: sequence,
    }
}

#[test]
fn each_profile_admits_a_minimal_manifest() {
    for profile in [FacadeProfile::Service, FacadeProfile::StateMachine, FacadeProfile::Task] {
        let admitted = admitted_facade(profile);
        assert_eq!(admitted.profile, profile);
        assert_eq!(admitted.concurrency_limit, 2);
    }
    let state_machine = admitted_state_machine();
    assert_eq!(state_machine.phase_table.len(), 2);
}

#[test]
fn admission_rejects_illegal_tables_unbounded_postponement_and_unfenced_timers() {
    let mut input = facade_input(FacadeProfile::StateMachine);
    let mut unbounded_phase = phase("idle", &[], &["configure"]);
    unbounded_phase.postponement_bound = None;
    input.phase_table = vec![unbounded_phase];
    let unbounded = admit_facade_manifest(&input).expect_err("unbounded postponement must deny");
    assert!(unbounded.contains(&FacadeAdmissionIssue::UnboundedPostponement {
        phase: "idle".to_string()
    }));

    let mut input = facade_input(FacadeProfile::StateMachine);
    let mut unfenced = phase("idle", &[], &[]);
    unfenced.state_timeout_ticks = Some(30);
    input.phase_table = vec![unfenced];
    let issues = admit_facade_manifest(&input).expect_err("an unfenced state timeout must deny");
    assert!(issues.contains(&FacadeAdmissionIssue::UnfencedStateTimeout {
        phase: "idle".to_string()
    }));

    let mut input = facade_input(FacadeProfile::Service);
    input.phase_table = vec![phase("idle", &["start"], &[])];
    let issues = admit_facade_manifest(&input).expect_err("a service profile must not carry a phase table");
    assert!(issues.contains(&FacadeAdmissionIssue::UnexpectedPhaseTable));

    let mut input = facade_input(FacadeProfile::Task);
    input.completion_schema_ref = None;
    let issues = admit_facade_manifest(&input).expect_err("a task profile needs a completion schema");
    assert!(issues.contains(&FacadeAdmissionIssue::MissingTaskCompletionSchema));

    let mut input = facade_input(FacadeProfile::Service);
    input.concurrency_limit = 0;
    let issues = admit_facade_manifest(&input).expect_err("a concurrency limit below one must deny");
    assert!(issues.contains(&FacadeAdmissionIssue::ConcurrencyLimitBelowOne));
}

#[test]
fn illegal_event_for_phase_is_rejected_and_the_state_does_not_change() {
    let facade = admitted_state_machine();
    let state = runtime_state("running");
    let issues = admit_facade_event(&facade, &state, &event("unknown", state.turn_sequence))
        .expect_err("an event outside the phase table must deny");
    assert!(issues.contains(&FacadeTurnIssue::IllegalEventForPhase {
        phase: "running".to_string(),
        event: "unknown".to_string(),
    }));
    assert_eq!(state.phase, "running");
    assert_eq!(state.turn_sequence, 1);
}

#[test]
fn stale_state_timer_cannot_transition_a_replacement() {
    let facade = admitted_state_machine();
    let state = runtime_state("idle");
    let earlier_phase = FacadeTimerDelivery {
        timer_ref: "timer-early-phase".to_string(),
        phase: "running".to_string(),
        incarnation: state.incarnation,
        logical_tick: 10,
    };
    let issues = admit_facade_timer(&facade, &state, &earlier_phase).expect_err("a timer from another phase must deny");
    assert!(issues.contains(&FacadeTurnIssue::StaleTimer {
        timer_ref: "timer-early-phase".to_string(),
        phase: "running".to_string(),
        incarnation: state.incarnation,
    }));

    let replacement = FacadeRuntimeState {
        incarnation: 1,
        ..state.clone()
    };
    let earlier_incarnation = FacadeTimerDelivery {
        timer_ref: "timer-early-incarnation".to_string(),
        phase: "idle".to_string(),
        incarnation: 0,
        logical_tick: 10,
    };
    let issues = admit_facade_timer(&facade, &replacement, &earlier_incarnation)
        .expect_err("a timer from an earlier incarnation must deny");
    assert!(issues.contains(&FacadeTurnIssue::StaleTimer {
        timer_ref: "timer-early-incarnation".to_string(),
        phase: "idle".to_string(),
        incarnation: 0,
    }));

    let current = FacadeTimerDelivery {
        timer_ref: "timer-current".to_string(),
        phase: "idle".to_string(),
        incarnation: state.incarnation,
        logical_tick: 10,
    };
    admit_facade_timer(&facade, &state, &current).expect("a current timer is admitted");
}

#[test]
fn postponed_event_and_stale_timer_are_rejected_together() {
    let facade = admitted_state_machine();
    let mut state = runtime_state("idle");
    state.postponed = vec![FacadePostponedEvent {
        kind: "configure".to_string(),
        payload_ref: Some(HASH_B.to_string()),
        logical_tick: 5,
    }];
    let admission = admit_facade_event(&facade, &state, &event("configure", state.turn_sequence))
        .expect("a postponable event is admitted while the bound holds");
    assert!(matches!(admission, FacadeEventAdmission::Postponed { .. }));

    let stale_timer = FacadeTimerDelivery {
        timer_ref: "timer-stale".to_string(),
        phase: "running".to_string(),
        incarnation: state.incarnation,
        logical_tick: 10,
    };
    let issues = admit_facade_timer(&facade, &state, &stale_timer).expect_err("the stale timer still denies");
    assert!(issues.contains(&FacadeTurnIssue::StaleTimer {
        timer_ref: "timer-stale".to_string(),
        phase: "running".to_string(),
        incarnation: state.incarnation,
    }));
}

#[test]
fn postponement_stays_bounded_and_dead_letters() {
    let facade = admitted_state_machine();
    let mut state = runtime_state("idle");
    state.postponed = vec![
        FacadePostponedEvent {
            kind: "configure".to_string(),
            payload_ref: None,
            logical_tick: 1,
        },
        FacadePostponedEvent {
            kind: "configure".to_string(),
            payload_ref: None,
            logical_tick: 2,
        },
    ];
    let admission = admit_facade_event(&facade, &state, &event("configure", state.turn_sequence))
        .expect("postponement beyond the bound dead-letters instead of extending the queue");
    assert_eq!(admission, FacadeEventAdmission::DeadLettered {
        dead_lettered: vec![FacadePostponedEvent {
            kind: "configure".to_string(),
            payload_ref: Some(HASH_B.to_string()),
            logical_tick: 10,
        }],
    });
}

#[test]
fn parallel_events_to_one_service_cannot_interleave_inside_one_turn() {
    let facade = admitted_facade(FacadeProfile::Service);
    let state = runtime_state("idle");
    let stale = event("command", state.turn_sequence - 1);
    let issues = admit_facade_event(&facade, &state, &stale).expect_err("a parallel event must not interleave");
    assert!(issues.contains(&FacadeTurnIssue::StaleTurn {
        actual: state.turn_sequence - 1,
        expected: state.turn_sequence,
    }));
}

#[test]
fn a_completed_task_stops_without_consuming_restart_budget() {
    let facade = admitted_facade(FacadeProfile::Task);
    let state = runtime_state("running");
    let model_turn = FacadeModelTurn {
        next_phase: "running".to_string(),
        next_state_ref: Some(HASH_D.to_string()),
        replies: vec![HASH_A.to_string()],
        effect_intents: Vec::new(),
        timer_requests: Vec::new(),
        completed: true,
    };
    let turn = lower_facade_turn(&extension_manifest(), &facade, &state, &model_turn).expect("a completed task lowers");
    assert!(turn.completed);
    assert_eq!(turn.next_phase, TASK_COMPLETED_PHASE);
    assert_eq!(task_completion_decision(&facade, true), Some(TASK_COMPLETED_PHASE));

    let completed_state = FacadeRuntimeState {
        completed: true,
        ..state
    };
    let issues = admit_facade_event(&facade, &completed_state, &event("command", completed_state.turn_sequence))
        .expect_err("a completed task admits no further event");
    assert!(issues.contains(&FacadeTurnIssue::TaskAlreadyCompleted));
}

#[test]
fn facade_lowered_turn_equals_a_hand_written_turn() {
    let facade = admitted_facade(FacadeProfile::Service);
    let state = runtime_state("idle");
    let model_turn = FacadeModelTurn {
        next_phase: "idle".to_string(),
        next_state_ref: Some(HASH_D.to_string()),
        replies: vec![HASH_A.to_string()],
        effect_intents: vec![FacadeEffectIntent {
            port_id: PORT_ID.to_string(),
            port_version: PORT_VERSION.to_string(),
            operation: OPERATION.to_string(),
            input_schema_ref: INPUT_SCHEMA.to_string(),
            output_schema_ref: OUTPUT_SCHEMA.to_string(),
            request_ref: HASH_C.to_string(),
            accounted_bytes: 32,
        }],
        timer_requests: Vec::new(),
        completed: false,
    };
    let turn =
        lower_facade_turn(&extension_manifest(), &facade, &state, &model_turn).expect("the facade lowers the turn");
    let lowered = lower_facade_outcome(&turn);

    let hand_written = crate::system_extension::CallbackOutcome {
        output_refs: vec![HASH_A.to_string()],
        effects: vec![crate::system_extension::TypedEffectRequest {
            target: crate::system_extension::EffectTarget::FabricPort(FabricPortKey {
                port_id: PORT_ID.to_string(),
                version: PORT_VERSION.to_string(),
            }),
            operation: OPERATION.to_string(),
            input_schema_ref: INPUT_SCHEMA.to_string(),
            output_schema_ref: OUTPUT_SCHEMA.to_string(),
            request_ref: HASH_C.to_string(),
            generation: state.generation,
            incarnation: state.incarnation,
            accounted_bytes: 32,
        }],
        state_ref: Some(HASH_D.to_string()),
        checkpoint_ref: None,
        health: crate::system_extension::HealthState::Healthy,
    };

    assert_eq!(lowered, hand_written);
}

fn running_lifecycle_state() -> crate::system_extension::LifecycleState {
    use crate::system_extension::LifecycleEventKind;
    use crate::system_extension::LifecyclePhase;
    let installed = crate::system_extension::plan_lifecycle_transition(
        &crate::system_extension::LifecycleState::absent(),
        &crate::system_extension::LifecycleEvent::simple(
            LifecycleEventKind::Install,
            INITIAL_SYSTEM_EXTENSION_GENERATION,
            0,
        ),
        crate::system_extension::ResourceUsage::default(),
        2,
    )
    .expect("install");
    let mut state = installed;
    for kind in [
        LifecycleEventKind::Admit,
        LifecycleEventKind::BeginInitialize,
        LifecycleEventKind::InitializeSucceeded,
        LifecycleEventKind::BeginStart,
        LifecycleEventKind::StartSucceeded,
    ] {
        state = crate::system_extension::plan_lifecycle_transition(
            &state,
            &crate::system_extension::LifecycleEvent::simple(kind, state.generation, state.incarnation),
            crate::system_extension::ResourceUsage::default(),
            2,
        )
        .expect("lifecycle transition");
    }
    assert_eq!(state.phase, LifecyclePhase::Running);
    state
}

#[test]
fn a_completed_task_maps_to_the_existing_stopped_transition() {
    let facade = admitted_facade(FacadeProfile::Task);
    let state = runtime_state("running");
    let mut lifecycle = running_lifecycle_state();
    let restart_attempts = lifecycle.restart_attempts;
    for event in task_completion_events(&state) {
        lifecycle = crate::system_extension::plan_lifecycle_transition(
            &lifecycle,
            &event,
            crate::system_extension::ResourceUsage::default(),
            2,
        )
        .expect("the task stop transition");
    }
    assert_eq!(lifecycle.phase, crate::system_extension::LifecyclePhase::Stopped);
    assert_eq!(lifecycle.restart_attempts, restart_attempts);
    assert_eq!(facade.profile, FacadeProfile::Task);
}

#[test]
fn the_state_timeout_is_fenced_by_phase_and_incarnation() {
    let facade = admitted_state_machine();
    let state = runtime_state("idle");
    let entry = facade.phase_entry("idle").expect("the idle phase");
    let intent = state_timeout_intent(entry, &state, 10).expect("a fenced state timeout");
    assert_eq!(intent.phase, "idle");
    assert_eq!(intent.incarnation, state.incarnation);
    assert_eq!(intent.deadline_tick, 40);

    let replacement = FacadeRuntimeState {
        incarnation: 1,
        ..state.clone()
    };
    let issues = admit_facade_timer(&facade, &replacement, &FacadeTimerDelivery {
        timer_ref: intent.timer_ref.clone(),
        phase: intent.phase.clone(),
        incarnation: intent.incarnation,
        logical_tick: intent.logical_tick,
    })
    .expect_err("a state timeout from the previous incarnation must deny");
    assert!(issues.contains(&FacadeTurnIssue::StaleTimer {
        timer_ref: intent.timer_ref,
        phase: "idle".to_string(),
        incarnation: state.incarnation,
    }));
}
