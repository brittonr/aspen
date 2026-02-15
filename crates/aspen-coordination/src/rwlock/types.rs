//! RWLock types: mode, reader/writer entries, and lock state.

use serde::Deserialize;
use serde::Serialize;

use crate::types::now_unix_ms;

/// Lock mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RWLockMode {
    /// No lock held.
    Free,
    /// One or more readers holding shared lock.
    Read,
    /// Exclusive writer holding the lock.
    Write,
}

impl RWLockMode {
    /// Convert the mode to a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            RWLockMode::Free => "free",
            RWLockMode::Read => "read",
            RWLockMode::Write => "write",
        }
    }
}

/// A reader holding a shared lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReaderEntry {
    /// Unique holder identifier.
    pub holder_id: String,
    /// Deadline for automatic release (ms since epoch).
    pub deadline_ms: u64,
}

impl ReaderEntry {
    /// Check if this reader entry has expired.
    pub fn is_expired(&self) -> bool {
        crate::verified::is_reader_expired(self.deadline_ms, now_unix_ms())
    }
}

/// The writer holding an exclusive lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriterEntry {
    /// Unique holder identifier.
    pub holder_id: String,
    /// Fencing token for this write acquisition.
    pub fencing_token: u64,
    /// Deadline for automatic release (ms since epoch).
    pub deadline_ms: u64,
}

impl WriterEntry {
    /// Check if this writer entry has expired.
    pub fn is_expired(&self) -> bool {
        crate::verified::is_writer_expired(self.deadline_ms, now_unix_ms())
    }
}

/// RWLock state stored in the key-value store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RWLockState {
    /// Lock name.
    pub name: String,
    /// Current lock mode.
    pub mode: RWLockMode,
    /// Writer holding exclusive lock (if mode == Write).
    pub writer: Option<WriterEntry>,
    /// Readers holding shared lock (if mode == Read).
    pub readers: Vec<ReaderEntry>,
    /// Count of pending writers (for writer-preference fairness).
    pub pending_writers: u32,
    /// Global fencing token (incremented on each write acquisition).
    pub fencing_token: u64,
    /// Creation timestamp.
    pub created_at_ms: u64,
}

impl RWLockState {
    /// Create a new free lock state.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            mode: RWLockMode::Free,
            writer: None,
            readers: Vec::new(),
            pending_writers: 0,
            fencing_token: 0,
            created_at_ms: now_unix_ms(),
        }
    }

    /// Remove expired readers.
    pub fn cleanup_expired_readers(&mut self) {
        let count_before = self.readers.len();
        self.readers.retain(|r| !r.is_expired());
        debug_assert!(
            self.readers.len() <= count_before,
            "RWLOCK: cleanup should not increase reader count: {} > {count_before}",
            self.readers.len()
        );
        if self.readers.is_empty() && self.mode == RWLockMode::Read {
            self.mode = RWLockMode::Free;
        }
    }

    /// Check if writer has expired and cleanup if so.
    pub fn cleanup_expired_writer(&mut self) {
        if let Some(ref writer) = self.writer
            && writer.is_expired()
        {
            self.writer = None;
            if self.mode == RWLockMode::Write {
                self.mode = RWLockMode::Free;
            }
        }
    }

    /// Find a reader by holder ID.
    pub fn find_reader(&self, holder_id: &str) -> Option<&ReaderEntry> {
        self.readers.iter().find(|r| r.holder_id == holder_id)
    }

    /// Check if this holder already has a read lock.
    pub fn has_read_lock(&self, holder_id: &str) -> bool {
        self.readers.iter().any(|r| r.holder_id == holder_id && !r.is_expired())
    }

    /// Check if this holder has the write lock.
    pub fn has_write_lock(&self, holder_id: &str) -> bool {
        self.writer.as_ref().map(|w| w.holder_id == holder_id && !w.is_expired()).unwrap_or(false)
    }

    /// Get the number of active (non-expired) readers.
    pub fn active_reader_count(&self) -> u32 {
        self.readers.iter().filter(|r| !r.is_expired()).count() as u32
    }
}
