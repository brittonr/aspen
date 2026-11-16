// r[impl molten.modularity.fabric_boundary.adapters]

use redb::ReadableDatabase;
use redb::ReadableTable;

use super::*;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "durability mechanisms implement the application-owned typed port contract"
)]
use crate::fabric::FabricPortError;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "durability mechanisms implement the application-owned typed port contract"
)]
use crate::fabric::FabricPortResult;

const STORE_FILE: &str = "fabric-durability.redb";
const SNAPSHOT_DIRECTORY: &str = "snapshots";
const LOG_TABLE: redb::TableDefinition<u64, &[u8]> = redb::TableDefinition::new("fabric_durable_log_v1");
const ORDERED_TABLE: redb::TableDefinition<&[u8], &[u8]> = redb::TableDefinition::new("fabric_ordered_store_v1");
const SNAPSHOT_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("fabric_snapshots_v1");
const EFFECT_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("fabric_effect_transactions_v1");

const LEVEL_BUFFERED: u8 = 0;
const LEVEL_PROCESS_LOSS: u8 = 1;
const LEVEL_MACHINE_LOSS: u8 = 2;
const SNAPSHOT_KIND_SNAPSHOT: u8 = 0;
const SNAPSHOT_KIND_CHECKPOINT: u8 = 1;
const PHASE_RESERVED: u8 = 0;
const PHASE_COMMITTED: u8 = 1;
const PHASE_ABORTED: u8 = 2;
const PHASE_EXPIRED: u8 = 3;
const PHASE_UNCERTAIN: u8 = 4;
const PHASE_RECONCILED_COMMITTED: u8 = 5;
const PHASE_RECONCILED_ABORTED: u8 = 6;
const LENGTH_PREFIX_BYTES: usize = std::mem::size_of::<u64>();

pub struct RedbDurableStateAdapter {
    profile: DurableProfile,
    /// The last acknowledged state. It advances only on acknowledgement.
    state: DurableState,
    root: crate::local_store::DurableStoreRoot,
    database: redb::Database,
    health: AdapterHealth,
    /// The state an uncertain mutation would have produced.
    pending: Option<DurableState>,
    source: DatabaseSource,
    publication: PublicationSource,
    /// Whether `state` is a trusted reconciliation reference: false after a
    /// repaired open until a reconciliation succeeds.
    is_reference_trusted: bool,
    /// Storage repaired its own structures at an open of this adapter.
    is_repair_observed: bool,
}

impl std::fmt::Debug for RedbDurableStateAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedbDurableStateAdapter")
            .field("profile", &self.profile.profile.profile_id)
            .field("namespace", &self.state.descriptor.namespace_id)
            .field("generation", &self.state.descriptor.generation)
            .field("health", &self.health)
            .finish_non_exhaustive()
    }
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_durability/parts/adapters/p000/storage.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulatedFault {
    CrashBeforeMutation,
    ResponseLostAfterCommit,
    CapacityExhausted,
    DelayCompletion { ticks: u64 },
    ProcessCrash,
    CorruptSnapshot { snapshot_ref: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedDurableStateAdapter {
    profile: DurableProfile,
    state: DurableState,
    simulated_ticks: u64,
}
