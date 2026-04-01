//! Bounded ring buffer for recent snapshot transfer records.
//!
//! Tracks the last N snapshot transfers (send/receive) for observability.
//! The ring buffer is thread-safe (interior mutex) and bounded to prevent
//! unbounded memory growth.
//!
//! # Tiger Style
//!
//! - Fixed capacity: `MAX_SNAPSHOT_HISTORY` entries
//! - Oldest entries evicted on overflow (ring buffer semantics)
//! - Lock held only during push/drain (microsecond-scale)

use std::collections::VecDeque;
use std::sync::Mutex;

/// Maximum number of snapshot transfer records to retain.
const MAX_SNAPSHOT_HISTORY: usize = 100;

/// A record of a single snapshot transfer event.
#[derive(Debug, Clone)]
pub struct SnapshotTransferEntry {
    /// Peer involved in the transfer.
    pub peer_id: u64,
    /// Direction: "send" or "receive".
    pub direction: &'static str,
    /// Snapshot size in bytes.
    pub size_bytes: u64,
    /// Transfer duration in milliseconds.
    pub duration_ms: u64,
    /// Outcome: "success" or "error".
    pub outcome: &'static str,
    /// Transfer timestamp in Unix microseconds.
    pub timestamp_us: u64,
}

/// Thread-safe bounded ring buffer of snapshot transfer records.
///
/// Uses a `VecDeque` under a `Mutex`. The lock is held only during
/// `push` (single insertion) and `recent` (clone of VecDeque), keeping
/// critical sections under a microsecond.
#[derive(Debug)]
pub struct SnapshotTransferHistory {
    entries: Mutex<VecDeque<SnapshotTransferEntry>>,
}

impl SnapshotTransferHistory {
    /// Create a new empty history buffer.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(MAX_SNAPSHOT_HISTORY)),
        }
    }

    /// Record a snapshot transfer. Evicts the oldest entry if at capacity.
    pub fn push(&self, entry: SnapshotTransferEntry) {
        let mut buf = self.entries.lock().expect("snapshot history lock poisoned");
        if buf.len() >= MAX_SNAPSHOT_HISTORY {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    /// Return a snapshot of recent transfers, newest last.
    pub fn recent(&self) -> Vec<SnapshotTransferEntry> {
        let buf = self.entries.lock().expect("snapshot history lock poisoned");
        buf.iter().cloned().collect()
    }

    /// Number of entries currently in the buffer.
    pub fn len(&self) -> usize {
        let buf = self.entries.lock().expect("snapshot history lock poisoned");
        buf.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for SnapshotTransferHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_retrieve() {
        let history = SnapshotTransferHistory::new();
        history.push(SnapshotTransferEntry {
            peer_id: 2,
            direction: "send",
            size_bytes: 1024,
            duration_ms: 50,
            outcome: "success",
            timestamp_us: 1_000_000,
        });

        let entries = history.recent();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].peer_id, 2);
        assert_eq!(entries[0].direction, "send");
    }

    #[test]
    fn evicts_oldest_at_capacity() {
        let history = SnapshotTransferHistory::new();

        // Fill to capacity + 10 overflow
        for i in 0..(MAX_SNAPSHOT_HISTORY + 10) {
            history.push(SnapshotTransferEntry {
                peer_id: i as u64,
                direction: "receive",
                size_bytes: 512,
                duration_ms: 10,
                outcome: "success",
                timestamp_us: i as u64 * 1000,
            });
        }

        let entries = history.recent();
        assert_eq!(entries.len(), MAX_SNAPSHOT_HISTORY);

        // Oldest entry should be the 11th push (index 10), not the 1st.
        assert_eq!(entries[0].peer_id, 10);
        // Newest entry should be the last push.
        assert_eq!(entries[MAX_SNAPSHOT_HISTORY - 1].peer_id, (MAX_SNAPSHOT_HISTORY + 9) as u64);
    }

    #[test]
    fn empty_on_creation() {
        let history = SnapshotTransferHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert!(history.recent().is_empty());
    }
}
