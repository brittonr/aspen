// Restart-fencing conformance trace and durable readmission fixtures.
//
// The trace installs and starts instance A, creates runtime-local work
// addressed to A, restarts the instance as B without changing the
// implementation generation, and then delivers every delayed item. The
// verdict names each delivery that the replacement instance would accept.
//
// r[verify molten.system_extension.incarnation_fencing]
// r[verify molten.system_extension.durable_readmission]
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
const CALLBACK_DEADLINE_TICKS: u64 = 10;
const MAX_INFLIGHT_BYTES_FIXTURE: u64 = 4_096;
const REQUEST_BYTES: u64 = 32;
const REQUEST_LOGICAL_TICK: u64 = 4;
const REQUEST_DEADLINE_TICK: u64 = 8;

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

fn resources() -> ResourceEnvelope {
    ResourceEnvelope {
        max_concurrent_callbacks: 4,
        max_queued_events: 4,
        max_inflight_bytes: MAX_INFLIGHT_BYTES_FIXTURE,
        max_open_streams: 1,
        max_timers: 4,
        max_effect_requests: 2,
        callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
        shutdown_grace_ticks: 20,
        max_restart_attempts: 2,
        overload_policy: OverloadPolicy::Reject,
    }
}

fn tier_admission() -> ExtensionTierAdmission {
    validate_extension_tier(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![
            FabricAuthority::Transport,
            FabricAuthority::Resources,
            FabricAuthority::Supervision,
            FabricAuthority::Evidence,
        ],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .expect("valid system-extension tier admission")
}

fn admitted_manifest() -> AdmittedSystemExtensionManifest {
    let registry = build_fabric_port_registry(&[descriptor()]).expect("valid fixture registry");
    let tier = tier_admission();
    admit_system_extension_manifest(
        &SystemExtensionManifestInput {
            schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
            extension_id: "molten.fixture.echo".to_string(),
            service_id: "molten.fixture.echo.service".to_string(),
            implementation_ref: HASH_A.to_string(),
            callback_groups: vec![
                "initialize".to_string(),
                "start".to_string(),
                "request".to_string(),
                "message".to_string(),
                "timer".to_string(),
                "health".to_string(),
                "checkpoint".to_string(),
                "recover".to_string(),
                "drain".to_string(),
                "shutdown".to_string(),
            ],
            required_ports: vec![requirement()],
            optional_ports: Vec::new(),
            capability_refs: vec![HASH_B.to_string()],
            policy_refs: vec![HASH_C.to_string()],
            provenance_refs: vec![HASH_D.to_string()],
            resources: resources(),
            recovery_group: None,
            execution_profile: ExecutionProfile::SandboxedComponent,
            state_schema: "molten.fixture.echo.state.v1".to_string(),
            compatible_state_schemas: vec!["molten.fixture.echo.state.v1".to_string()],
            evidence_profile_ref: HASH_A.to_string(),
            initial_generation: INITIAL_SYSTEM_EXTENSION_GENERATION,
            non_claims: REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
        },
        SystemExtensionAdmissionContext {
            registry: &registry,
            tier_admission: &tier,
            admitted_execution_profiles: &[ExecutionProfile::SandboxedComponent],
        },
    )
    .expect("valid fixture manifest")
}

fn transition(state: &LifecycleState, kind: LifecycleEventKind) -> LifecycleState {
    plan_lifecycle_transition(
        state,
        &LifecycleEvent::simple(kind, state.generation, state.incarnation),
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect("valid fixture transition")
}

fn failure_event(state: &LifecycleState) -> LifecycleEvent {
    LifecycleEvent {
        kind: LifecycleEventKind::Failure,
        generation: state.generation,
        incarnation: state.incarnation,
        next_generation: None,
        checkpoint_ref: None,
        failure_class: Some(FailureClass::Retryable),
    }
}

fn running_instance() -> LifecycleState {
    let installed = plan_lifecycle_transition(
        &LifecycleState::absent(),
        &LifecycleEvent::simple(LifecycleEventKind::Install, INITIAL_SYSTEM_EXTENSION_GENERATION, 0),
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect("install");
    let admitted = transition(&installed, LifecycleEventKind::Admit);
    let initializing = transition(&admitted, LifecycleEventKind::BeginInitialize);
    let initialized = transition(&initializing, LifecycleEventKind::InitializeSucceeded);
    let starting = transition(&initialized, LifecycleEventKind::BeginStart);
    transition(&starting, LifecycleEventKind::StartSucceeded)
}

fn restarted_instance(state: &LifecycleState) -> LifecycleState {
    let failed = plan_lifecycle_transition(
        state,
        &failure_event(state),
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect("instance failure");
    let restarting = transition(&failed, LifecycleEventKind::BeginRestart);
    let starting = transition(&restarting, LifecycleEventKind::BeginStart);
    transition(&starting, LifecycleEventKind::StartSucceeded)
}

fn timer_event(state: &LifecycleState) -> CallbackEvent {
    CallbackEvent {
        callback: CallbackKind::Timer,
        generation: state.generation,
        incarnation: state.incarnation,
        event_ref: HASH_B.to_string(),
        payload_ref: None,
        accounted_bytes: REQUEST_BYTES,
        logical_tick: REQUEST_LOGICAL_TICK,
        deadline_tick: Some(REQUEST_DEADLINE_TICK),
        cancellation_requested: false,
    }
}

fn request_event(state: &LifecycleState) -> CallbackEvent {
    CallbackEvent {
        callback: CallbackKind::Request,
        generation: state.generation,
        incarnation: state.incarnation,
        event_ref: HASH_B.to_string(),
        payload_ref: Some(HASH_C.to_string()),
        accounted_bytes: REQUEST_BYTES,
        logical_tick: REQUEST_LOGICAL_TICK,
        deadline_tick: Some(REQUEST_DEADLINE_TICK),
        cancellation_requested: false,
    }
}

fn effect(state: &LifecycleState) -> TypedEffectRequest {
    TypedEffectRequest {
        target: EffectTarget::FabricPort(FabricPortKey {
            port_id: PORT_ID.to_string(),
            version: PORT_VERSION.to_string(),
        }),
        operation: OPERATION.to_string(),
        input_schema_ref: INPUT_SCHEMA.to_string(),
        output_schema_ref: OUTPUT_SCHEMA.to_string(),
        request_ref: HASH_D.to_string(),
        generation: state.generation,
        incarnation: state.incarnation,
        accounted_bytes: REQUEST_BYTES,
    }
}

/// The verdict of one restart-fencing conformance trace.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RestartFencingVerdict {
    timer_issues: Vec<DispatchIssue>,
    completion_issues: Vec<CallbackOutcomeIssue>,
    failure_issues: Vec<LifecycleIssue>,
}

impl RestartFencingVerdict {
    fn accepted_deliveries(&self) -> Vec<&'static str> {
        let mut accepted = Vec::new();
        if self.timer_issues.is_empty() {
            accepted.push("delayed timer");
        }
        if self.completion_issues.is_empty() {
            accepted.push("callback completion");
        }
        if self.failure_issues.is_empty() {
            accepted.push("failure notification");
        }
        accepted
    }
}

fn restart_fencing_trace() -> RestartFencingVerdict {
    let manifest = admitted_manifest();
    let instance_a = running_instance();

    // Runtime-local work created under instance A.
    let timer = timer_event(&instance_a);
    let request = request_event(&instance_a);
    let invocation = plan_callback_dispatch(&manifest, &instance_a, ResourceUsage::default(), &request, 0)
        .expect("request admitted for instance A")
        .invocation
        .expect("request invocation for instance A");
    let completion = CallbackOutcome {
        output_refs: vec![HASH_C.to_string()],
        effects: vec![effect(&instance_a)],
        state_ref: Some(HASH_D.to_string()),
        checkpoint_ref: None,
        health: HealthState::Healthy,
    };

    // Instance A fails and restarts as instance B without a new generation.
    let instance_b = restarted_instance(&instance_a);
    assert_eq!(instance_b.generation, instance_a.generation);
    assert!(instance_b.incarnation > instance_a.incarnation);

    // Every delayed item is delivered after the replacement instance runs.
    let timer_issues = plan_callback_dispatch(&manifest, &instance_b, ResourceUsage::default(), &timer, 1)
        .expect_err("the delayed timer must be rejected");
    let completion_issues =
        validate_callback_outcome(&manifest, instance_b.generation, instance_b.incarnation, &invocation, &completion);
    let failure_issues = plan_lifecycle_transition(
        &instance_b,
        &failure_event(&instance_a),
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect_err("the failure notification must be rejected");

    RestartFencingVerdict {
        timer_issues,
        completion_issues,
        failure_issues,
    }
}

#[test]
fn restart_fencing_trace_rejects_every_delayed_runtime_local_delivery() {
    let verdict = restart_fencing_trace();
    let accepted = verdict.accepted_deliveries();
    assert!(
        accepted.is_empty(),
        "the replacement instance accepted stale runtime-local deliveries: {accepted:?}"
    );
    assert!(verdict.timer_issues.contains(&DispatchIssue::StaleIncarnation { actual: 0, active: 1 }));
    assert!(verdict.completion_issues.contains(&CallbackOutcomeIssue::StaleIncarnation { actual: 0, active: 1 }));
    assert!(verdict.failure_issues.contains(&LifecycleIssue::StaleIncarnation { actual: 0, active: 1 }));
}

#[test]
fn restart_entry_advances_the_incarnation_and_preserves_the_generation() {
    let instance_a = running_instance();
    assert_eq!(instance_a.incarnation, 0);

    let instance_b = restarted_instance(&instance_a);
    assert_eq!(instance_b.generation, instance_a.generation);
    assert_eq!(instance_b.incarnation, instance_a.incarnation + 1);

    let recovering = transition(&instance_b, LifecycleEventKind::BeginDrain);
    assert_eq!(recovering.incarnation, instance_b.incarnation);
}

#[test]
fn durable_work_is_readmitted_under_a_fresh_claim_after_restart() {
    let instance_a = running_instance();
    let instance_b = restarted_instance(&instance_a);
    let durable = WorkPresentation {
        work_ref: HASH_C.to_string(),
        class: WorkClass::Durable,
        generation: instance_a.generation,
        incarnation: instance_a.incarnation,
    };
    let claim = DeliveryClaim {
        claim_ref: HASH_D.to_string(),
        class: WorkClass::Durable,
        generation: instance_b.generation,
        incarnation: instance_b.incarnation,
    };

    let admission = admit_work_delivery(&instance_b, &durable, Some(&claim)).expect("durable readmission");
    assert_eq!(admission.incarnation, instance_b.incarnation);
    assert_eq!(admission.claim_ref.as_deref(), Some(HASH_D));
}

#[test]
fn old_callback_presented_as_durable_work_is_rejected() {
    let instance_a = running_instance();
    let instance_b = restarted_instance(&instance_a);
    let old_callback = WorkPresentation {
        work_ref: HASH_C.to_string(),
        class: WorkClass::RuntimeLocal,
        generation: instance_a.generation,
        incarnation: instance_a.incarnation,
    };
    let claim = DeliveryClaim {
        claim_ref: HASH_D.to_string(),
        class: WorkClass::Durable,
        generation: instance_b.generation,
        incarnation: instance_b.incarnation,
    };

    let issues = admit_work_delivery(&instance_b, &old_callback, Some(&claim))
        .expect_err("runtime-local work must not be readmitted as durable work");
    assert!(issues.contains(&ReadmissionIssue::ClassMismatch {
        presented: WorkClass::RuntimeLocal,
        claimed: WorkClass::Durable,
    }));
}

#[test]
fn durable_work_presented_as_a_live_callback_is_rejected() {
    let instance_a = running_instance();
    let instance_b = restarted_instance(&instance_a);
    let durable = WorkPresentation {
        work_ref: HASH_C.to_string(),
        class: WorkClass::Durable,
        generation: instance_a.generation,
        incarnation: instance_a.incarnation,
    };

    let issues = admit_work_delivery(&instance_b, &durable, None).expect_err("durable work needs a fresh claim");
    assert!(issues.contains(&ReadmissionIssue::MissingClaim));

    let stale_claim = DeliveryClaim {
        claim_ref: HASH_D.to_string(),
        class: WorkClass::Durable,
        generation: instance_b.generation,
        incarnation: instance_a.incarnation,
    };
    let stale_issues = admit_work_delivery(&instance_b, &durable, Some(&stale_claim))
        .expect_err("a claim from the previous incarnation must be rejected");
    assert!(stale_issues.contains(&ReadmissionIssue::StaleIncarnation {
        actual: instance_a.incarnation,
        active: instance_b.incarnation,
    }));
}

#[test]
fn future_incarnation_work_is_rejected() {
    let instance = running_instance();
    let mut future_timer = timer_event(&instance);
    future_timer.incarnation = instance.incarnation + 1;
    let issues = plan_callback_dispatch(&admitted_manifest(), &instance, ResourceUsage::default(), &future_timer, 0)
        .expect_err("work from a future incarnation must be denied");
    assert!(issues.contains(&DispatchIssue::StaleIncarnation {
        actual: instance.incarnation + 1,
        active: instance.incarnation,
    }));

    let mut future_failure = failure_event(&instance);
    future_failure.incarnation = instance.incarnation + 1;
    let lifecycle_issues = plan_lifecycle_transition(
        &instance,
        &future_failure,
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect_err("a failure notification from a future incarnation must be denied");
    assert!(lifecycle_issues.contains(&LifecycleIssue::StaleIncarnation {
        actual: instance.incarnation + 1,
        active: instance.incarnation,
    }));
}
