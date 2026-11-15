//! Assignment-owner persistence. Both mechanisms use the same canonical state and CAS protocol.

use redb::ReadableDatabase;
use redb::ReadableTable;

mod codec;

const ASSIGNMENTS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("fabric-assignments-v1");
const ROLES: redb::TableDefinition<&str, &str> = redb::TableDefinition::new("fabric-assignment-role-slots-v1");
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
/// epoch/token. Canonical packed Preserves records are bounded to 4 MiB, with the core's existing
/// 4,096 transition-reference and 256-byte identity-token limits. There is no implicit retry,
/// role-effect recovery, quorum, or hardware-loss durability claim; unresolved intents require
/// reconciliation. This supplies node-local persistence, not distributed exclusion or operator
/// authorization.
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
}

impl crate::fabric_membership::AssignmentPersistence for Redb {
    fn initialize(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
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
            let predecessor = roles.get(key.as_str()).map_err(storage_error)?.map(|value| value.value().to_string());
            let predecessor = predecessor
                .map(|id| {
                    let bytes = table
                        .get(id.as_str())
                        .map_err(storage_error)?
                        .ok_or_else(|| crate::fabric::FabricPortError::malformed("role slot predecessor is missing"))?;
                    let predecessor = codec::decode_record(bytes.value())?;
                    require_key(&predecessor, &id)?;
                    predecessor.current()
                })
                .transpose()?;
            admit_initialization(predecessor.as_ref(), &record.assignment)?;
            roles.insert(key.as_str(), proposal.assignment_id.as_str()).map_err(storage_error)?;
            table.insert(proposal.assignment_id.as_str(), bytes.as_slice()).map_err(storage_error)?;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(current)
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
            let (reference, bytes) = record.finish(transition, intent_ref, role_effect_ref)?;
            commit_ref = reference;
            table.insert(transition.next.assignment_id.as_str(), bytes.as_slice()).map_err(uncertain_error)?;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(commit_ref)
    }
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_membership/persistence/parts/memory/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_membership/persistence/parts/protocol/body.rs"));
