use super::raft::*;
use super::*;
use crate::system_extension::SystemExtensionExecutor;

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
const SERVICE_EVENT_CAPACITY: usize = 1;
const SERVICE_CONTROL_CAPACITY: usize = 1;

#[derive(Debug, Clone)]
struct TestPortSpec {
    port_id: &'static str,
    class: crate::fabric::FabricPortClass,
    authorities: Vec<crate::fabric::FabricAuthority>,
    resources: Vec<crate::fabric::FabricResource>,
}

#[derive(Debug, Clone, Copy)]
struct TestExecutor;

impl SystemExtensionExecutor for TestExecutor {
    fn execution_profile(&self) -> crate::system_extension::ExecutionProfile {
        crate::system_extension::ExecutionProfile::InProcessNative
    }

    fn invoke(
        &mut self,
        _invocation: &crate::system_extension::CallbackInvocation,
    ) -> std::result::Result<crate::system_extension::CallbackOutcome, String> {
        Ok(crate::system_extension::CallbackOutcome {
            output_refs: vec![test_ref("callback-output")],
            effects: Vec::new(),
            state_ref: Some(test_ref("callback-state")),
            checkpoint_ref: None,
            health: crate::system_extension::HealthState::Healthy,
        })
    }
}

fn port_specs() -> Vec<TestPortSpec> {
    vec![
        TestPortSpec {
            port_id: crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID,
            class: crate::fabric::FabricPortClass::Transport,
            authorities: vec![
                crate::fabric::FabricAuthority::Transport,
                crate::fabric::FabricAuthority::ProtocolOwnership,
            ],
            resources: vec![
                crate::fabric::FabricResource::NetworkBytes,
                crate::fabric::FabricResource::Concurrency,
            ],
        },
        TestPortSpec {
            port_id: crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID,
            class: crate::fabric::FabricPortClass::DurableState,
            authorities: vec![crate::fabric::FabricAuthority::DurableState],
            resources: vec![
                crate::fabric::FabricResource::StorageBytes,
                crate::fabric::FabricResource::QueueDepth,
            ],
        },
        TestPortSpec {
            port_id: crate::fabric_durability::FABRIC_SNAPSHOT_PORT_ID,
            class: crate::fabric::FabricPortClass::DurableState,
            authorities: vec![crate::fabric::FabricAuthority::DurableState],
            resources: vec![
                crate::fabric::FabricResource::StorageBytes,
                crate::fabric::FabricResource::QueueDepth,
            ],
        },
        TestPortSpec {
            port_id: crate::fabric_time::FABRIC_TIMER_PORT_ID,
            class: crate::fabric::FabricPortClass::Time,
            authorities: vec![crate::fabric::FabricAuthority::Time],
            resources: vec![crate::fabric::FabricResource::LogicalTime],
        },
        TestPortSpec {
            port_id: crate::fabric_time::FABRIC_ENTROPY_PORT_ID,
            class: crate::fabric::FabricPortClass::Time,
            authorities: vec![crate::fabric::FabricAuthority::Time],
            resources: vec![crate::fabric::FabricResource::Memory],
        },
        TestPortSpec {
            port_id: crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_ID,
            class: crate::fabric::FabricPortClass::Membership,
            authorities: vec![
                crate::fabric::FabricAuthority::Membership,
                crate::fabric::FabricAuthority::Policy,
            ],
            resources: vec![crate::fabric::FabricResource::Diagnostics],
        },
        TestPortSpec {
            port_id: crate::fabric_membership::FABRIC_PLACEMENT_PORT_ID,
            class: crate::fabric::FabricPortClass::Placement,
            authorities: vec![
                crate::fabric::FabricAuthority::Placement,
                crate::fabric::FabricAuthority::Policy,
                crate::fabric::FabricAuthority::Resources,
            ],
            resources: vec![crate::fabric::FabricResource::Diagnostics],
        },
    ]
}

fn port_descriptor(spec: &TestPortSpec) -> crate::fabric::FabricPortDescriptor {
    crate::fabric::FabricPortDescriptor {
        schema: molten_core::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: spec.port_id.to_string(),
        version: "v1".to_string(),
        class: spec.class,
        operation_classes: vec![TEST_PORT_OPERATION.to_string()],
        input_schema_refs: vec![TEST_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![TEST_OUTPUT_SCHEMA.to_string()],
        authority_requirements: spec.authorities.clone(),
        resource_requirements: spec.resources.clone(),
        determinism: crate::fabric::DeterminismClass::ExternalEffect,
        replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: TEST_PORT_PROFILE.to_string(),
        conformance_refs: vec![test_ref(spec.port_id)],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn port_requirement(spec: &TestPortSpec) -> crate::fabric::FabricPortRequirement {
    crate::fabric::FabricPortRequirement {
        port_id: spec.port_id.to_string(),
        version: "v1".to_string(),
        class: spec.class,
        operation_classes: vec![TEST_PORT_OPERATION.to_string()],
        input_schema_refs: vec![TEST_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![TEST_OUTPUT_SCHEMA.to_string()],
        allowed_authorities: spec.authorities.clone(),
        available_resources: spec.resources.clone(),
        expected_determinism: crate::fabric::DeterminismClass::ExternalEffect,
        expected_replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        expected_profile: TEST_PORT_PROFILE.to_string(),
    }
}

fn host_without(omitted_port_id: Option<&str>) -> crate::system_extension::SystemExtensionHost<TestExecutor> {
    let specs = port_specs().into_iter().filter(|spec| Some(spec.port_id) != omitted_port_id).collect::<Vec<_>>();
    let descriptors = specs.iter().map(port_descriptor).collect::<Vec<_>>();
    let requirements = specs.iter().map(port_requirement).collect::<Vec<_>>();
    let tier = crate::fabric::canonical_extension_tier_admission(&crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SystemExtension,
        requested_authorities: vec![
            crate::fabric::FabricAuthority::ProtocolOwnership,
            crate::fabric::FabricAuthority::Transport,
            crate::fabric::FabricAuthority::DurableState,
            crate::fabric::FabricAuthority::Time,
            crate::fabric::FabricAuthority::Membership,
            crate::fabric::FabricAuthority::Placement,
            crate::fabric::FabricAuthority::Consistency,
            crate::fabric::FabricAuthority::Supervision,
            crate::fabric::FabricAuthority::Policy,
            crate::fabric::FabricAuthority::Resources,
            crate::fabric::FabricAuthority::Evidence,
        ],
        admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .expect("system-extension tier");
    let admitted = crate::system_extension::canonical_admit_system_extension_manifest(
        &crate::system_extension::SystemExtensionManifestInput {
            schema: crate::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
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
            resources: crate::system_extension::ResourceEnvelope {
                max_concurrent_callbacks: MAX_CONCURRENT_CALLBACKS,
                max_queued_events: MAX_QUEUED_EVENTS,
                max_inflight_bytes: MAX_INFLIGHT_BYTES,
                max_open_streams: MAX_OPEN_STREAMS,
                max_timers: MAX_TIMERS,
                max_effect_requests: MAX_EFFECT_REQUESTS,
                callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
                shutdown_grace_ticks: SHUTDOWN_GRACE_TICKS,
                max_restart_attempts: MAX_RESTART_ATTEMPTS,
                overload_policy: crate::system_extension::OverloadPolicy::UpstreamBackpressure,
            },
            execution_profile: crate::system_extension::ExecutionProfile::InProcessNative,
            state_schema: TEST_STATE_SCHEMA.to_string(),
            compatible_state_schemas: vec![TEST_STATE_SCHEMA.to_string()],
            evidence_profile_ref: test_ref("evidence-profile"),
            initial_generation: SERVICE_GENERATION,
            non_claims: crate::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
        },
        &descriptors,
        &tier,
        &[crate::system_extension::ExecutionProfile::InProcessNative],
    )
    .expect("admitted live Raft host manifest");
    crate::system_extension::SystemExtensionHost::new(admitted, TestExecutor).expect("live Raft host")
}

fn active_group_for_host(host: &crate::system_extension::SystemExtensionHost<TestExecutor>) -> ConsistencyGroupBinding {
    active_group_for_host_with_policies(host, host.manifest().manifest().policy_refs.clone())
}

fn active_group_for_host_with_policies(
    host: &crate::system_extension::SystemExtensionHost<TestExecutor>,
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
