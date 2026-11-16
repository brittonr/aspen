mod pump;

use super::*;

#[derive(Debug)]
pub struct ReplicaTransportPort {
    protocol_ref: String,
    peers: std::collections::BTreeMap<String, crate::fabric_transport::ClientInput>,
    timeout: std::time::Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedReplicaEvent {
    pub event: ReplicaEvent,
    pub transport_evidence: crate::fabric_transport::FrameEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaTransportRefs {
    pub request_ref: String,
}

pub const MAX_REPLICA_INGRESS_EVENTS: usize = 256;
pub const MAX_REPLICA_INGRESS_DELIVERIES: u64 = 65_536;
pub const MAX_REPLICA_INGRESS_DELIVERY_WINDOW: std::time::Duration = std::time::Duration::from_secs(3_600);

/// Ingress admits at most `delivery_limit` deliveries per `delivery_window`: a rate, not a lifetime
/// total. A spent budget pauses polling until the window ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaIngressConfig {
    pub session_ref: String,
    pub accept_timeout: std::time::Duration,
    pub event_capacity: usize,
    pub delivery_limit: u64,
    pub delivery_window: std::time::Duration,
}

/// One bounded payload poll: a decoded event, idle deadline, or recoverable decode failure.
/// A decode failure ends the frame's session without acknowledgement. The frame source's outer
/// result separately carries fatal transport or listener failures; the decoded event stays inline.
pub type PolledReplicaFrame = std::result::Result<Option<ReceivedReplicaEvent>, crate::error::Failure>;

pub type ReplicaFrameFuture<'a> =
    std::pin::Pin<Box<dyn Future<Output = crate::error::Result<PolledReplicaFrame>> + Send + 'a>>;

/// A source of peer frames for the ingress pump. An outer `Err` is a transport or listener failure
/// and ends ingress; an inner `Err` is an undecodable payload that the pump drops and counts.
pub trait ReplicaFrameSource: Send + 'static {
    fn poll_frame<'a>(&'a mut self, session_ref: &'a str, timeout: std::time::Duration) -> ReplicaFrameFuture<'a>;
}

impl ReplicaFrameSource for crate::fabric_transport::Listener {
    fn poll_frame<'a>(&'a mut self, session_ref: &'a str, timeout: std::time::Duration) -> ReplicaFrameFuture<'a> {
        Box::pin(poll_listener_frame(self, session_ref, timeout))
    }
}

pub trait ReplicaListenerSource {
    fn admitted_listener(&mut self) -> crate::error::Result<&mut crate::fabric_transport::Listener>;
}

impl ReplicaListenerSource for crate::fabric_transport::Listener {
    fn admitted_listener(&mut self) -> crate::error::Result<&mut crate::fabric_transport::Listener> {
        Ok(self)
    }
}

impl ReplicaListenerSource for Option<crate::fabric_transport::Listener> {
    fn admitted_listener(&mut self) -> crate::error::Result<&mut crate::fabric_transport::Listener> {
        self.as_mut()
            .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft listener is detached from its owner"))
    }
}

pub struct ReplicaIngressPump<S: ReplicaFrameSource = crate::fabric_transport::Listener> {
    events: tokio::sync::mpsc::Receiver<ReceivedReplicaEvent>,
    cancel: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<crate::error::Result<S>>>,
    dropped_frames: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl ReplicaTransportPort {
    pub fn new(
        protocol_ref: String,
        peers: std::collections::BTreeMap<String, crate::fabric_transport::ClientInput>,
        timeout: std::time::Duration,
    ) -> crate::error::Result<Self> {
        crate::preserves_rail::validate_content_ref(&protocol_ref)?;
        if peers.is_empty() {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft Iroh transport requires at least one admitted peer",
            ));
        }
        if timeout.is_zero() {
            return Err(crate::error::Failure::invalid_harness("live Raft Iroh transport timeout must be positive"));
        }
        for peer in peers.keys() {
            validate_peer_id(peer)?;
        }
        Ok(Self {
            protocol_ref,
            peers,
            timeout,
        })
    }

    pub fn protocol_ref(&self) -> &str {
        &self.protocol_ref
    }
}

impl ReplicaIngressPump {
    pub fn spawn(
        listener: crate::fabric_transport::Listener,
        config: ReplicaIngressConfig,
    ) -> crate::error::Result<Self> {
        Self::spawn_source(listener, config)
    }
}

impl ReplicaTransportEffects for ReplicaTransportPort {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        let prepared = prepare_send(&self.peers, self.timeout, envelope);
        Box::pin(async move {
            let (input, message, timeout) = prepared?;
            let evidence = crate::fabric_transport::exchange_frame(input, &message.bytes, timeout).await?;
            if evidence.delivery != crate::fabric_transport::DeliveryOutcome::Delivered {
                return Err(crate::error::Failure::invalid_harness(
                    "live Raft Iroh transport completed without delivered frame evidence",
                ));
            }
            Ok(evidence.acknowledgement_ref)
        })
    }
}

// r[impl molten.fabric_consistency.live_service_ports]
pub async fn receive_replica_event<L: ReplicaListenerSource>(
    listener: &mut L,
    session_ref: &str,
    timeout: std::time::Duration,
) -> crate::error::Result<ReceivedReplicaEvent> {
    poll_replica_event(listener, session_ref, timeout)
        .await?
        .ok_or_else(|| crate::error::Failure::invalid_harness("cross-process listener accept timed out"))
}

async fn poll_replica_event<L: ReplicaListenerSource>(
    listener: &mut L,
    session_ref: &str,
    timeout: std::time::Duration,
) -> crate::error::Result<Option<ReceivedReplicaEvent>> {
    let listener = listener.admitted_listener()?;
    let Some(received) = listener.poll_one_derived_frame(session_ref, timeout, derive_replica_request_ref).await?
    else {
        return Ok(None);
    };
    received_event(received).map(Some)
}

/// Poll one listener session, separating a payload that fails to decode from transport failures.
async fn poll_listener_frame(
    listener: &mut crate::fabric_transport::Listener,
    session_ref: &str,
    timeout: std::time::Duration,
) -> crate::error::Result<PolledReplicaFrame> {
    let mut is_undecodable = false;
    let derive = |payload: &[u8]| derive_replica_request_ref(payload).inspect_err(|_error| is_undecodable = true);
    match listener.poll_one_derived_frame(session_ref, timeout, derive).await {
        Ok(None) => Ok(Ok(None)),
        Ok(Some(received)) => received_event(received).map(|event| Ok(Some(event))),
        Err(error) if is_undecodable => Ok(Err(error)),
        Err(error) => Err(error),
    }
}

fn received_event(received: crate::fabric_transport::ReceivedFrame) -> crate::error::Result<ReceivedReplicaEvent> {
    let envelope = parse_replica_message(&received.payload)?;
    let refs = replica_transport_refs(&envelope)?;
    let payload_bytes = u64::try_from(received.payload.len())
        .map_err(|_| crate::error::Failure::invalid_harness("live Raft received payload length exceeds u64"))?;
    if received.evidence.request_ref != refs.request_ref || received.evidence.payload_bytes != payload_bytes {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft received payload shape does not match transport evidence",
        ));
    }
    Ok(ReceivedReplicaEvent {
        event: ReplicaEvent::Message { envelope },
        transport_evidence: received.evidence,
    })
}

pub fn replica_transport_refs(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<ReplicaTransportRefs> {
    Ok(ReplicaTransportRefs {
        request_ref: replica_message(envelope)?.envelope_ref,
    })
}

fn prepare_send(
    peers: &std::collections::BTreeMap<String, crate::fabric_transport::ClientInput>,
    timeout: std::time::Duration,
    envelope: &ReplicaMessageEnvelope,
) -> crate::error::Result<(crate::fabric_transport::ClientInput, ReplicaMessage, std::time::Duration)> {
    let message = replica_message(envelope)?;
    let mut input = peers.get(&envelope.to).cloned().ok_or_else(|| {
        crate::error::Failure::invalid_harness("live Raft Iroh transport has no admitted endpoint for peer")
    })?;
    input.request_ref.clone_from(&message.envelope_ref);
    Ok((input, message, timeout))
}

fn derive_replica_request_ref(payload: &[u8]) -> crate::error::Result<String> {
    let envelope = parse_replica_message(payload)?;
    Ok(replica_message(&envelope)?.envelope_ref)
}

pub(super) fn validate_ingress_config(config: &ReplicaIngressConfig) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(&config.session_ref)?;
    if config.accept_timeout.is_zero() {
        return Err(crate::error::Failure::invalid_harness("live Raft ingress timeout must be positive"));
    }
    if config.event_capacity == 0 || config.event_capacity > MAX_REPLICA_INGRESS_EVENTS {
        return Err(crate::error::Failure::invalid_harness("live Raft ingress capacity is outside its static bound"));
    }
    if config.delivery_limit == 0 || config.delivery_limit > MAX_REPLICA_INGRESS_DELIVERIES {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft ingress delivery limit is outside its static bound",
        ));
    }
    if config.delivery_window.is_zero() || config.delivery_window > MAX_REPLICA_INGRESS_DELIVERY_WINDOW {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft ingress delivery window is outside its static bound",
        ));
    }
    Ok(())
}

fn validate_peer_id(peer: &str) -> crate::error::Result<()> {
    if peer.is_empty()
        || !peer.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(crate::error::Failure::invalid_harness("live Raft Iroh peer id is empty or malformed"));
    }
    Ok(())
}
