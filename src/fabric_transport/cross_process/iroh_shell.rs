use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

use super::super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub const IROH_SECRET_KEY_BYTES: usize = 32;
pub const CROSS_PROCESS_FRAME_PREFIX_BYTES: usize = 8;

const IROH_CLOSE_CODE: u8 = 0;
const CLIENT_CLOSE_REASON: &[u8] = b"cross-process-client-complete";
const FRAME_DOMAIN: &str = "molten.fabric.transport.cross-process-frame.v1";
const CLEANUP_DOMAIN: &str = "molten.fabric.transport.cross-process-cleanup.v1";

#[derive(Clone)]
pub struct IrohEndpointCapability {
    secret_key: iroh::SecretKey,
    capability_ref: String,
}

impl IrohEndpointCapability {
    pub fn from_secret_bytes(secret_bytes: [u8; IROH_SECRET_KEY_BYTES], capability_ref: String) -> Result<Self> {
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

impl fmt::Debug for IrohEndpointCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IrohEndpointCapability")
            .field("capability_ref", &self.capability_ref)
            .field("secret_key", &"redacted")
            .finish()
    }
}

#[derive(Debug)]
pub struct IrohCrossProcessListenerInput {
    pub profile: CanonicalTransportProfile,
    pub protocol: ProtocolDescriptor,
    pub capability: IrohEndpointCapability,
    pub bind_addr: SocketAddr,
    pub listener_identity_ref: String,
    pub expected_peer_context_ref: String,
    pub locator_cohort_ref: String,
    pub disclosure: EndpointDisclosurePolicy,
    pub validity: EndpointValidityCohort,
    pub admission: EndpointAdmissionState,
    pub observed_tick: u64,
}

#[derive(Debug, Clone)]
pub struct IrohCrossProcessClientInput {
    pub profile: CanonicalTransportProfile,
    pub protocol: ProtocolDescriptor,
    pub capability: IrohEndpointCapability,
    pub bind_addr: SocketAddr,
    pub endpoint: CanonicalCrossProcessEndpoint,
    pub expected: ExpectedEndpointBinding,
    pub admission: EndpointAdmissionState,
    pub session_ref: String,
    pub request_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessFrameEvidence {
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

pub struct CrossProcessReceivedFrame {
    pub payload: Vec<u8>,
    pub evidence: CrossProcessFrameEvidence,
}

impl fmt::Debug for CrossProcessReceivedFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
pub struct CrossProcessListenerCleanup {
    pub listener_identity_ref: String,
    pub descriptor_ref: String,
    pub generation: u64,
    pub drain_reason: ListenerDrainReason,
    pub terminal_class: ListenerTerminalClass,
    pub cleanup_evidence_ref: String,
}

pub struct IrohCrossProcessListener {
    endpoint: iroh::Endpoint,
    profile: CanonicalTransportProfile,
    protocol: ProtocolDescriptor,
    admission: EndpointAdmissionState,
    endpoint_artifact: CanonicalCrossProcessEndpoint,
    endpoint_status: CanonicalEndpointStatus,
    state: CrossProcessListenerState,
}

impl fmt::Debug for IrohCrossProcessListener {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IrohCrossProcessListener")
            .field("descriptor_ref", &self.endpoint_artifact.descriptor_ref)
            .field("listener_identity_ref", &self.state.identity.listener_identity_ref)
            .field("phase", &self.state.phase)
            .finish()
    }
}

impl IrohCrossProcessListener {
    // r[impl molten.fabric_transport.cross_process_listener]
    // r[impl molten.fabric_transport.cross_process_session]
    pub async fn bind(input: IrohCrossProcessListenerInput) -> Result<Self> {
        validate_listener_shell_input(&input)?;
        let alpn = input.protocol.alpn.as_bytes().to_vec();
        let endpoint = bind_explicit_endpoint(input.bind_addr, input.capability, &alpn).await?;
        let endpoint_addr = endpoint.addr();
        let locators = endpoint_locators(&endpoint_addr)?;
        let bindings = EndpointDescriptorBindings {
            public_endpoint_identity: format!("iroh:{}", endpoint_addr.id),
            listener_identity_ref: input.listener_identity_ref,
            expected_peer_context_ref: input.expected_peer_context_ref,
            locator_cohort_ref: input.locator_cohort_ref,
            locators,
            disclosure: input.disclosure,
            resources: EndpointResourceBounds {
                max_sessions: input.profile.profile.limits.max_sessions,
                max_frame_bytes: input.protocol.framing.max_frame_bytes,
                max_queued_bytes: input.profile.profile.limits.max_queued_bytes,
                max_inflight_bytes: input.profile.profile.limits.max_inflight_bytes,
            },
            validity: input.validity,
        };
        let endpoint_artifact = canonical_cross_process_endpoint(&input.profile.profile, &input.protocol, &bindings)?;
        let mut state =
            plan_cross_process_listener(&input.profile.profile, &input.protocol, &endpoint_artifact.descriptor, &[])
                .map_err(|issues| shell_validation_error("cross-process listener plan", &issues))?;
        state = apply_cross_process_listener_command(&state, &CrossProcessListenerCommand::Start)
            .map_err(|issues| shell_validation_error("cross-process listener start", &issues))?
            .next;
        state = apply_cross_process_listener_command(
            &state,
            &CrossProcessListenerCommand::MarkReady(ListenerReadinessObservation {
                endpoint_setup: true,
                exact_alpn_active: true,
                registration_owned: input.admission.registration_active,
                transport_capability_active: input.admission.transport_capability_active,
                protocol_capability_active: input.admission.protocol_capability_active,
                profile_active: input.admission.profile_active,
            }),
        )
        .map_err(|issues| shell_validation_error("cross-process listener readiness", &issues))?
        .next;
        let _export = plan_endpoint_export(
            &input.profile.profile,
            &input.protocol,
            &endpoint_artifact.descriptor,
            &state,
            input.admission,
            input.observed_tick,
        )
        .map_err(|issues| shell_validation_error("cross-process endpoint publication", &issues))?;
        let endpoint_status = canonical_endpoint_status(&endpoint_artifact.descriptor)?;
        Ok(Self {
            endpoint,
            profile: input.profile,
            protocol: input.protocol,
            admission: input.admission,
            endpoint_artifact,
            endpoint_status,
            state,
        })
    }

    pub fn profile(&self) -> &CanonicalTransportProfile {
        &self.profile
    }

    pub const fn admission(&self) -> EndpointAdmissionState {
        self.admission
    }

    pub fn handoff(&self) -> &CanonicalCrossProcessEndpoint {
        &self.endpoint_artifact
    }

    pub fn status(&self) -> &CanonicalEndpointStatus {
        &self.endpoint_status
    }

    pub fn state(&self) -> &CrossProcessListenerState {
        &self.state
    }

    // r[impl molten.fabric_transport.cross_process_listener]
    // r[impl molten.fabric_transport.cross_process_session]
    pub async fn accept_one(
        &mut self,
        session_ref: &str,
        request_ref: &str,
        timeout: Duration,
    ) -> Result<CrossProcessFrameEvidence> {
        Ok(self.accept_one_frame(session_ref, request_ref, timeout).await?.evidence)
    }

    // r[impl molten.fabric_consistency.live_service_ports]
    pub async fn accept_one_frame(
        &mut self,
        session_ref: &str,
        request_ref: &str,
        timeout: Duration,
    ) -> Result<CrossProcessReceivedFrame> {
        validate_exchange_refs(session_ref, request_ref)?;
        if !self.state.is_ready() {
            return Err(MoltenError::invalid_harness("cross-process listener is not ready"));
        }
        let incoming = tokio::time::timeout(timeout, self.endpoint.accept())
            .await
            .map_err(|_| MoltenError::invalid_harness("cross-process listener accept timed out"))?
            .ok_or_else(|| MoltenError::invalid_harness("cross-process listener closed before accept"))?;
        let connection = tokio::time::timeout(timeout, incoming)
            .await
            .map_err(|_| MoltenError::invalid_harness("cross-process listener handshake timed out"))?
            .map_err(iroh_error)?;
        let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::AcceptSession {
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process listener accept", &issues))?
        .next;

        let dial_plan = dial_plan_from_descriptor(&self.endpoint_artifact.descriptor);
        let mut session = plan_cross_process_session(&dial_plan, session_ref, EndpointParticipantRole::Listener)
            .map_err(|issues| shell_validation_error("cross-process inbound session plan", &issues))?;
        session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginAccept {
            observed_descriptor_ref: self.endpoint_artifact.descriptor_ref.clone(),
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process inbound accept", &issues))?
        .next;
        session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Established {
            observed_peer_context_ref: self.endpoint_artifact.descriptor.expected_peer_context_ref.clone(),
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process inbound establishment", &issues))?
        .next;

        let exchange =
            run_server_exchange(&connection, &mut session, request_ref, self.protocol.generation, timeout).await;
        let received = match exchange {
            Ok(exchange) => {
                let evidence = finalize_successful_session(
                    session,
                    EndpointParticipantRole::Listener,
                    &self.endpoint_artifact.descriptor_ref,
                    session_ref,
                    request_ref,
                    &remote_transport_identity_ref,
                    exchange.frame,
                )?;
                CrossProcessReceivedFrame {
                    payload: exchange.payload,
                    evidence,
                }
            }
            Err(error) => {
                let _failed = finalize_failed_session(session, SessionTerminalClass::AdapterFailure)?;
                self.finish_listener_session()?;
                return Err(error);
            }
        };
        self.finish_listener_session()?;
        Ok(received)
    }

    // r[impl molten.fabric_transport.cross_process_listener]
    pub async fn drain_and_close(mut self, reason: ListenerDrainReason) -> Result<CrossProcessListenerCleanup> {
        self.state =
            apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginDrain { reason })
                .map_err(|issues| shell_validation_error("cross-process listener drain", &issues))?
                .next;
        if self.state.active_sessions != 0 {
            return Err(MoltenError::invalid_harness(
                "cross-process listener drain requires all sessions to be terminal",
            ));
        }
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::Close)
            .map_err(|issues| shell_validation_error("cross-process listener close", &issues))?
            .next;
        self.endpoint.close().await;
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::BeginCleanup)
            .map_err(|issues| shell_validation_error("cross-process listener cleanup", &issues))?
            .next;
        let cleanup_evidence_ref = cleanup_ref(
            &self.state.identity.listener_identity_ref,
            &self.endpoint_artifact.descriptor_ref,
            self.protocol.generation,
        );
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::CompleteCleanup {
            cleanup_evidence_ref: cleanup_evidence_ref.clone(),
        })
        .map_err(|issues| shell_validation_error("cross-process listener cleanup completion", &issues))?
        .next;
        Ok(CrossProcessListenerCleanup {
            listener_identity_ref: self.state.identity.listener_identity_ref.clone(),
            descriptor_ref: self.endpoint_artifact.descriptor_ref.clone(),
            generation: self.protocol.generation,
            drain_reason: reason,
            terminal_class: self.state.terminal_class.unwrap_or(ListenerTerminalClass::Clean),
            cleanup_evidence_ref,
        })
    }

    fn finish_listener_session(&mut self) -> Result<()> {
        self.state = apply_cross_process_listener_command(&self.state, &CrossProcessListenerCommand::SessionTerminal {
            callback_generation: self.protocol.generation,
        })
        .map_err(|issues| shell_validation_error("cross-process listener session terminal", &issues))?
        .next;
        Ok(())
    }
}

// r[impl molten.fabric_transport.cross_process_endpoint]
// r[impl molten.fabric_transport.cross_process_session]
pub async fn exchange_cross_process_frame(
    input: IrohCrossProcessClientInput,
    payload: &[u8],
    timeout: Duration,
) -> Result<CrossProcessFrameEvidence> {
    validate_client_shell_input(&input, payload)?;
    let dial_plan = admit_endpoint_import(
        &input.profile.profile,
        &input.protocol,
        &input.endpoint.descriptor,
        &input.expected,
        input.admission,
    )
    .map_err(|issues| shell_validation_error("cross-process endpoint import", &issues))?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    if payload_bytes == 0 || payload_bytes > dial_plan.resources.max_frame_bytes {
        return Err(MoltenError::invalid_harness("cross-process client payload exceeds the admitted frame bound"));
    }
    let mut session = plan_cross_process_session(&dial_plan, &input.session_ref, EndpointParticipantRole::Client)
        .map_err(|issues| shell_validation_error("cross-process client session plan", &issues))?;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginDial {
        observed_descriptor_ref: input.endpoint.descriptor_ref.clone(),
        callback_generation: input.protocol.generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client dial plan", &issues))?
    .next;

    let alpn = input.protocol.alpn.as_bytes().to_vec();
    let endpoint = bind_explicit_endpoint(input.bind_addr, input.capability, &alpn).await?;
    let endpoint_addr = iroh_endpoint_addr(&dial_plan)?;
    let network = run_client_exchange(
        &endpoint,
        endpoint_addr,
        &alpn,
        &mut session,
        &input.request_ref,
        payload,
        input.protocol.generation,
        timeout,
    )
    .await;
    endpoint.close().await;
    let exchange = match network {
        Ok(exchange) => exchange,
        Err(error) => {
            let _failed = finalize_failed_session(session, SessionTerminalClass::AdapterFailure)?;
            return Err(error);
        }
    };
    let mut evidence = finalize_successful_session(
        session,
        EndpointParticipantRole::Client,
        &input.endpoint.descriptor_ref,
        &input.session_ref,
        &input.request_ref,
        &exchange.remote_transport_identity_ref,
        exchange.frame,
    )?;
    evidence.cleanup_evidence_ref =
        cleanup_ref(&evidence.cleanup_evidence_ref, &input.endpoint.descriptor_ref, input.protocol.generation);
    Ok(evidence)
}

struct NetworkFrame {
    payload_ref: String,
    acknowledgement_ref: String,
    payload_bytes: u64,
}

struct ClientNetworkFrame {
    frame: NetworkFrame,
    remote_transport_identity_ref: String,
}

struct ServerNetworkFrame {
    frame: NetworkFrame,
    payload: Vec<u8>,
}

async fn run_server_exchange(
    connection: &iroh::endpoint::Connection,
    session: &mut CrossProcessSessionState,
    request_ref: &str,
    generation: u64,
    timeout: Duration,
) -> Result<ServerNetworkFrame> {
    let (mut send, mut receive) = tokio::time::timeout(timeout, connection.accept_bi())
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process stream accept timed out"))?
        .map_err(iroh_error)?;
    let payload = read_bounded_frame(&mut receive, session.resources.max_frame_bytes, timeout).await?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::ReceiveFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process server receive", &issues))?
    .next;
    write_bounded_frame(&mut send, &payload, session.resources.max_frame_bytes, timeout).await?;
    tokio::time::timeout(timeout, connection.closed())
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process peer close timed out"))?;
    let payload_ref = cross_process_frame_ref(request_ref, &payload);
    Ok(ServerNetworkFrame {
        frame: NetworkFrame {
            acknowledgement_ref: payload_ref.clone(),
            payload_ref,
            payload_bytes,
        },
        payload,
    })
}

async fn run_client_exchange(
    endpoint: &iroh::Endpoint,
    endpoint_addr: iroh::EndpointAddr,
    alpn: &[u8],
    session: &mut CrossProcessSessionState,
    request_ref: &str,
    payload: &[u8],
    generation: u64,
    timeout: Duration,
) -> Result<ClientNetworkFrame> {
    let connection = tokio::time::timeout(timeout, endpoint.connect(endpoint_addr, alpn))
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process connect timed out"))?
        .map_err(iroh_error)?;
    let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::Established {
        observed_peer_context_ref: session.identity.expected_peer_context_ref.clone(),
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client establishment", &issues))?
    .next;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client queue", &issues))?
    .next;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::FrameSubmitted {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client submission", &issues))?
    .next;

    let (mut send, mut receive) = tokio::time::timeout(timeout, connection.open_bi())
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process stream open timed out"))?
        .map_err(iroh_error)?;
    write_bounded_frame(&mut send, payload, session.resources.max_frame_bytes, timeout).await?;
    let acknowledgement = read_bounded_frame(&mut receive, session.resources.max_frame_bytes, timeout).await?;
    if acknowledgement != payload {
        return Err(MoltenError::invalid_harness("cross-process acknowledgement payload mismatch"));
    }
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::AcknowledgeFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process acknowledgement", &issues))?
    .next;
    connection.close(IROH_CLOSE_CODE.into(), CLIENT_CLOSE_REASON);
    let payload_ref = cross_process_frame_ref(request_ref, payload);
    Ok(ClientNetworkFrame {
        frame: NetworkFrame {
            acknowledgement_ref: payload_ref.clone(),
            payload_ref,
            payload_bytes,
        },
        remote_transport_identity_ref,
    })
}

async fn bind_explicit_endpoint(
    bind_addr: SocketAddr,
    capability: IrohEndpointCapability,
    alpn: &[u8],
) -> Result<iroh::Endpoint> {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(bind_addr)
        .map_err(iroh_error)?
        .secret_key(capability.secret_key)
        .alpns(vec![alpn.to_vec()])
        .bind()
        .await
        .map_err(iroh_error)
}

async fn write_bounded_frame(
    send: &mut iroh::endpoint::SendStream,
    payload: &[u8],
    max_frame_bytes: u64,
    timeout: Duration,
) -> Result<()> {
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    if payload_bytes == 0 || payload_bytes > max_frame_bytes {
        return Err(MoltenError::invalid_harness("cross-process outbound frame exceeds bound"));
    }
    let prefix = payload_bytes.to_be_bytes();
    tokio::time::timeout(timeout, send.write_all(&prefix))
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process frame prefix write timed out"))?
        .map_err(iroh_error)?;
    tokio::time::timeout(timeout, send.write_all(payload))
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process frame payload write timed out"))?
        .map_err(iroh_error)?;
    send.finish().map_err(iroh_error)
}

async fn read_bounded_frame(
    receive: &mut iroh::endpoint::RecvStream,
    max_frame_bytes: u64,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut prefix = [0_u8; CROSS_PROCESS_FRAME_PREFIX_BYTES];
    tokio::time::timeout(timeout, receive.read_exact(&mut prefix))
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process frame prefix read timed out"))?
        .map_err(iroh_error)?;
    let payload_bytes = u64::from_be_bytes(prefix);
    if payload_bytes == 0 || payload_bytes > max_frame_bytes {
        return Err(MoltenError::invalid_harness("cross-process inbound frame exceeds bound"));
    }
    let payload_len = usize::try_from(payload_bytes)
        .map_err(|_| MoltenError::invalid_harness("cross-process frame size does not fit usize"))?;
    let mut payload = vec![0_u8; payload_len];
    tokio::time::timeout(timeout, receive.read_exact(&mut payload))
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process frame payload read timed out"))?
        .map_err(iroh_error)?;
    let trailing = tokio::time::timeout(timeout, receive.read_to_end(0))
        .await
        .map_err(|_| MoltenError::invalid_harness("cross-process frame terminal read timed out"))?
        .map_err(iroh_error)?;
    if !trailing.is_empty() {
        return Err(MoltenError::invalid_harness("cross-process frame has trailing bytes"));
    }
    Ok(payload)
}

fn finalize_successful_session(
    mut session: CrossProcessSessionState,
    role: EndpointParticipantRole,
    descriptor_ref: &str,
    session_ref: &str,
    request_ref: &str,
    remote_transport_identity_ref: &str,
    frame: NetworkFrame,
) -> Result<CrossProcessFrameEvidence> {
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Close)
        .map_err(|issues| shell_validation_error("cross-process session close", &issues))?
        .next;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginCleanup)
        .map_err(|issues| shell_validation_error("cross-process session cleanup", &issues))?
        .next;
    let cleanup_evidence_ref = cleanup_ref(session_ref, descriptor_ref, session.identity.generation);
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref: cleanup_evidence_ref.clone(),
    })
    .map_err(|issues| shell_validation_error("cross-process session cleanup completion", &issues))?
    .next;
    Ok(CrossProcessFrameEvidence {
        role,
        descriptor_ref: descriptor_ref.to_string(),
        session_ref: session_ref.to_string(),
        request_ref: request_ref.to_string(),
        payload_ref: frame.payload_ref,
        acknowledgement_ref: frame.acknowledgement_ref,
        remote_transport_identity_ref: remote_transport_identity_ref.to_string(),
        payload_bytes: frame.payload_bytes,
        delivery: session.delivery,
        retry: session.retry,
        automatic_retry_count: session.automatic_retry_count,
        terminal_class: session.terminal_class.unwrap_or(SessionTerminalClass::Clean),
        cleanup_evidence_ref,
    })
}

fn finalize_failed_session(
    mut session: CrossProcessSessionState,
    class: SessionTerminalClass,
) -> Result<CrossProcessSessionState> {
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::Fail {
        class,
        delivery_definitive: false,
    })
    .map_err(|issues| shell_validation_error("cross-process session failure", &issues))?
    .next;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginCleanup)
        .map_err(|issues| shell_validation_error("cross-process failed-session cleanup", &issues))?
        .next;
    let cleanup_evidence_ref =
        cleanup_ref(&session.identity.session_ref, &session.identity.descriptor_ref, session.identity.generation);
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref,
    })
    .map_err(|issues| shell_validation_error("cross-process failed-session cleanup completion", &issues))?
    .next;
    Ok(session)
}

fn validate_listener_shell_input(input: &IrohCrossProcessListenerInput) -> Result<()> {
    crate::preserves_rail::validate_content_ref(input.capability.capability_ref())?;
    crate::preserves_rail::validate_content_ref(&input.listener_identity_ref)?;
    crate::preserves_rail::validate_content_ref(&input.expected_peer_context_ref)?;
    crate::preserves_rail::validate_content_ref(&input.locator_cohort_ref)?;
    if !input.bind_addr.ip().is_loopback() {
        return Err(MoltenError::invalid_harness(
            "initial cross-process Iroh profile requires an explicit loopback bind address",
        ));
    }
    if !input.admission.registration_active
        || !input.admission.transport_capability_active
        || !input.admission.protocol_capability_active
        || !input.admission.profile_active
        || !input.admission.listener_ready
    {
        return Err(MoltenError::invalid_harness("cross-process listener capability admission is not fully active"));
    }
    Ok(())
}

fn validate_client_shell_input(input: &IrohCrossProcessClientInput, payload: &[u8]) -> Result<()> {
    crate::preserves_rail::validate_content_ref(input.capability.capability_ref())?;
    validate_exchange_refs(&input.session_ref, &input.request_ref)?;
    if payload.is_empty() {
        return Err(MoltenError::invalid_harness("cross-process payload must not be empty"));
    }
    if !input.bind_addr.ip().is_loopback() {
        return Err(MoltenError::invalid_harness(
            "initial cross-process Iroh client profile requires an explicit loopback bind address",
        ));
    }
    Ok(())
}

fn validate_exchange_refs(session_ref: &str, request_ref: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(session_ref)?;
    crate::preserves_rail::validate_content_ref(request_ref)
}

fn endpoint_locators(endpoint_addr: &iroh::EndpointAddr) -> Result<Vec<EndpointLocator>> {
    let mut locators = Vec::new();
    for address in &endpoint_addr.addrs {
        let class = match address {
            iroh::TransportAddr::Ip(_) => EndpointLocatorClass::Ip,
            iroh::TransportAddr::Relay(_) => EndpointLocatorClass::Relay,
            iroh::TransportAddr::Custom(_) => {
                return Err(MoltenError::invalid_harness(
                    "custom Iroh transport addresses are outside the admitted profile",
                ));
            }
            _ => {
                return Err(MoltenError::invalid_harness(
                    "unknown Iroh transport address is outside the admitted profile",
                ));
            }
        };
        if locators.len() >= MAX_ENDPOINT_LOCATORS {
            return Err(MoltenError::invalid_harness("Iroh endpoint locator count exceeds bound"));
        }
        locators.push(EndpointLocator {
            class,
            value: address.to_string(),
        });
    }
    Ok(locators)
}

fn iroh_endpoint_addr(plan: &EndpointDialPlan) -> Result<iroh::EndpointAddr> {
    let endpoint_id = plan
        .public_endpoint_identity
        .strip_prefix("iroh:")
        .ok_or_else(|| MoltenError::invalid_harness("cross-process endpoint identity must use iroh prefix"))?
        .parse::<iroh::EndpointId>()
        .map_err(iroh_error)?;
    let mut addresses = Vec::with_capacity(plan.locators.len());
    for locator in &plan.locators {
        let address = match locator.class {
            EndpointLocatorClass::Ip => {
                let address = locator
                    .value
                    .strip_prefix("ip:")
                    .ok_or_else(|| MoltenError::invalid_harness("cross-process IP locator prefix mismatch"))?
                    .parse::<SocketAddr>()
                    .map_err(iroh_error)?;
                iroh::TransportAddr::Ip(address)
            }
            EndpointLocatorClass::Relay => {
                let relay = locator
                    .value
                    .strip_prefix("relay:")
                    .ok_or_else(|| MoltenError::invalid_harness("cross-process relay locator prefix mismatch"))?
                    .parse::<iroh::RelayUrl>()
                    .map_err(iroh_error)?;
                iroh::TransportAddr::Relay(relay)
            }
            EndpointLocatorClass::Custom | EndpointLocatorClass::Private => {
                return Err(MoltenError::invalid_harness(
                    "cross-process endpoint contains an unsupported locator class",
                ));
            }
        };
        addresses.push(address);
    }
    Ok(iroh::EndpointAddr::from_parts(endpoint_id, addresses))
}

fn dial_plan_from_descriptor(descriptor: &CrossProcessEndpointDescriptor) -> EndpointDialPlan {
    EndpointDialPlan {
        descriptor_ref: descriptor.descriptor_ref.clone(),
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        locators: descriptor.locators.clone(),
        profile_id: descriptor.profile_id.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        alpn: descriptor.alpn.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        peer_context_ref: descriptor.expected_peer_context_ref.clone(),
        resources: descriptor.resources.clone(),
    }
}

pub fn cross_process_frame_ref(request_ref: &str, payload: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(FRAME_DOMAIN.as_bytes());
    hasher.update(request_ref.as_bytes());
    hasher.update(payload);
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn cleanup_ref(identity_ref: &str, descriptor_ref: &str, generation: u64) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CLEANUP_DOMAIN.as_bytes());
    hasher.update(identity_ref.as_bytes());
    hasher.update(descriptor_ref.as_bytes());
    hasher.update(&generation.to_be_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn iroh_error(error: impl fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("cross-process Iroh transport failed: {error}"))
}

fn shell_validation_error(label: &str, issues: &impl fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}
