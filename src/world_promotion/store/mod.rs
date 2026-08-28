use molten_node_host::node_state::NodeStateNamespace;
use redb::ReadableDatabase;

use crate::error::MoltenError;
use crate::error::Result;
use crate::world_head::LocalWorldHeadStore;

mod outbox;
mod transaction;

pub(super) const WORLD_HEADS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("world_heads_v1");
pub(super) const PROMOTIONS_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_promotions_v1");
pub(super) const RESERVATIONS_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_release_reservations_v1");
pub(super) const ATTEMPTS_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("world_release_attempts_v1");

pub struct LocalWorldPromotionStore {
    pub(super) heads: LocalWorldHeadStore,
}

impl LocalWorldPromotionStore {
    pub fn open(storage: &NodeStateNamespace) -> Result<Self> {
        let heads = LocalWorldHeadStore::open(storage)?;
        initialize_tables(heads.database())?;
        Ok(Self { heads })
    }

    pub(super) const fn database(&self) -> &redb::Database {
        self.heads.database()
    }

    #[cfg(test)]
    pub(crate) fn head_store_mut(&mut self) -> &mut LocalWorldHeadStore {
        &mut self.heads
    }
}

fn initialize_tables(database: &redb::Database) -> Result<()> {
    let write = database.begin_write().map_err(store_error)?;
    {
        write.open_table(PROMOTIONS_TABLE).map_err(store_error)?;
        write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
        write.open_table(ATTEMPTS_TABLE).map_err(store_error)?;
    }
    write.commit().map_err(store_error)
}

pub(super) fn store_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("world promotion store failed: {error}"))
}
