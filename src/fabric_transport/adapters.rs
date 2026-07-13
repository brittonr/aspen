use std::collections::BTreeMap;
use std::time::Duration;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

const LIVE_LOOPBACK_TIMEOUT_SECONDS: u64 = 10;
const IROH_CLOSE_CODE: u8 = 0;
const IROH_CLOSE_REASON: &[u8] = b"fabric-transport-loopback-complete";

pub trait TransportCommandShell {
    fn profile_id(&self) -> &str;
    fn execute_command(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulatedTransportFault {
    LocalOverload,
    RemoteRefusal,
    Partition,
    Timeout,
    DisconnectAfterSubmission,
    AdapterFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicTransportAdapter {
    profile: CanonicalTransportProfile,
    state: TransportState,
    latest_evidence_ref: Option<String>,
}

impl DeterministicTransportAdapter {
    // r[impl molten.fabric_transport.live_sim_parity]
    pub fn new(profile: CanonicalTransportProfile) -> Result<Self> {
        if profile.profile.adapter_kind != TransportAdapterKind::DeterministicSimulation {
            return Err(MoltenError::invalid_harness(
                "deterministic transport adapter requires a deterministic-simulation profile",
            ));
        }
        Ok(Self {
            profile,
            state: TransportState::default(),
            latest_evidence_ref: None,
        })
    }

    pub fn state(&self) -> &TransportState {
        &self.state
    }

    pub fn profile(&self) -> &CanonicalTransportProfile {
        &self.profile
    }

    pub fn status(&self) -> Result<TransportStatusReadback> {
        transport_status_readback(&self.profile, &self.state, self.latest_evidence_ref.as_deref())
    }

    // r[impl molten.fabric_transport.failure_semantics]
    pub fn execute_with_fault(
        &mut self,
        command: &TransportCommand,
        fault: Option<SimulatedTransportFault>,
    ) -> Result<CanonicalTransportTransition> {
        match fault {
            None => self.execute(command),
            Some(SimulatedTransportFault::LocalOverload) => {
                Err(MoltenError::invalid_harness("simulated transport overload before adapter I/O"))
            }
            Some(SimulatedTransportFault::RemoteRefusal) => {
                self.fail_for_command(command, TransportFailureClass::RemoteRefusal, true)
            }
            Some(SimulatedTransportFault::Partition) => {
                self.fail_for_command(command, TransportFailureClass::Partition, false)
            }
            Some(SimulatedTransportFault::Timeout) => {
                self.fail_for_command(command, TransportFailureClass::Timeout, false)
            }
            Some(SimulatedTransportFault::DisconnectAfterSubmission) => {
                let _submitted = self.execute(command)?;
                let session_id = command_session_id(command).cloned().ok_or_else(|| {
                    MoltenError::invalid_harness("disconnect-after-submission requires a session command")
                })?;
                self.fail_session(command.operation_id(), &session_id, TransportFailureClass::Disconnect, false)
            }
            Some(SimulatedTransportFault::AdapterFailure) => {
                self.fail_for_command(command, TransportFailureClass::AdapterFailure, false)
            }
        }
    }

    fn execute(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition> {
        let transition = apply_transport_command(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("simulated transport command", &issues))?;
        self.apply_transition(transition)
    }

    fn fail_for_command(
        &mut self,
        command: &TransportCommand,
        class: TransportFailureClass,
        delivery_definitive: bool,
    ) -> Result<CanonicalTransportTransition> {
        let session_id = command_session_id(command)
            .ok_or_else(|| MoltenError::invalid_harness("simulated transport fault requires a session command"))?;
        self.fail_session(command.operation_id(), session_id, class, delivery_definitive)
    }

    fn fail_session(
        &mut self,
        operation_id: &str,
        session_id: &ScopedTransportId,
        class: TransportFailureClass,
        delivery_definitive: bool,
    ) -> Result<CanonicalTransportTransition> {
        let command = TransportCommand::FailSession {
            operation_id: operation_id.to_string(),
            session_id: session_id.clone(),
            class,
            delivery_definitive,
        };
        self.execute(&command)
    }

    fn apply_transition(&mut self, transition: TransportTransition) -> Result<CanonicalTransportTransition> {
        let canonical = canonical_transport_transition(&self.profile, transition)?;
        self.state = canonical.state.clone();
        self.latest_evidence_ref = Some(canonical.transition_ref.clone());
        Ok(canonical)
    }
}

impl TransportCommandShell for DeterministicTransportAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition> {
        self.execute(command)
    }
}

#[derive(Debug)]
pub struct IrohTransportAdapter {
    profile: CanonicalTransportProfile,
    state: TransportState,
    latest_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveIrohLoopbackResult {
    pub submitted: CanonicalTransportTransition,
    pub acknowledged: CanonicalTransportTransition,
    pub echoed_payload_ref: String,
    pub remote_transport_identity_ref: String,
}

impl IrohTransportAdapter {
    // r[impl molten.fabric_transport.live_sim_parity]
    pub fn new(profile: CanonicalTransportProfile) -> Result<Self> {
        if profile.profile.adapter_kind != TransportAdapterKind::IrohLive {
            return Err(MoltenError::invalid_harness("Iroh transport adapter requires an iroh-live profile"));
        }
        Ok(Self {
            profile,
            state: TransportState::default(),
            latest_evidence_ref: None,
        })
    }

    pub fn state(&self) -> &TransportState {
        &self.state
    }

    pub fn profile(&self) -> &CanonicalTransportProfile {
        &self.profile
    }

    pub fn status(&self) -> Result<TransportStatusReadback> {
        transport_status_readback(&self.profile, &self.state, self.latest_evidence_ref.as_deref())
    }

    fn execute(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition> {
        let transition = apply_transport_command(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("Iroh transport command", &issues))?;
        self.apply_transition(transition)
    }

    fn apply_transition(&mut self, transition: TransportTransition) -> Result<CanonicalTransportTransition> {
        let canonical = canonical_transport_transition(&self.profile, transition)?;
        self.state = canonical.state.clone();
        self.latest_evidence_ref = Some(canonical.transition_ref.clone());
        Ok(canonical)
    }

    // r[impl molten.fabric_transport.live_sim_parity]
    // r[impl molten.fabric_transport.flow_control]
    pub async fn live_loopback_frame(
        &mut self,
        session_id: &ScopedTransportId,
        stream_id: &ScopedTransportId,
        operation_id: &str,
        alpn: &str,
        payload: &[u8],
        observed_tick: u64,
    ) -> Result<LiveIrohLoopbackResult> {
        let payload_bytes = u64::try_from(payload.len())
            .map_err(|_| MoltenError::invalid_harness("live Iroh payload size does not fit u64"))?;
        let payload_ref = blake3_ref(payload);
        validate_outer_frame(&self.profile.profile, &payload_ref, &payload_ref, payload_bytes)
            .map_err(|issues| adapter_validation_error("live Iroh outer frame", &issues))?;
        let send = TransportCommand::SendFrame {
            operation_id: operation_id.to_string(),
            session_id: session_id.clone(),
            stream_id: stream_id.clone(),
            payload_ref: payload_ref.clone(),
            payload_bytes,
            observed_tick,
        };
        let submitted = self.execute(&send)?;
        if submitted.decision != TransportTransitionDecision::Applied {
            return Err(MoltenError::invalid_harness("live Iroh frame was backpressured before adapter I/O"));
        }

        let network = run_iroh_loopback(alpn, payload, self.profile.profile.limits.max_frame_bytes).await;
        let (echoed, remote_transport_identity_ref) = match network {
            Ok(result) => result,
            Err(error) => {
                let failure = TransportCommand::FailSession {
                    operation_id: operation_id.to_string(),
                    session_id: session_id.clone(),
                    class: TransportFailureClass::AdapterFailure,
                    delivery_definitive: false,
                };
                let _failure_evidence = self.execute(&failure)?;
                return Err(error);
            }
        };
        if echoed != payload {
            let failure = TransportCommand::FailSession {
                operation_id: operation_id.to_string(),
                session_id: session_id.clone(),
                class: TransportFailureClass::MalformedInput,
                delivery_definitive: false,
            };
            let _failure_evidence = self.execute(&failure)?;
            return Err(MoltenError::invalid_harness("live Iroh loopback payload mismatch"));
        }
        let echoed_payload_ref = blake3_ref(&echoed);
        let acknowledged = self.execute(&TransportCommand::AcknowledgeFrame {
            operation_id: operation_id.to_string(),
            session_id: session_id.clone(),
            stream_id: stream_id.clone(),
            payload_bytes,
        })?;
        Ok(LiveIrohLoopbackResult {
            submitted,
            acknowledged,
            echoed_payload_ref,
            remote_transport_identity_ref,
        })
    }
}

impl TransportCommandShell for IrohTransportAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition> {
        self.execute(command)
    }
}

pub struct RegisteredTransportEffectPort<A: TransportCommandShell> {
    adapter: A,
    context: ExtensionTransportContext,
    profile: CanonicalTransportProfile,
    requests: BTreeMap<String, TransportCommand>,
}

impl<A: TransportCommandShell> RegisteredTransportEffectPort<A> {
    pub fn new(adapter: A, context: ExtensionTransportContext, profile: CanonicalTransportProfile) -> Result<Self> {
        if adapter.profile_id() != profile.profile.profile_id {
            return Err(MoltenError::invalid_harness("registered transport adapter profile mismatch"));
        }
        Ok(Self {
            adapter,
            context,
            profile,
            requests: BTreeMap::new(),
        })
    }

    pub fn register(&mut self, request_ref: String, command: TransportCommand) -> Result<()> {
        crate::preserves_rail::validate_content_ref(&request_ref)?;
        if self.requests.insert(request_ref.clone(), command).is_some() {
            return Err(MoltenError::invalid_harness(format!("transport request {request_ref} is already registered")));
        }
        Ok(())
    }

    pub fn adapter(&self) -> &A {
        &self.adapter
    }
}

// r[impl molten.fabric_transport.port_contract]
// r[impl molten.fabric_transport.session_streams]
impl<A: TransportCommandShell> crate::system_extension::FabricEffectPort for RegisteredTransportEffectPort<A> {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> std::result::Result<crate::system_extension::PortEffectOutput, String> {
        if binding.binding.key.port_id != FABRIC_TRANSPORT_PORT_ID
            || binding.binding.key.version != FABRIC_TRANSPORT_PORT_VERSION
        {
            return Err("transport effect routed through the wrong fabric port".to_string());
        }
        if binding.binding.implementation_profile != self.adapter.profile_id() {
            return Err("transport effect profile substitution denied".to_string());
        }
        match &effect.target {
            crate::system_extension::EffectTarget::FabricPort(key) if key == &binding.binding.key => {}
            crate::system_extension::EffectTarget::FabricPort(_) => {
                return Err("transport effect target does not match its bound port".to_string());
            }
            crate::system_extension::EffectTarget::Ambient(_) => {
                return Err("ambient effect cannot route through a transport port".to_string());
            }
        }
        let command = self
            .requests
            .get(&effect.request_ref)
            .cloned()
            .ok_or_else(|| "transport effect request is not registered".to_string())?;
        self.context
            .admit_command(&self.profile, &command, effect.accounted_bytes)
            .map_err(|error| error.to_string())?;
        if effect.generation != command.generation() {
            return Err("transport effect generation does not match its registered command".to_string());
        }
        let transition = self.adapter.execute_command(&command).map_err(|error| error.to_string())?;
        Ok(crate::system_extension::PortEffectOutput {
            output_schema_ref: effect.output_schema_ref.clone(),
            output_ref: transition.transition_ref,
        })
    }
}

async fn run_iroh_loopback(alpn: &str, payload: &[u8], max_frame_bytes: u64) -> Result<(Vec<u8>, String)> {
    let read_limit = usize::try_from(max_frame_bytes)
        .map_err(|_| MoltenError::invalid_harness("Iroh read bound does not fit usize"))?;
    let timeout = Duration::from_secs(LIVE_LOOPBACK_TIMEOUT_SECONDS);
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let alpn_bytes = alpn.as_bytes().to_vec();
    let server = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .address_lookup(lookup.clone())
        .alpns(vec![alpn_bytes.clone()])
        .bind()
        .await
        .map_err(iroh_error)?;
    let client = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .address_lookup(lookup.clone())
        .alpns(vec![alpn_bytes.clone()])
        .bind()
        .await
        .map_err(iroh_error)?;
    lookup.add_endpoint_info(server.addr());
    lookup.add_endpoint_info(client.addr());

    let server_endpoint = server.clone();
    let server_task = tokio::spawn(async move {
        let incoming = tokio::time::timeout(timeout, server_endpoint.accept())
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh accept timed out"))?
            .ok_or_else(|| MoltenError::invalid_harness("live Iroh endpoint closed before accept"))?;
        let connection = tokio::time::timeout(timeout, incoming)
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh handshake timed out"))?
            .map_err(iroh_error)?;
        let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
        let (mut send, mut receive) = tokio::time::timeout(timeout, connection.accept_bi())
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh stream accept timed out"))?
            .map_err(iroh_error)?;
        let received = tokio::time::timeout(timeout, receive.read_to_end(read_limit))
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh bounded frame read timed out"))?
            .map_err(iroh_error)?;
        tokio::time::timeout(timeout, send.write_all(&received))
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh echo write timed out"))?
            .map_err(iroh_error)?;
        send.finish().map_err(iroh_error)?;
        tokio::time::timeout(timeout, connection.closed())
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh peer close timed out"))?;
        Ok::<_, MoltenError>((received, remote_transport_identity_ref))
    });

    let client_result = async {
        let connection = tokio::time::timeout(timeout, client.connect(server.addr(), &alpn_bytes))
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh connect timed out"))?
            .map_err(iroh_error)?;
        let (mut send, mut receive) = tokio::time::timeout(timeout, connection.open_bi())
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh stream open timed out"))?
            .map_err(iroh_error)?;
        tokio::time::timeout(timeout, send.write_all(payload))
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh frame write timed out"))?
            .map_err(iroh_error)?;
        send.finish().map_err(iroh_error)?;
        let echoed = tokio::time::timeout(timeout, receive.read_to_end(read_limit))
            .await
            .map_err(|_| MoltenError::invalid_harness("live Iroh echo read timed out"))?
            .map_err(iroh_error)?;
        connection.close(IROH_CLOSE_CODE.into(), IROH_CLOSE_REASON);
        Ok::<_, MoltenError>(echoed)
    }
    .await;

    let server_result = server_task
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh server task failed: {error}")))?;
    client.close().await;
    server.close().await;
    let echoed = client_result?;
    let (received, remote_transport_identity_ref) = server_result?;
    if received != payload {
        return Err(MoltenError::invalid_harness("live Iroh server observed a different frame"));
    }
    Ok((echoed, remote_transport_identity_ref))
}

fn command_session_id(command: &TransportCommand) -> Option<&ScopedTransportId> {
    match command {
        TransportCommand::OpenSession { session_id, .. }
        | TransportCommand::OpenStream { session_id, .. }
        | TransportCommand::SendFrame { session_id, .. }
        | TransportCommand::ReceiveFrame { session_id, .. }
        | TransportCommand::AcknowledgeFrame { session_id, .. }
        | TransportCommand::SendDatagram { session_id, .. }
        | TransportCommand::CompleteDatagram { session_id, .. }
        | TransportCommand::GrantCredit { session_id, .. }
        | TransportCommand::HalfCloseStream { session_id, .. }
        | TransportCommand::CloseStream { session_id, .. }
        | TransportCommand::CloseSession { session_id, .. }
        | TransportCommand::FailSession { session_id, .. } => Some(session_id),
        TransportCommand::Cancel { target, .. } => match target {
            CancelTarget::Session(session_id) | CancelTarget::Stream { session_id, .. } => Some(session_id),
        },
        TransportCommand::Register { .. }
        | TransportCommand::TransferOwnership { .. }
        | TransportCommand::BeginDrain { .. }
        | TransportCommand::CleanupListener { .. } => None,
    }
}

fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn iroh_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("live Iroh transport failed: {error}"))
}

fn adapter_validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}
