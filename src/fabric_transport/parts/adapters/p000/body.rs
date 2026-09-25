// r[impl molten.modularity.fabric_boundary.adapters]

use super::*;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "transport mechanisms implement the application-owned typed port contract"
)]
use crate::fabric::FabricPortError;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "transport mechanisms implement the application-owned typed port contract"
)]
use crate::fabric::FabricPortResult;

const LIVE_LOOPBACK_TIMEOUT_SECONDS: u64 = 10;
const IROH_CLOSE_CODE: u8 = 0;
const IROH_CLOSE_REASON: &[u8] = b"fabric-transport-loopback-complete";

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
    pub fn new(profile: CanonicalTransportProfile) -> crate::error::Result<Self> {
        if profile.profile.adapter_kind != TransportAdapterKind::DeterministicSimulation {
            return Err(crate::error::MoltenError::invalid_harness(
                "deterministic transport adapter requires a deterministic-simulation profile",
            ));
        }
        Ok(Self {
            profile,
            state: TransportState::new(),
            latest_evidence_ref: None,
        })
    }

    pub fn state(&self) -> &TransportState {
        &self.state
    }

    pub fn profile(&self) -> &CanonicalTransportProfile {
        &self.profile
    }

    pub fn status(&self) -> crate::error::Result<TransportStatusReadback> {
        transport_status_readback(&self.profile, &self.state, self.latest_evidence_ref.as_deref())
    }

    // r[impl molten.fabric_transport.failure_semantics]
    pub fn execute_with_fault(
        &mut self,
        command: &TransportCommand,
        fault: Option<SimulatedTransportFault>,
    ) -> crate::error::Result<CanonicalTransportTransition> {
        match fault {
            None => self.execute(command),
            Some(SimulatedTransportFault::LocalOverload) => {
                Err(crate::error::MoltenError::invalid_harness("simulated transport overload before adapter I/O"))
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
                    crate::error::MoltenError::invalid_harness("disconnect-after-submission requires a session command")
                })?;
                self.fail_session(command.operation_id(), &session_id, TransportFailureClass::Disconnect, false)
            }
            Some(SimulatedTransportFault::AdapterFailure) => {
                self.fail_for_command(command, TransportFailureClass::AdapterFailure, false)
            }
        }
    }

    fn execute(&mut self, command: &TransportCommand) -> crate::error::Result<CanonicalTransportTransition> {
        let transition = apply_transport_command(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("simulated transport command", &issues))?;
        self.apply_transition(transition)
    }

    fn fail_for_command(
        &mut self,
        command: &TransportCommand,
        class: TransportFailureClass,
        delivery_definitive: bool,
    ) -> crate::error::Result<CanonicalTransportTransition> {
        let session_id = command_session_id(command).ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("simulated transport fault requires a session command")
        })?;
        self.fail_session(command.operation_id(), session_id, class, delivery_definitive)
    }

    fn fail_session(
        &mut self,
        operation_id: &str,
        session_id: &ScopedTransportId,
        class: TransportFailureClass,
        delivery_definitive: bool,
    ) -> crate::error::Result<CanonicalTransportTransition> {
        let command = TransportCommand::FailSession {
            operation_id: operation_id.to_string(),
            session_id: session_id.clone(),
            class,
            delivery_definitive,
        };
        self.execute(&command)
    }

    fn apply_transition(
        &mut self,
        transition: TransportTransition,
    ) -> crate::error::Result<CanonicalTransportTransition> {
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

    fn execute_command(&mut self, command: &TransportCommand) -> FabricPortResult<CanonicalTransportTransition> {
        self.execute(command).map_err(|error| FabricPortError::transport(error.to_string()))
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

pub struct LoopbackFrameInput<'a> {
    pub session_id: &'a ScopedTransportId,
    pub stream_id: &'a ScopedTransportId,
    pub operation_id: &'a str,
    pub alpn: &'a str,
    pub payload: &'a [u8],
    pub observed_tick: u64,
}
