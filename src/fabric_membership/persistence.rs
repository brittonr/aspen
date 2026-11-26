//! Assignment-owner persistence. Both mechanisms use the same canonical state and CAS protocol.

use redb::ReadableDatabase;
use redb::ReadableTable;

mod codec;

const ASSIGNMENTS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("fabric-assignments-v1");
const ROLES: redb::TableDefinition<&str, &str> = redb::TableDefinition::new("fabric-assignment-role-slots-v1");
/// Owner-written quarantine reconciliation records, keyed by quarantined assignment ID. Created on
/// first write, so databases from before this table existed open unchanged.
const RECONCILIATIONS: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("fabric-assignment-quarantine-reconciliations-v1");
const STORE_FILE: &str = "assignments.redb";

/// Node-local durable implementation of the existing assignment owner.
///
/// Open with a capability-derived `DurableStoreRoot`, preferably a dedicated ledger subdirectory.
/// Initialization accepts only a pure proposal; every state advance uses `record_intent` and
/// `commit`. Transactions compare the entire expected assignment and intent. Epoch, token,
/// generation and transition history are part of the canonical bytes. Pending intents survive
/// reopen and block current admission and further effects; opening never replays a role effect or
/// repairs old bytes. One same-owner role index (extension/service/role, excluding node) fences
/// superseded IDs. Succession requires an exactly bound, fully released predecessor and advanced
/// epoch/token, or a quarantined predecessor with explicit `QuarantineClearance` evidence.
/// Canonical packed Preserves records are bounded to 4 MiB, with the core's existing 4,096
/// transition-reference and 256-byte identity-token limits. There is no implicit retry, quorum,
/// or hardware-loss durability claim. Unresolved intents are recovered only explicitly:
/// `abort_pre_effect_intent` with definite non-execution evidence, or `reconcile_pending_intent`
/// with confirmed effect evidence or an uncertainty-preserving quarantine. This supplies
/// node-local persistence, not distributed exclusion or operator authorization.
#[derive(Debug)]
pub struct Redb {
    database: redb::Database,
}

impl Redb {
    /// Open or create `assignments.redb` beneath the supplied open capability.
    /// Existing empty or repair-requiring databases fail closed and are never initialized afresh.
    pub fn open(root: &crate::local_store::DurableStoreRoot) -> crate::fabric::FabricPortResult<Self> {
        let path = crate::local_store::Locator::parse(STORE_FILE).map_err(crate::fabric::FabricPortError::from)?;
        let has_existing_database =
            root.root().entry_kind_optional(&path).map_err(crate::fabric::FabricPortError::from)?.is_some();
        let file = if has_existing_database {
            root.root().open_database_file(&path)
        } else {
            root.root().create_database_file(&path)
        }
        .map_err(crate::fabric::FabricPortError::from)?;
        if has_existing_database && file.metadata().map_err(storage_error)?.len() == 0 {
            return Err(crate::fabric::FabricPortError::uncertain(
                "existing assignment database is empty; reconciliation required",
            ));
        }
        let mut builder = redb::Database::builder();
        if has_existing_database {
            builder.set_repair_callback(|session| session.abort());
        }
        let database = builder.create_file(file).map_err(storage_error)?;
        if has_existing_database {
            // A preexisting database without our table is not a new assignment owner.
            let read = database.begin_read().map_err(storage_error)?;
            read.open_table(ASSIGNMENTS).map_err(storage_error)?;
            read.open_table(ROLES).map_err(storage_error)?;
        } else {
            let write = database.begin_write().map_err(storage_error)?;
            write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            write.open_table(ROLES).map_err(storage_error)?;
            write.commit().map_err(uncertain_error)?;
        }
        Ok(Self { database })
    }

    fn initialize_slot(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
        quarantine: Option<QuarantineAdmission<'_>>,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        let record = codec::AssignmentRecord::propose(proposal)?;
        let current = record.current()?;
        let bytes = codec::encode_record(&record)?;
        let write = self.database.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            if table.get(proposal.assignment_id.as_str()).map_err(storage_error)?.is_some() {
                return Err(crate::fabric::FabricPortError::storage(
                    "assignment already exists; initialization cannot overwrite",
                ));
            }
            let key = role_key(&record.assignment)?;
            let mut roles = write.open_table(ROLES).map_err(storage_error)?;
            let predecessor_id = roles.get(key.as_str()).map_err(storage_error)?.map(|value| value.value().to_string());
            let predecessor = predecessor_id
                .as_ref()
                .map(|id| {
                    let bytes = table
                        .get(id.as_str())
                        .map_err(storage_error)?
                        .ok_or_else(|| crate::fabric::FabricPortError::malformed("role slot predecessor is missing"))?;
                    let predecessor = codec::decode_record(bytes.value())?;
                    require_key(&predecessor, id)?;
                    predecessor.current()
                })
                .transpose()?;
            let reconciliation = match (&quarantine, &predecessor_id) {
                (Some(_), Some(id)) => {
                    let reconciliations = write.open_table(RECONCILIATIONS).map_err(storage_error)?;
                    let stored = reconciliations.get(id.as_str()).map_err(storage_error)?;
                    stored.map(|bytes| codec::decode_reconciliation(bytes.value(), id)).transpose()?
                }
                _ => None,
            };
            admit_initialization(predecessor.as_ref(), &record.assignment, quarantine, reconciliation.as_ref())?;
            roles.insert(key.as_str(), proposal.assignment_id.as_str()).map_err(storage_error)?;
            table.insert(proposal.assignment_id.as_str(), bytes.as_slice()).map_err(storage_error)?;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(current)
    }

    /// Read-modify-write of one role-slot-owning record, plus any reconciliation record it
    /// creates, under a single transaction.
    fn recover<T>(
        &mut self,
        assignment_id: &str,
        resolve: impl FnOnce(&mut codec::AssignmentRecord) -> crate::fabric::FabricPortResult<codec::Resolution<T>>,
    ) -> crate::fabric::FabricPortResult<T> {
        let write = self.database.begin_write().map_err(storage_error)?;
        let resolved;
        {
            let mut table = write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            let mut record = {
                let bytes = table
                    .get(assignment_id)
                    .map_err(storage_error)?
                    .ok_or_else(|| crate::fabric::FabricPortError::storage("assignment missing during recovery"))?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, assignment_id)?;
            let roles = write.open_table(ROLES).map_err(storage_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(storage_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let resolution = resolve(&mut record)?;
            table.insert(assignment_id, resolution.bytes.as_slice()).map_err(storage_error)?;
            if let Some((_, bytes)) = &resolution.reconciliation {
                let mut reconciliations = write.open_table(RECONCILIATIONS).map_err(storage_error)?;
                reconciliations.insert(assignment_id, bytes.as_slice()).map_err(storage_error)?;
            }
            resolved = resolution.value;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(resolved)
    }
}

impl crate::fabric_membership::AssignmentPersistence for Redb {
    fn initialize(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        self.initialize_slot(proposal, None)
    }

    fn current(
        &self,
        assignment_id: &str,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::CurrentAssignment>> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let table = read.open_table(ASSIGNMENTS).map_err(storage_error)?;
        let Some(bytes) = table.get(assignment_id).map_err(storage_error)? else {
            return Ok(None);
        };
        let record = codec::decode_record(bytes.value())?;
        require_key(&record, assignment_id)?;
        let roles = read.open_table(ROLES).map_err(storage_error)?;
        let key = role_key(&record.assignment)?;
        let owner = roles.get(key.as_str()).map_err(storage_error)?;
        require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
        record.current().map(Some)
    }

    fn record_intent(
        &mut self,
        current: &crate::fabric_membership::RoleAssignment,
        transition: &crate::fabric_membership::AssignmentTransition,
    ) -> crate::fabric::FabricPortResult<String> {
        let write = self.database.begin_write().map_err(storage_error)?;
        let intent_ref;
        {
            let mut table = write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            let mut record = {
                let bytes = table.get(current.assignment_id.as_str()).map_err(storage_error)?.ok_or_else(|| {
                    crate::fabric::FabricPortError::storage("assignment must be initialized before intent")
                })?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, &current.assignment_id)?;
            let roles = write.open_table(ROLES).map_err(storage_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(storage_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let (reference, bytes) = record.prepare(current, transition)?;
            intent_ref = reference;
            table.insert(current.assignment_id.as_str(), bytes.as_slice()).map_err(storage_error)?;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(intent_ref)
    }

    fn commit(
        &mut self,
        transition: &crate::fabric_membership::AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> crate::fabric::FabricPortResult<String> {
        let write = self.database.begin_write().map_err(uncertain_error)?;
        let commit_ref;
        {
            let mut table = write.open_table(ASSIGNMENTS).map_err(uncertain_error)?;
            let mut record = {
                let bytes = table
                    .get(transition.next.assignment_id.as_str())
                    .map_err(uncertain_error)?
                    .ok_or_else(|| crate::fabric::FabricPortError::uncertain("assignment missing during commit"))?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, &transition.next.assignment_id)?;
            let roles = write.open_table(ROLES).map_err(uncertain_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(uncertain_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let resolution = record.finish(transition, intent_ref, role_effect_ref)?;
            table
                .insert(transition.next.assignment_id.as_str(), resolution.bytes.as_slice())
                .map_err(uncertain_error)?;
            if let Some((_, bytes)) = &resolution.reconciliation {
                let mut reconciliations = write.open_table(RECONCILIATIONS).map_err(uncertain_error)?;
                reconciliations
                    .insert(transition.next.assignment_id.as_str(), bytes.as_slice())
                    .map_err(uncertain_error)?;
            }
            commit_ref = resolution.value;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(commit_ref)
    }

    fn inspect_pending(
        &self,
        assignment_id: &str,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::PendingAssignmentIntent>> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let table = read.open_table(ASSIGNMENTS).map_err(storage_error)?;
        let Some(bytes) = table.get(assignment_id).map_err(storage_error)? else {
            return Ok(None);
        };
        let record = codec::decode_record(bytes.value())?;
        require_key(&record, assignment_id)?;
        let roles = read.open_table(ROLES).map_err(storage_error)?;
        let key = role_key(&record.assignment)?;
        let owner = roles.get(key.as_str()).map_err(storage_error)?;
        require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
        record.pending_intent()
    }

    fn abort_pre_effect_intent(
        &mut self,
        assignment_id: &str,
        intent_ref: &str,
        lifecycle: &mut dyn crate::fabric_membership::ExtensionRoleLifecyclePort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        self.recover(assignment_id, |record| record.abort(intent_ref, lifecycle))
    }

    fn reconcile_pending_intent(
        &mut self,
        assignment_id: &str,
        intent_ref: &str,
        reconciliation: crate::fabric_membership::PendingIntentReconciliation<'_>,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::ReconciledAssignment> {
        self.recover(assignment_id, |record| record.reconcile(intent_ref, reconciliation))
    }

    fn quarantine_reconciliation(
        &self,
        assignment_id: &str,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::QuarantineReconciliation>> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let reconciliations = match read.open_table(RECONCILIATIONS) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        let Some(bytes) = reconciliations.get(assignment_id).map_err(storage_error)? else {
            return Ok(None);
        };
        codec::decode_reconciliation(bytes.value(), assignment_id).map(Some)
    }

    fn reconcile_legacy_quarantine(
        &mut self,
        assignment_id: &str,
        authority: &mut dyn crate::fabric_membership::QuarantineAuthorityPort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::QuarantineReconciliation> {
        let write = self.database.begin_write().map_err(storage_error)?;
        let reconciliation;
        {
            let table = write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            let record = {
                let bytes = table
                    .get(assignment_id)
                    .map_err(storage_error)?
                    .ok_or_else(|| crate::fabric::FabricPortError::storage("assignment missing during migration"))?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, assignment_id)?;
            let roles = write.open_table(ROLES).map_err(storage_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(storage_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let mut reconciliations = write.open_table(RECONCILIATIONS).map_err(storage_error)?;
            if reconciliations.get(assignment_id).map_err(storage_error)?.is_some() {
                return Err(crate::fabric::FabricPortError::storage(
                    "quarantine reconciliation already recorded; migration cannot overwrite it",
                ));
            }
            let (migrated, bytes) = record.legacy_reconciliation(authority)?;
            reconciliations.insert(assignment_id, bytes.as_slice()).map_err(storage_error)?;
            reconciliation = migrated;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(reconciliation)
    }

    fn initialize_quarantine_successor(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
        clearance: &crate::fabric_membership::QuarantineClearance,
        fencing: &mut dyn crate::fabric_membership::FencingAuthorityPort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        let fence = observe_successor_fence(proposal, fencing)?;
        self.initialize_slot(proposal, Some(QuarantineAdmission { clearance, fence }))
    }
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_membership/persistence/parts/memory/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_membership/persistence/parts/protocol/body.rs"));
