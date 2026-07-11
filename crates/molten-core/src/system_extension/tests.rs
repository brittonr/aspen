use super::*;
use crate::fabric::DeterminismClass;
use crate::fabric::ExtensionTier;
use crate::fabric::ExtensionTierAdmission;
use crate::fabric::ExtensionTierRequest;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricNonClaim;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricPortIssue;
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
const SHUTDOWN_GRACE_TICKS: u64 = 20;
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
        max_concurrent_callbacks: 1,
        max_queued_events: 1,
        max_inflight_bytes: MAX_INFLIGHT_BYTES_FIXTURE,
        max_open_streams: 1,
        max_timers: 1,
        max_effect_requests: 2,
        callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
        shutdown_grace_ticks: SHUTDOWN_GRACE_TICKS,
        max_restart_attempts: 1,
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

fn manifest_input() -> SystemExtensionManifestInput {
    SystemExtensionManifestInput {
        schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: "molten.fixture.echo".to_string(),
        service_id: "molten.fixture.echo.service".to_string(),
        implementation_ref: HASH_A.to_string(),
        callback_groups: vec![
            "initialize".to_string(),
            "start".to_string(),
            "request".to_string(),
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
        execution_profile: ExecutionProfile::SandboxedComponent,
        state_schema: "molten.fixture.echo.state.v1".to_string(),
        compatible_state_schemas: vec!["molten.fixture.echo.state.v1".to_string()],
        evidence_profile_ref: HASH_A.to_string(),
        initial_generation: INITIAL_SYSTEM_EXTENSION_GENERATION,
        non_claims: REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    }
}

fn admitted_manifest() -> AdmittedSystemExtensionManifest {
    let registry = build_fabric_port_registry(&[descriptor()]).expect("valid fixture registry");
    let tier = tier_admission();
    admit_system_extension_manifest(&manifest_input(), SystemExtensionAdmissionContext {
        registry: &registry,
        tier_admission: &tier,
        admitted_execution_profiles: &[ExecutionProfile::SandboxedComponent],
    })
    .expect("valid fixture manifest")
}

fn transition(state: &LifecycleState, kind: LifecycleEventKind) -> LifecycleState {
    plan_lifecycle_transition(
        state,
        &LifecycleEvent::simple(kind, INITIAL_SYSTEM_EXTENSION_GENERATION),
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect("valid fixture transition")
}

fn running_state() -> LifecycleState {
    let installed = plan_lifecycle_transition(
        &LifecycleState::absent(),
        &LifecycleEvent::simple(LifecycleEventKind::Install, INITIAL_SYSTEM_EXTENSION_GENERATION),
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

fn request_event() -> CallbackEvent {
    CallbackEvent {
        callback: CallbackKind::Request,
        generation: INITIAL_SYSTEM_EXTENSION_GENERATION,
        event_ref: HASH_B.to_string(),
        payload_ref: Some(HASH_C.to_string()),
        accounted_bytes: REQUEST_BYTES,
        logical_tick: REQUEST_LOGICAL_TICK,
        deadline_tick: Some(REQUEST_DEADLINE_TICK),
        cancellation_requested: false,
    }
}

// r[verify molten.system_extension.manifest]
#[test]
fn manifest_admission_binds_exact_ports_and_profile() {
    let admitted = admitted_manifest();

    assert_eq!(admitted.execution_profile, ExecutionProfile::SandboxedComponent);
    assert_eq!(admitted.required_port_bindings.len(), 1);
    assert_eq!(admitted.required_port_bindings[0].key, FabricPortKey {
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
    });
    assert!(admitted.declares_callback(CallbackKind::Request));
}

// r[verify molten.system_extension.manifest]
#[test]
fn plugin_metadata_and_unadmitted_profiles_cannot_activate_as_system_extensions() {
    let registry = build_fabric_port_registry(&[descriptor()]).expect("valid fixture registry");
    let mut input = manifest_input();
    input.schema = "molten.plugin.manifest.v1".to_string();
    input.execution_profile = ExecutionProfile::InProcessNative;
    let forged_plugin_admission = ExtensionTierAdmission {
        tier: ExtensionTier::SandboxedPlugin,
        admitted_authorities: vec![FabricAuthority::Transport],
        supporting_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    };

    let issues = admit_system_extension_manifest(&input, SystemExtensionAdmissionContext {
        registry: &registry,
        tier_admission: &forged_plugin_admission,
        admitted_execution_profiles: &[ExecutionProfile::SandboxedComponent],
    })
    .expect_err("plugin metadata and fallback profile must deny");

    assert!(issues.iter().any(|issue| matches!(issue, ManifestIssue::SchemaMismatch { .. })));
    assert!(issues.contains(&ManifestIssue::WrongExtensionTier(ExtensionTier::SandboxedPlugin)));
    assert!(issues.contains(&ManifestIssue::ExecutionProfileNotAdmitted(ExecutionProfile::InProcessNative)));
}

// r[verify molten.system_extension.manifest]
#[test]
fn manifest_rejects_unknown_callbacks_missing_lifecycle_and_missing_nonclaims() {
    let registry = build_fabric_port_registry(&[descriptor()]).expect("valid fixture registry");
    let tier = tier_admission();
    let mut input = manifest_input();
    input.callback_groups.push("ambient-loop".to_string());
    input.callback_groups.retain(|callback| callback != "shutdown");
    input
        .non_claims
        .retain(|non_claim| *non_claim != SystemExtensionNonClaim::CallbackSuccessIsNotDurability);

    let issues = admit_system_extension_manifest(&input, SystemExtensionAdmissionContext {
        registry: &registry,
        tier_admission: &tier,
        admitted_execution_profiles: &[ExecutionProfile::SandboxedComponent],
    })
    .expect_err("unknown callback and incomplete authority boundary must deny");

    assert!(issues.contains(&ManifestIssue::UnknownCallback("ambient-loop".to_string())));
    assert!(issues.contains(&ManifestIssue::MissingLifecycleCallback(CallbackKind::Shutdown)));
    assert!(issues.contains(&ManifestIssue::MissingNonClaim(SystemExtensionNonClaim::CallbackSuccessIsNotDurability)));
}

// r[verify molten.system_extension.lifecycle]
#[test]
fn lifecycle_is_explicit_and_rejects_delayed_old_generation_work() {
    let running = running_state();
    assert_eq!(running.phase, LifecyclePhase::Running);
    assert_eq!(running.health, HealthState::Healthy);

    let upgrade = LifecycleEvent {
        kind: LifecycleEventKind::BeginUpgrade,
        generation: running.generation,
        next_generation: Some(running.generation + 1),
        checkpoint_ref: None,
        failure_class: None,
    };
    let upgrading =
        plan_lifecycle_transition(&running, &upgrade, ResourceUsage::default(), resources().max_restart_attempts)
            .expect("sequential generation upgrade");
    assert_eq!(upgrading.generation, running.generation + 1);

    let mut delayed = request_event();
    delayed.generation = running.generation;
    let issues = plan_callback_dispatch(&admitted_manifest(), &upgrading, ResourceUsage::default(), &delayed, 0)
        .expect_err("old generation callback must deny before invocation");
    assert!(issues.contains(&DispatchIssue::StaleGeneration {
        actual: running.generation,
        active: upgrading.generation,
    }));

    let mut exhausted = running;
    exhausted.generation = u64::MAX;
    let overflow = plan_lifecycle_transition(
        &exhausted,
        &LifecycleEvent {
            kind: LifecycleEventKind::BeginUpgrade,
            generation: u64::MAX,
            next_generation: Some(0),
            checkpoint_ref: None,
            failure_class: None,
        },
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect_err("generation overflow must deny replacement");
    assert!(overflow.contains(&LifecycleIssue::GenerationOverflow));
}

// r[verify molten.system_extension.callbacks]
#[test]
fn callback_cancellation_deadline_and_byte_pressure_deny_before_invocation() {
    let manifest = admitted_manifest();
    let state = running_state();
    let mut undeclared_manifest = manifest.clone();
    undeclared_manifest.callbacks.retain(|callback| *callback != CallbackKind::Request);
    let undeclared_issues =
        plan_callback_dispatch(&undeclared_manifest, &state, ResourceUsage::default(), &request_event(), 0)
            .expect_err("undeclared callback must deny");
    assert!(undeclared_issues.contains(&DispatchIssue::CallbackNotDeclared(CallbackKind::Request)));

    let mut cancelled = request_event();
    cancelled.cancellation_requested = true;
    let cancelled_issues = plan_callback_dispatch(&manifest, &state, ResourceUsage::default(), &cancelled, 0)
        .expect_err("cancelled callback must deny");
    assert!(cancelled_issues.contains(&DispatchIssue::CancellationRequested));

    let mut expired = request_event();
    expired.deadline_tick = Some(REQUEST_LOGICAL_TICK - 1);
    let expired_issues = plan_callback_dispatch(&manifest, &state, ResourceUsage::default(), &expired, 0)
        .expect_err("expired callback must deny");
    assert!(expired_issues.iter().any(|issue| matches!(issue, DispatchIssue::DeadlineExpired { .. })));

    let mut oversized = request_event();
    oversized.accounted_bytes = manifest.resources.max_inflight_bytes + 1;
    let pressure_issues = plan_callback_dispatch(&manifest, &state, ResourceUsage::default(), &oversized, 0)
        .expect_err("byte pressure must deny");
    assert!(pressure_issues.iter().any(|issue| matches!(
        issue,
        DispatchIssue::Resource(ResourceIssue::UsageExceedsEnvelope {
            field: "inflight-bytes",
            ..
        })
    )));
}

// r[verify molten.system_extension.callbacks]
// r[verify molten.system_extension.typed_effects]
#[test]
fn admitted_callback_produces_only_validated_typed_port_effects() {
    let manifest = admitted_manifest();
    let state = running_state();
    let plan = plan_callback_dispatch(&manifest, &state, ResourceUsage::default(), &request_event(), 0)
        .expect("request callback admitted");
    let invocation = plan.invocation.expect("scheduled callback has invocation");
    let effect = TypedEffectRequest {
        target: EffectTarget::FabricPort(FabricPortKey {
            port_id: PORT_ID.to_string(),
            version: PORT_VERSION.to_string(),
        }),
        operation: OPERATION.to_string(),
        input_schema_ref: INPUT_SCHEMA.to_string(),
        output_schema_ref: OUTPUT_SCHEMA.to_string(),
        request_ref: HASH_D.to_string(),
        generation: state.generation,
        accounted_bytes: REQUEST_BYTES,
    };
    let outcome = CallbackOutcome {
        output_refs: vec![HASH_A.to_string()],
        effects: vec![effect],
        state_ref: Some(HASH_B.to_string()),
        checkpoint_ref: None,
        health: HealthState::Healthy,
    };

    assert!(validate_callback_outcome(&manifest, &invocation, &outcome).is_empty());
}

// r[verify molten.system_extension.typed_effects]
#[test]
fn ambient_and_unbound_effects_are_denied() {
    let manifest = admitted_manifest();
    let ambient = TypedEffectRequest {
        target: EffectTarget::Ambient(AmbientEffect::Filesystem),
        operation: OPERATION.to_string(),
        input_schema_ref: INPUT_SCHEMA.to_string(),
        output_schema_ref: OUTPUT_SCHEMA.to_string(),
        request_ref: HASH_A.to_string(),
        generation: INITIAL_SYSTEM_EXTENSION_GENERATION,
        accounted_bytes: REQUEST_BYTES,
    };
    let mut unbound = ambient.clone();
    unbound.target = EffectTarget::FabricPort(FabricPortKey {
        port_id: "molten.fabric.unbound".to_string(),
        version: PORT_VERSION.to_string(),
    });
    unbound.request_ref = HASH_B.to_string();

    let issues = validate_typed_effects(&manifest, INITIAL_SYSTEM_EXTENSION_GENERATION, &[ambient, unbound]);

    assert!(issues.contains(&EffectIssue::AmbientEffectDenied(AmbientEffect::Filesystem)));
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, EffectIssue::PortNotBound(key) if key.port_id == "molten.fabric.unbound"))
    );
}

// r[verify molten.system_extension.backpressure]
#[test]
fn overload_policy_distinguishes_reject_delay_and_upstream_backpressure() {
    let usage = ResourceUsage {
        concurrent_callbacks: resources().max_concurrent_callbacks,
        ..ResourceUsage::default()
    };
    let mut envelope = resources();
    let rejected = plan_resource_admission(&envelope, usage, REQUEST_BYTES).expect("bounded rejection");
    assert_eq!(rejected.decision, AdmissionDecision::Reject);

    envelope.overload_policy = OverloadPolicy::Delay;
    let queued = plan_resource_admission(&envelope, usage, REQUEST_BYTES).expect("bounded queue");
    assert_eq!(queued.decision, AdmissionDecision::Queue);
    assert_eq!(queued.next_usage.queued_events, 1);

    envelope.overload_policy = OverloadPolicy::UpstreamBackpressure;
    let backpressured = plan_resource_admission(&envelope, usage, REQUEST_BYTES).expect("bounded backpressure");
    assert_eq!(backpressured.decision, AdmissionDecision::Backpressure);
    assert_eq!(backpressured.next_usage, usage);
}

// r[verify molten.system_extension.manifest]
#[test]
fn manifest_rejects_incompatible_required_port_without_version_fallback() {
    let registry = build_fabric_port_registry(&[descriptor()]).expect("valid fixture registry");
    let tier = tier_admission();
    let mut input = manifest_input();
    input.required_ports[0].version = "v2".to_string();

    let issues = admit_system_extension_manifest(&input, SystemExtensionAdmissionContext {
        registry: &registry,
        tier_admission: &tier,
        admitted_execution_profiles: &[ExecutionProfile::SandboxedComponent],
    })
    .expect_err("incompatible required port must deny without fallback");

    assert!(issues.iter().any(|issue| matches!(
        issue,
        ManifestIssue::RequiredPortDenied { issues, .. }
            if issues.iter().any(|port_issue| matches!(
                port_issue,
                FabricPortIssue::UnsupportedVersion { requested, .. } if requested == "v2"
            ))
    )));
}

// r[verify molten.system_extension.lifecycle]
#[test]
fn failed_checkpoint_and_recovery_take_explicit_failure_paths() {
    let running = running_state();
    let checkpointing = transition(&running, LifecycleEventKind::BeginCheckpoint);
    let checkpoint_failed = plan_lifecycle_transition(
        &checkpointing,
        &LifecycleEvent {
            kind: LifecycleEventKind::CheckpointFailed,
            generation: checkpointing.generation,
            next_generation: None,
            checkpoint_ref: None,
            failure_class: Some(FailureClass::Retryable),
        },
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect("retryable checkpoint failure is explicit");
    assert_eq!(checkpoint_failed.phase, LifecyclePhase::Failed);

    let recovering = transition(&checkpoint_failed, LifecycleEventKind::BeginRecovery);
    let recovery_failed = plan_lifecycle_transition(
        &recovering,
        &LifecycleEvent {
            kind: LifecycleEventKind::RecoveryFailed,
            generation: recovering.generation,
            next_generation: None,
            checkpoint_ref: None,
            failure_class: Some(FailureClass::Fatal),
        },
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect("fatal recovery failure is explicit");
    assert_eq!(recovery_failed.phase, LifecyclePhase::Quarantined);

    let illegal = plan_lifecycle_transition(
        &LifecycleState::absent(),
        &LifecycleEvent::simple(LifecycleEventKind::BeginStart, 0),
        ResourceUsage::default(),
        resources().max_restart_attempts,
    )
    .expect_err("illegal transition must deny");
    assert!(illegal.contains(&LifecycleIssue::IllegalTransition {
        phase: LifecyclePhase::Absent,
        event: LifecycleEventKind::BeginStart,
    }));
}

// r[verify molten.system_extension.lifecycle]
#[test]
fn drain_completion_rejects_live_resources() {
    let running = running_state();
    let draining = transition(&running, LifecycleEventKind::BeginDrain);
    let usage = ResourceUsage {
        concurrent_callbacks: 1,
        ..ResourceUsage::default()
    };
    let issues = plan_lifecycle_transition(
        &draining,
        &LifecycleEvent::simple(LifecycleEventKind::DrainSucceeded, draining.generation),
        usage,
        resources().max_restart_attempts,
    )
    .expect_err("drain with live callback must deny");
    assert!(issues.contains(&LifecycleIssue::ResourcesNotDrained(usage)));
}

// r[verify molten.system_extension.supervision]
#[test]
fn retry_budget_is_bounded_and_fatal_failures_quarantine() {
    assert_eq!(plan_supervision(FailureClass::Retryable, 0, 1), SupervisionDecision::Restart);
    assert_eq!(plan_supervision(FailureClass::Retryable, 1, 1), SupervisionDecision::Quarantine);
    assert_eq!(plan_supervision(FailureClass::Fatal, 0, 1), SupervisionDecision::Quarantine);

    let running = running_state();
    let event = LifecycleEvent {
        kind: LifecycleEventKind::Failure,
        generation: running.generation,
        next_generation: None,
        checkpoint_ref: None,
        failure_class: Some(FailureClass::Fatal),
    };
    let failed =
        plan_lifecycle_transition(&running, &event, ResourceUsage::default(), resources().max_restart_attempts)
            .expect("fatal failure has explicit quarantine transition");
    assert_eq!(failed.phase, LifecyclePhase::Quarantined);
}

// r[verify molten.system_extension.final_validation]
#[test]
fn executable_conformance_requires_observed_invocations_not_receipts_alone() {
    let callbacks = vec![CallbackKind::Initialize, CallbackKind::Start, CallbackKind::Request];
    let positive = ExecutableConformanceInput {
        execution_profile: ExecutionProfile::SandboxedComponent,
        required_callbacks: callbacks.clone(),
        observations: callbacks
            .iter()
            .map(|callback| CallbackObservation {
                callback: *callback,
                invocation_count: 1,
            })
            .collect(),
        execution_binding_refs: vec![HASH_A.to_string()],
    };
    assert!(validate_executable_conformance(&positive).is_empty());

    let receipt_only = ExecutableConformanceInput {
        execution_profile: ExecutionProfile::SandboxedComponent,
        required_callbacks: callbacks,
        observations: Vec::new(),
        execution_binding_refs: vec![HASH_B.to_string()],
    };
    let issues = validate_executable_conformance(&receipt_only);
    assert!(issues.contains(&ExecutableConformanceIssue::MissingInvocation(CallbackKind::Initialize)));
    assert!(issues.contains(&ExecutableConformanceIssue::MissingInvocation(CallbackKind::Start)));
    assert!(issues.contains(&ExecutableConformanceIssue::MissingInvocation(CallbackKind::Request)));
}

// r[verify molten.system_extension.lifecycle]
#[test]
fn state_migration_requires_explicit_compatible_source_and_current_target() {
    let manifest = admitted_manifest();
    let plan = plan_state_migration(&manifest, &manifest.state_schema, &manifest.state_schema)
        .expect("self-compatible state migration");
    assert_eq!(plan.source_schema, manifest.state_schema);
    assert_eq!(plan.target_schema, manifest.state_schema);

    let issues = plan_state_migration(&manifest, "molten.fixture.echo.state.v0", "molten.fixture.echo.state.v2")
        .expect_err("unknown source and wrong target must deny");
    assert!(
        issues.contains(&StateMigrationIssue::SourceSchemaNotCompatible("molten.fixture.echo.state.v0".to_string()))
    );
    assert!(issues.contains(&StateMigrationIssue::TargetSchemaMismatch {
        actual: "molten.fixture.echo.state.v2".to_string(),
        expected: manifest.state_schema,
    }));
}

// r[verify molten.system_extension.backpressure]
#[test]
fn stream_and_timer_counters_are_bounded_and_fail_closed_on_underflow() {
    let envelope = resources();
    let stream = reserve_stream(&envelope, ResourceUsage::default()).expect("first stream admitted");
    assert_eq!(stream.open_streams, 1);
    assert!(reserve_stream(&envelope, stream).is_err());
    assert_eq!(release_stream(stream).expect("stream release").open_streams, 0);
    assert!(release_stream(ResourceUsage::default()).is_err());

    let timer = reserve_timer(&envelope, ResourceUsage::default()).expect("first timer admitted");
    assert_eq!(timer.timers, 1);
    assert!(reserve_timer(&envelope, timer).is_err());
    assert_eq!(release_timer(timer).expect("timer release").timers, 0);
    assert!(release_timer(ResourceUsage::default()).is_err());
}

#[test]
fn fabric_non_claims_remain_distinct_from_extension_non_claims() {
    assert!(REQUIRED_FABRIC_NON_CLAIMS.contains(&FabricNonClaim::ExtensionSemanticCorrectness));
    assert!(
        REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS
            .contains(&SystemExtensionNonClaim::CallbackSuccessIsNotSemanticCorrectness)
    );
}
