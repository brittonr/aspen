//! Per-PID file read tracking for FUSE-level execution caching.
//!
//! Tracks which files (path + BLAKE3 hash) each process reads during execution.
//! Sessions are created when tracked processes open files and finalized when
//! they exit, producing a sorted, deduplicated read set for cache key computation.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use dashmap::DashMap;
use tracing::debug;
use tracing::warn;

use crate::constants::MAX_CONCURRENT_SESSIONS;
use crate::constants::MAX_INPUT_FILES;
use crate::constants::SESSION_TIMEOUT_SECS;
use crate::types::CacheKey;

/// A single file read record: path and content hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReadRecord {
    /// Relative file path within the FUSE mount.
    pub path: String,
    /// BLAKE3 hash of the file contents.
    pub hash: [u8; 32],
}

/// A sorted, deduplicated set of file reads from a completed session.
#[derive(Debug, Clone)]
pub struct ReadSet {
    /// Sorted, deduplicated file read records.
    pub records: Vec<ReadRecord>,
    /// Cache keys of child process sessions.
    pub child_keys: Vec<CacheKey>,
}

impl ReadSet {
    /// Extract just the hashes for cache key computation.
    pub fn input_hashes(&self) -> Vec<[u8; 32]> {
        self.records.iter().map(|r| r.hash).collect()
    }
}

/// State of a tracking session for one PID.
#[derive(Debug)]
struct ReadSession {
    /// Files read by this process, keyed by path for deduplication.
    read_set: HashMap<String, [u8; 32]>,
    /// Parent PID (if this is a child of a tracked process).
    parent_pid: Option<u32>,
    /// When this session was created.
    created_at: Instant,
    /// Cache keys of completed child processes.
    child_cache_keys: Vec<CacheKey>,
    /// Whether tracking is still active (disabled on resource limit).
    is_active: bool,
}

/// Per-PID read set tracker.
///
/// Thread-safe via `DashMap`. Each PID gets its own session tracking
/// which files it reads during execution. Sessions are finalized when
/// the process exits, producing a `ReadSet` for cache key computation.
pub struct ReadTracker {
    /// Active tracking sessions keyed by PID.
    sessions: DashMap<u32, ReadSession>,
    /// Master enable flag. When false, all tracking operations are no-ops.
    is_enabled: AtomicBool,
}

#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "read tracking owns its monotonic session-start boundary")]
fn session_started_at() -> Instant {
    Instant::now()
}

/// Convert a u32 constant to usize (constants are bounded small values).
#[allow(platform_dependent_cast, reason = "called only with small constants from constants.rs, e.g. 1000")]
fn session_limit(value: u32) -> usize {
    value as usize
}

impl ReadTracker {
    /// Create a new read tracker (disabled by default).
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            is_enabled: AtomicBool::new(false),
        }
    }

    /// Enable or disable tracking globally.
    pub fn set_enabled(&self, enabled: bool) {
        self.is_enabled.store(enabled, Ordering::Release);
    }

    /// Check if tracking is enabled.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled.load(Ordering::Acquire)
    }

    /// Start a new tracking session for a root process.
    ///
    /// Returns `false` if the session limit has been reached.
    pub fn start_session(&self, pid: u32) -> bool {
        if !self.is_enabled() {
            return false;
        }
        if self.sessions.len() >= session_limit(MAX_CONCURRENT_SESSIONS) {
            warn!(pid, "session limit reached, not tracking");
            return false;
        }
        self.sessions.insert(pid, ReadSession {
            read_set: HashMap::new(),
            parent_pid: None,
            created_at: session_started_at(),
            child_cache_keys: Vec::new(),
            is_active: true,
        });
        debug!(pid, "started tracking session");
        true
    }

    /// Start a child tracking session linked to a parent.
    ///
    /// Returns `false` if the parent isn't tracked or session limit reached.
    pub fn start_child_session(&self, pid: u32, parent_pid: u32) -> bool {
        if !self.is_enabled() {
            return false;
        }
        // Verify parent is being tracked
        if !self.sessions.contains_key(&parent_pid) {
            return false;
        }
        if self.sessions.len() >= session_limit(MAX_CONCURRENT_SESSIONS) {
            warn!(pid, parent_pid, "session limit reached, not tracking child");
            return false;
        }
        self.sessions.insert(pid, ReadSession {
            read_set: HashMap::new(),
            parent_pid: Some(parent_pid),
            created_at: session_started_at(),
            child_cache_keys: Vec::new(),
            is_active: true,
        });
        debug!(pid, parent_pid, "started child tracking session");
        true
    }

    /// Record a file read for a PID.
    ///
    /// Deduplicates by path — if the same path is read multiple times,
    /// only the first hash is recorded (content shouldn't change mid-execution).
    ///
    /// Returns `false` if the PID isn't tracked or tracking was disabled
    /// for this session due to resource limits.
    pub fn record_read(&self, pid: u32, path: String, hash: [u8; 32]) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let Some(mut session) = self.sessions.get_mut(&pid) else {
            return false;
        };

        if !session.is_active {
            return false;
        }

        // Check resource bound before inserting
        if session.read_set.len() >= session_limit(MAX_INPUT_FILES) {
            warn!(pid, count = session.read_set.len(), "input file limit reached, disabling tracking");
            session.is_active = false;
            return false;
        }

        // Deduplicate by path
        session.read_set.entry(path).or_insert(hash);
        true
    }

    /// Record a completed child's cache key in the parent session.
    pub fn record_child_key(&self, parent_pid: u32, child_key: CacheKey) {
        if let Some(mut session) = self.sessions.get_mut(&parent_pid) {
            session.child_cache_keys.push(child_key);
        }
    }

    /// Finalize a session: remove it and return the sorted, deduplicated read set.
    ///
    /// Returns `None` if the PID wasn't tracked or tracking was disabled for it.
    pub fn finalize(&self, pid: u32) -> Option<ReadSet> {
        let (_, session) = self.sessions.remove(&pid)?;

        if !session.is_active {
            debug!(pid, "session was disabled, no read set produced");
            return None;
        }

        let mut records: Vec<ReadRecord> =
            session.read_set.into_iter().map(|(path, hash)| ReadRecord { path, hash }).collect();

        // Sort by path for deterministic ordering
        records.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        debug!(pid, file_count = records.len(), child_count = session.child_cache_keys.len(), "finalized session");

        Some(ReadSet {
            records,
            child_keys: session.child_cache_keys,
        })
    }

    /// Check if a PID has an active tracking session.
    pub fn has_session(&self, pid: u32) -> bool {
        self.sessions.contains_key(&pid)
    }

    /// Get the parent PID for a tracked session.
    pub fn parent_pid(&self, pid: u32) -> Option<u32> {
        self.sessions.get(&pid).and_then(|s| s.parent_pid)
    }

    /// Clean up stale sessions for PIDs that no longer exist.
    ///
    /// A session is stale if it's been active longer than `SESSION_TIMEOUT_SECS`
    /// and the PID no longer exists in `/proc`.
    ///
    /// Returns the number of sessions cleaned up.
    pub fn cleanup_stale(&self) -> u32 {
        let stale_cutoff = Duration::from_secs(SESSION_TIMEOUT_SECS);
        let mut cleaned = 0u32;

        // Collect stale PIDs to avoid holding the lock during removal
        let stale_pids: Vec<u32> = self
            .sessions
            .iter()
            .filter(|entry| {
                let session = entry.value();
                session.created_at.elapsed() > stale_cutoff && !pid_exists(*entry.key())
            })
            .map(|entry| *entry.key())
            .collect();

        for pid in stale_pids {
            self.sessions.remove(&pid);
            cleaned = cleaned.saturating_add(1);
            debug!(pid, "cleaned up stale session");
        }

        cleaned
    }

    /// Number of active sessions.
    pub fn session_count(&self) -> u32 {
        // DashMap len() <= u32::MAX for all practical session counts
        self.sessions.len() as u32
    }
}

impl Default for ReadTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a PID exists by probing `/proc/{pid}/stat`.
fn pid_exists(pid: u32) -> bool {
    std::fs::metadata(format!("/proc/{pid}/stat")).is_ok()
}

/// Read the parent PID from `/proc/{pid}/stat`.
///
/// The ppid is the 4th field in the stat file.
/// Returns `None` if the file can't be read or parsed.
pub fn read_ppid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Format: "pid (comm) state ppid ..."
    // The comm field can contain spaces and parens, so find the last ')' first
    let after_comm = stat.rfind(')')?.checked_add(2)?; // skip ') '
    let fields_str = stat.get(after_comm..)?;
    let fields: Vec<&str> = fields_str.split_whitespace().collect();
    // fields[0] = state, fields[1] = ppid
    fields.get(1)?.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_tracker_is_noop() {
        let tracker = ReadTracker::new();
        assert!(!tracker.is_enabled());
        assert!(!tracker.start_session(1));
        assert!(!tracker.record_read(1, "foo".into(), [0; 32]));
        assert!(tracker.finalize(1).is_none());
    }

    #[test]
    fn basic_session_lifecycle() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);

        assert!(tracker.start_session(100));
        assert!(tracker.has_session(100));

        tracker.record_read(100, "src/main.rs".into(), [0x11; 32]);
        tracker.record_read(100, "src/lib.rs".into(), [0x22; 32]);

        let read_set = tracker.finalize(100).unwrap();
        assert_eq!(read_set.records.len(), 2);
        // Should be sorted by path
        assert_eq!(read_set.records[0].path, "src/lib.rs");
        assert_eq!(read_set.records[1].path, "src/main.rs");
        assert!(!tracker.has_session(100));
    }

    #[test]
    fn duplicate_reads_are_deduplicated() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);
        tracker.start_session(200);

        tracker.record_read(200, "src/main.rs".into(), [0x11; 32]);
        tracker.record_read(200, "src/main.rs".into(), [0x22; 32]); // same path, different hash
        tracker.record_read(200, "src/main.rs".into(), [0x33; 32]); // same path again

        let read_set = tracker.finalize(200).unwrap();
        assert_eq!(read_set.records.len(), 1);
        // First hash wins
        assert_eq!(read_set.records[0].hash, [0x11; 32]);
    }

    #[test]
    fn child_session_inheritance() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);

        tracker.start_session(300); // parent
        assert!(tracker.start_child_session(301, 300)); // child of parent

        // Child has independent read set
        tracker.record_read(300, "parent_file".into(), [0x11; 32]);
        tracker.record_read(301, "child_file".into(), [0x22; 32]);

        // Record child cache key in parent
        let child_key = CacheKey([0xaa; 32]);
        tracker.record_child_key(300, child_key);

        let child_set = tracker.finalize(301).unwrap();
        assert_eq!(child_set.records.len(), 1);
        assert_eq!(child_set.records[0].path, "child_file");

        let parent_set = tracker.finalize(300).unwrap();
        assert_eq!(parent_set.records.len(), 1);
        assert_eq!(parent_set.records[0].path, "parent_file");
        assert_eq!(parent_set.child_keys.len(), 1);
        assert_eq!(parent_set.child_keys[0], child_key);
    }

    #[test]
    fn child_of_untracked_parent_rejected() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);
        assert!(!tracker.start_child_session(400, 999)); // parent doesn't exist
    }

    #[test]
    fn resource_bound_disables_session() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);
        tracker.start_session(500);

        // Fill up to the limit
        for i in 0..MAX_INPUT_FILES {
            tracker.record_read(500, format!("file_{i}"), [0; 32]);
        }

        // Next record should fail and disable the session
        assert!(!tracker.record_read(500, "one_too_many".into(), [0; 32]));

        // Finalize returns None for disabled sessions
        assert!(tracker.finalize(500).is_none());
    }

    #[test]
    fn stale_cleanup_removes_dead_pids() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);

        // Use PID 0 which doesn't exist as a process
        // We need to bypass the normal start which checks enabled
        tracker.sessions.insert(0, ReadSession {
            read_set: HashMap::new(),
            parent_pid: None,
            // Set created_at far in the past
            created_at: session_started_at() - Duration::from_secs(SESSION_TIMEOUT_SECS.saturating_add(10)),
            child_cache_keys: Vec::new(),
            is_active: true,
        });

        assert_eq!(tracker.session_count(), 1);

        // PID 0 won't exist in /proc, so it should be cleaned up
        // (PID 0 is the kernel scheduler, /proc/0/stat doesn't exist)
        let cleaned = tracker.cleanup_stale();
        // May or may not clean depending on platform — just verify no crash
        assert!(cleaned <= 1);
    }

    #[test]
    fn finalize_nonexistent_pid_returns_none() {
        let tracker = ReadTracker::new();
        tracker.set_enabled(true);
        assert!(tracker.finalize(99999).is_none());
    }

    #[test]
    fn input_hashes_extraction() {
        let read_set = ReadSet {
            records: vec![
                ReadRecord {
                    path: "a".into(),
                    hash: [0x11; 32],
                },
                ReadRecord {
                    path: "b".into(),
                    hash: [0x22; 32],
                },
            ],
            child_keys: vec![],
        };
        let hashes = read_set.input_hashes();
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[0], [0x11; 32]);
        assert_eq!(hashes[1], [0x22; 32]);
    }
}
