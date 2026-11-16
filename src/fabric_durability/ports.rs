//! Application-owned durable command capability.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the durable port exposes explicit domain commands and canonical transitions"
)]

use super::*;
use crate::fabric::FabricPortResult;

// r[impl molten.modularity.fabric_boundary.ports]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurablePortCommand {
    Append(AppendRequest),
    Flush {
        generation: u64,
        durability: DurabilityLevel,
    },
    Truncate {
        generation: u64,
        retain_from_sequence: u64,
        authority_ref: Option<String>,
    },
    AtomicBatch(AtomicBatchRequest),
    Snapshot {
        request: SnapshotRequest,
        bytes: Vec<u8>,
    },
    /// Retire snapshots superseded by the kept valid snapshot; requires retention authority.
    RetireSupersededSnapshots {
        generation: u64,
        keep_snapshot_ref: String,
        authority_ref: Option<String>,
    },
    /// Truncate records that a valid snapshot covers; requires retention authority.
    TruncateSnapshotCovered {
        generation: u64,
        snapshot_ref: String,
        retain_from_sequence: u64,
        authority_ref: Option<String>,
    },
    Effect(EffectTransactionCommand),
}

impl DurablePortCommand {
    pub const fn port_id(&self) -> &'static str {
        match self {
            Self::Append(_) | Self::Flush { .. } | Self::Truncate { .. } | Self::TruncateSnapshotCovered { .. } => {
                FABRIC_DURABLE_LOG_PORT_ID
            }
            Self::AtomicBatch(_) => FABRIC_ORDERED_STORE_PORT_ID,
            Self::Snapshot { .. } | Self::RetireSupersededSnapshots { .. } => FABRIC_SNAPSHOT_PORT_ID,
            Self::Effect(_) => FABRIC_EFFECT_TRANSACTION_PORT_ID,
        }
    }

    pub const fn generation(&self) -> u64 {
        match self {
            Self::Append(request) => request.generation,
            Self::Flush { generation, .. }
            | Self::Truncate { generation, .. }
            | Self::RetireSupersededSnapshots { generation, .. }
            | Self::TruncateSnapshotCovered { generation, .. } => *generation,
            Self::AtomicBatch(request) => request.generation,
            Self::Snapshot { request, .. } => request.generation,
            Self::Effect(command) => match command {
                EffectTransactionCommand::Reserve { generation, .. }
                | EffectTransactionCommand::Commit { generation, .. }
                | EffectTransactionCommand::Abort { generation, .. }
                | EffectTransactionCommand::Expire { generation, .. }
                | EffectTransactionCommand::MarkUncertain { generation, .. }
                | EffectTransactionCommand::Reconcile { generation, .. } => *generation,
            },
        }
    }
}

pub trait DurableCommandShell {
    fn profile_id(&self) -> &str;
    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<TransitionEnvelope>;
}
