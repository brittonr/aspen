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

impl IrohReplicaTransportPort {
    pub fn new(peers: BTreeMap<String, IrohCrossProcessClientInput>, timeout: Duration) -> Result<Self> {
        if peers.is_empty() {
            return Err(MoltenError::invalid_harness("live Raft Iroh transport requires at least one admitted peer"));
        }
        if timeout.is_zero() {
            return Err(MoltenError::invalid_harness("live Raft Iroh transport timeout must be positive"));
        }
        for peer in peers.keys() {
            validate_peer_id(peer)?;
        }
        Ok(Self { peers, timeout })
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
pub async fn receive_replica_event(
    listener: &mut IrohCrossProcessListener,
    session_ref: &str,
    timeout: Duration,
) -> Result<ReceivedReplicaEvent> {
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

fn validate_peer_id(peer: &str) -> Result<()> {
    if peer.is_empty()
        || !peer.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(MoltenError::invalid_harness("live Raft Iroh peer id is empty or malformed"));
    }
    Ok(())
}
