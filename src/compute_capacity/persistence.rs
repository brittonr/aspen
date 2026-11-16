//! Capability-rooted, atomically compared canonical compute state.

use redb::ReadableDatabase;
use redb::ReadableTable;

use super::ports::ComputeStorePort;

const STATE_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("compute-capacity-state-v2");
const AUTHORITY_TABLE: redb::TableDefinition<&str, &str> = redb::TableDefinition::new("compute-capacity-authority-v1");
const STORE_FILE: &str = "compute-capacity.redb";
const STATE_KEY: &str = "current";
const MAX_STATE_BYTES: usize = 4 * 1024 * 1024;
const STATE_LABEL: &str = "compute-capacity-owner-state-v2";

/// One durable node-local allocation owner. Cross-host exclusion requires an admitted
/// selected owner, not concurrent independent copies of this store.
#[derive(Debug)]
pub struct Redb {
    database: redb::Database,
    authority_id: String,
}

impl Redb {
    // r[impl molten.compute_capacity.reservation_authority]
    // r[impl molten.compute_capacity.durable_reconciliation]
    pub fn open(
        root: &crate::local_store::DurableStoreRoot,
        authority_id: &str,
    ) -> crate::fabric::FabricPortResult<Self> {
        if authority_id.is_empty()
            || authority_id.len() > molten_core::compute_capacity::MAX_TEXT
            || authority_id.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(crate::fabric::FabricPortError::malformed("invalid selected compute authority"));
        }
        let path = crate::local_store::Locator::parse(STORE_FILE).map_err(crate::fabric::FabricPortError::from)?;
        let is_existing =
            root.root().entry_kind_optional(&path).map_err(crate::fabric::FabricPortError::from)?.is_some();
        let file = if is_existing {
            root.root().open_database_file(&path)
        } else {
            root.root().create_database_file(&path)
        }
        .map_err(crate::fabric::FabricPortError::from)?;
        if is_existing && file.metadata().map_err(storage_error)?.len() == 0 {
            return Err(crate::fabric::FabricPortError::uncertain(
                "existing compute database is empty; reconciliation required",
            ));
        }
        let mut builder = redb::Database::builder();
        if is_existing {
            builder.set_repair_callback(|session| session.abort());
        }
        let database = builder.create_file(file).map_err(storage_error)?;
        if is_existing {
            let read = database.begin_read().map_err(storage_error)?;
            read.open_table(STATE_TABLE).map_err(storage_error)?;
            let authority = read.open_table(AUTHORITY_TABLE).map_err(storage_error)?;
            let recorded = authority
                .get("owner")
                .map_err(storage_error)?
                .ok_or_else(|| crate::fabric::FabricPortError::malformed("compute authority identity missing"))?;
            if recorded.value() != authority_id {
                return Err(crate::fabric::FabricPortError::capability(
                    "compute authority identity differs from selected owner",
                ));
            }
        } else {
            let write = database.begin_write().map_err(storage_error)?;
            write.open_table(STATE_TABLE).map_err(storage_error)?;
            {
                let mut authority = write.open_table(AUTHORITY_TABLE).map_err(storage_error)?;
                authority.insert("owner", authority_id).map_err(storage_error)?;
            }
            write.commit().map_err(uncertain_error)?;
        }
        Ok(Self {
            database,
            authority_id: authority_id.to_string(),
        })
    }
}

// r[impl molten.compute_capacity.durable_reconciliation]
impl ComputeStorePort for Redb {
    fn load(&self) -> crate::fabric::FabricPortResult<molten_core::compute_capacity::CapacityState> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let table = read.open_table(STATE_TABLE).map_err(storage_error)?;
        match table.get(STATE_KEY).map_err(storage_error)? {
            Some(bytes) => decode_state(bytes.value()),
            None => Ok(molten_core::compute_capacity::CapacityState {
                offers: std::collections::BTreeMap::new(),
                allocations: std::collections::BTreeMap::new(),
                charge_cursor: 0,
                retired_allocations: std::collections::VecDeque::new(),
            }),
        }
    }
    fn selected_authority_id(&self) -> &str {
        &self.authority_id
    }

    fn compare_and_store(
        &mut self,
        expected: &molten_core::compute_capacity::CapacityState,
        next: &molten_core::compute_capacity::CapacityState,
    ) -> crate::fabric::FabricPortResult<()> {
        for allocation in next.allocations.values() {
            if allocation.authority_ids.iter().any(|authority| authority != &self.authority_id) {
                return Err(crate::fabric::FabricPortError::capability(
                    "selected compute owner does not control every claimed pool",
                ));
            }
        }
        let expected_bytes = encode_state(expected)?;
        let next_bytes = encode_state(next)?;
        let write = self.database.begin_write().map_err(storage_error)?;
        {
            let mut table = write.open_table(STATE_TABLE).map_err(storage_error)?;
            let is_match = {
                let current = table.get(STATE_KEY).map_err(storage_error)?;
                match current.as_ref() {
                    Some(bytes) => bytes.value() == expected_bytes.as_slice(),
                    None => {
                        *expected
                            == (molten_core::compute_capacity::CapacityState {
                                offers: std::collections::BTreeMap::new(),
                                allocations: std::collections::BTreeMap::new(),
                                charge_cursor: 0,
                                retired_allocations: std::collections::VecDeque::new(),
                            })
                    }
                }
            };
            if !is_match {
                return Err(crate::fabric::FabricPortError::storage("compute state changed before atomic commit"));
            }
            table.insert(STATE_KEY, next_bytes.as_slice()).map_err(storage_error)?;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(())
    }
}

// Canonical state encoding is kept separate from the Redb transaction boundary.
include!("persistence/codec.rs");

fn storage_error(error: impl std::fmt::Display) -> crate::fabric::FabricPortError {
    crate::fabric::FabricPortError::storage(format!("compute store: {error}"))
}

fn uncertain_error(error: impl std::fmt::Display) -> crate::fabric::FabricPortError {
    crate::fabric::FabricPortError::uncertain(format!("compute store outcome uncertain: {error}"))
}
