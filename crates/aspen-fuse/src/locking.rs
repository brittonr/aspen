//! Local file lock manager for POSIX advisory locking.
//!
//! Provides POSIX flock/fcntl advisory locking semantics backed by local
//! lock tracking. This is a local-only manager (single-process) suitable
//! for VirtioFS where all FUSE operations come through a single daemon.
//!
//! # Lock Types
//!
//! - `F_RDLCK` (0): Read lock — multiple readers allowed, blocks writers
//! - `F_WRLCK` (1): Write lock — exclusive, blocks all other locks
//! - `F_UNLCK` (2): Unlock — releases held locks
//!
//! # Lock Ranges
//!
//! Locks cover byte ranges `[start, end)`:
//! - `end = 0` means "to end of file" (EOF)
//! - Same owner can upgrade/downgrade locks
//! - Overlapping locks from same owner are coalesced
//!
//! # Conflict Rules
//!
//! Two locks conflict if:
//! 1. Same inode
//! 2. Ranges overlap
//! 3. Different owners
//! 4. At least one is a write lock
//!
//! Read locks from different owners do NOT conflict.
//!
//! # Tiger Style
//!
//! - Bounded by `MAX_LOCKS_PER_INODE` and `MAX_LOCKED_INODES`
//! - Thread-safe with `RwLock` for concurrent FUSE threads
//! - Explicit timeouts for blocking operations (`LOCK_WAIT_TIMEOUT`)

use std::collections::HashMap;
use std::io;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::constants::LOCK_WAIT_TIMEOUT;
use crate::constants::MAX_LOCKED_INODES;
use crate::constants::MAX_LOCKS_PER_INODE;

// POSIX lock type constants (from fcntl.h)
/// Read lock (shared).
pub const F_RDLCK: u32 = 0;
/// Write lock (exclusive).
pub const F_WRLCK: u32 = 1;
/// Unlock.
pub const F_UNLCK: u32 = 2;

/// A held lock on a file region.
#[derive(Debug, Clone)]
struct HeldLock {
    /// Lock owner (FUSE lock_owner from the kernel).
    owner: u64,
    /// Process ID that holds the lock.
    pid: u32,
    /// Range start (inclusive).
    start: u64,
    /// Range end (exclusive, 0 = EOF).
    end: u64,
    /// Lock type: F_RDLCK or F_WRLCK.
    lock_type: u32,
}

impl HeldLock {
    /// Check if this lock conflicts with another lock request.
    fn conflicts_with(&self, owner: u64, lock_type: u32, start: u64, end: u64) -> bool {
        // Same owner never conflicts with itself
        if self.owner == owner {
            return false;
        }

        // Check if ranges overlap
        if !ranges_overlap(self.start, self.end, start, end) {
            return false;
        }

        // At least one must be a write lock for conflict
        self.lock_type == F_WRLCK || lock_type == F_WRLCK
    }

    /// Check if this lock overlaps or is adjacent to a given range.
    fn overlaps_or_adjacent(&self, start: u64, end: u64) -> bool {
        // Adjacent: self.end == start or end == self.start
        // Overlapping: ranges_overlap
        ranges_overlap(self.start, self.end, start, end) || self.end == start || (end != 0 && end == self.start)
    }
}

/// Per-inode lock state.
#[derive(Debug, Default)]
struct InodeLocks {
    /// Active locks on this inode.
    locks: Vec<HeldLock>,
}

impl InodeLocks {
    /// Find a lock that conflicts with the given request.
    fn find_conflict(&self, owner: u64, lock_type: u32, start: u64, end: u64) -> Option<&HeldLock> {
        self.locks.iter().find(|lock| lock.conflicts_with(owner, lock_type, start, end))
    }

    /// Set a lock with coalescing for same-owner locks.
    fn set_lock(&mut self, owner: u64, pid: u32, lock_type: u32, start: u64, end: u64) -> io::Result<()> {
        // Check resource limit
        if self.locks.len() >= MAX_LOCKS_PER_INODE && !self.locks.iter().any(|lock| lock.owner == owner) {
            return Err(io::Error::from_raw_os_error(libc::ENOLCK));
        }

        if lock_type == F_UNLCK {
            // Remove overlapping locks from this owner
            self.locks.retain(|lock| lock.owner != owner || !ranges_overlap(lock.start, lock.end, start, end));
            return Ok(());
        }

        // Remove overlapping/adjacent locks from same owner
        let mut merged_start = start;
        let mut merged_end = end;

        self.locks.retain(|lock| {
            if lock.owner == owner && lock.overlaps_or_adjacent(start, end) {
                // Merge ranges
                merged_start = merged_start.min(lock.start);
                if merged_end == 0 || lock.end == 0 {
                    merged_end = 0; // Either extends to EOF
                } else {
                    merged_end = merged_end.max(lock.end);
                }
                false // Remove this lock
            } else {
                true // Keep this lock
            }
        });

        // Add the new (possibly merged) lock
        self.locks.push(HeldLock {
            owner,
            pid,
            start: merged_start,
            end: merged_end,
            lock_type,
        });

        Ok(())
    }

    /// Release all locks for a given owner.
    fn release_owner(&mut self, owner: u64) {
        self.locks.retain(|lock| lock.owner != owner);
    }
}

/// Local file lock manager.
///
/// Tracks advisory locks per-inode with POSIX semantics.
/// Thread-safe: FUSE operations come from multiple threads.
#[derive(Debug)]
pub struct LockManager {
    /// Per-inode lock tables: inode -> InodeLocks
    locks: RwLock<HashMap<u64, InodeLocks>>,
}

impl LockManager {
    /// Create a new lock manager.
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
        }
    }

    /// Query lock status (getlk).
    ///
    /// Returns the first conflicting lock if one exists, or an unlocked
    /// FileLockResult if no conflict is found.
    pub fn get_lock(&self, inode: u64, owner: u64, lock_type: u32, start: u64, end: u64) -> FileLockResult {
        let locks = self.locks.read().unwrap_or_else(|e| e.into_inner());

        if let Some(inode_locks) = locks.get(&inode)
            && let Some(conflict) = inode_locks.find_conflict(owner, lock_type, start, end)
        {
            return FileLockResult {
                lock_type: conflict.lock_type,
                start: conflict.start,
                end: conflict.end,
                pid: conflict.pid,
            };
        }

        // No conflict — return unlocked
        FileLockResult {
            lock_type: F_UNLCK,
            start,
            end,
            pid: 0,
        }
    }

    /// Set a lock (setlk, non-blocking).
    ///
    /// Returns `Ok(())` on success, `Err(EAGAIN)` if a conflicting lock exists,
    /// or `Err(ENOLCK)` if resource limits are exceeded.
    pub fn set_lock(&self, inode: u64, owner: u64, pid: u32, lock_type: u32, start: u64, end: u64) -> io::Result<()> {
        let mut locks = self.locks.write().unwrap_or_else(|e| e.into_inner());

        // Check global inode limit (if adding a new inode)
        if !locks.contains_key(&inode) && locks.len() >= MAX_LOCKED_INODES {
            return Err(io::Error::from_raw_os_error(libc::ENOLCK));
        }

        let inode_locks = locks.entry(inode).or_default();

        // Check for conflicts (unless unlocking)
        if lock_type != F_UNLCK
            && let Some(_conflict) = inode_locks.find_conflict(owner, lock_type, start, end)
        {
            // Conflict exists — return EAGAIN for non-blocking
            return Err(io::Error::from_raw_os_error(libc::EAGAIN));
        }

        // Set the lock (may coalesce with existing same-owner locks)
        inode_locks.set_lock(owner, pid, lock_type, start, end)?;

        // Clean up empty inode entries
        if inode_locks.locks.is_empty() {
            locks.remove(&inode);
        }

        Ok(())
    }

    /// Set a lock, blocking (setlkw).
    ///
    /// Waits with exponential backoff for conflicting locks to release.
    /// Bounded by `LOCK_WAIT_TIMEOUT`.
    pub fn set_lock_wait(
        &self,
        inode: u64,
        owner: u64,
        pid: u32,
        lock_type: u32,
        start: u64,
        end: u64,
    ) -> io::Result<()> {
        let deadline = Instant::now() + LOCK_WAIT_TIMEOUT;
        let mut backoff = Duration::from_millis(1);
        let max_backoff = Duration::from_millis(100);

        loop {
            match self.set_lock(inode, owner, pid, lock_type, start, end) {
                Ok(()) => return Ok(()),
                Err(e) if e.raw_os_error() == Some(libc::EAGAIN) => {
                    // Conflict exists — check timeout
                    if Instant::now() >= deadline {
                        return Err(io::Error::from_raw_os_error(libc::ETIMEDOUT));
                    }

                    // Sleep with exponential backoff
                    thread::sleep(backoff);
                    backoff = (backoff * 2).min(max_backoff);
                }
                Err(e) => return Err(e), // Other error (e.g., ENOLCK)
            }
        }
    }

    /// Release all locks for a given owner on an inode.
    ///
    /// Called from `release()` when a file handle is closed.
    pub fn release_owner(&self, inode: u64, owner: u64) {
        let mut locks = self.locks.write().unwrap_or_else(|e| e.into_inner());

        if let Some(inode_locks) = locks.get_mut(&inode) {
            inode_locks.release_owner(owner);

            // Clean up empty inode entries
            if inode_locks.locks.is_empty() {
                locks.remove(&inode);
            }
        }
    }

    /// Release all locks on an inode.
    ///
    /// Called when a file is deleted.
    pub fn release_inode(&self, inode: u64) {
        let mut locks = self.locks.write().unwrap_or_else(|e| e.into_inner());
        locks.remove(&inode);
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a lock query (getlk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLockResult {
    /// Lock type of conflict (F_RDLCK, F_WRLCK), or F_UNLCK if no conflict.
    pub lock_type: u32,
    /// Range start of conflicting lock.
    pub start: u64,
    /// Range end of conflicting lock.
    pub end: u64,
    /// PID of conflicting lock holder.
    pub pid: u32,
}

/// Check if two ranges overlap.
///
/// Ranges are `[start, end)` where `end = 0` means EOF.
fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    // Handle EOF (end = 0) specially
    let a_ends_at_eof = a_end == 0;
    let b_ends_at_eof = b_end == 0;

    if a_ends_at_eof && b_ends_at_eof {
        // Both extend to EOF — they overlap if starts overlap
        return true;
    }

    if a_ends_at_eof {
        // a extends to EOF, b is bounded — overlap if b doesn't end before a starts
        return b_end == 0 || b_end > a_start;
    }

    if b_ends_at_eof {
        // b extends to EOF, a is bounded — overlap if a doesn't end before b starts
        return a_end > b_start;
    }

    // Both bounded — standard interval overlap check
    !(a_end <= b_start || b_end <= a_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_lock_allows_concurrent_readers() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires read lock
        assert!(lm.set_lock(inode, 1, 100, F_RDLCK, 0, 100).is_ok());

        // Owner 2 can also acquire read lock on same range
        assert!(lm.set_lock(inode, 2, 200, F_RDLCK, 0, 100).is_ok());

        // Both locks should exist
        let result = lm.get_lock(inode, 3, F_WRLCK, 0, 100);
        assert_eq!(result.lock_type, F_RDLCK); // Conflict with read lock
    }

    #[test]
    fn test_write_lock_exclusive() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 2 cannot acquire read lock
        let result = lm.set_lock(inode, 2, 200, F_RDLCK, 0, 100);
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EAGAIN));

        // Owner 2 cannot acquire write lock
        let result = lm.set_lock(inode, 2, 200, F_WRLCK, 0, 100);
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EAGAIN));
    }

    #[test]
    fn test_same_owner_upgrade() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires read lock
        assert!(lm.set_lock(inode, 1, 100, F_RDLCK, 0, 100).is_ok());

        // Owner 1 upgrades to write lock (same owner, no conflict)
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Verify write lock is held
        let result = lm.get_lock(inode, 2, F_RDLCK, 0, 100);
        assert_eq!(result.lock_type, F_WRLCK);
    }

    #[test]
    fn test_same_owner_downgrade() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 1 downgrades to read lock
        assert!(lm.set_lock(inode, 1, 100, F_RDLCK, 0, 100).is_ok());

        // Verify read lock is held
        let result = lm.get_lock(inode, 2, F_RDLCK, 0, 100);
        assert_eq!(result.lock_type, F_UNLCK); // No conflict with read lock
    }

    #[test]
    fn test_unlock_releases() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 1 releases lock
        assert!(lm.set_lock(inode, 1, 100, F_UNLCK, 0, 100).is_ok());

        // Owner 2 can now acquire lock
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 0, 100).is_ok());
    }

    #[test]
    fn test_overlapping_ranges() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 locks [0, 100)
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 2 tries to lock [50, 150) — overlaps
        let result = lm.set_lock(inode, 2, 200, F_WRLCK, 50, 150);
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EAGAIN));

        // Owner 2 can lock [100, 200) — no overlap
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 100, 200).is_ok());
    }

    #[test]
    fn test_non_overlapping_no_conflict() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 locks [0, 100)
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 2 locks [200, 300) — no overlap
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 200, 300).is_ok());

        // Both locks should coexist
        let result = lm.get_lock(inode, 3, F_RDLCK, 0, 100);
        assert_eq!(result.lock_type, F_WRLCK);

        let result = lm.get_lock(inode, 3, F_RDLCK, 200, 300);
        assert_eq!(result.lock_type, F_WRLCK);
    }

    #[test]
    fn test_release_owner() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires multiple locks
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 200, 300).is_ok());

        // Release all locks for owner 1
        lm.release_owner(inode, 1);

        // Owner 2 can now acquire locks
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 0, 100).is_ok());
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 200, 300).is_ok());
    }

    #[test]
    fn test_release_inode() {
        let lm = LockManager::new();
        let inode = 42;

        // Multiple owners acquire locks
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 200, 300).is_ok());

        // Release all locks on inode
        lm.release_inode(inode);

        // New owner can acquire any lock
        assert!(lm.set_lock(inode, 3, 300, F_WRLCK, 0, 100).is_ok());
        assert!(lm.set_lock(inode, 3, 300, F_WRLCK, 200, 300).is_ok());
    }

    #[test]
    fn test_getlk_returns_conflict() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 50, 150).is_ok());

        // Query for conflicting lock
        let result = lm.get_lock(inode, 2, F_RDLCK, 100, 200);
        assert_eq!(result.lock_type, F_WRLCK);
        assert_eq!(result.start, 50);
        assert_eq!(result.end, 150);
        assert_eq!(result.pid, 100);
    }

    #[test]
    fn test_getlk_no_conflict() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Query for non-conflicting range
        let result = lm.get_lock(inode, 2, F_RDLCK, 200, 300);
        assert_eq!(result.lock_type, F_UNLCK);
    }

    #[test]
    fn test_max_locks_per_inode() {
        let lm = LockManager::new();
        let inode = 42;

        // Fill up to MAX_LOCKS_PER_INODE with different owners
        for i in 0..MAX_LOCKS_PER_INODE {
            let start = i as u64 * 100;
            let end = start + 100;
            assert!(lm.set_lock(inode, i as u64, i as u32, F_WRLCK, start, end).is_ok());
        }

        // Next lock from new owner should fail
        let result = lm.set_lock(inode, MAX_LOCKS_PER_INODE as u64, 9999, F_WRLCK, 999999, 1000000);
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::ENOLCK));
    }

    #[test]
    fn test_eof_range() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 locks [100, EOF)
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 100, 0).is_ok());

        // Owner 2 tries to lock [200, 300) — conflicts with EOF lock
        let result = lm.set_lock(inode, 2, 200, F_WRLCK, 200, 300);
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EAGAIN));

        // Owner 2 can lock [0, 100) — no overlap
        assert!(lm.set_lock(inode, 2, 200, F_WRLCK, 0, 100).is_ok());
    }

    #[test]
    fn test_ranges_overlap_function() {
        // No overlap: a ends before b starts
        assert!(!ranges_overlap(0, 100, 100, 200));
        assert!(!ranges_overlap(0, 100, 200, 300));

        // Overlap: standard cases
        assert!(ranges_overlap(0, 100, 50, 150));
        assert!(ranges_overlap(50, 150, 0, 100));
        assert!(ranges_overlap(0, 200, 50, 150));

        // EOF cases: end = 0 means to end of file
        assert!(ranges_overlap(100, 0, 200, 300)); // [100, EOF) overlaps [200, 300)
        assert!(ranges_overlap(100, 0, 50, 150)); // [100, EOF) overlaps [50, 150)
        assert!(ranges_overlap(100, 200, 150, 0)); // [100, 200) overlaps [150, EOF)
        assert!(ranges_overlap(100, 0, 200, 0)); // Both extend to EOF

        // No overlap with EOF
        assert!(!ranges_overlap(200, 0, 0, 100)); // [200, EOF) doesn't overlap [0, 100)
    }

    #[test]
    fn test_lock_coalescing() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 locks [0, 100)
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 1 locks [50, 150) — should coalesce to [0, 150)
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 50, 150).is_ok());

        // Verify merged lock blocks others
        let result = lm.get_lock(inode, 2, F_RDLCK, 0, 150);
        assert_eq!(result.lock_type, F_WRLCK);
        assert_eq!(result.start, 0);
        assert_eq!(result.end, 150);
    }

    #[test]
    fn test_set_lock_wait_timeout() {
        let lm = LockManager::new();
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Owner 2 tries to acquire with wait — should timeout
        let start = Instant::now();
        let result = lm.set_lock_wait(inode, 2, 200, F_WRLCK, 0, 100);
        let elapsed = start.elapsed();

        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::ETIMEDOUT));
        assert!(elapsed >= LOCK_WAIT_TIMEOUT);
    }

    #[test]
    fn test_set_lock_wait_succeeds() {
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let lm = Arc::new(LockManager::new());
        let inode = 42;

        // Owner 1 acquires write lock
        assert!(lm.set_lock(inode, 1, 100, F_WRLCK, 0, 100).is_ok());

        // Spawn thread to release lock after 500ms
        let lm_clone = Arc::clone(&lm);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            let _ = lm_clone.set_lock(inode, 1, 100, F_UNLCK, 0, 100);
        });

        // Owner 2 waits for lock — should succeed after ~500ms
        let start = Instant::now();
        let result = lm.set_lock_wait(inode, 2, 200, F_WRLCK, 0, 100);
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(elapsed >= Duration::from_millis(500));
        assert!(elapsed < LOCK_WAIT_TIMEOUT);
    }
}
