use super::*;
use crate::system_extension::SystemExtensionExecutor;

const REFERENCE_MAX_CONCURRENT_CALLBACKS: u64 = 1;
const REFERENCE_MAX_QUEUED_EVENTS: u64 = 64;
const REFERENCE_MAX_INFLIGHT_BYTES: u64 = 65_536;
const REFERENCE_MAX_OPEN_STREAMS: u64 = 16;
const REFERENCE_MAX_TIMERS: u64 = 64;
const REFERENCE_MAX_EFFECT_REQUESTS: u64 = 16;
const REFERENCE_CALLBACK_DEADLINE_TICKS: u64 = 1_024;
const REFERENCE_SHUTDOWN_GRACE_TICKS: u64 = 1_024;
const REFERENCE_MAX_RESTART_ATTEMPTS: u64 = 4;
const REFERENCE_EFFECT_BYTES: u64 = 1;
const REFERENCE_INITIAL_GENERATION: u64 = 1;
const REFERENCE_PORT_OPERATION: &str = "apply";

#[derive(Debug, Clone)]
pub struct ReferenceServiceExecutor {
    state: ReferenceServiceState,
    operations: std::collections::BTreeMap<String, ReferenceServiceOperation>,
    port_profiles: std::collections::BTreeMap<crate::fabric::FabricPortClass, SimulatedPortProfile>,
    last_transition: Option<ReferenceServiceTransition>,
}

impl ReferenceServiceExecutor {
    pub fn new(
        kind: crate::fabric::ReferenceSystemKind,
        operations: std::collections::BTreeMap<String, ReferenceServiceOperation>,
        port_profiles: &[SimulatedPortProfile],
    ) -> crate::error::Result<Self> {
        if operations.values().any(|operation| operation.kind() != kind) {
            return Err(crate::error::MoltenError::invalid_harness(
                "reference executor operation kind does not match its extension service",
            ));
        }
        let port_profiles = port_profiles.iter().cloned().map(|profile| (profile.class, profile)).collect();
        Ok(Self {
            state: initial_reference_state(kind),
            operations,
            port_profiles,
            last_transition: None,
        })
    }

    pub fn state(&self) -> &ReferenceServiceState {
        &self.state
    }

    pub fn last_transition(&self) -> Option<&ReferenceServiceTransition> {
        self.last_transition.as_ref()
    }

    fn current_state_ref(&self) -> String {
        blake3_ref(format!("{:?}", self.state).as_bytes())
    }
}

impl SystemExtensionExecutor for ReferenceServiceExecutor {
    fn execution_profile(&self) -> crate::system_extension::ExecutionProfile {
        crate::system_extension::ExecutionProfile::InProcessNative
    }

    fn invoke(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
    ) -> std::result::Result<crate::system_extension::CallbackOutcome, String> {
        if invocation.callback != crate::system_extension::CallbackKind::Request {
            let state_ref = self.current_state_ref();
            let checkpoint_ref = if invocation.callback == crate::system_extension::CallbackKind::Checkpoint {
                Some(state_ref.clone())
            } else {
                None
            };
            return Ok(crate::system_extension::CallbackOutcome {
                output_refs: vec![state_ref.clone()],
                effects: Vec::new(),
                state_ref: Some(state_ref),
                checkpoint_ref,
                health: crate::system_extension::HealthState::Healthy,
            });
        }
        let payload_ref = invocation
            .payload_ref
            .as_deref()
            .ok_or_else(|| "reference request callback requires a payload ref".to_string())?;
        let operation = self
            .operations
            .get(payload_ref)
            .ok_or_else(|| "reference request payload is not registered".to_string())?;
        let transition = apply_reference_operation(&self.state, operation)
            .map_err(|error| format!("reference service transition denied: {error:?}"))?;
        let effects = transition
            .required_ports
            .iter()
            .map(|class| self.effect_for(*class, payload_ref, invocation.generation))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        self.state = transition.next.clone();
        let state_ref = blake3_ref(transition.state_material.as_bytes());
        let decision_ref = blake3_ref(transition.decision.as_str().as_bytes());
        self.last_transition = Some(transition);
        Ok(crate::system_extension::CallbackOutcome {
            output_refs: vec![decision_ref],
            effects,
            state_ref: Some(state_ref),
            checkpoint_ref: None,
            health: crate::system_extension::HealthState::Healthy,
        })
    }
}

impl ReferenceServiceExecutor {
    fn effect_for(
        &self,
        class: crate::fabric::FabricPortClass,
        payload_ref: &str,
        generation: u64,
    ) -> std::result::Result<crate::system_extension::TypedEffectRequest, String> {
        let profile = self
            .port_profiles
            .get(&class)
            .ok_or_else(|| format!("reference service has no deterministic {} port profile", class.as_str()))?;
        let request_material = format!("{payload_ref}:{}:{generation}", class.as_str());
        Ok(crate::system_extension::TypedEffectRequest {
            target: crate::system_extension::EffectTarget::FabricPort(crate::fabric::FabricPortKey {
                port_id: profile.port_id.clone(),
                version: profile.version.clone(),
            }),
            operation: REFERENCE_PORT_OPERATION.to_string(),
            input_schema_ref: profile.command_schema_ref.clone(),
            output_schema_ref: profile.event_schema_ref.clone(),
            request_ref: blake3_ref(request_material.as_bytes()),
            generation,
            accounted_bytes: REFERENCE_EFFECT_BYTES,
        })
    }
}

pub fn reference_port_profiles() -> Vec<SimulatedPortProfile> {
    REQUIRED_SIMULATION_PORT_CLASSES
        .into_iter()
        .map(|class| {
            let port_id = simulation_port_id(class);
            let descriptor_ref = blake3_ref(format!("descriptor:{port_id}").as_bytes());
            SimulatedPortProfile {
                class,
                port_id,
                version: FABRIC_SIMULATION_PORT_VERSION.to_string(),
                implementation_profile: FABRIC_SIMULATION_PROFILE_ID.to_string(),
                descriptor_ref,
                command_schema_ref: simulation_command_schema(class),
                event_schema_ref: simulation_event_schema(class),
                deterministic: true,
                declared_faults: declared_faults(class),
            }
        })
        .collect()
}

pub fn reference_port_descriptors(profiles: &[SimulatedPortProfile]) -> Vec<crate::fabric::FabricPortDescriptor> {
    profiles
        .iter()
        .map(|profile| crate::fabric::FabricPortDescriptor {
            schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: profile.port_id.clone(),
            version: profile.version.clone(),
            class: profile.class,
            operation_classes: vec![REFERENCE_PORT_OPERATION.to_string()],
            input_schema_refs: vec![profile.command_schema_ref.clone()],
            output_schema_refs: vec![profile.event_schema_ref.clone()],
            authority_requirements: vec![authority_for_class(profile.class)],
            resource_requirements: vec![resource_for_class(profile.class)],
            determinism: crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            replay: crate::fabric::ReplayClass::Recompute,
            implementation_profile: profile.implementation_profile.clone(),
            conformance_refs: vec![profile.descriptor_ref.clone()],
            non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        })
        .collect()
}

pub fn reference_manifest_input(
    kind: crate::fabric::ReferenceSystemKind,
    implementation_ref: String,
    profiles: &[SimulatedPortProfile],
) -> crate::error::Result<crate::system_extension::SystemExtensionManifestInput> {
    let required_classes = reference_required_ports(kind);
    let mut required_ports = Vec::with_capacity(required_classes.len());
    for class in required_classes {
        let profile = profiles.iter().find(|profile| profile.class == class).ok_or_else(|| {
            crate::error::MoltenError::invalid_harness(format!("missing {} reference port profile", class.as_str()))
        })?;
        required_ports.push(crate::fabric::FabricPortRequirement {
            port_id: profile.port_id.clone(),
            version: profile.version.clone(),
            class,
            operation_classes: vec![REFERENCE_PORT_OPERATION.to_string()],
            input_schema_refs: vec![profile.command_schema_ref.clone()],
            output_schema_refs: vec![profile.event_schema_ref.clone()],
            allowed_authorities: vec![authority_for_class(class)],
            available_resources: vec![resource_for_class(class)],
            expected_determinism: crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            expected_replay: crate::fabric::ReplayClass::Recompute,
            expected_profile: profile.implementation_profile.clone(),
        });
    }
    let service = kind.as_str();
    Ok(crate::system_extension::SystemExtensionManifestInput {
        schema: crate::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: format!("molten.reference.{service}.extension"),
        service_id: format!("molten.reference.{service}"),
        implementation_ref,
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
        required_ports,
        optional_ports: Vec::new(),
        capability_refs: vec![blake3_ref(format!("capability:{service}").as_bytes())],
        policy_refs: vec![blake3_ref(format!("policy:{service}").as_bytes())],
        provenance_refs: vec![blake3_ref(format!("provenance:{service}").as_bytes())],
        resources: crate::system_extension::ResourceEnvelope {
            max_concurrent_callbacks: REFERENCE_MAX_CONCURRENT_CALLBACKS,
            max_queued_events: REFERENCE_MAX_QUEUED_EVENTS,
            max_inflight_bytes: REFERENCE_MAX_INFLIGHT_BYTES,
            max_open_streams: REFERENCE_MAX_OPEN_STREAMS,
            max_timers: REFERENCE_MAX_TIMERS,
            max_effect_requests: REFERENCE_MAX_EFFECT_REQUESTS,
            callback_deadline_ticks: REFERENCE_CALLBACK_DEADLINE_TICKS,
            shutdown_grace_ticks: REFERENCE_SHUTDOWN_GRACE_TICKS,
            max_restart_attempts: REFERENCE_MAX_RESTART_ATTEMPTS,
            overload_policy: crate::system_extension::OverloadPolicy::UpstreamBackpressure,
        },
        execution_profile: crate::system_extension::ExecutionProfile::InProcessNative,
        state_schema: format!("molten.reference.{service}.state.v1"),
        compatible_state_schemas: vec![format!("molten.reference.{service}.state.v1")],
        evidence_profile_ref: blake3_ref(format!("evidence:{service}").as_bytes()),
        initial_generation: REFERENCE_INITIAL_GENERATION,
        non_claims: crate::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    })
}

pub fn reference_required_ports(_kind: crate::fabric::ReferenceSystemKind) -> Vec<crate::fabric::FabricPortClass> {
    REQUIRED_SIMULATION_PORT_CLASSES.to_vec()
}

pub fn all_reference_authorities() -> Vec<crate::fabric::FabricAuthority> {
    let mut authorities = REQUIRED_SIMULATION_PORT_CLASSES.into_iter().map(authority_for_class).collect::<Vec<_>>();
    authorities.sort();
    authorities.dedup();
    authorities
}

pub fn default_reference_operations() -> Vec<(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)> {
    let kv_request = blake3_ref(b"reference-kv-commit");
    let log_append_request = blake3_ref(b"reference-log-append");
    let log_replicate_request = blake3_ref(b"reference-log-replicate");
    let scheduler_submit_request = blake3_ref(b"reference-scheduler-submit");
    let scheduler_lease_request = blake3_ref(b"reference-scheduler-lease");
    let scheduler_complete_request = blake3_ref(b"reference-scheduler-complete");
    vec![
        (
            crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
            kv_request,
            ReferenceServiceOperation::TransactionalKeyValue(TransactionalKeyValueOperation::Commit {
                expected_version: initial_transaction_version(),
                writes: vec![("key-a".to_string(), blake3_ref(b"value-a"))],
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::ReplicatedLog,
            log_append_request,
            ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::Append {
                payload_ref: blake3_ref(b"log-entry-a"),
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::ReplicatedLog,
            log_replicate_request,
            ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::ReplicateThrough { offset: 0 }),
        ),
        (
            crate::fabric::ReferenceSystemKind::DistributedScheduler,
            scheduler_submit_request,
            ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Submit {
                job_id: "job-a".to_string(),
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::DistributedScheduler,
            scheduler_lease_request,
            ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Lease {
                job_id: "job-a".to_string(),
                owner: "worker-a".to_string(),
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::DistributedScheduler,
            scheduler_complete_request,
            ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Complete {
                job_id: "job-a".to_string(),
                owner: "worker-a".to_string(),
                completion_ref: blake3_ref(b"job-a-completion"),
            }),
        ),
    ]
}

pub fn operations_for_kind(
    operations: &[(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)],
    kind: crate::fabric::ReferenceSystemKind,
) -> std::collections::BTreeMap<String, ReferenceServiceOperation> {
    operations
        .iter()
        .filter(|(operation_kind, _, _)| *operation_kind == kind)
        .map(|(_, request_ref, operation)| (request_ref.clone(), operation.clone()))
        .collect()
}

pub fn simulation_port_id(class: crate::fabric::FabricPortClass) -> String {
    format!("molten.fabric.simulation.{}", class.as_str())
}

pub fn simulation_command_schema(class: crate::fabric::FabricPortClass) -> String {
    format!("molten.fabric.simulation.{}.command.v1", class.as_str())
}

pub fn simulation_event_schema(class: crate::fabric::FabricPortClass) -> String {
    format!("molten.fabric.simulation.{}.event.v1", class.as_str())
}

pub fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn authority_for_class(class: crate::fabric::FabricPortClass) -> crate::fabric::FabricAuthority {
    match class {
        crate::fabric::FabricPortClass::Authority => crate::fabric::FabricAuthority::ProtocolOwnership,
        crate::fabric::FabricPortClass::Transport => crate::fabric::FabricAuthority::Transport,
        crate::fabric::FabricPortClass::DurableState => crate::fabric::FabricAuthority::DurableState,
        crate::fabric::FabricPortClass::Execution => crate::fabric::FabricAuthority::Execution,
        crate::fabric::FabricPortClass::Time => crate::fabric::FabricAuthority::Time,
        crate::fabric::FabricPortClass::Scheduling => crate::fabric::FabricAuthority::Scheduling,
        crate::fabric::FabricPortClass::Membership => crate::fabric::FabricAuthority::Membership,
        crate::fabric::FabricPortClass::Placement => crate::fabric::FabricAuthority::Placement,
        crate::fabric::FabricPortClass::Consistency => crate::fabric::FabricAuthority::Consistency,
        crate::fabric::FabricPortClass::Supervision => crate::fabric::FabricAuthority::Supervision,
        crate::fabric::FabricPortClass::Policy => crate::fabric::FabricAuthority::Policy,
        crate::fabric::FabricPortClass::Resources => crate::fabric::FabricAuthority::Resources,
        crate::fabric::FabricPortClass::Simulation => crate::fabric::FabricAuthority::Simulation,
        crate::fabric::FabricPortClass::Evidence => crate::fabric::FabricAuthority::Evidence,
    }
}

fn resource_for_class(class: crate::fabric::FabricPortClass) -> crate::fabric::FabricResource {
    match class {
        crate::fabric::FabricPortClass::Transport => crate::fabric::FabricResource::NetworkBytes,
        crate::fabric::FabricPortClass::DurableState => crate::fabric::FabricResource::StorageBytes,
        crate::fabric::FabricPortClass::Execution => crate::fabric::FabricResource::ExecutionMillis,
        crate::fabric::FabricPortClass::Time | crate::fabric::FabricPortClass::Scheduling => {
            crate::fabric::FabricResource::LogicalTime
        }
        crate::fabric::FabricPortClass::Evidence => crate::fabric::FabricResource::Diagnostics,
        crate::fabric::FabricPortClass::Authority
        | crate::fabric::FabricPortClass::Membership
        | crate::fabric::FabricPortClass::Placement
        | crate::fabric::FabricPortClass::Consistency
        | crate::fabric::FabricPortClass::Supervision
        | crate::fabric::FabricPortClass::Policy
        | crate::fabric::FabricPortClass::Resources
        | crate::fabric::FabricPortClass::Simulation => crate::fabric::FabricResource::Memory,
    }
}

fn declared_faults(class: crate::fabric::FabricPortClass) -> Vec<SimulationFaultKind> {
    match class {
        crate::fabric::FabricPortClass::Transport => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::Drop,
            SimulationFaultKind::Duplicate,
            SimulationFaultKind::Reorder,
            SimulationFaultKind::Partition,
            SimulationFaultKind::Reset,
        ],
        crate::fabric::FabricPortClass::DurableState => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::BoundedCorruption,
            SimulationFaultKind::CapacityExhaustion,
            SimulationFaultKind::Crash,
        ],
        crate::fabric::FabricPortClass::Execution => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::CapacityExhaustion,
            SimulationFaultKind::Pause,
            SimulationFaultKind::Crash,
        ],
        crate::fabric::FabricPortClass::Time | crate::fabric::FabricPortClass::Scheduling => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::ClockSkew,
            SimulationFaultKind::ClockJump,
            SimulationFaultKind::Pause,
        ],
        crate::fabric::FabricPortClass::Membership => {
            vec![SimulationFaultKind::MembershipChange, SimulationFaultKind::Partition]
        }
        crate::fabric::FabricPortClass::Placement => vec![SimulationFaultKind::PlacementReplacement],
        crate::fabric::FabricPortClass::Consistency => vec![
            SimulationFaultKind::ConsistencyQuorumLoss,
            SimulationFaultKind::Partition,
        ],
        crate::fabric::FabricPortClass::Authority | crate::fabric::FabricPortClass::Policy => {
            vec![SimulationFaultKind::AuthorityRevocation]
        }
        crate::fabric::FabricPortClass::Supervision => vec![
            SimulationFaultKind::Pause,
            SimulationFaultKind::Crash,
            SimulationFaultKind::Restart,
        ],
        crate::fabric::FabricPortClass::Resources => vec![SimulationFaultKind::CapacityExhaustion],
        crate::fabric::FabricPortClass::Simulation | crate::fabric::FabricPortClass::Evidence => {
            vec![SimulationFaultKind::Delay]
        }
    }
}
