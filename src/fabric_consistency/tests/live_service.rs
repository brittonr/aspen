use std::collections::BTreeSet;
use std::time::Duration;

use molten_core::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;

use super::raft::*;
use super::*;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::ExtensionTier;
use crate::fabric::ExtensionTierRequest;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricPortRequirement;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE;
use crate::fabric::ReplayClass;
use crate::fabric::canonical_extension_tier_admission;
use crate::system_extension::CallbackInvocation;
use crate::system_extension::CallbackOutcome;
use crate::system_extension::ExecutionProfile;
use crate::system_extension::HealthState;
use crate::system_extension::OverloadPolicy;
use crate::system_extension::ResourceEnvelope;
use crate::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA;
use crate::system_extension::SystemExtensionExecutor;
use crate::system_extension::SystemExtensionHost;
use crate::system_extension::SystemExtensionManifestInput;
use crate::system_extension::canonical_admit_system_extension_manifest;

const TEST_PORT_PROFILE: &str = "live-replica-test-port-v1";
const TEST_PORT_OPERATION: &str = "operate";
const TEST_INPUT_SCHEMA: &str = "molten.fabric-consistency.live-test-input.v1";
const TEST_OUTPUT_SCHEMA: &str = "molten.fabric-consistency.live-test-output.v1";
const TEST_STATE_SCHEMA: &str = "molten.fabric-consistency.live-test-state.v1";
const NODE_A: &str = "node-a";
const NODE_B: &str = "node-b";
const NODE_C: &str = "node-c";
const HEARTBEAT_TICKS: u64 = 2;
const ELECTION_MIN_TICKS: u64 = 4;
const ELECTION_MAX_TICKS: u64 = 8;
const EFFECT_LIMIT: usize = 16;
const MAX_CONCURRENT_CALLBACKS: u64 = 4;
const MAX_QUEUED_EVENTS: u64 = 16;
const MAX_INFLIGHT_BYTES: u64 = 65_536;
const MAX_OPEN_STREAMS: u64 = 16;
const MAX_TIMERS: u64 = 16;
const MAX_EFFECT_REQUESTS: u64 = 16;
const CALLBACK_DEADLINE_TICKS: u64 = 64;
const SHUTDOWN_GRACE_TICKS: u64 = 16;
const MAX_RESTART_ATTEMPTS: u64 = 2;
const ACTIVATION_TICK: u64 = 1;
const SERVICE_EVENT_TIMEOUT_MILLISECONDS: u64 = 200;
const EXPECTED_STARTUP_OBSERVATIONS: usize = 2;
const EXPECTED_ELECTION_PORT_CALLS: usize = 2;

#[derive(Debug, Clone)]
struct TestPortSpec {
    port_id: &'static str,
    class: FabricPortClass,
    authorities: Vec<FabricAuthority>,
    resources: Vec<FabricResource>,
}

#[derive(Debug, Clone, Copy)]
struct TestExecutor;

impl SystemExtensionExecutor for TestExecutor {
    fn execution_profile(&self) -> ExecutionProfile {
        ExecutionProfile::InProcessNative
    }

    fn invoke(&mut self, _invocation: &CallbackInvocation) -> std::result::Result<CallbackOutcome, String> {
        Ok(CallbackOutcome {
            output_refs: vec![test_ref("callback-output")],
            effects: Vec::new(),
            state_ref: Some(test_ref("callback-state")),
            checkpoint_ref: None,
            health: HealthState::Healthy,
        })
    }
}

fn port_specs() -> Vec<TestPortSpec> {
    vec![
        TestPortSpec {
            port_id: crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID,
            class: FabricPortClass::Transport,
            authorities: vec![FabricAuthority::Transport, FabricAuthority::ProtocolOwnership],
            resources: vec![FabricResource::NetworkBytes, FabricResource::Concurrency],
        },
        TestPortSpec {
            port_id: crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID,
            class: FabricPortClass::DurableState,
            authorities: vec![FabricAuthority::DurableState],
            resources: vec![FabricResource::StorageBytes, FabricResource::QueueDepth],
        },
        TestPortSpec {
            port_id: crate::fabric_durability::FABRIC_SNAPSHOT_PORT_ID,
            class: FabricPortClass::DurableState,
            authorities: vec![FabricAuthority::DurableState],
            resources: vec![FabricResource::StorageBytes, FabricResource::QueueDepth],
        },
        TestPortSpec {
            port_id: crate::fabric_time::FABRIC_TIMER_PORT_ID,
            class: FabricPortClass::Time,
            authorities: vec![FabricAuthority::Time],
            resources: vec![FabricResource::LogicalTime],
        },
        TestPortSpec {
            port_id: crate::fabric_time::FABRIC_ENTROPY_PORT_ID,
            class: FabricPortClass::Time,
            authorities: vec![FabricAuthority::Time],
            resources: vec![FabricResource::Memory],
        },
        TestPortSpec {
            port_id: crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_ID,
            class: FabricPortClass::Membership,
            authorities: vec![FabricAuthority::Membership, FabricAuthority::Policy],
            resources: vec![FabricResource::Diagnostics],
        },
        TestPortSpec {
            port_id: crate::fabric_membership::FABRIC_PLACEMENT_PORT_ID,
            class: FabricPortClass::Placement,
            authorities: vec![
                FabricAuthority::Placement,
                FabricAuthority::Policy,
                FabricAuthority::Resources,
            ],
            resources: vec![FabricResource::Diagnostics],
        },
    ]
}

fn port_descriptor(spec: &TestPortSpec) -> FabricPortDescriptor {
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: spec.port_id.to_string(),
        version: "v1".to_string(),
        class: spec.class,
        operation_classes: vec![TEST_PORT_OPERATION.to_string()],
        input_schema_refs: vec![TEST_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![TEST_OUTPUT_SCHEMA.to_string()],
        authority_requirements: spec.authorities.clone(),
        resource_requirements: spec.resources.clone(),
        determinism: DeterminismClass::ExternalEffect,
        replay: ReplayClass::RecordedEffectRequired,
        implementation_profile: TEST_PORT_PROFILE.to_string(),
        conformance_refs: vec![test_ref(spec.port_id)],
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn port_requirement(spec: &TestPortSpec) -> FabricPortRequirement {
    FabricPortRequirement {
        port_id: spec.port_id.to_string(),
        version: "v1".to_string(),
        class: spec.class,
        operation_classes: vec![TEST_PORT_OPERATION.to_string()],
        input_schema_refs: vec![TEST_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![TEST_OUTPUT_SCHEMA.to_string()],
        allowed_authorities: spec.authorities.clone(),
        available_resources: spec.resources.clone(),
        expected_determinism: DeterminismClass::ExternalEffect,
        expected_replay: ReplayClass::RecordedEffectRequired,
        expected_profile: TEST_PORT_PROFILE.to_string(),
    }
}

fn host_without(omitted_port_id: Option<&str>) -> SystemExtensionHost<TestExecutor> {
    let specs = port_specs().into_iter().filter(|spec| Some(spec.port_id) != omitted_port_id).collect::<Vec<_>>();
    let descriptors = specs.iter().map(port_descriptor).collect::<Vec<_>>();
    let requirements = specs.iter().map(port_requirement).collect::<Vec<_>>();
    let tier = canonical_extension_tier_admission(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![
            FabricAuthority::ProtocolOwnership,
            FabricAuthority::Transport,
            FabricAuthority::DurableState,
            FabricAuthority::Time,
            FabricAuthority::Membership,
            FabricAuthority::Placement,
            FabricAuthority::Consistency,
            FabricAuthority::Supervision,
            FabricAuthority::Policy,
            FabricAuthority::Resources,
            FabricAuthority::Evidence,
        ],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .expect("system-extension tier");
    let admitted = canonical_admit_system_extension_manifest(
        &SystemExtensionManifestInput {
            schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
            extension_id: "extension-live-raft".to_string(),
            service_id: "service-live-raft".to_string(),
            implementation_ref: test_ref("implementation"),
            callback_groups: vec![
                "initialize".to_string(),
                "start".to_string(),
                "drain".to_string(),
                "shutdown".to_string(),
            ],
            required_ports: requirements,
            optional_ports: Vec::new(),
            capability_refs: vec![test_ref("capability")],
            policy_refs: vec![test_ref("policy")],
            provenance_refs: vec![test_ref("provenance")],
            resources: ResourceEnvelope {
                max_concurrent_callbacks: MAX_CONCURRENT_CALLBACKS,
                max_queued_events: MAX_QUEUED_EVENTS,
                max_inflight_bytes: MAX_INFLIGHT_BYTES,
                max_open_streams: MAX_OPEN_STREAMS,
                max_timers: MAX_TIMERS,
                max_effect_requests: MAX_EFFECT_REQUESTS,
                callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
                shutdown_grace_ticks: SHUTDOWN_GRACE_TICKS,
                max_restart_attempts: MAX_RESTART_ATTEMPTS,
                overload_policy: OverloadPolicy::UpstreamBackpressure,
            },
            execution_profile: ExecutionProfile::InProcessNative,
            state_schema: TEST_STATE_SCHEMA.to_string(),
            compatible_state_schemas: vec![TEST_STATE_SCHEMA.to_string()],
            evidence_profile_ref: test_ref("evidence-profile"),
            initial_generation: SERVICE_GENERATION,
            non_claims: crate::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
        },
        &descriptors,
        &tier,
        &[ExecutionProfile::InProcessNative],
    )
    .expect("admitted live Raft host manifest");
    SystemExtensionHost::new(admitted, TestExecutor).expect("live Raft host")
}

fn active_group_for_host(host: &SystemExtensionHost<TestExecutor>) -> ConsistencyGroupBinding {
    active_group_for_host_with_policies(host, host.manifest().manifest().policy_refs.clone())
}

fn active_group_for_host_with_policies(
    host: &SystemExtensionHost<TestExecutor>,
    policy_refs: Vec<String>,
) -> ConsistencyGroupBinding {
    let mut input = binding_input();
    input.group_id = "group:live-raft".to_string();
    input.extension_id = host.manifest().manifest().extension_id.clone();
    input.service_id = host.manifest().manifest().service_id.clone();
    input.application_manifest_ref = host.manifest().manifest_ref().to_string();
    input.engine_algorithm_profile = LIVE_RAFT_ALGORITHM_PROFILE.to_string();
    input.engine_implementation_profile = LIVE_RAFT_IMPLEMENTATION_PROFILE.to_string();
    input.policy_refs = policy_refs;
    let declared = canonical_consistency_group_binding(input).expect("declared live group");
    let plan = plan_consistency_operation(
        &declared,
        command_for(&declared, ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        }),
    )
    .expect("live group open plan");
    let outcome = normalized_success(&declared, &plan, ConsistencyOutcomeKind::Opened);
    apply_consistency_outcome(&declared, &plan, &outcome).expect("active live group")
}

fn profile(group: &ConsistencyGroupBinding) -> ReplicaProfile {
    ReplicaProfile {
        profile_ref: test_ref("replica-profile"),
        group_binding_ref: group.binding_ref.clone(),
        service_generation: group.service_generation,
        protocol_ref: test_ref("protocol"),
        durable_log_ref: test_ref("durable-log"),
        snapshot_store_ref: test_ref("snapshot-store"),
        timer_profile_ref: test_ref("timer-profile"),
        entropy_profile_ref: test_ref("entropy-profile"),
        placement_ref: group.placement_ref.clone(),
        fencing_ref: group.fencing_ref.clone(),
        fencing_epoch: group.fencing_epoch,
        supervision_ref: test_ref("supervision"),
        resource_profile_ref: group.resource_profile_ref.clone(),
        heartbeat_ticks: HEARTBEAT_TICKS,
        election_min_ticks: ELECTION_MIN_TICKS,
        election_max_ticks: ELECTION_MAX_TICKS,
        max_log_entries: MAX_REPLICA_LOG_ENTRIES,
        max_message_entries: MAX_REPLICA_MESSAGE_ENTRIES,
        max_effects_per_step: EFFECT_LIMIT,
    }
}

fn membership(group: &ConsistencyGroupBinding) -> StaticMembership {
    StaticMembership {
        membership_ref: group.membership_ref.clone(),
        config_epoch: group.config_epoch,
        voters: vec![NODE_A.to_string(), NODE_B.to_string(), NODE_C.to_string()],
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn running_host_projects_exact_required_live_replica_ports() {
    let mut host = host_without(None);
    host.activate(ACTIVATION_TICK).expect("active host");
    let group = active_group_for_host(&host);
    let plan =
        plan_live_replica_start_for_host(&host, group.clone(), NODE_A.to_string(), membership(&group), profile(&group))
            .expect("host-backed replica start plan");

    assert_eq!(plan.port_binding_refs.len(), REQUIRED_REPLICA_PORTS.len());
    assert!(!plan.production_admitted);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn host_start_denies_inactive_supervision_and_missing_durable_log() {
    let inactive = host_without(None);
    let inactive_group = active_group_for_host(&inactive);
    let inactive_error = plan_live_replica_start_for_host(
        &inactive,
        inactive_group.clone(),
        NODE_A.to_string(),
        membership(&inactive_group),
        profile(&inactive_group),
    )
    .expect_err("inactive host must deny");
    assert!(inactive_error.to_string().contains("running supervised"));

    let mut missing_log = host_without(Some(crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID));
    missing_log.activate(ACTIVATION_TICK).expect("active incomplete host");
    let missing_log_group = active_group_for_host(&missing_log);
    let missing_log_error = plan_live_replica_start_for_host(
        &missing_log,
        missing_log_group.clone(),
        NODE_A.to_string(),
        membership(&missing_log_group),
        profile(&missing_log_group),
    )
    .expect_err("missing durable log must deny");
    assert!(missing_log_error.to_string().contains(crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn host_start_denies_group_policy_not_admitted_by_manifest() {
    let mut host = host_without(None);
    host.activate(ACTIVATION_TICK).expect("active host");
    let mut policies = host.manifest().manifest().policy_refs.clone();
    policies.push(test_ref("unadmitted-policy"));
    let group = active_group_for_host_with_policies(&host, policies);
    let error =
        plan_live_replica_start_for_host(&host, group.clone(), NODE_A.to_string(), membership(&group), profile(&group))
            .expect_err("unadmitted group policy must deny");
    assert!(error.to_string().contains("policy refs are not admitted"));
}

#[derive(Debug, Default)]
struct ServiceDurabilityPort {
    hard_state_writes: usize,
}

impl ReplicaDurabilityEffects for ServiceDurabilityPort {
    fn persist_hard_state(&mut self, _term: u64, _voted_for: Option<&str>) -> Result<String> {
        self.hard_state_writes += 1;
        Ok(test_ref("service-hard-state"))
    }

    fn persist_entries(&mut self, _truncate_from: Option<u64>, _entries: &[ReplicatedEntry]) -> Result<String> {
        Ok(test_ref("service-entries"))
    }

    fn flush_log(&mut self, _through_index: u64) -> Result<String> {
        Ok(test_ref("service-flush"))
    }

    fn persist_commit(&mut self, _through_index: u64) -> Result<String> {
        Ok(test_ref("service-commit"))
    }

    fn persist_snapshot(&mut self, _snapshot: &ReplicaSnapshot) -> Result<String> {
        Ok(test_ref("service-snapshot"))
    }
}

#[derive(Debug, Default)]
struct ServiceTransportPort {
    sent: usize,
}

impl ReplicaTransportEffects for ServiceTransportPort {
    fn send<'a>(&'a mut self, _envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        self.sent += 1;
        Box::pin(async { Ok(test_ref("service-transport")) })
    }
}

#[derive(Debug, Default)]
struct ServiceTimePort {
    election_timer_refs: Vec<String>,
    heartbeat_arms: usize,
}

impl ReplicaTimeEffects for ServiceTimePort {
    fn arm_election_timer(&mut self, timer_ref: &str) -> Result<String> {
        self.election_timer_refs.push(timer_ref.to_string());
        Ok(test_ref("service-election-timer"))
    }

    fn arm_heartbeat_timer(&mut self) -> Result<String> {
        self.heartbeat_arms += 1;
        Ok(test_ref("service-heartbeat-timer"))
    }
}

#[derive(Debug, Default)]
struct ServiceApplicationHandler;

impl CommittedBatchHandler for ServiceApplicationHandler {
    fn restore_snapshot(&mut self, _snapshot: &ApplicationSnapshotRestore) -> Result<String> {
        Ok(test_ref("service-application-snapshot-handler"))
    }

    fn apply_batch(&mut self, _commands: &[ApplicationCommand]) -> Result<String> {
        Ok(test_ref("service-application-handler"))
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn scoped_service_executes_startup_and_current_timer_through_separate_ports() {
    let mut host = host_without(None);
    host.activate(ACTIVATION_TICK).expect("active host");
    let group = active_group_for_host(&host);
    let replica_profile = profile(&group);
    let plan = plan_live_replica_start_for_host(
        &host,
        group.clone(),
        NODE_A.to_string(),
        membership(&group),
        replica_profile.clone(),
    )
    .expect("host-backed replica start plan");
    let initial_timer_ref = plan.state.active_election_timer_ref.clone();
    let runtime_identity = runtime_identity(&plan);
    let mut mismatched_identity = runtime_identity.clone();
    mismatched_identity.protocol_ref = test_ref("substituted-runtime-protocol");
    let mismatch = validate_replica_runtime_identity_for_start(&mismatched_identity, &plan)
        .expect_err("substituted runtime identity must deny before effects");
    assert!(mismatch.to_string().contains("does not match the admitted start plan"));
    let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (control_sender, _control_receiver) = tokio::sync::mpsc::unbounded_channel();
    let application = AdmittedReplicaApplicationPort::new(
        ReplicaApplicationConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            handler_ref: test_ref("service-application-binding"),
            command_schema_refs: BTreeSet::from([test_ref("service-command-schema")]),
            initial_applied_index: INITIAL_COMMIT_INDEX,
        },
        ServiceApplicationHandler,
    )
    .expect("application port");
    let control = ChannelReplicaControlPort::new(
        ReplicaControlConfig {
            service_id: group.service_id.clone(),
            service_generation: group.service_generation,
            supervision_ref: replica_profile.supervision_ref,
        },
        control_sender,
    )
    .expect("control port");
    let ports = ReplicaPortBundle::new(
        runtime_identity,
        ServiceDurabilityPort::default(),
        ServiceTransportPort::default(),
        ServiceTimePort::default(),
        application,
        control,
    )
    .expect("bound runtime port bundle");
    let mut service = ScopedLiveReplicaService::start(plan, ports, event_receiver).await.expect("scoped live service");

    assert!(!service.production_admitted());
    assert_eq!(service.startup_observations().len(), EXPECTED_STARTUP_OBSERVATIONS);
    assert_eq!(service.ports().durability.hard_state_writes, 1);
    assert_eq!(service.ports().time.election_timer_refs, vec![initial_timer_ref.clone()]);

    event_sender
        .send(ReplicaEvent::ElectionTimeout {
            timer_ref: initial_timer_ref.clone(),
        })
        .expect("queue election timeout");
    let outcome = service
        .run_next(Duration::from_millis(SERVICE_EVENT_TIMEOUT_MILLISECONDS))
        .await
        .expect("bounded service turn");
    assert!(matches!(outcome, ReplicaExecutionOutcome::Applied(_)));
    assert_eq!(service.state().role, ReplicaRole::Candidate);
    assert_eq!(service.ports().transport.sent, EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().durability.hard_state_writes, EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().time.election_timer_refs.len(), EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().time.election_timer_refs.last(), Some(&service.state().active_election_timer_ref));

    let stale = service
        .handle_event(ReplicaEvent::ElectionTimeout {
            timer_ref: initial_timer_ref,
        })
        .await;
    assert!(matches!(stale, ReplicaExecutionOutcome::Denied { .. }));
    assert_eq!(service.ports().transport.sent, EXPECTED_ELECTION_PORT_CALLS);
    assert_eq!(service.ports().durability.hard_state_writes, EXPECTED_ELECTION_PORT_CALLS);
}

fn runtime_identity(plan: &ReplicaStartPlan) -> ReplicaRuntimePortIdentity {
    ReplicaRuntimePortIdentity {
        service_id: plan.service_id.clone(),
        service_generation: plan.state.profile.service_generation,
        group_binding_ref: plan.state.profile.group_binding_ref.clone(),
        application_manifest_ref: plan.application_manifest_ref.clone(),
        protocol_ref: plan.state.profile.protocol_ref.clone(),
        durable_log_ref: plan.state.profile.durable_log_ref.clone(),
        snapshot_store_ref: plan.state.profile.snapshot_store_ref.clone(),
        timer_profile_ref: plan.state.profile.timer_profile_ref.clone(),
        entropy_profile_ref: plan.state.profile.entropy_profile_ref.clone(),
        membership_ref: plan.state.membership.membership_ref.clone(),
        placement_ref: plan.state.profile.placement_ref.clone(),
        fencing_ref: plan.state.profile.fencing_ref.clone(),
        supervision_ref: plan.state.profile.supervision_ref.clone(),
        resource_profile_ref: plan.state.profile.resource_profile_ref.clone(),
        fabric_binding_refs: plan.port_binding_refs.clone(),
    }
}
