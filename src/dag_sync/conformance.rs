use molten_core::dag_sync::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::*;

const DAG_TRANSPORT_PROFILE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DAG_TRANSPORT_FRAMING_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DAG_TRANSPORT_AUTHORITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DAG_TRANSPORT_OPERATION_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const DAG_TRANSPORT_SESSION_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DAG_TRANSPORT_STREAM_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const DAG_TRANSPORT_PEER_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const DAG_TRANSPORT_MEMBERSHIP_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const DAG_TRANSPORT_PRINCIPAL_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const DAG_TRANSPORT_TRUST_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const DAG_TRANSPORT_CAPABILITY_REF: &str = "blake3:5555555555555555555555555555555555555555555555555555555555555555";
const DAG_TRANSPORT_ALPN: &str = "molten/dag-sync/1";
const DAG_TRANSPORT_SERVICE: &str = "dag-sync";
const DAG_TRANSPORT_PROTOCOL: &str = "dag-sync-v1";
const DAG_TRANSPORT_EXTENSION: &str = "dag-sync-system-extension";
const DAG_TRANSPORT_GENERATION: u64 = 1;
const DAG_TRANSPORT_PROFILE_LIMIT: u64 = 16;
const DAG_TRANSPORT_FRAME_LIMIT: u64 = 4_096;
const DAG_TRANSPORT_DATAGRAM_LIMIT: u64 = 1_024;
const DAG_TRANSPORT_QUEUE_BYTE_LIMIT: u64 = 16_384;
const DAG_TRANSPORT_INFLIGHT_LIMIT: u64 = 8_192;
const DAG_TRANSPORT_DEADLINE_WINDOW: u64 = 64;
const DAG_TRANSPORT_INITIAL_TICK: u64 = 1;
const DAG_TRANSPORT_LENGTH_PREFIX_BYTES: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagTransportFixtureKind {
    DeterministicSimulation,
    IrohLiveLoopback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagTransportFixtureFault {
    CancelAt { sequence: usize },
    PartitionAt { sequence: usize },
}

enum DagTransportMechanism {
    Deterministic(DeterministicTransportAdapter),
    IrohLive {
        adapter: IrohTransportAdapter,
        runtime: tokio::runtime::Runtime,
    },
}

pub struct DagFabricTransportAdapter {
    mechanism: DagTransportMechanism,
    fault: Option<DagTransportFixtureFault>,
    session_id: ScopedTransportId,
    stream_id: ScopedTransportId,
    request_count: usize,
}

impl DagFabricTransportAdapter {
    pub fn open(kind: DagTransportFixtureKind, fault: Option<DagTransportFixtureFault>) -> Result<Self> {
        let mechanism = match kind {
            DagTransportFixtureKind::DeterministicSimulation => {
                let profile = dag_transport_profile(TransportAdapterKind::DeterministicSimulation)?;
                DagTransportMechanism::Deterministic(DeterministicTransportAdapter::new(profile)?)
            }
            DagTransportFixtureKind::IrohLiveLoopback => {
                if fault.is_some() {
                    return Err(MoltenError::invalid_harness("live DAG loopback does not synthesize transport faults"));
                }
                let profile = dag_transport_profile(TransportAdapterKind::IrohLive)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| MoltenError::invalid_harness(format!("DAG Iroh runtime failed: {error}")))?;
                DagTransportMechanism::IrohLive {
                    adapter: IrohTransportAdapter::new(profile)?,
                    runtime,
                }
            }
        };
        let mut adapter = Self {
            mechanism,
            fault,
            session_id: scoped_id(DAG_TRANSPORT_SESSION_REF),
            stream_id: scoped_id(DAG_TRANSPORT_STREAM_REF),
            request_count: 0,
        };
        adapter.apply_setup()?;
        Ok(adapter)
    }

    pub const fn request_count(&self) -> usize {
        self.request_count
    }

    fn apply_setup(&mut self) -> Result<()> {
        for command in dag_setup_commands() {
            self.execute_command(&command)?;
        }
        Ok(())
    }

    fn execute_command(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition> {
        match &mut self.mechanism {
            DagTransportMechanism::Deterministic(adapter) => adapter
                .execute_command(command)
                .map_err(|error| MoltenError::invalid_harness(format!("simulated DAG transport failed: {error}"))),
            DagTransportMechanism::IrohLive { adapter, .. } => adapter
                .execute_command(command)
                .map_err(|error| MoltenError::invalid_harness(format!("live DAG transport failed: {error}"))),
        }
    }

    fn cancel(&mut self) -> Result<DagTransferOutcome> {
        let command = TransportCommand::Cancel {
            operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
            target: CancelTarget::Stream {
                session_id: self.session_id.clone(),
                stream_id: self.stream_id.clone(),
            },
        };
        let transition = self.execute_command(&command)?;
        Ok(DagTransferOutcome::Cancelled(transition.transition_ref))
    }

    fn partition(&mut self, request: &DagFetchRequest) -> Result<DagTransferOutcome> {
        let command = self.send_command(request)?;
        let DagTransportMechanism::Deterministic(adapter) = &mut self.mechanism else {
            return Err(MoltenError::invalid_harness(
                "partition injection requires the deterministic transport adapter",
            ));
        };
        let transition = adapter.execute_with_fault(&command, Some(SimulatedTransportFault::Partition))?;
        Ok(DagTransferOutcome::Deferred(transition.transition_ref))
    }

    fn transfer(&mut self, request: &DagFetchRequest) -> Result<DagTransferOutcome> {
        let payload = dag_request_payload(request);
        let encoded_bytes = u64::try_from(payload.len())
            .map_err(|_| MoltenError::invalid_harness("DAG transport payload length exceeds u64"))?;
        let observation_ref = match &mut self.mechanism {
            DagTransportMechanism::Deterministic(adapter) => {
                let send = send_command(&self.session_id, &self.stream_id, request, &payload)?;
                let _submitted = adapter
                    .execute_command(&send)
                    .map_err(|error| MoltenError::invalid_harness(format!("simulated DAG send failed: {error}")))?;
                let acknowledged = adapter
                    .execute_command(&TransportCommand::AcknowledgeFrame {
                        operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
                        session_id: self.session_id.clone(),
                        stream_id: self.stream_id.clone(),
                        payload_bytes: encoded_bytes,
                    })
                    .map_err(|error| MoltenError::invalid_harness(format!("simulated DAG ack failed: {error}")))?;
                acknowledged.transition_ref
            }
            DagTransportMechanism::IrohLive { adapter, runtime } => {
                runtime
                    .block_on(adapter.live_loopback_frame(
                        &self.session_id,
                        &self.stream_id,
                        DAG_TRANSPORT_OPERATION_REF,
                        DAG_TRANSPORT_ALPN,
                        &payload,
                        observed_tick(request.sequence)?,
                    ))?
                    .acknowledged
                    .transition_ref
            }
        };
        Ok(DagTransferOutcome::Received(DagTransportEnvelope {
            object_ref: request.object_ref.clone(),
            assigned_peer: request.assigned_peer.clone(),
            encoded_bytes,
            transport_observation_ref: observation_ref,
        }))
    }

    fn send_command(&self, request: &DagFetchRequest) -> Result<TransportCommand> {
        send_command(&self.session_id, &self.stream_id, request, &dag_request_payload(request))
    }
}

impl DagTransportPort for DagFabricTransportAdapter {
    fn request(&mut self, request: &DagFetchRequest) -> Result<DagTransferOutcome> {
        self.request_count = self
            .request_count
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("DAG transport request count overflow"))?;
        match self.fault {
            Some(DagTransportFixtureFault::CancelAt { sequence }) if sequence == request.sequence => self.cancel(),
            Some(DagTransportFixtureFault::PartitionAt { sequence }) if sequence == request.sequence => {
                self.partition(request)
            }
            _ => self.transfer(request),
        }
    }
}

fn dag_transport_profile(kind: TransportAdapterKind) -> Result<CanonicalTransportProfile> {
    canonical_transport_profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: format!("dag-sync-{}", kind.as_str()),
        profile_ref: DAG_TRANSPORT_PROFILE_REF.to_string(),
        adapter_kind: kind,
        capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
            TransportCapability::Datagrams,
        ],
        limits: TransportLimits {
            max_listeners: DAG_TRANSPORT_PROFILE_LIMIT,
            max_sessions: DAG_TRANSPORT_PROFILE_LIMIT,
            max_streams_per_session: DAG_TRANSPORT_PROFILE_LIMIT,
            max_frame_bytes: DAG_TRANSPORT_FRAME_LIMIT,
            max_datagram_bytes: DAG_TRANSPORT_DATAGRAM_LIMIT,
            max_queued_events: DAG_TRANSPORT_PROFILE_LIMIT,
            max_queued_bytes: DAG_TRANSPORT_QUEUE_BYTE_LIMIT,
            max_inflight_bytes: DAG_TRANSPORT_INFLIGHT_LIMIT,
            operation_deadline_ticks: DAG_TRANSPORT_DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}

fn dag_protocol_descriptor() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: DAG_TRANSPORT_PROTOCOL.to_string(),
        version: "v1".to_string(),
        alpn: DAG_TRANSPORT_ALPN.to_string(),
        extension_id: DAG_TRANSPORT_EXTENSION.to_string(),
        service_id: DAG_TRANSPORT_SERVICE.to_string(),
        generation: DAG_TRANSPORT_GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![TransportCapability::BidirectionalStreams],
        framing: FramingProfile {
            profile_id: "dag-sync-length-delimited-blake3-v1".to_string(),
            profile_ref: DAG_TRANSPORT_FRAMING_REF.to_string(),
            max_frame_bytes: DAG_TRANSPORT_FRAME_LIMIT,
            length_prefix_bytes: DAG_TRANSPORT_LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DAG_TRANSPORT_DEADLINE_WINDOW,
        },
        registration_authority_ref: DAG_TRANSPORT_AUTHORITY_REF.to_string(),
        profile_ref: DAG_TRANSPORT_PROFILE_REF.to_string(),
    }
}

fn dag_setup_commands() -> Vec<TransportCommand> {
    let descriptor = dag_protocol_descriptor();
    vec![
        TransportCommand::Register {
            operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
            descriptor: descriptor.clone(),
        },
        TransportCommand::OpenSession {
            operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
            session_id: scoped_id(DAG_TRANSPORT_SESSION_REF),
            alpn: descriptor.alpn,
            direction: SessionDirection::Outbound,
            peer: dag_peer(),
            observed_tick: DAG_TRANSPORT_INITIAL_TICK,
            deadline_tick: DAG_TRANSPORT_INITIAL_TICK.saturating_add(DAG_TRANSPORT_DEADLINE_WINDOW),
        },
        TransportCommand::OpenStream {
            operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
            session_id: scoped_id(DAG_TRANSPORT_SESSION_REF),
            stream_id: scoped_id(DAG_TRANSPORT_STREAM_REF),
            direction: StreamDirection::Bidirectional,
            initial_credit_bytes: DAG_TRANSPORT_INFLIGHT_LIMIT,
        },
    ]
}

fn dag_peer() -> PeerIdentityRefs {
    PeerIdentityRefs {
        transport_identity_ref: DAG_TRANSPORT_PEER_REF.to_string(),
        membership_ref: Some(DAG_TRANSPORT_MEMBERSHIP_REF.to_string()),
        application_principal_ref: Some(DAG_TRANSPORT_PRINCIPAL_REF.to_string()),
        trust_decision_ref: Some(DAG_TRANSPORT_TRUST_REF.to_string()),
        capability_authority_ref: Some(DAG_TRANSPORT_CAPABILITY_REF.to_string()),
        bootstrap_policy_ref: None,
    }
}

fn scoped_id(opaque_ref: &str) -> ScopedTransportId {
    ScopedTransportId {
        opaque_ref: opaque_ref.to_string(),
        service_id: DAG_TRANSPORT_SERVICE.to_string(),
        generation: DAG_TRANSPORT_GENERATION,
    }
}

fn send_command(
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    request: &DagFetchRequest,
    payload: &[u8],
) -> Result<TransportCommand> {
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("DAG transport payload length exceeds u64"))?;
    Ok(TransportCommand::SendFrame {
        operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
        session_id: session_id.clone(),
        stream_id: stream_id.clone(),
        payload_ref: format!("blake3:{}", blake3::hash(payload).to_hex()),
        payload_bytes,
        observed_tick: observed_tick(request.sequence)?,
    })
}

fn observed_tick(sequence: usize) -> Result<u64> {
    let sequence =
        u64::try_from(sequence).map_err(|_| MoltenError::invalid_harness("DAG request sequence exceeds u64"))?;
    Ok(DAG_TRANSPORT_INITIAL_TICK.saturating_add(sequence))
}

fn dag_request_payload(request: &DagFetchRequest) -> Vec<u8> {
    format!(
        "{}:{}:{}:{}",
        request.object_ref.kind(),
        request.object_ref.as_str(),
        request.assigned_peer.as_ref().map_or("unassigned", DagPeerId::as_str),
        request.sequence
    )
    .into_bytes()
}
