use super::super::*;

pub const IROH_SECRET_KEY_BYTES: usize = 32;
pub const CROSS_PROCESS_FRAME_PREFIX_BYTES: usize = 8;
/// Static bound on the remote endpoints one listener admits.
pub const MAX_LISTENER_ADMITTED_PEERS: usize = 64;

const IROH_CLOSE_CODE: u8 = 0;
const IROH_REFUSED_CLOSE_CODE: u8 = 1;
const CLIENT_CLOSE_REASON: &[u8] = b"cross-process-client-complete";
const UNADMITTED_PEER_CLOSE_REASON: &[u8] = b"cross-process-peer-not-admitted";
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

/// A remote endpoint the listener admits. The listener derives a session's observed peer context
/// from this entry for the actual authenticated remote `EndpointId`, never from its own expectation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedPeer {
    /// Canonical public endpoint identity (`iroh:<EndpointId>`) the peer authenticates with.
    pub public_endpoint_identity: String,
    /// Peer context the peer was admitted under.
    pub peer_context_ref: String,
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
    /// Remote endpoints admitted to open sessions, keyed by the caller's peer id. Any other
    /// authenticated endpoint is refused after its handshake.
    pub admitted_peers: std::collections::BTreeMap<String, AdmittedPeer>,
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
    /// Admitted peer id whose authenticated endpoint delivered the frame.
    pub peer_id: String,
}

impl std::fmt::Debug for ReceivedFrame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CrossProcessReceivedFrame")
            .field("payload_ref", &self.evidence.payload_ref)
            .field("payload_bytes", &self.evidence.payload_bytes)
            .field("peer_id", &self.peer_id)
            .field("evidence", &self.evidence)
            .field("payload", &"redacted")
            .finish()
    }
}

/// One bounded listener poll. A rejected connection ends only that connection: the listener stays
/// ready and the caller keeps polling.
#[derive(Debug)]
pub enum ListenerPoll {
    /// The accept deadline passed without an incoming connection.
    Idle,
    Received(ReceivedFrame),
    Rejected(ConnectionRejection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionRejectionKind {
    /// The handshake failed or did not complete before the deadline.
    Handshake,
    /// The authenticated remote endpoint is not admitted, or its admitted peer context differs.
    UnadmittedPeer,
    /// An admitted session failed: stream, frame bound, payload derivation, or peer close.
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionRejection {
    pub kind: ConnectionRejectionKind,
    pub failure: crate::error::Failure,
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
    admitted_peers: std::collections::BTreeMap<String, AdmittedPeer>,
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
