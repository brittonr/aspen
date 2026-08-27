#![allow(
    tigerstyle::non_trait_imports,
    reason = "the adapter keeps content-to-manifest bindings in a deterministic ordered map"
)]

use std::collections::BTreeMap;

use molten_core::content_replication::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::*;

const PROFILE_REF: &str = "blake3:6161616161616161616161616161616161616161616161616161616161616161";
const FRAMING_REF: &str = "blake3:6262626262626262626262626262626262626262626262626262626262626262";
const AUTHORITY_REF: &str = "blake3:6363636363636363636363636363636363636363636363636363636363636363";
const OPERATION_REF: &str = "blake3:6464646464646464646464646464646464646464646464646464646464646464";
const SESSION_REF: &str = "blake3:6565656565656565656565656565656565656565656565656565656565656565";
const STREAM_REF: &str = "blake3:6666666666666666666666666666666666666666666666666666666666666666";
const PEER_REF: &str = "blake3:6767676767676767676767676767676767676767676767676767676767676767";
const MEMBERSHIP_REF: &str = "blake3:6868686868686868686868686868686868686868686868686868686868686868";
const PRINCIPAL_REF: &str = "blake3:6969696969696969696969696969696969696969696969696969696969696969";
const TRUST_REF: &str = "blake3:6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a6a";
const CAPABILITY_REF: &str = "blake3:6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b";
const ALPN: &str = "molten/content-replication/1";
const SERVICE_ID: &str = "content-replication";
const PROTOCOL_ID: &str = "content-replication-v1";
const EXTENSION_ID: &str = "content-replication-system-extension";
const PROFILE_LIMIT: u64 = 16;
const FRAME_LIMIT: u64 = 4_096;
const DATAGRAM_LIMIT: u64 = 1_024;
const QUEUE_BYTE_LIMIT: u64 = 16_384;
const INFLIGHT_LIMIT: u64 = 8_192;
const DEADLINE_WINDOW: u64 = 64;
const INITIAL_TICK: u64 = 1;
const LENGTH_PREFIX_BYTES: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferProfile {
    DeterministicSimulation,
    IrohLiveLoopback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferFault {
    CancelAt { call: usize },
    PartitionAt { call: usize },
    TimeoutAt { call: usize },
    UnavailableAt { call: usize },
}

enum Mechanism {
    Deterministic(DeterministicTransportAdapter),
    IrohLive {
        adapter: IrohTransportAdapter,
        runtime: tokio::runtime::Runtime,
    },
}

pub struct FabricTransferAdapter {
    mechanism: Mechanism,
    manifest_refs: BTreeMap<String, String>,
    generation: u64,
    membership_epoch: u64,
    placement_epoch: u64,
    fault: Option<TransferFault>,
    call_count: usize,
    session_id: ScopedTransportId,
    stream_id: ScopedTransportId,
}

impl FabricTransferAdapter {
    pub fn open(manifest: &Manifest, profile: TransferProfile, fault: Option<TransferFault>) -> Result<Self> {
        let mechanism = match profile {
            TransferProfile::DeterministicSimulation => Mechanism::Deterministic(DeterministicTransportAdapter::new(
                transport_profile(TransportAdapterKind::DeterministicSimulation)?,
            )?),
            TransferProfile::IrohLiveLoopback => {
                if fault.is_some() {
                    return Err(MoltenError::invalid_harness(
                        "live replication loopback does not synthesize transport faults",
                    ));
                }
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| MoltenError::invalid_harness(format!("replication runtime failed: {error}")))?;
                Mechanism::IrohLive {
                    adapter: IrohTransportAdapter::new(transport_profile(TransportAdapterKind::IrohLive)?)?,
                    runtime,
                }
            }
        };
        let mut opened = Self {
            mechanism,
            manifest_refs: manifest
                .contents
                .iter()
                .map(|content| (content.content_ref.clone(), content.manifest_ref.clone()))
                .collect(),
            generation: manifest.generation,
            membership_epoch: manifest.membership_epoch,
            placement_epoch: manifest.placement_epoch,
            fault,
            call_count: 0,
            session_id: scoped_id(SESSION_REF, manifest.generation),
            stream_id: scoped_id(STREAM_REF, manifest.generation),
        };
        opened.apply_setup()?;
        Ok(opened)
    }

    pub const fn call_count(&self) -> usize {
        self.call_count
    }

    fn apply_setup(&mut self) -> Result<()> {
        for command in setup_commands(self.generation) {
            self.execute_command(&command)?;
        }
        Ok(())
    }

    fn execute_command(&mut self, command: &TransportCommand) -> Result<CanonicalTransportTransition> {
        match &mut self.mechanism {
            Mechanism::Deterministic(adapter) => adapter.execute_command(command).map_err(|error| {
                MoltenError::invalid_harness(format!("simulated replication transport failed: {error}"))
            }),
            Mechanism::IrohLive { adapter, .. } => adapter
                .execute_command(command)
                .map_err(|error| MoltenError::invalid_harness(format!("live replication transport failed: {error}"))),
        }
    }

    fn transfer(&mut self, action: &Action) -> Result<TransferOutcome> {
        let payload = action_payload(action);
        let payload_bytes = u64::try_from(payload.len())
            .map_err(|_| MoltenError::invalid_harness("replication transport payload exceeds u64"))?;
        let transfer_ref = match &mut self.mechanism {
            Mechanism::Deterministic(adapter) => {
                let send = send_command(&self.session_id, &self.stream_id, action, &payload)?;
                let _submitted = adapter
                    .execute_command(&send)
                    .map_err(|error| MoltenError::invalid_harness(format!("simulated transfer failed: {error}")))?;
                adapter
                    .execute_command(&TransportCommand::AcknowledgeFrame {
                        operation_id: OPERATION_REF.to_string(),
                        session_id: self.session_id.clone(),
                        stream_id: self.stream_id.clone(),
                        payload_bytes,
                    })
                    .map_err(|error| MoltenError::invalid_harness(format!("simulated ack failed: {error}")))?
                    .transition_ref
            }
            Mechanism::IrohLive { adapter, runtime } => {
                runtime
                    .block_on(adapter.live_loopback_frame(
                        &self.session_id,
                        &self.stream_id,
                        OPERATION_REF,
                        ALPN,
                        &payload,
                        observed_tick(self.call_count)?,
                    ))?
                    .acknowledged
                    .transition_ref
            }
        };
        self.received(action, transfer_ref)
    }

    fn received(&self, action: &Action, transfer_ref: String) -> Result<TransferOutcome> {
        let manifest_ref = self
            .manifest_refs
            .get(&action.content_ref)
            .cloned()
            .ok_or_else(|| MoltenError::invalid_harness("replication action lacks a manifest binding"))?;
        Ok(TransferOutcome::Received(TransferEnvelope {
            transport_verification_ref: transfer_ref.clone(),
            transfer_ref,
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            manifest_ref,
            source_peer: action.source_peer.clone().unwrap_or_default(),
            target_peer: action.target_peer.clone(),
            generation: self.generation,
            membership_epoch: self.membership_epoch,
            placement_epoch: self.placement_epoch,
            encoded_bytes: action.encoded_bytes,
            protected: action.preserve_protected_form,
        }))
    }

    fn deterministic_fault(&mut self, action: &Action, fault: TransferFault) -> Result<TransferOutcome> {
        let payload = action_payload(action);
        let send = send_command(&self.session_id, &self.stream_id, action, &payload)?;
        let Mechanism::Deterministic(adapter) = &mut self.mechanism else {
            return Err(MoltenError::invalid_harness("replication fault injection requires deterministic transport"));
        };
        match fault {
            TransferFault::CancelAt { .. } => {
                let cancelled = adapter
                    .execute_command(&TransportCommand::Cancel {
                        operation_id: OPERATION_REF.to_string(),
                        target: CancelTarget::Stream {
                            session_id: self.session_id.clone(),
                            stream_id: self.stream_id.clone(),
                        },
                    })
                    .map_err(|error| MoltenError::invalid_harness(format!("simulated cancel failed: {error}")))?;
                Ok(TransferOutcome::Cancelled(cancelled.transition_ref))
            }
            TransferFault::PartitionAt { .. } => {
                let failed = adapter.execute_with_fault(&send, Some(SimulatedTransportFault::Partition))?;
                Ok(TransferOutcome::Uncertain(failed.transition_ref))
            }
            TransferFault::TimeoutAt { .. } => {
                let failed = adapter.execute_with_fault(&send, Some(SimulatedTransportFault::Timeout))?;
                Ok(TransferOutcome::TimedOut(failed.transition_ref))
            }
            TransferFault::UnavailableAt { .. } => {
                let failed = adapter.execute_with_fault(&send, Some(SimulatedTransportFault::RemoteRefusal))?;
                Ok(TransferOutcome::Unavailable(failed.transition_ref))
            }
        }
    }
}

impl TransportPort for FabricTransferAdapter {
    fn fetch(&mut self, action: &Action) -> Result<TransferOutcome> {
        let call = self.call_count;
        self.call_count = self
            .call_count
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("replication transport call count overflow"))?;
        if let Some(fault) = self.fault.filter(|fault| fault_call(*fault) == call) {
            return self.deterministic_fault(action, fault);
        }
        self.transfer(action)
    }
}

fn fault_call(fault: TransferFault) -> usize {
    match fault {
        TransferFault::CancelAt { call }
        | TransferFault::PartitionAt { call }
        | TransferFault::TimeoutAt { call }
        | TransferFault::UnavailableAt { call } => call,
    }
}

mod profile;
use profile::*;
