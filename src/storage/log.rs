//! Log storage primitives.
//!
//! The log contract is intentionally tiny so we can cleanly swap `redb` and
//! deterministic in-memory engines without touching the Raft actor. Every
//! concrete type must enforce the “no gaps” invariant and keep ordering explicit
//! to satisfy Tiger Style.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::sync::Mutex;

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
    fn next_index(&self) -> u64;
    /// Append entries contiguously, enforcing Tiger Style gaps prevention.
    fn append(&self, start_index: u64, bytes: &[u8]) -> anyhow::Result<()>;
    /// Truncate entries starting at `from` (inclusive).
    fn truncate(&self, from: u64) -> anyhow::Result<()>;
    /// Purge entries up to `upto` (inclusive).
    fn purge(&self, upto: u64) -> anyhow::Result<()>;
    /// Snapshot entries in the provided range.
    fn read(&self, range: RangeInclusive<u64>) -> Vec<LogEntry>;
}

/// In-memory log that keeps ordering explicit for deterministic harnesses.
#[derive(Debug, Default)]
pub struct InMemoryLog {
    entries: Mutex<BTreeMap<u64, Vec<u8>>>,
    seed: Mutex<Option<u64>>,
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
    fn next_index(&self) -> u64 {
        let entries = self.entries.lock().unwrap();
        Self::tail_index(&entries) + 1
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

    fn read(&self, range: RangeInclusive<u64>) -> Vec<LogEntry> {
        let entries = self.entries.lock().unwrap();
        entries
            .range(range)
            .map(|(idx, bytes)| LogEntry {
                index: *idx,
                bytes: bytes.clone(),
            })
            .collect()
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
        assert_eq!(log.read(1..=3).len(), 1);
        log.truncate(2).unwrap();
        assert!(log.read(1..=3).is_empty());
    }

    proptest! {
        #[test]
        fn proptest_roundtrip(entries in proptest::collection::vec(".*", 1..16)) {
            let log = InMemoryLog::default();
            for (idx, entry) in entries.iter().enumerate() {
                log.append((idx + 1) as u64, entry.as_bytes()).unwrap();
            }
            let snapshot = log.read(1..=((entries.len() as u64)+1));
            prop_assert_eq!(snapshot.len(), entries.len());
        }
    }
}
