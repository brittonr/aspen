use std::collections::BTreeMap;
use std::time::Duration;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::CrossProcessFrameEvidence;
use crate::fabric_transport::DeliveryOutcome;
use crate::fabric_transport::IrohCrossProcessClientInput;
use crate::fabric_transport::IrohCrossProcessListener;
use crate::fabric_transport::exchange_cross_process_frame;

#[derive(Debug)]
pub struct IrohReplicaTransportPort {
    protocol_ref: String,
    peers: BTreeMap<String, IrohCrossProcessClientInput>,
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedReplicaEvent {
    pub event: ReplicaEvent,
    pub transport_evidence: CrossProcessFrameEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaTransportRefs {
    pub request_ref: String,
}

pub const MAX_REPLICA_INGRESS_EVENTS: usize = 256;
pub const MAX_REPLICA_INGRESS_DELIVERIES: u64 = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohReplicaIngressConfig {
    pub session_ref: String,
    pub accept_timeout: Duration,
    pub event_capacity: usize,
    pub delivery_limit: u64,
}

pub trait ReplicaListenerSource {
    fn admitted_listener(&mut self) -> Result<&mut IrohCrossProcessListener>;
}

impl ReplicaListenerSource for IrohCrossProcessListener {
    fn admitted_listener(&mut self) -> Result<&mut IrohCrossProcessListener> {
        Ok(self)
    }
}

impl ReplicaListenerSource for Option<IrohCrossProcessListener> {
    fn admitted_listener(&mut self) -> Result<&mut IrohCrossProcessListener> {
        self.as_mut()
            .ok_or_else(|| MoltenError::invalid_harness("live Raft listener is detached from its owner"))
    }
}

pub struct IrohReplicaIngressPump {
    events: tokio::sync::mpsc::Receiver<ReceivedReplicaEvent>,
    cancel: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<IrohCrossProcessListener>>>,
}

impl IrohReplicaTransportPort {
    pub fn new(
        protocol_ref: String,
        peers: BTreeMap<String, IrohCrossProcessClientInput>,
        timeout: Duration,
    ) -> Result<Self> {
        crate::preserves_rail::validate_content_ref(&protocol_ref)?;
        if peers.is_empty() {
            return Err(MoltenError::invalid_harness("live Raft Iroh transport requires at least one admitted peer"));
        }
        if timeout.is_zero() {
            return Err(MoltenError::invalid_harness("live Raft Iroh transport timeout must be positive"));
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

impl IrohReplicaIngressPump {
    pub fn spawn(listener: IrohCrossProcessListener, config: IrohReplicaIngressConfig) -> Result<Self> {
        validate_ingress_config(&config)?;
        let (event_sender, events) = tokio::sync::mpsc::channel(config.event_capacity);
        let (cancel, mut cancellation) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut listener = listener;
            for _delivery in 0..config.delivery_limit {
                tokio::select! {
                    _ = &mut cancellation => return Ok(listener),
                    result = receive_replica_event(&mut listener, &config.session_ref, config.accept_timeout) => {
                        let event = result?;
                        event_sender
                            .send(event)
                            .await
                            .map_err(|_| MoltenError::invalid_harness("live Raft ingress consumer closed"))?;
                    }
                }
            }
            Err(MoltenError::invalid_harness("live Raft ingress exhausted its delivery limit"))
        });
        Ok(Self {
            events,
            cancel: Some(cancel),
            task: Some(task),
        })
    }

    pub async fn next(&mut self) -> Result<ReceivedReplicaEvent> {
        self.events
            .recv()
            .await
            .ok_or_else(|| MoltenError::invalid_harness("live Raft ingress pump terminated before delivery"))
    }

    pub async fn shutdown(mut self) -> Result<IrohCrossProcessListener> {
        let cancel = self
            .cancel
            .take()
            .ok_or_else(|| MoltenError::invalid_harness("live Raft ingress cancellation handle is absent"))?;
        let _cancel_result = cancel.send(());
        let task = self
            .task
            .take()
            .ok_or_else(|| MoltenError::invalid_harness("live Raft ingress task handle is absent"))?;
        task.await
            .map_err(|error| MoltenError::invalid_harness(format!("live Raft ingress task failed: {error}")))?
    }
}

impl Drop for IrohReplicaIngressPump {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _cancel_result = cancel.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl ReplicaTransportEffects for IrohReplicaTransportPort {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        let prepared = prepare_send(&self.peers, self.timeout, envelope);
        Box::pin(async move {
            let (input, message, timeout) = prepared?;
            let evidence = exchange_cross_process_frame(input, &message.bytes, timeout).await?;
            if evidence.delivery != DeliveryOutcome::Delivered {
                return Err(MoltenError::invalid_harness(
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
    timeout: Duration,
) -> Result<ReceivedReplicaEvent> {
    let listener = listener.admitted_listener()?;
    let received = listener.accept_one_derived_frame(session_ref, timeout, derive_replica_request_ref).await?;
    let envelope = parse_canonical_replica_message(&received.payload)?;
    let refs = replica_transport_refs(&envelope)?;
    let payload_bytes = u64::try_from(received.payload.len())
        .map_err(|_| MoltenError::invalid_harness("live Raft received payload length exceeds u64"))?;
    if received.evidence.request_ref != refs.request_ref || received.evidence.payload_bytes != payload_bytes {
        return Err(MoltenError::invalid_harness("live Raft received payload shape does not match transport evidence"));
    }
    Ok(ReceivedReplicaEvent {
        event: ReplicaEvent::Message { envelope },
        transport_evidence: received.evidence,
    })
}

pub fn replica_transport_refs(envelope: &ReplicaMessageEnvelope) -> Result<ReplicaTransportRefs> {
    Ok(ReplicaTransportRefs {
        request_ref: canonical_replica_message(envelope)?.envelope_ref,
    })
}

fn prepare_send(
    peers: &BTreeMap<String, IrohCrossProcessClientInput>,
    timeout: Duration,
    envelope: &ReplicaMessageEnvelope,
) -> Result<(IrohCrossProcessClientInput, CanonicalReplicaMessage, Duration)> {
    let message = canonical_replica_message(envelope)?;
    let mut input = peers
        .get(&envelope.to)
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("live Raft Iroh transport has no admitted endpoint for peer"))?;
    input.request_ref.clone_from(&message.envelope_ref);
    Ok((input, message, timeout))
}

fn derive_replica_request_ref(payload: &[u8]) -> Result<String> {
    let envelope = parse_canonical_replica_message(payload)?;
    Ok(canonical_replica_message(&envelope)?.envelope_ref)
}

pub(super) fn validate_ingress_config(config: &IrohReplicaIngressConfig) -> Result<()> {
    crate::preserves_rail::validate_content_ref(&config.session_ref)?;
    if config.accept_timeout.is_zero() {
        return Err(MoltenError::invalid_harness("live Raft ingress timeout must be positive"));
    }
    if config.event_capacity == 0 || config.event_capacity > MAX_REPLICA_INGRESS_EVENTS {
        return Err(MoltenError::invalid_harness("live Raft ingress capacity is outside its static bound"));
    }
    if config.delivery_limit == 0 || config.delivery_limit > MAX_REPLICA_INGRESS_DELIVERIES {
        return Err(MoltenError::invalid_harness("live Raft ingress delivery limit is outside its static bound"));
    }
    Ok(())
}

fn validate_peer_id(peer: &str) -> Result<()> {
    if peer.is_empty()
        || !peer.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(MoltenError::invalid_harness("live Raft Iroh peer id is empty or malformed"));
    }
    Ok(())
}
