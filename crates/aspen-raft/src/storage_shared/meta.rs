//! Metadata operations: Raft and state machine metadata read/write, accessors, and SQL executor.

use std::path::Path;
use std::sync::Arc;

use redb::Database;
use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CommitSnafu;
use super::DeserializeSnafu;
use super::GetSnafu;
use super::InsertSnafu;
use super::OpenTableSnafu;
use super::RAFT_META_TABLE;
use super::RemoveSnafu;
use super::SM_META_TABLE;
use super::SerializeSnafu;
use super::SharedRedbStorage;
use super::SharedStorageError;

impl SharedRedbStorage {
    /// Get the path to the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get a reference to the underlying database.
    ///
    /// This is useful for maintenance operations that need direct database access.
    pub fn db(&self) -> Arc<Database> {
        self.db.clone()
    }

    /// Create a SQL executor for this storage backend.
    ///
    /// Returns a DataFusion-based SQL executor that can query the KV data.
    /// The executor is thread-safe and can be cached for reuse.
    #[cfg(feature = "sql")]
    pub fn create_sql_executor(&self) -> aspen_sql::RedbSqlExecutor {
        aspen_sql::RedbSqlExecutor::new(self.db.clone())
    }

    /// Read metadata from RAFT_META_TABLE.
    pub(super) fn read_raft_meta<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => {
                let data: T = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Write metadata to RAFT_META_TABLE.
    pub(super) fn write_raft_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            let serialized = bincode::serialize(value).context(SerializeSnafu)?;
            table.insert(key, serialized.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Delete metadata from RAFT_META_TABLE.
    pub(super) fn delete_raft_meta(&self, key: &str) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            table.remove(key).context(RemoveSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Read metadata from SM_META_TABLE.
    pub(super) fn read_sm_meta<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => {
                let data: T = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }
}
