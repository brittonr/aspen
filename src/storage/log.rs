//! Log storage primitives.
//!
//! The log contract is intentionally tiny so we can cleanly swap `redb` and
//! deterministic in-memory engines without touching the Raft actor. Every
//! concrete type must enforce the “no gaps” invariant and keep ordering explicit
//! to satisfy Tiger Style.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, bail};
use redb::{Database, ReadableTable, TableDefinition};

/// Represents a raw log entry. Higher layers decide how to serialize payloads.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogEntry {
    /// Monotonic index.
    pub index: u64,
    /// Serialized entry payload.
    pub bytes: Vec<u8>,
}

/// Trait satisfied by log backends.
pub trait LogHandle: Send + Sync + 'static {
    /// Next index after the current tail.
    fn next_index(&self) -> anyhow::Result<u64>;
    /// Append entries contiguously, enforcing Tiger Style gaps prevention.
    fn append(&self, start_index: u64, bytes: &[u8]) -> anyhow::Result<()>;
    /// Truncate entries starting at `from` (inclusive).
    fn truncate(&self, from: u64) -> anyhow::Result<()>;
    /// Purge entries up to `upto` (inclusive).
    fn purge(&self, upto: u64) -> anyhow::Result<()>;
    /// Snapshot entries in the provided range.
    fn read(&self, range: RangeInclusive<u64>) -> anyhow::Result<Vec<LogEntry>>;
}

/// In-memory log that keeps ordering explicit for deterministic harnesses.
#[derive(Debug, Default)]
pub struct InMemoryLog {
    entries: Mutex<BTreeMap<u64, Vec<u8>>>,
    seed: Mutex<Option<u64>>,
}

/// Persistent log backed by `redb`.
pub struct RedbLog {
    db: Arc<Database>,
}

impl std::fmt::Debug for RedbLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedbLog").finish()
    }
}

impl RedbLog {
    const TABLE: TableDefinition<'static, u64, Vec<u8>> = TableDefinition::new("aspen.log");

    /// Open (or create) the log at the provided path.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create log parent {:?}", parent))?;
        }
        let db = if path.exists() {
            Database::open(path).context("failed to open redb log")?
        } else {
            Database::create(path).context("failed to create redb log")?
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
            .context("open log table for read")?;
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
                .context("open log table for write")?;
            f(&mut table)?
        };
        txn.commit().context("commit log txn failed")?;
        Ok(out)
    }
}

impl LogHandle for RedbLog {
    fn next_index(&self) -> anyhow::Result<u64> {
        self.with_table_read(|table| {
            let next = table.last()?.map(|entry| entry.0.value() + 1).unwrap_or(1);
            Ok(next)
        })
    }

    fn append(&self, start_index: u64, bytes: &[u8]) -> anyhow::Result<()> {
        self.with_table_write(|table| {
            let expected = table.last()?.map(|entry| entry.0.value() + 1).unwrap_or(1);
            if expected != start_index {
                bail!(
                    "log append must be contiguous: expected {}, got {}",
                    expected,
                    start_index
                );
            }
            table.insert(start_index, bytes.to_vec())?;
            Ok(())
        })
    }

    fn truncate(&self, from: u64) -> anyhow::Result<()> {
        self.with_table_write(|table| {
            let mut doomed = Vec::new();
            for entry in table.range(from..=u64::MAX)? {
                let (key, _) = entry?;
                doomed.push(key.value());
            }
            for key in doomed {
                table.remove(key)?;
            }
            Ok(())
        })
    }

    fn purge(&self, upto: u64) -> anyhow::Result<()> {
        self.with_table_write(|table| {
            let mut doomed = Vec::new();
            for entry in table.range(..=upto)? {
                let (key, _) = entry?;
                doomed.push(key.value());
            }
            for key in doomed {
                table.remove(key)?;
            }
            Ok(())
        })
    }

    fn read(&self, range: RangeInclusive<u64>) -> anyhow::Result<Vec<LogEntry>> {
        let start = *range.start();
        let end = *range.end();
        self.with_table_read(|table| {
            let mut out = Vec::new();
            for entry in table.range(start..=end)? {
                let (key, value) = entry?;
                out.push(LogEntry {
                    index: key.value(),
                    bytes: value.value().to_vec(),
                });
            }
            Ok(out)
        })
    }
}

impl InMemoryLog {
    /// Seed deterministic harness metadata.
    pub fn seed(&mut self, seed: u64) {
        *self.seed.get_mut().unwrap() = Some(seed);
    }

    fn tail_index(entries: &BTreeMap<u64, Vec<u8>>) -> u64 {
        entries.keys().next_back().copied().unwrap_or(0)
    }

    fn assert_next_index_locked(entries: &BTreeMap<u64, Vec<u8>>, start_index: u64) {
        let expected = Self::tail_index(entries) + 1;
        if start_index != expected {
            panic!(
                "log append must be contiguous: expected {}, got {}",
                expected, start_index
            );
        }
    }
}

impl LogHandle for InMemoryLog {
    fn next_index(&self) -> anyhow::Result<u64> {
        let entries = self.entries.lock().unwrap();
        Ok(Self::tail_index(&entries) + 1)
    }

    fn append(&self, start_index: u64, bytes: &[u8]) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().unwrap();
        Self::assert_next_index_locked(&entries, start_index);
        entries.insert(start_index, bytes.to_vec());
        Ok(())
    }

    fn truncate(&self, from: u64) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().unwrap();
        entries.split_off(&from);
        Ok(())
    }

    fn purge(&self, upto: u64) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().unwrap();
        let keys: Vec<u64> = entries
            .keys()
            .copied()
            .take_while(|idx| idx <= &upto)
            .collect();
        for k in keys {
            entries.remove(&k);
        }
        Ok(())
    }

    fn read(&self, range: RangeInclusive<u64>) -> anyhow::Result<Vec<LogEntry>> {
        let entries = self.entries.lock().unwrap();
        Ok(entries
            .range(range)
            .map(|(idx, bytes)| LogEntry {
                index: *idx,
                bytes: bytes.clone(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn append_requires_contiguous_indices() {
        let log = InMemoryLog::default();
        log.append(1, b"a").unwrap();
        let err = std::panic::catch_unwind(|| {
            log.append(3, b"c").unwrap();
        });
        assert!(err.is_err());
    }

    #[test]
    fn purge_and_truncate_enforce_order() {
        let log = InMemoryLog::default();
        log.append(1, b"a").unwrap();
        log.append(2, b"b").unwrap();
        log.purge(1).unwrap();
        assert_eq!(log.read(1..=3).unwrap().len(), 1);
        log.truncate(2).unwrap();
        assert!(log.read(1..=3).unwrap().is_empty());
    }

    proptest! {
        #[test]
        fn proptest_roundtrip(entries in proptest::collection::vec(".*", 1..16)) {
            let log = InMemoryLog::default();
            for (idx, entry) in entries.iter().enumerate() {
                log.append((idx + 1) as u64, entry.as_bytes()).unwrap();
            }
            let snapshot = log.read(1..=((entries.len() as u64)+1)).unwrap();
            prop_assert_eq!(snapshot.len(), entries.len());
        }
    }
}
