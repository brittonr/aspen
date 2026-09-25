// Explicit transport and durability inputs for the fabric boundary compatibility fixtures.
// Shared verbatim with the pre-migration generator; see `inputs.rs`.

const TRANSPORT_PROFILE_LIMIT: u64 = 16;
const TRANSPORT_FRAME_LIMIT: u64 = 4_096;
const TRANSPORT_DATAGRAM_LIMIT: u64 = 1_024;
const TRANSPORT_QUEUE_BYTE_LIMIT: u64 = 16_384;
const TRANSPORT_INFLIGHT_LIMIT: u64 = 8_192;
const TRANSPORT_DEADLINE_WINDOW: u64 = 64;
const TRANSPORT_LENGTH_PREFIX_BYTES: u64 = 4;
const TRANSPORT_GENERATION: u64 = 1;
const DURABLE_PROFILE_LIMIT: u64 = 32;
const DURABLE_OPERATION_BYTE_LIMIT: u64 = 1_024;
const DURABLE_NAMESPACE_BYTE_LIMIT: u64 = 16_384;
const DURABLE_GENERATION: u64 = 1;
const DURABLE_FIRST_SEQUENCE: u64 = 0;

fn transport_capabilities() -> Vec<molten::fabric_transport::TransportCapability> {
    vec![
        molten::fabric_transport::TransportCapability::BidirectionalStreams,
        molten::fabric_transport::TransportCapability::UnidirectionalStreams,
        molten::fabric_transport::TransportCapability::Datagrams,
    ]
}

pub fn transport_profile() -> molten::fabric_transport::TransportProfile {
    molten::fabric_transport::TransportProfile {
        schema: molten::fabric_transport::TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "fabric-boundary-deterministic-transport".to_string(),
        profile_ref: super::inputs::input_ref("transport-profile"),
        adapter_kind: molten::fabric_transport::TransportAdapterKind::DeterministicSimulation,
        capabilities: transport_capabilities(),
        limits: molten::fabric_transport::TransportLimits {
            max_listeners: TRANSPORT_PROFILE_LIMIT,
            max_sessions: TRANSPORT_PROFILE_LIMIT,
            max_streams_per_session: TRANSPORT_PROFILE_LIMIT,
            max_frame_bytes: TRANSPORT_FRAME_LIMIT,
            max_datagram_bytes: TRANSPORT_DATAGRAM_LIMIT,
            max_queued_events: TRANSPORT_PROFILE_LIMIT,
            max_queued_bytes: TRANSPORT_QUEUE_BYTE_LIMIT,
            max_inflight_bytes: TRANSPORT_INFLIGHT_LIMIT,
            operation_deadline_ticks: TRANSPORT_DEADLINE_WINDOW,
        },
        non_claims: molten::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    }
}

pub fn protocol_descriptor(
    profile: &molten::fabric_transport::TransportProfile,
) -> molten::fabric_transport::ProtocolDescriptor {
    molten::fabric_transport::ProtocolDescriptor {
        schema: molten::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "echo-protocol".to_string(),
        version: "v1".to_string(),
        alpn: "molten/extension-echo/1".to_string(),
        extension_id: "echo-extension".to_string(),
        service_id: "echo-service".to_string(),
        generation: TRANSPORT_GENERATION,
        listener_limit: 1,
        requested_capabilities: transport_capabilities(),
        framing: molten::fabric_transport::FramingProfile {
            profile_id: "length-delimited-blake3-v1".to_string(),
            profile_ref: super::inputs::input_ref("transport-framing"),
            max_frame_bytes: TRANSPORT_FRAME_LIMIT,
            length_prefix_bytes: TRANSPORT_LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: molten::fabric_transport::ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: TRANSPORT_DEADLINE_WINDOW,
        },
        registration_authority_ref: super::inputs::input_ref("transport-registration-authority"),
        profile_ref: profile.profile_ref.clone(),
    }
}

pub fn register_command(
    descriptor: molten::fabric_transport::ProtocolDescriptor,
) -> molten::fabric_transport::TransportCommand {
    molten::fabric_transport::TransportCommand::Register {
        operation_id: super::inputs::input_ref("transport-register-operation"),
        descriptor,
    }
}

fn durable_levels() -> Vec<molten::fabric_durability::DurabilityLevel> {
    vec![
        molten::fabric_durability::DurabilityLevel::Buffered,
        molten::fabric_durability::DurabilityLevel::ProcessLoss,
        molten::fabric_durability::DurabilityLevel::MachineLoss,
    ]
}

pub fn durable_state_profile() -> molten::fabric_durability::DurableStateProfile {
    molten::fabric_durability::DurableStateProfile {
        schema: molten::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: "fabric-boundary-deterministic-durability".to_string(),
        profile_ref: super::inputs::input_ref("durable-profile"),
        adapter_kind: molten::fabric_durability::DurableAdapterKind::DeterministicSimulation,
        supported_levels: durable_levels(),
        max_namespaces: DURABLE_PROFILE_LIMIT,
        max_log_records: DURABLE_PROFILE_LIMIT,
        max_ordered_entries: DURABLE_PROFILE_LIMIT,
        max_operation_bytes: DURABLE_OPERATION_BYTE_LIMIT,
        max_namespace_bytes: DURABLE_NAMESPACE_BYTE_LIMIT,
        max_batch_operations: DURABLE_PROFILE_LIMIT,
        max_snapshots: DURABLE_PROFILE_LIMIT,
        max_effect_transactions: DURABLE_PROFILE_LIMIT,
        non_claims: molten::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
    }
}

fn atomicity_domain() -> molten::fabric_durability::AtomicityDomain {
    molten::fabric_durability::AtomicityDomain {
        domain_id: "domain-a".to_string(),
        adapter_id: "adapter-a".to_string(),
        namespace_id: "namespace-a".to_string(),
        generation: DURABLE_GENERATION,
        object_classes: vec![
            molten::fabric_durability::DurableObjectClass::LogRecord,
            molten::fabric_durability::DurableObjectClass::OrderedValue,
            molten::fabric_durability::DurableObjectClass::Snapshot,
            molten::fabric_durability::DurableObjectClass::Checkpoint,
            molten::fabric_durability::DurableObjectClass::EffectTransaction,
        ],
        max_operations: DURABLE_PROFILE_LIMIT,
        max_bytes: DURABLE_OPERATION_BYTE_LIMIT,
        supported_levels: durable_levels(),
    }
}

pub fn durable_state(
    profile: &molten::fabric_durability::DurableStateProfile,
) -> molten::fabric_durability::DurableState {
    molten::fabric_durability::DurableState::empty(molten::fabric_durability::DurableNamespaceDescriptor {
        schema: molten::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: profile.profile_ref.clone(),
        adapter_id: "adapter-a".to_string(),
        namespace_id: "namespace-a".to_string(),
        generation: DURABLE_GENERATION,
        value_schema_ref: super::inputs::input_ref("durable-value-schema"),
        atomicity_domain: atomicity_domain(),
        retention_authority_ref: Some(super::inputs::input_ref("durable-retention-authority")),
        quota_bytes: DURABLE_NAMESPACE_BYTE_LIMIT,
    })
}

pub fn durable_append_request() -> molten::fabric_durability::AppendRequest {
    molten::fabric_durability::AppendRequest {
        adapter_id: "adapter-a".to_string(),
        namespace_id: "namespace-a".to_string(),
        generation: DURABLE_GENERATION,
        expected_sequence: DURABLE_FIRST_SEQUENCE,
        value: b"durable-record".to_vec(),
        value_ref: super::inputs::input_ref("durable-record-value"),
        durability: molten::fabric_durability::DurabilityLevel::ProcessLoss,
    }
}
