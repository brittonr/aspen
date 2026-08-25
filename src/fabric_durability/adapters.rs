// r[impl molten.modularity.fabric_boundary.adapters]
use std::path::Path;

use redb::ReadableDatabase;
use redb::ReadableTable;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
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
use crate::local_store::DurableStoreRoot;
use crate::local_store::LocalStorePath;

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
    profile: CanonicalDurableProfile,
    state: DurableState,
    root: DurableStoreRoot,
    database: redb::Database,
}

impl std::fmt::Debug for RedbDurableStateAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedbDurableStateAdapter")
            .field("profile", &self.profile.profile.profile_id)
            .field("namespace", &self.state.descriptor.namespace_id)
            .field("generation", &self.state.descriptor.generation)
            .finish_non_exhaustive()
    }
}

impl RedbDurableStateAdapter {
    // r[impl molten.fabric_durability.live_sim_parity]
    pub fn open(
        root_path: &Path,
        profile: CanonicalDurableProfile,
        descriptor: DurableNamespaceDescriptor,
    ) -> Result<Self> {
        if profile.profile.adapter_kind != DurableAdapterKind::LiveRedb {
            return Err(MoltenError::invalid_harness("Redb durability adapter requires a live Redb profile"));
        }
        validate_namespace_descriptor(&profile.profile, &descriptor)
            .map_err(|issues| adapter_validation_error("namespace", &issues))?;
        let root = DurableStoreRoot::open(root_path)?;
        let database_file = root.root().open_database_file(&LocalStorePath::parse(STORE_FILE)?)?;
        let database = redb::Database::builder().create_file(database_file).map_err(adapter_error)?;
        initialize_tables(&database)?;
        let state = load_state(&database, descriptor)?;
        Ok(Self {
            profile,
            state,
            root,
            database,
        })
    }

    pub fn state(&self) -> &DurableState {
        &self.state
    }

    pub fn profile(&self) -> &CanonicalDurableProfile {
        &self.profile
    }

    // r[impl molten.fabric_durability.durable_log]
    pub fn append(&mut self, request: &AppendRequest) -> Result<CanonicalDurableTransition> {
        let transition = append_log(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("append", &issues))?;
        if transition.outcome == MutationOutcome::Durable {
            persist_log(&self.database, &transition.next.durable_log)?;
        }
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn flush(&mut self, generation: u64, durability: DurabilityLevel) -> Result<CanonicalDurableTransition> {
        let transition = flush_log(&self.profile.profile, &self.state, generation, durability)
            .map_err(|issues| adapter_validation_error("flush", &issues))?;
        persist_log(&self.database, &transition.next.durable_log)?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn truncate(
        &mut self,
        generation: u64,
        retain_from_sequence: u64,
        authority_ref: Option<&str>,
    ) -> Result<CanonicalDurableTransition> {
        let transition =
            truncate_log(&self.profile.profile, &self.state, generation, retain_from_sequence, authority_ref)
                .map_err(|issues| adapter_validation_error("truncate", &issues))?;
        replace_log(&self.database, &transition.next.durable_log)?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn read_log(&self, sequence: u64) -> Option<&LogRecord> {
        read_log(&self.state, sequence)
    }

    pub fn scan_log(&self, start_sequence: u64, limit: u64) -> Result<LogScanPage> {
        scan_log(&self.state, start_sequence, limit).map_err(|issue| adapter_validation_error("log scan", &[issue]))
    }

    // r[impl molten.fabric_durability.ordered_store]
    // r[impl molten.fabric_durability.atomic_batch]
    pub fn apply_batch(&mut self, request: &AtomicBatchRequest) -> Result<CanonicalDurableTransition> {
        let transition = apply_atomic_batch(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("ordered batch", &issues))?;
        persist_ordered_batch(&self.database, request, &transition.next)?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    // r[impl molten.fabric_durability.snapshot_recovery]
    pub fn create_snapshot(&mut self, request: &SnapshotRequest, bytes: &[u8]) -> Result<CanonicalDurableTransition> {
        let actual_ref = blake3_ref(bytes);
        if actual_ref != request.content_ref {
            return Err(MoltenError::invalid_harness(format!(
                "snapshot content ref mismatch: expected={} actual={actual_ref}",
                request.content_ref
            )));
        }
        let transition = create_snapshot(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("snapshot", &issues))?;
        persist_snapshot(&self.root, &self.database, request, bytes, &transition.next)?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn restore_snapshot(&self, snapshot_ref: &str, target_generation: u64) -> Result<SnapshotRestorePlan> {
        Ok(self.load_snapshot_bytes(snapshot_ref, target_generation)?.0)
    }

    pub fn load_snapshot_bytes(
        &self,
        snapshot_ref: &str,
        target_generation: u64,
    ) -> Result<(SnapshotRestorePlan, Vec<u8>)> {
        let snapshot = self
            .state
            .snapshots
            .get(snapshot_ref)
            .ok_or_else(|| adapter_validation_error("snapshot restore", &[DurabilityIssue::SnapshotNotFound]))?;
        let relative = format!("{SNAPSHOT_DIRECTORY}/{}.bin", snapshot_file_stem(&snapshot.content_ref)?);
        let bytes = self.root.root().read(&LocalStorePath::parse(&relative)?)?;
        let actual_ref = blake3_ref(&bytes);
        let plan = plan_snapshot_restore(&self.state, snapshot_ref, target_generation, &actual_ref)
            .map_err(|issues| adapter_validation_error("snapshot restore", &issues))?;
        Ok((plan, bytes))
    }

    // r[impl molten.fabric_durability.effect_transaction]
    pub fn apply_effect(&mut self, command: &EffectTransactionCommand) -> Result<CanonicalDurableTransition> {
        let transition = apply_effect_transaction(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("effect transaction", &issues))?;
        persist_effect(&self.database, command, &transition.next)?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn recovery(&self, inventory: &RecoveryInventory) -> Result<CanonicalRecoveryDecision> {
        canonical_recovery_decision(&self.profile, &self.state, evaluate_recovery(&self.state, inventory))
    }

    pub fn status(&self) -> Result<DurableStatusReadback> {
        durable_status_readback(&self.profile, &self.state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulatedDurabilityFault {
    CrashBeforeMutation,
    ResponseLostAfterCommit,
    CapacityExhausted,
    DelayCompletion { ticks: u64 },
    ProcessCrash,
    CorruptSnapshot { snapshot_ref: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedDurableStateAdapter {
    profile: CanonicalDurableProfile,
    state: DurableState,
    simulated_ticks: u64,
}

impl SimulatedDurableStateAdapter {
    // r[impl molten.fabric_durability.live_sim_parity]
    pub fn new(profile: CanonicalDurableProfile, descriptor: DurableNamespaceDescriptor) -> Result<Self> {
        if profile.profile.adapter_kind != DurableAdapterKind::DeterministicSimulation {
            return Err(MoltenError::invalid_harness(
                "simulated durability adapter requires a deterministic-simulation profile",
            ));
        }
        validate_namespace_descriptor(&profile.profile, &descriptor)
            .map_err(|issues| adapter_validation_error("simulated namespace", &issues))?;
        Ok(Self {
            profile,
            state: DurableState::empty(descriptor),
            simulated_ticks: 0,
        })
    }

    pub fn state(&self) -> &DurableState {
        &self.state
    }

    pub const fn simulated_ticks(&self) -> u64 {
        self.simulated_ticks
    }

    pub fn append(
        &mut self,
        request: &AppendRequest,
        fault: Option<&SimulatedDurabilityFault>,
    ) -> Result<CanonicalDurableTransition> {
        if matches!(
            fault,
            Some(SimulatedDurabilityFault::CrashBeforeMutation | SimulatedDurabilityFault::CapacityExhausted)
        ) {
            let operation = if matches!(fault, Some(SimulatedDurabilityFault::CapacityExhausted)) {
                "simulated-capacity-exhausted-before-append"
            } else {
                "simulated-crash-before-append"
            };
            return self.synthetic_transition(MutationOutcome::FailedBeforeMutation, operation, true, false);
        }
        let mut transition = append_log(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("simulated append", &issues))?;
        if matches!(fault, Some(SimulatedDurabilityFault::ResponseLostAfterCommit)) {
            transition.outcome = MutationOutcome::Uncertain;
            transition.operation = "simulated-response-loss-after-append".to_string();
            transition.retry_safe = false;
            transition.reconciliation_required = true;
        }
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn flush(&mut self, generation: u64, durability: DurabilityLevel) -> Result<CanonicalDurableTransition> {
        let transition = flush_log(&self.profile.profile, &self.state, generation, durability)
            .map_err(|issues| adapter_validation_error("simulated flush", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn apply_batch(
        &mut self,
        request: &AtomicBatchRequest,
        fault: Option<&SimulatedDurabilityFault>,
    ) -> Result<CanonicalDurableTransition> {
        if matches!(
            fault,
            Some(SimulatedDurabilityFault::CrashBeforeMutation | SimulatedDurabilityFault::CapacityExhausted)
        ) {
            let operation = if matches!(fault, Some(SimulatedDurabilityFault::CapacityExhausted)) {
                "simulated-capacity-exhausted-before-batch"
            } else {
                "simulated-crash-before-batch"
            };
            return self.synthetic_transition(MutationOutcome::FailedBeforeMutation, operation, true, false);
        }
        let mut transition = apply_atomic_batch(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("simulated batch", &issues))?;
        if matches!(fault, Some(SimulatedDurabilityFault::ResponseLostAfterCommit)) {
            transition.outcome = MutationOutcome::Uncertain;
            transition.operation = "simulated-response-loss-after-batch".to_string();
            transition.retry_safe = false;
            transition.reconciliation_required = true;
        }
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn truncate(
        &mut self,
        generation: u64,
        retain_from_sequence: u64,
        authority_ref: Option<&str>,
    ) -> Result<CanonicalDurableTransition> {
        let transition =
            truncate_log(&self.profile.profile, &self.state, generation, retain_from_sequence, authority_ref)
                .map_err(|issues| adapter_validation_error("simulated truncate", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn create_snapshot(&mut self, request: &SnapshotRequest) -> Result<CanonicalDurableTransition> {
        let transition = create_snapshot(&self.profile.profile, &self.state, request)
            .map_err(|issues| adapter_validation_error("simulated snapshot", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn apply_effect(&mut self, command: &EffectTransactionCommand) -> Result<CanonicalDurableTransition> {
        let transition = apply_effect_transaction(&self.profile.profile, &self.state, command)
            .map_err(|issues| adapter_validation_error("simulated effect", &issues))?;
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn inject_fault(&mut self, fault: &SimulatedDurabilityFault) -> Result<CanonicalDurableTransition> {
        let transition = match fault {
            SimulatedDurabilityFault::DelayCompletion { ticks } => {
                self.simulated_ticks = self
                    .simulated_ticks
                    .checked_add(*ticks)
                    .ok_or_else(|| MoltenError::invalid_harness("simulated durability time overflow"))?;
                DurableTransition {
                    next: self.state.clone(),
                    outcome: MutationOutcome::Validated,
                    operation: "simulated-latency".to_string(),
                    affected_items: 0,
                    affected_bytes: 0,
                    retry_safe: true,
                    reconciliation_required: false,
                }
            }
            SimulatedDurabilityFault::ProcessCrash => simulate_process_crash(&self.state),
            SimulatedDurabilityFault::CorruptSnapshot { snapshot_ref } => {
                let next = mark_snapshot_corrupt(&self.state, snapshot_ref)
                    .map_err(|issue| adapter_validation_error("corrupt snapshot", &[issue]))?;
                DurableTransition {
                    next,
                    outcome: MutationOutcome::FailedAfterPossibleMutation,
                    operation: "simulated-snapshot-corruption".to_string(),
                    affected_items: 1,
                    affected_bytes: 0,
                    retry_safe: false,
                    reconciliation_required: true,
                }
            }
            SimulatedDurabilityFault::CrashBeforeMutation
            | SimulatedDurabilityFault::ResponseLostAfterCommit
            | SimulatedDurabilityFault::CapacityExhausted => {
                return Err(MoltenError::invalid_harness(
                    "operation-scoped durability fault requires append or batch execution",
                ));
            }
        };
        let canonical = canonical_durable_transition(&self.profile, &transition)?;
        self.state = transition.next;
        Ok(canonical)
    }

    pub fn recovery(&self, inventory: &RecoveryInventory) -> Result<CanonicalRecoveryDecision> {
        canonical_recovery_decision(&self.profile, &self.state, evaluate_recovery(&self.state, inventory))
    }

    fn synthetic_transition(
        &self,
        outcome: MutationOutcome,
        operation: &str,
        retry_safe: bool,
        reconciliation_required: bool,
    ) -> Result<CanonicalDurableTransition> {
        canonical_durable_transition(&self.profile, &DurableTransition {
            next: self.state.clone(),
            outcome,
            operation: operation.to_string(),
            affected_items: 0,
            affected_bytes: 0,
            retry_safe,
            reconciliation_required,
        })
    }
}

impl DurableCommandShell for RedbDurableStateAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<CanonicalDurableTransition> {
        let result = match command {
            DurablePortCommand::Append(request) => self.append(request),
            DurablePortCommand::Flush { generation, durability } => self.flush(*generation, *durability),
            DurablePortCommand::Truncate {
                generation,
                retain_from_sequence,
                authority_ref,
            } => self.truncate(*generation, *retain_from_sequence, authority_ref.as_deref()),
            DurablePortCommand::AtomicBatch(request) => self.apply_batch(request),
            DurablePortCommand::Snapshot { request, bytes } => self.create_snapshot(request, bytes),
            DurablePortCommand::Effect(command) => self.apply_effect(command),
        };
        result.map_err(|error| FabricPortError::storage(error.to_string()))
    }
}

impl DurableCommandShell for SimulatedDurableStateAdapter {
    fn profile_id(&self) -> &str {
        &self.profile.profile.profile_id
    }

    fn execute_command(&mut self, command: &DurablePortCommand) -> FabricPortResult<CanonicalDurableTransition> {
        let result = match command {
            DurablePortCommand::Append(request) => self.append(request, None),
            DurablePortCommand::Flush { generation, durability } => self.flush(*generation, *durability),
            DurablePortCommand::Truncate {
                generation,
                retain_from_sequence,
                authority_ref,
            } => self.truncate(*generation, *retain_from_sequence, authority_ref.as_deref()),
            DurablePortCommand::AtomicBatch(request) => self.apply_batch(request, None),
            DurablePortCommand::Snapshot { request, .. } => self.create_snapshot(request),
            DurablePortCommand::Effect(command) => self.apply_effect(command),
        };
        result.map_err(|error| FabricPortError::storage(error.to_string()))
    }
}

fn initialize_tables(database: &redb::Database) -> Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        write.open_table(LOG_TABLE).map_err(adapter_error)?;
        write.open_table(ORDERED_TABLE).map_err(adapter_error)?;
        write.open_table(SNAPSHOT_TABLE).map_err(adapter_error)?;
        write.open_table(EFFECT_TABLE).map_err(adapter_error)?;
    }
    write.commit().map_err(adapter_error)
}

fn load_state(database: &redb::Database, descriptor: DurableNamespaceDescriptor) -> Result<DurableState> {
    let mut state = DurableState::empty(descriptor);
    let read = database.begin_read().map_err(adapter_error)?;
    {
        let table = read.open_table(LOG_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (sequence, bytes) = item.map_err(adapter_error)?;
            let record = decode_log_record(sequence.value(), bytes.value())?;
            state.durable_bytes = checked_adapter_add(state.durable_bytes, byte_count(record.value.len())?)?;
            state.durable_log.push(record);
        }
    }
    {
        let table = read.open_table(ORDERED_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (key, bytes) = item.map_err(adapter_error)?;
            let value = decode_versioned_value(bytes.value())?;
            state.durable_bytes = checked_adapter_add(state.durable_bytes, byte_count(key.value().len())?)?;
            state.durable_bytes = checked_adapter_add(state.durable_bytes, byte_count(value.value.len())?)?;
            state.ordered.insert(key.value().to_vec(), value);
        }
    }
    {
        let table = read.open_table(SNAPSHOT_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (snapshot_ref, bytes) = item.map_err(adapter_error)?;
            let snapshot = decode_snapshot(snapshot_ref.value(), bytes.value())?;
            state.snapshots.insert(snapshot.snapshot_ref.clone(), snapshot);
        }
    }
    {
        let table = read.open_table(EFFECT_TABLE).map_err(adapter_error)?;
        for item in table.iter().map_err(adapter_error)? {
            let (transaction_id, bytes) = item.map_err(adapter_error)?;
            let effect = decode_effect(transaction_id.value(), bytes.value())?;
            state.effects.insert(effect.transaction_id.clone(), effect);
        }
    }
    Ok(state)
}

fn persist_log(database: &redb::Database, records: &[LogRecord]) -> Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(LOG_TABLE).map_err(adapter_error)?;
        for record in records {
            let bytes = encode_log_record(record)?;
            table.insert(record.sequence, bytes.as_slice()).map_err(adapter_error)?;
        }
    }
    write.commit().map_err(adapter_error)
}

fn replace_log(database: &redb::Database, records: &[LogRecord]) -> Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(LOG_TABLE).map_err(adapter_error)?;
        let keys = table
            .iter()
            .map_err(adapter_error)?
            .map(|item| item.map(|(key, _value)| key.value()).map_err(adapter_error))
            .collect::<Result<Vec<_>>>()?;
        for key in keys {
            table.remove(key).map_err(adapter_error)?;
        }
        for record in records {
            let bytes = encode_log_record(record)?;
            table.insert(record.sequence, bytes.as_slice()).map_err(adapter_error)?;
        }
    }
    write.commit().map_err(adapter_error)
}

fn persist_ordered_batch(database: &redb::Database, request: &AtomicBatchRequest, next: &DurableState) -> Result<()> {
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(ORDERED_TABLE).map_err(adapter_error)?;
        for mutation in &request.mutations {
            match mutation {
                OrderedMutation::Put { key, .. } => {
                    let value = next
                        .ordered
                        .get(key)
                        .ok_or_else(|| MoltenError::invalid_harness("admitted ordered mutation produced no value"))?;
                    let bytes = encode_versioned_value(value)?;
                    table.insert(key.as_slice(), bytes.as_slice()).map_err(adapter_error)?;
                }
                OrderedMutation::Delete { key, .. } => {
                    table.remove(key.as_slice()).map_err(adapter_error)?;
                }
            }
        }
    }
    write.commit().map_err(adapter_error)
}

fn persist_snapshot(
    root: &DurableStoreRoot,
    database: &redb::Database,
    request: &SnapshotRequest,
    bytes: &[u8],
    next: &DurableState,
) -> Result<()> {
    let snapshot = next
        .snapshots
        .get(&request.snapshot_ref)
        .ok_or_else(|| MoltenError::invalid_harness("admitted snapshot transition produced no snapshot"))?;
    let relative = format!("{SNAPSHOT_DIRECTORY}/{}.bin", snapshot_file_stem(&request.content_ref)?);
    root.root().write(&LocalStorePath::parse(&relative)?, bytes)?;
    let encoded = encode_snapshot(snapshot)?;
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(SNAPSHOT_TABLE).map_err(adapter_error)?;
        table.insert(request.snapshot_ref.as_str(), encoded.as_slice()).map_err(adapter_error)?;
    }
    write.commit().map_err(adapter_error)
}

fn persist_effect(database: &redb::Database, command: &EffectTransactionCommand, next: &DurableState) -> Result<()> {
    let transaction_id = effect_transaction_id(command);
    let effect = next
        .effects
        .get(transaction_id)
        .ok_or_else(|| MoltenError::invalid_harness("admitted effect transition produced no effect state"))?;
    let encoded = encode_effect(effect)?;
    let write = database.begin_write().map_err(adapter_error)?;
    {
        let mut table = write.open_table(EFFECT_TABLE).map_err(adapter_error)?;
        table.insert(transaction_id, encoded.as_slice()).map_err(adapter_error)?;
    }
    write.commit().map_err(adapter_error)
}

fn effect_transaction_id(command: &EffectTransactionCommand) -> &str {
    match command {
        EffectTransactionCommand::Reserve { transaction_id, .. }
        | EffectTransactionCommand::Commit { transaction_id, .. }
        | EffectTransactionCommand::Abort { transaction_id, .. }
        | EffectTransactionCommand::Expire { transaction_id, .. }
        | EffectTransactionCommand::MarkUncertain { transaction_id, .. }
        | EffectTransactionCommand::Reconcile { transaction_id, .. } => transaction_id,
    }
}

fn encode_log_record(record: &LogRecord) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_byte(&mut bytes, encode_level(record.durability));
    push_blob(&mut bytes, record.value_ref.as_bytes())?;
    push_blob(&mut bytes, &record.value)?;
    Ok(bytes)
}

fn decode_log_record(sequence: u64, bytes: &[u8]) -> Result<LogRecord> {
    let mut cursor = ByteCursor::new(bytes);
    let durability = decode_level(cursor.take_byte()?)?;
    let value_ref = cursor.take_string()?;
    let value = cursor.take_blob()?.to_vec();
    cursor.finish()?;
    Ok(LogRecord {
        sequence,
        value,
        value_ref,
        durability,
    })
}

fn encode_versioned_value(value: &VersionedValue) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, value.version);
    push_blob(&mut bytes, value.value_ref.as_bytes())?;
    push_blob(&mut bytes, &value.value)?;
    Ok(bytes)
}

fn decode_versioned_value(bytes: &[u8]) -> Result<VersionedValue> {
    let mut cursor = ByteCursor::new(bytes);
    let version = cursor.take_u64()?;
    let value_ref = cursor.take_string()?;
    let value = cursor.take_blob()?.to_vec();
    cursor.finish()?;
    Ok(VersionedValue {
        value,
        value_ref,
        version,
    })
}

fn encode_snapshot(snapshot: &SnapshotRecord) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_byte(&mut bytes, encode_snapshot_kind(snapshot.kind));
    push_blob(&mut bytes, snapshot.content_ref.as_bytes())?;
    push_blob(&mut bytes, snapshot.source_namespace.as_bytes())?;
    push_u64(&mut bytes, snapshot.source_generation);
    push_blob(&mut bytes, snapshot.value_schema_ref.as_bytes())?;
    push_optional_u64(&mut bytes, snapshot.covered_log_sequence);
    push_blob(&mut bytes, snapshot.ordered_state_ref.as_bytes())?;
    push_byte(&mut bytes, encode_level(snapshot.durability));
    push_byte(&mut bytes, u8::from(snapshot.corrupted));
    Ok(bytes)
}

fn decode_snapshot(snapshot_ref: &str, bytes: &[u8]) -> Result<SnapshotRecord> {
    let mut cursor = ByteCursor::new(bytes);
    let kind = decode_snapshot_kind(cursor.take_byte()?)?;
    let content_ref = cursor.take_string()?;
    let source_namespace = cursor.take_string()?;
    let source_generation = cursor.take_u64()?;
    let value_schema_ref = cursor.take_string()?;
    let covered_log_sequence = cursor.take_optional_u64()?;
    let ordered_state_ref = cursor.take_string()?;
    let durability = decode_level(cursor.take_byte()?)?;
    let corrupted = decode_bool(cursor.take_byte()?)?;
    cursor.finish()?;
    Ok(SnapshotRecord {
        kind,
        snapshot_ref: snapshot_ref.to_string(),
        content_ref,
        source_namespace,
        source_generation,
        value_schema_ref,
        covered_log_sequence,
        ordered_state_ref,
        durability,
        corrupted,
    })
}

fn encode_effect(effect: &EffectTransactionState) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, effect.generation);
    push_blob(&mut bytes, effect.operation_ref.as_bytes())?;
    push_byte(&mut bytes, encode_phase(effect.phase));
    push_optional_u64(&mut bytes, effect.expires_at_tick);
    for flag in [
        effect.profile.durable_reservation,
        effect.profile.exclusive,
        effect.profile.expiring,
        effect.profile.idempotent_commit,
        effect.profile.compensating_abort,
    ] {
        push_byte(&mut bytes, u8::from(flag));
    }
    Ok(bytes)
}

fn decode_effect(transaction_id: &str, bytes: &[u8]) -> Result<EffectTransactionState> {
    let mut cursor = ByteCursor::new(bytes);
    let generation = cursor.take_u64()?;
    let operation_ref = cursor.take_string()?;
    let phase = decode_phase(cursor.take_byte()?)?;
    let expires_at_tick = cursor.take_optional_u64()?;
    let durable_reservation = decode_bool(cursor.take_byte()?)?;
    let exclusive = decode_bool(cursor.take_byte()?)?;
    let expiring = decode_bool(cursor.take_byte()?)?;
    let idempotent_commit = decode_bool(cursor.take_byte()?)?;
    let compensating_abort = decode_bool(cursor.take_byte()?)?;
    cursor.finish()?;
    Ok(EffectTransactionState {
        transaction_id: transaction_id.to_string(),
        generation,
        operation_ref,
        phase,
        expires_at_tick,
        profile: EffectTransactionProfile {
            durable_reservation,
            exclusive,
            expiring,
            idempotent_commit,
            compensating_abort,
        },
    })
}

fn encode_level(level: DurabilityLevel) -> u8 {
    match level {
        DurabilityLevel::Buffered => LEVEL_BUFFERED,
        DurabilityLevel::ProcessLoss => LEVEL_PROCESS_LOSS,
        DurabilityLevel::MachineLoss => LEVEL_MACHINE_LOSS,
    }
}

fn decode_level(value: u8) -> Result<DurabilityLevel> {
    match value {
        LEVEL_BUFFERED => Ok(DurabilityLevel::Buffered),
        LEVEL_PROCESS_LOSS => Ok(DurabilityLevel::ProcessLoss),
        LEVEL_MACHINE_LOSS => Ok(DurabilityLevel::MachineLoss),
        _ => Err(MoltenError::invalid_harness(format!("unknown durability level code {value}"))),
    }
}

fn encode_snapshot_kind(kind: SnapshotKind) -> u8 {
    match kind {
        SnapshotKind::Snapshot => SNAPSHOT_KIND_SNAPSHOT,
        SnapshotKind::Checkpoint => SNAPSHOT_KIND_CHECKPOINT,
    }
}

fn decode_snapshot_kind(value: u8) -> Result<SnapshotKind> {
    match value {
        SNAPSHOT_KIND_SNAPSHOT => Ok(SnapshotKind::Snapshot),
        SNAPSHOT_KIND_CHECKPOINT => Ok(SnapshotKind::Checkpoint),
        _ => Err(MoltenError::invalid_harness(format!("unknown snapshot kind code {value}"))),
    }
}

fn encode_phase(phase: EffectTransactionPhase) -> u8 {
    match phase {
        EffectTransactionPhase::Reserved => PHASE_RESERVED,
        EffectTransactionPhase::Committed => PHASE_COMMITTED,
        EffectTransactionPhase::Aborted => PHASE_ABORTED,
        EffectTransactionPhase::Expired => PHASE_EXPIRED,
        EffectTransactionPhase::Uncertain => PHASE_UNCERTAIN,
        EffectTransactionPhase::ReconciledCommitted => PHASE_RECONCILED_COMMITTED,
        EffectTransactionPhase::ReconciledAborted => PHASE_RECONCILED_ABORTED,
    }
}

fn decode_phase(value: u8) -> Result<EffectTransactionPhase> {
    match value {
        PHASE_RESERVED => Ok(EffectTransactionPhase::Reserved),
        PHASE_COMMITTED => Ok(EffectTransactionPhase::Committed),
        PHASE_ABORTED => Ok(EffectTransactionPhase::Aborted),
        PHASE_EXPIRED => Ok(EffectTransactionPhase::Expired),
        PHASE_UNCERTAIN => Ok(EffectTransactionPhase::Uncertain),
        PHASE_RECONCILED_COMMITTED => Ok(EffectTransactionPhase::ReconciledCommitted),
        PHASE_RECONCILED_ABORTED => Ok(EffectTransactionPhase::ReconciledAborted),
        _ => Err(MoltenError::invalid_harness(format!("unknown effect phase code {value}"))),
    }
}

fn push_byte(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_optional_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    push_byte(bytes, u8::from(value.is_some()));
    if let Some(value) = value {
        push_u64(bytes, value);
    }
}

fn push_blob(bytes: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    push_u64(bytes, byte_count(value.len())?);
    bytes.extend_from_slice(value);
    Ok(())
}

struct ByteCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take_byte(&mut self) -> Result<u8> {
        let value = self
            .bytes
            .get(self.offset)
            .copied()
            .ok_or_else(|| MoltenError::invalid_harness("truncated durable adapter record"))?;
        self.offset = self
            .offset
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("durable adapter cursor overflow"))?;
        Ok(value)
    }

    fn take_u64(&mut self) -> Result<u64> {
        let end = self
            .offset
            .checked_add(LENGTH_PREFIX_BYTES)
            .ok_or_else(|| MoltenError::invalid_harness("durable adapter cursor overflow"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| MoltenError::invalid_harness("truncated durable adapter integer"))?;
        let array: [u8; LENGTH_PREFIX_BYTES] = bytes
            .try_into()
            .map_err(|_| MoltenError::invalid_harness("invalid durable adapter integer width"))?;
        self.offset = end;
        Ok(u64::from_be_bytes(array))
    }

    fn take_blob(&mut self) -> Result<&'a [u8]> {
        let length = usize::try_from(self.take_u64()?)
            .map_err(|_| MoltenError::invalid_harness("durable adapter blob length overflow"))?;
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| MoltenError::invalid_harness("durable adapter cursor overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| MoltenError::invalid_harness("truncated durable adapter blob"))?;
        self.offset = end;
        Ok(value)
    }

    fn take_string(&mut self) -> Result<String> {
        String::from_utf8(self.take_blob()?.to_vec())
            .map_err(|error| MoltenError::invalid_harness(format!("durable adapter string is not UTF-8: {error}")))
    }

    fn take_optional_u64(&mut self) -> Result<Option<u64>> {
        if decode_bool(self.take_byte()?)? {
            self.take_u64().map(Some)
        } else {
            Ok(None)
        }
    }

    fn finish(&self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(MoltenError::invalid_harness("durable adapter record contains trailing bytes"))
        }
    }
}

fn decode_bool(value: u8) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(MoltenError::invalid_harness(format!("invalid durable adapter boolean {value}"))),
    }
}

fn snapshot_file_stem(snapshot_ref: &str) -> Result<&str> {
    snapshot_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| MoltenError::invalid_harness("snapshot ref must be a BLAKE3 content ref"))
}

fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn byte_count(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| MoltenError::invalid_harness("durable adapter byte count overflow"))
}

fn checked_adapter_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness("durable adapter byte accounting overflow"))
}

fn adapter_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("durable adapter error: {error}"))
}

fn adapter_validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("durable adapter {label} denied: {issues:?}"))
}
