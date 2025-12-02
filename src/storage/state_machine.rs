//! State machine adapters that run alongside the log backend.
//!
//! The state-machine interface owns two responsibilities:
//! - Apply deterministic byte payloads in order, returning the monotonic apply
//!   clock so Raft can compare progress across nodes.
//! - Produce snapshots for failover/testing and hydrate state back when needed.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use redb::{Database, ReadableTable, TableDefinition};

/// State-machine trait implemented by persistent/deterministic engines.
pub trait StateMachineHandle: Send + Sync + 'static {
    /// Apply bytes, returning the logical clock.
    fn apply(&self, bytes: &[u8]) -> anyhow::Result<u64>;
    /// Highest applied clock.
    fn last_applied(&self) -> anyhow::Result<u64>;
    /// Serialize current state for snapshots.
    fn snapshot(&self) -> anyhow::Result<Vec<u8>>;
    /// Restore state from a snapshot.
    fn hydrate(&self, bytes: &[u8]) -> anyhow::Result<()>;
}

/// In-memory handle that keeps a deterministic log of mutations.
#[derive(Debug, Default)]
pub struct InMemoryStateMachine {
    applied: Mutex<Vec<Vec<u8>>>,
}

/// Persistent state machine backed by `redb`.
pub struct RedbStateMachine {
    db: Arc<Database>,
}

impl std::fmt::Debug for RedbStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedbStateMachine").finish()
    }
}

impl RedbStateMachine {
    const TABLE: TableDefinition<'static, u64, Vec<u8>> = TableDefinition::new("aspen.sm");

    /// Open (or create) the state-machine backing file.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create state machine dir {:?}", parent))?;
        }
        let db = if path.exists() {
            Database::open(path).context("open redb state machine")?
        } else {
            Database::create(path).context("create redb state machine")?
        };
        Ok(Self { db: Arc::new(db) })
    }

    fn with_table_read<T>(
        &self,
        f: impl FnOnce(&redb::ReadOnlyTable<u64, Vec<u8>>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let txn = self.db.begin_read().context("begin read txn failed")?;
        let table = txn
            .open_table(Self::TABLE)
            .context("open state table for read")?;
        let out = f(&table)?;
        Ok(out)
    }

    fn with_table_write<T>(
        &self,
        f: impl FnOnce(&mut redb::Table<u64, Vec<u8>>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let txn = self.db.begin_write().context("begin write txn failed")?;
        let out = {
            let mut table = txn
                .open_table(Self::TABLE)
                .context("open state table for write")?;
            f(&mut table)?
        };
        txn.commit().context("commit state txn failed")?;
        Ok(out)
    }
}

impl StateMachineHandle for InMemoryStateMachine {
    fn apply(&self, bytes: &[u8]) -> anyhow::Result<u64> {
        let mut applied = self.applied.lock().unwrap();
        applied.push(bytes.to_vec());
        Ok(applied.len() as u64)
    }

    fn last_applied(&self) -> anyhow::Result<u64> {
        Ok(self.applied.lock().unwrap().len() as u64)
    }

    fn snapshot(&self) -> anyhow::Result<Vec<u8>> {
        let applied = self.applied.lock().unwrap();
        Ok(applied.concat())
    }

    fn hydrate(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut applied = self.applied.lock().unwrap();
        applied.clear();
        applied.push(bytes.to_vec());
        Ok(())
    }
}

impl StateMachineHandle for RedbStateMachine {
    fn apply(&self, bytes: &[u8]) -> anyhow::Result<u64> {
        self.with_table_write(|table| {
            let next = table.last()?.map(|entry| entry.0.value() + 1).unwrap_or(1);
            table.insert(next, bytes.to_vec())?;
            Ok(next)
        })
    }

    fn last_applied(&self) -> anyhow::Result<u64> {
        self.with_table_read(|table| Ok(table.last()?.map(|entry| entry.0.value()).unwrap_or(0)))
    }

    fn snapshot(&self) -> anyhow::Result<Vec<u8>> {
        self.with_table_read(|table| {
            let mut bytes = Vec::new();
            for entry in table.iter()? {
                let (_key, value) = entry?;
                bytes.extend_from_slice(&value.value());
            }
            Ok(bytes)
        })
    }

    fn hydrate(&self, bytes: &[u8]) -> anyhow::Result<()> {
        self.with_table_write(|table| {
            let mut keys = Vec::new();
            for entry in table.iter()? {
                let (key, _) = entry?;
                keys.push(key.value());
            }
            for key in keys {
                table.remove(key)?;
            }
            table.insert(1, bytes.to_vec())?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_updates_clock() {
        let sm = InMemoryStateMachine::default();
        assert_eq!(sm.apply(b"alpha").unwrap(), 1);
        assert_eq!(sm.apply(b"beta").unwrap(), 2);
        assert_eq!(sm.last_applied().unwrap(), 2);
    }

    #[test]
    fn snapshot_and_hydrate_restores_state() {
        let sm = InMemoryStateMachine::default();
        sm.apply(b"alpha").unwrap();
        sm.apply(b"beta").unwrap();
        let snapshot = sm.snapshot().unwrap();
        let sm2 = InMemoryStateMachine::default();
        sm2.hydrate(&snapshot).unwrap();
        assert_eq!(sm2.last_applied().unwrap(), 1);
    }
}
