use redb::ReadableDatabase;

const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("node_service_host_v1");
const MAX_STATE_BYTES: usize = 16_777_216;

pub(super) struct Database {
    database: redb::Database,
}

impl Database {
    pub(super) fn open(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Self> {
        let ledger = root.ledger()?;
        let path = crate::node_state::RelativePath::parse("node-service-host.redb")?;
        let file = ledger.open_database_file(&path)?;
        let database = redb::Database::builder().create_file(file).map_err(storage_error)?;
        let write = database.begin_write().map_err(storage_error)?;
        {
            let _table = write.open_table(TABLE).map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)?;
        Ok(Self { database })
    }

    pub(super) fn load(&self) -> crate::error::Result<super::State> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let table = read.open_table(TABLE).map_err(storage_error)?;
        let row = table.get("state").map_err(storage_error)?;
        match row {
            None => Ok(super::State::default()),
            Some(row) => {
                if row.value().len() > MAX_STATE_BYTES {
                    return Err(super::invalid("stored node hosting state exceeds byte bound"));
                }
                super::codec::decode(row.value())
            }
        }
    }

    pub(super) fn save(&mut self, state: &super::State) -> crate::error::Result<()> {
        let bytes = crate::preserves_rail::canonical_bytes(&super::codec::encode(state)?)?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(super::invalid("node hosting state exceeds byte bound"));
        }
        let mut write = self.database.begin_write().map_err(storage_error)?;
        write.set_durability(redb::Durability::Immediate).map_err(storage_error)?;
        {
            let mut table = write.open_table(TABLE).map_err(storage_error)?;
            table.insert("state", bytes.as_slice()).map_err(storage_error)?;
        }
        write.commit().map_err(storage_error)
    }
}

fn storage_error(error: impl std::fmt::Display) -> crate::error::Failure {
    super::invalid(format!("node hosting durable store: {error}"))
}
