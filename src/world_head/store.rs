use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadConflictSet;
use molten_core::world_head::WorldHeadState;
use molten_core::world_head::WorldHeadTransitionPlan;
use molten_node_host::node_state::NodeStateNamespace;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStatePath;
use redb::ReadableDatabase;
use redb::ReadableTable;

use super::CanonicalWorldHeadConflict;
use super::CanonicalWorldHeadTransitionReceipt;
use super::WorldHeadConflictPort;
use super::WorldHeadFreshAdmission;
use super::WorldHeadMutationOutcome;
use super::WorldHeadPortError;
use super::WorldHeadReconciliationPort;
use super::WorldHeadStatePort;
use super::canonical_world_head_state;
use super::parse_canonical_world_head_state;
use crate::error::MoltenError;
use crate::error::Result;

const WORLD_HEAD_DATABASE_FILE: &str = "world-heads.redb";
const WORLD_HEADS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_heads_v1");
const WORLD_HEAD_TRANSITIONS_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_head_transitions_v1");
const WORLD_HEAD_CONFLICTS_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_head_conflicts_v1");
const WORLD_HEAD_UNCERTAIN_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_head_uncertain_v1");
const CONFLICT_KEY_SEPARATOR: &str = ":";

pub struct LocalWorldHeadStore {
    database: redb::Database,
}

impl LocalWorldHeadStore {
    pub fn open(storage: &NodeStateNamespace) -> Result<Self> {
        if storage.kind() != NodeStateNamespaceKind::Storage {
            return Err(MoltenError::invalid_harness("local world-head store requires the storage namespace"));
        }
        let path = NodeStatePath::parse(WORLD_HEAD_DATABASE_FILE)?;
        let file = storage.open_database_file(&path)?;
        let database = redb::Database::builder().create_file(file).map_err(store_error)?;
        initialize_tables(&database)?;
        Ok(Self { database })
    }

    pub fn transition_receipt(&self, receipt_ref: &str) -> Result<Option<Vec<u8>>> {
        let read = self.database.begin_read().map_err(store_error)?;
        let table = read.open_table(WORLD_HEAD_TRANSITIONS_TABLE).map_err(store_error)?;
        table.get(receipt_ref).map_err(store_error).map(|value| value.map(|guard| guard.value().to_vec()))
    }

    pub(crate) const fn database(&self) -> &redb::Database {
        &self.database
    }
}

impl WorldHeadStatePort for LocalWorldHeadStore {
    fn read_head(&self, branch_id: &WorldBranchId) -> std::result::Result<Option<WorldHeadState>, WorldHeadPortError> {
        read_head(&self.database, branch_id)
    }

    fn apply_transition<F>(
        &mut self,
        plan: &WorldHeadTransitionPlan,
        receipt: &CanonicalWorldHeadTransitionReceipt,
        recheck: F,
    ) -> std::result::Result<WorldHeadMutationOutcome, WorldHeadPortError>
    where
        F: FnOnce(Option<&WorldHeadState>) -> std::result::Result<WorldHeadFreshAdmission, WorldHeadPortError>,
    {
        let write = self.database.begin_write().map_err(port_store_error)?;
        let observed = {
            let table = write.open_table(WORLD_HEADS_TABLE).map_err(port_store_error)?;
            table
                .get(plan.after.branch_id.as_str())
                .map_err(port_store_error)?
                .map(|guard| guard.value().to_vec())
                .map(|bytes| parse_canonical_world_head_state(&bytes).map_err(port_molten_error))
                .transpose()?
        };

        if observed.as_ref() == Some(&plan.after) {
            return Ok(WorldHeadMutationOutcome::AlreadyApplied);
        }
        if !before_matches(plan.before.as_ref(), observed.as_ref()) {
            return Ok(WorldHeadMutationOutcome::Stale);
        }
        let fresh = recheck(observed.as_ref())?;
        if !fresh.authentication_passed || !fresh.authority.admitted {
            return Err(WorldHeadPortError::new(
                "fresh-admission-denied",
                "authentication or authority denied inside the mutation boundary",
            ));
        }
        if fresh.authority.policy_ref != plan.after.policy_ref {
            return Err(WorldHeadPortError::new(
                "fresh-policy-mismatch",
                "fresh authority policy does not match the transition",
            ));
        }
        let expected_generation = plan.before.as_ref().map_or(0, |state| state.generation);
        if fresh.authority.observed_generation != expected_generation {
            return Ok(WorldHeadMutationOutcome::Stale);
        }

        let (_, state_bytes) = canonical_world_head_state(&plan.after).map_err(port_molten_error)?;
        {
            let mut heads = write.open_table(WORLD_HEADS_TABLE).map_err(port_store_error)?;
            heads.insert(plan.after.branch_id.as_str(), state_bytes.as_slice()).map_err(port_store_error)?;
        }
        {
            let mut transitions = write.open_table(WORLD_HEAD_TRANSITIONS_TABLE).map_err(port_store_error)?;
            transitions
                .insert(receipt.receipt_ref.as_str(), receipt.bytes.as_slice())
                .map_err(port_store_error)?;
        }
        match write.commit() {
            Ok(()) => Ok(WorldHeadMutationOutcome::Applied),
            Err(_) => Ok(WorldHeadMutationOutcome::Uncertain),
        }
    }
}

impl WorldHeadConflictPort for LocalWorldHeadStore {
    fn record_conflict(
        &mut self,
        conflict: &WorldHeadConflictSet,
        canonical: &CanonicalWorldHeadConflict,
    ) -> std::result::Result<(), WorldHeadPortError> {
        let key = conflict_key(&conflict.branch_id, &canonical.conflict_ref);
        let write = self.database.begin_write().map_err(port_store_error)?;
        {
            let mut table = write.open_table(WORLD_HEAD_CONFLICTS_TABLE).map_err(port_store_error)?;
            if let Some(existing) = table.get(key.as_str()).map_err(port_store_error)? {
                if existing.value() != canonical.bytes.as_slice() {
                    return Err(WorldHeadPortError::new("conflict-record-mismatch", "existing conflict bytes differ"));
                }
                return Ok(());
            }
            table.insert(key.as_str(), canonical.bytes.as_slice()).map_err(port_store_error)?;
        }
        write.commit().map_err(port_store_error)
    }

    fn read_conflicts(&self, branch_id: &WorldBranchId) -> std::result::Result<Vec<Vec<u8>>, WorldHeadPortError> {
        let prefix = format!("{}{CONFLICT_KEY_SEPARATOR}", branch_id.as_str());
        let read = self.database.begin_read().map_err(port_store_error)?;
        let table = read.open_table(WORLD_HEAD_CONFLICTS_TABLE).map_err(port_store_error)?;
        let mut records = Vec::new();
        for entry in table.iter().map_err(port_store_error)? {
            let (key, value) = entry.map_err(port_store_error)?;
            if key.value().starts_with(&prefix) {
                records.push(value.value().to_vec());
            }
        }
        Ok(records)
    }
}

impl WorldHeadReconciliationPort for LocalWorldHeadStore {
    fn record_uncertain_transition(
        &mut self,
        plan: &WorldHeadTransitionPlan,
        receipt: &CanonicalWorldHeadTransitionReceipt,
    ) -> std::result::Result<(), WorldHeadPortError> {
        let write = self.database.begin_write().map_err(port_store_error)?;
        {
            let mut table = write.open_table(WORLD_HEAD_UNCERTAIN_TABLE).map_err(port_store_error)?;
            table.insert(plan.claim_ref.as_str(), receipt.bytes.as_slice()).map_err(port_store_error)?;
        }
        write.commit().map_err(port_store_error)
    }
}

fn initialize_tables(database: &redb::Database) -> Result<()> {
    let write = database.begin_write().map_err(store_error)?;
    {
        write.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
        write.open_table(WORLD_HEAD_TRANSITIONS_TABLE).map_err(store_error)?;
        write.open_table(WORLD_HEAD_CONFLICTS_TABLE).map_err(store_error)?;
        write.open_table(WORLD_HEAD_UNCERTAIN_TABLE).map_err(store_error)?;
    }
    write.commit().map_err(store_error)
}

fn read_head(
    database: &redb::Database,
    branch_id: &WorldBranchId,
) -> std::result::Result<Option<WorldHeadState>, WorldHeadPortError> {
    let read = database.begin_read().map_err(port_store_error)?;
    let table = read.open_table(WORLD_HEADS_TABLE).map_err(port_store_error)?;
    table
        .get(branch_id.as_str())
        .map_err(port_store_error)?
        .map(|guard| parse_canonical_world_head_state(guard.value()).map_err(port_molten_error))
        .transpose()
}

fn before_matches(expected: Option<&WorldHeadState>, observed: Option<&WorldHeadState>) -> bool {
    expected == observed
}

fn conflict_key(branch_id: &WorldBranchId, conflict_ref: &str) -> String {
    format!("{}{CONFLICT_KEY_SEPARATOR}{conflict_ref}", branch_id.as_str())
}

fn store_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("world-head store failed: {error}"))
}

fn port_store_error(error: impl std::fmt::Display) -> WorldHeadPortError {
    WorldHeadPortError::new("world-head-store", error.to_string())
}

fn port_molten_error(error: MoltenError) -> WorldHeadPortError {
    WorldHeadPortError::new("world-head-codec", error.to_string())
}
