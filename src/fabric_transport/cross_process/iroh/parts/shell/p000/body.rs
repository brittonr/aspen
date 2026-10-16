use super::super::*;

pub const IROH_SECRET_KEY_BYTES: usize = 32;
pub const CROSS_PROCESS_FRAME_PREFIX_BYTES: usize = 8;

const IROH_CLOSE_CODE: u8 = 0;
const CLIENT_CLOSE_REASON: &[u8] = b"cross-process-client-complete";
const FRAME_DOMAIN: &str = "molten.fabric.transport.cross-process-frame.v1";
const CLEANUP_DOMAIN: &str = "molten.fabric.transport.cross-process-cleanup.v1";

#[derive(Clone)]
pub struct EndpointCapability {
    secret_key: iroh::SecretKey,
    capability_ref: String,
}

impl EndpointCapability {
    pub fn from_secret_bytes(
        secret_bytes: [u8; IROH_SECRET_KEY_BYTES],
        capability_ref: String,
    ) -> crate::error::Result<Self> {
        crate::preserves_rail::validate_content_ref(&capability_ref)?;
        Ok(Self {
            secret_key: iroh::SecretKey::from_bytes(&secret_bytes),
            capability_ref,
        })
    }

    pub fn capability_ref(&self) -> &str {
        &self.capability_ref
    }
}

impl std::fmt::Debug for EndpointCapability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IrohEndpointCapability")
            .field("capability_ref", &self.capability_ref)
            .field("secret_key", &"redacted")
            .finish()
    }
}

#[derive(Debug)]
pub struct ListenerInput {
    pub profile: Profile,
    pub protocol: ProtocolDescriptor,
    pub capability: EndpointCapability,
    pub bind_addr: std::net::SocketAddr,
    pub listener_identity_ref: String,
    pub expected_peer_context_ref: String,
    pub locator_cohort_ref: String,
    pub disclosure: EndpointDisclosurePolicy,
    pub validity: EndpointValidityCohort,
    pub admission: EndpointAdmissionState,
    pub observed_tick: u64,
}

#[derive(Debug, Clone)]
pub struct ClientInput {
    pub profile: Profile,
    pub protocol: ProtocolDescriptor,
    pub capability: EndpointCapability,
    pub bind_addr: std::net::SocketAddr,
    pub endpoint: Endpoint,
    pub expected: ExpectedEndpointBinding,
    pub admission: EndpointAdmissionState,
    pub session_ref: String,
    pub request_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameEvidence {
    pub role: EndpointParticipantRole,
    pub descriptor_ref: String,
    pub session_ref: String,
    pub request_ref: String,
    pub payload_ref: String,
    pub acknowledgement_ref: String,
    pub remote_transport_identity_ref: String,
    pub payload_bytes: u64,
    pub delivery: DeliveryOutcome,
    pub retry: RetryDisposition,
    pub automatic_retry_count: u64,
    pub terminal_class: SessionTerminalClass,
    pub cleanup_evidence_ref: String,
}

pub struct ReceivedFrame {
    pub payload: Vec<u8>,
    pub evidence: FrameEvidence,
}

impl std::fmt::Debug for ReceivedFrame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CrossProcessReceivedFrame")
            .field("payload_ref", &self.evidence.payload_ref)
            .field("payload_bytes", &self.evidence.payload_bytes)
            .field("evidence", &self.evidence)
            .field("payload", &"redacted")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerCleanup {
    pub listener_identity_ref: String,
    pub descriptor_ref: String,
    pub generation: u64,
    pub drain_reason: ListenerDrainReason,
    pub terminal_class: ListenerTerminalClass,
    pub cleanup_evidence_ref: String,
}

pub struct Listener {
    endpoint: iroh::Endpoint,
    profile: Profile,
    protocol: ProtocolDescriptor,
    admission: EndpointAdmissionState,
    endpoint_artifact: Endpoint,
    endpoint_status: EndpointStatus,
    state: CrossProcessListenerState,
}

impl std::fmt::Debug for Listener {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IrohCrossProcessListener")
            .field("descriptor_ref", &self.endpoint_artifact.descriptor_ref)
            .field("listener_identity_ref", &self.state.identity.listener_identity_ref)
            .field("phase", &self.state.phase)
            .finish()
    }
}
