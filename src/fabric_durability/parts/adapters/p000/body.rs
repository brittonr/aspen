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

impl RedbDurableStateAdapter {
    // r[impl molten.fabric_durability.live_sim_parity]
    pub fn open(
        root_path: &std::path::Path,
        profile: DurableProfile,
        descriptor: DurableNamespaceDescriptor,
    ) -> crate::error::Result<Self> {
        Self::open_from(root_path, profile, descriptor, DatabaseSource::File, PublicationSource::Linux)
    }

    /// Open over a selected Redb storage source and snapshot publication I/O.
    pub fn open_with(
        root_path: &std::path::Path,
        profile: DurableProfile,
        descriptor: DurableNamespaceDescriptor,
        source: DatabaseSource,
        publication: PublicationSource,
    ) -> crate::error::Result<Self> {
        Self::open_from(root_path, profile, descriptor, source, publication)
    }

    /// Open the store. A local repair during open, or a snapshot payload that
    /// does not match its identity, opens the adapter quarantined.
    // r[impl molten.fabric_durability.adapter_quarantine]
    fn open_from(
        root_path: &std::path::Path,
        profile: DurableProfile,
        descriptor: DurableNamespaceDescriptor,
        source: DatabaseSource,
        publication: PublicationSource,
    ) -> crate::error::Result<Self> {
        if profile.profile.adapter_kind != DurableAdapterKind::LiveRedb {
            return Err(crate::error::Failure::invalid_harness(
                "Redb durability adapter requires a live Redb profile",
            ));
        }
        validate_namespace_descriptor(&profile.profile, &descriptor)
            .map_err(|issues| adapter_validation_error("namespace", &issues))?;
        let root = crate::local_store::DurableStoreRoot::open(root_path)?;
        let is_repaired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let database = open_database(&root, &source, &is_repaired)?;
        initialize_tables(&database)?;
        let state = load_state(&database, &profile.profile, descriptor)?;
        let is_repair_observed = is_repaired.load(std::sync::atomic::Ordering::Acquire);
        let health = if is_repair_observed {
            AdapterHealth::Quarantined(QuarantineReason::RepairedOnOpen)
        } else if !snapshot_payloads_match(&root, &state) {
            AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent)
        } else {
            AdapterHealth::Serving
        };
        Ok(Self {
            profile,
            state,
            root,
            database,
            health,
            pending: None,
            source,
            publication,
            is_reference_trusted: health == AdapterHealth::Serving,
            is_repair_observed,
        })
    }

    /// The last acknowledged state. It is never advanced by an unacknowledged
    /// mutation; while quarantined, serving reads are refused.
    pub fn state(&self) -> &DurableState {
        &self.state
    }

    pub fn profile(&self) -> &DurableProfile {
        &self.profile
    }

    // r[impl molten.fabric_durability.durable_log]
    pub fn append(&mut self, request: &AppendRequest) -> crate::error::Result<TransitionEnvelope> {
        self.admit(AdapterOperation::Mutation)?;
        let transition = append_log(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("append", &issues))?;
        let attempt = if transition.outcome == MutationOutcome::Durable {
            persist_log(&self.database, &transition.next.durable_log)
        } else {
            PersistenceAttempt::process_visible()
        };
        self.settle(request.durability, attempt, transition)
    }

    pub fn flush(
        &mut self,
        generation: u64,
        durability: DurabilityLevel,
    ) -> crate::error::Result<TransitionEnvelope> {
        self.admit(AdapterOperation::Mutation)?;
        let transition = flush_log(&self.profile.profile, &self.state, generation, durability)
            .map_err(|issues| adapter_validation_error("flush", &issues))?;
        let attempt = persist_log(&self.database, &transition.next.durable_log);
        self.settle(durability, attempt, transition)
    }

    pub fn truncate(
        &mut self,
        generation: u64,
        retain_from_sequence: u64,
        authority_ref: Option<&str>,
    ) -> crate::error::Result<TransitionEnvelope> {
        self.admit(AdapterOperation::Mutation)?;
        let transition =
            truncate_log(&self.profile.profile, &self.state, generation, retain_from_sequence, authority_ref)
                .map_err(|issues| adapter_validation_error("truncate", &issues))?;
        let attempt = replace_log(&self.database, &transition.next.durable_log);
        self.settle(DurabilityLevel::MachineLoss, attempt, transition)
    }

    pub fn read_log(&self, sequence: u64) -> crate::error::Result<Option<&LogRecord>> {
        self.admit(AdapterOperation::Read)?;
        Ok(read_log(&self.state, sequence))
    }

    pub fn scan_log(&self, start_sequence: u64, limit: u64) -> crate::error::Result<LogScanPage> {
        self.admit(AdapterOperation::Read)?;
        scan_log(&self.state, start_sequence, limit).map_err(|issue| adapter_validation_error("log scan", &[issue]))
    }

    // r[impl molten.fabric_durability.ordered_store]
    // r[impl molten.fabric_durability.atomic_batch]
    pub fn apply_batch(&mut self, request: &AtomicBatchRequest) -> crate::error::Result<TransitionEnvelope> {
        self.admit(AdapterOperation::Mutation)?;
        let transition = apply_atomic_batch(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("ordered batch", &issues))?;
        let attempt = persist_ordered_batch(&self.database, request, &transition.next);
        self.settle(request.durability, attempt, transition)
    }

    // r[impl molten.fabric_durability.snapshot_recovery]
    pub fn create_snapshot(
        &mut self,
        request: &SnapshotRequest,
        bytes: &[u8],
    ) -> crate::error::Result<TransitionEnvelope> {
        self.admit(AdapterOperation::Mutation)?;
        let actual_ref = blake3_ref(bytes);
        if actual_ref != request.content_ref {
            return Err(crate::error::Failure::invalid_harness(format!(
                "snapshot content ref mismatch: expected={} actual={actual_ref}",
                request.content_ref
            )));
        }
        let transition = create_snapshot(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("snapshot", &issues))?;
        self.publish_then_commit_snapshot(request, bytes, transition)
    }

    pub fn restore_snapshot(
        &self,
        snapshot_ref: &str,
        target_generation: u64,
    ) -> crate::error::Result<SnapshotRestorePlan> {
        Ok(self.load_snapshot_bytes(snapshot_ref, target_generation)?.0)
    }

    pub fn load_snapshot_bytes(
        &self,
        snapshot_ref: &str,
        target_generation: u64,
    ) -> crate::error::Result<(SnapshotRestorePlan, Vec<u8>)> {
        self.admit(AdapterOperation::Read)?;
        let snapshot = self
            .state
            .snapshots
            .get(snapshot_ref)
            .ok_or_else(|| adapter_validation_error("snapshot restore", &[DurabilityIssue::SnapshotNotFound]))?;
        let relative = format!("{SNAPSHOT_DIRECTORY}/{}.bin", snapshot_file_stem(&snapshot.content_ref)?);
        let bytes = self.root.root().read(&crate::local_store::Locator::parse(&relative)?)?;
        let actual_ref = blake3_ref(&bytes);
        let plan = plan_snapshot_restore(&self.state, snapshot_ref, target_generation, &actual_ref)
            .map_err(|issues| adapter_validation_error("snapshot restore", &issues))?;
        Ok((plan, bytes))
    }

    // r[impl molten.fabric_durability.effect_transaction]
    pub fn apply_effect(
        &mut self,
        command: &EffectTransactionCommand,
    ) -> crate::error::Result<TransitionEnvelope> {
        self.admit(AdapterOperation::Mutation)?;
        let transition = apply_effect_transaction(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("effect transaction", &issues))?;
        let attempt = persist_effect(&self.database, command, &transition.next);
        self.settle(DurabilityLevel::MachineLoss, attempt, transition)
    }

    pub fn recovery(&self, inventory: &RecoveryInventory) -> crate::error::Result<RecoveryEnvelope> {
        self.admit(AdapterOperation::Read)?;
        recovery_decision(&self.profile, &self.state, evaluate_recovery(&self.state, inventory))
    }

    pub fn status(&self) -> crate::error::Result<DurableStatusReadback> {
        self.admit(AdapterOperation::Status)?;
        durable_status_readback(&self.profile, &self.state)
    }
}

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
