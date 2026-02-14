//! Pending request tracking for deduplication of sync operations.

use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use aspen_forge::identity::RepoId;
use tracing::warn;

use crate::constants::MAX_AWAITING_UPDATES_PER_CHANGE;
use crate::constants::MAX_PENDING_CHANGES;
use crate::constants::PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS;
use crate::constants::PIJUL_SYNC_REQUEST_DEDUP_SECS;
use crate::types::ChangeHash;

/// Pending channel update waiting for a change to be downloaded.
#[derive(Debug, Clone)]
pub(super) struct PendingChannelUpdate {
    /// Repository ID.
    pub(super) repo_id: RepoId,
    /// Channel name.
    pub(super) channel: String,
    /// When this update was recorded.
    pub(super) recorded_at: Instant,
}

/// Tracks pending requests to avoid duplicate work.
pub(super) struct PendingRequests {
    /// Changes currently being downloaded (hash -> request time).
    downloading: HashMap<ChangeHash, Instant>,
    /// Changes we've recently requested (hash -> request time).
    requested: HashMap<ChangeHash, Instant>,
    /// Repos we've checked for sync recently (repo_id:channel -> check time).
    channel_checks: HashMap<String, Instant>,
    /// Channel updates waiting for specific changes (hash -> channel update).
    /// When a change is downloaded, we check if any channel was waiting for it.
    awaiting_changes: HashMap<ChangeHash, Vec<PendingChannelUpdate>>,
}

impl PendingRequests {
    pub(super) fn new() -> Self {
        Self {
            downloading: HashMap::new(),
            requested: HashMap::new(),
            channel_checks: HashMap::new(),
            awaiting_changes: HashMap::new(),
        }
    }

    /// Total count of entries across all tracking maps (excluding awaiting_changes).
    ///
    /// Tiger Style: Used to enforce MAX_PENDING_CHANGES limit.
    fn total_count(&self) -> usize {
        self.downloading.len() + self.requested.len() + self.channel_checks.len()
    }

    /// Ensure we have capacity for new entries.
    ///
    /// Tiger Style: Cleanup old entries first, then check if under limit.
    /// Returns true if we have capacity, false if at limit after cleanup.
    fn ensure_capacity(&mut self) -> bool {
        if self.total_count() >= MAX_PENDING_CHANGES {
            self.cleanup();
            if self.total_count() >= MAX_PENDING_CHANGES {
                warn!(
                    count = self.total_count(),
                    max = MAX_PENDING_CHANGES,
                    "pending requests at capacity, rejecting new entry"
                );
                return false;
            }
        }
        true
    }

    /// Check if we should request this change (not already pending).
    pub(super) fn should_request(&self, hash: &ChangeHash) -> bool {
        let now = Instant::now();
        let dedup_duration = Duration::from_secs(PIJUL_SYNC_REQUEST_DEDUP_SECS);

        // Check if already downloading
        if let Some(started) = self.downloading.get(hash) {
            if now.duration_since(*started) < Duration::from_secs(PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS) {
                return false;
            }
        }

        // Check if recently requested
        if let Some(requested) = self.requested.get(hash) {
            if now.duration_since(*requested) < dedup_duration {
                return false;
            }
        }

        true
    }

    /// Mark a change as being downloaded.
    ///
    /// Tiger Style: Enforces MAX_PENDING_CHANGES limit.
    /// Returns true if marked, false if at capacity.
    pub(super) fn mark_downloading(&mut self, hash: ChangeHash) -> bool {
        if !self.ensure_capacity() {
            return false;
        }
        self.downloading.insert(hash, Instant::now());
        true
    }

    /// Mark a change download as complete.
    pub(super) fn mark_downloaded(&mut self, hash: &ChangeHash) {
        self.downloading.remove(hash);
    }

    /// Mark a change as requested.
    ///
    /// Tiger Style: Enforces MAX_PENDING_CHANGES limit.
    /// Returns true if marked, false if at capacity.
    pub(super) fn mark_requested(&mut self, hash: ChangeHash) -> bool {
        if !self.ensure_capacity() {
            return false;
        }
        self.requested.insert(hash, Instant::now());
        true
    }

    /// Check if we should check this channel for sync with a specific head.
    /// Returns true if we haven't recently processed this exact head for this channel.
    pub(super) fn should_check_channel(&self, repo_id: &RepoId, channel: &str, head: &ChangeHash) -> bool {
        // Key includes the head hash to allow different heads to be processed
        let key = format!("{}:{}:{}", repo_id.to_hex(), channel, head);
        let now = Instant::now();
        let dedup_duration = Duration::from_secs(PIJUL_SYNC_REQUEST_DEDUP_SECS);

        if let Some(checked) = self.channel_checks.get(&key) {
            if now.duration_since(*checked) < dedup_duration {
                return false;
            }
        }

        true
    }

    /// Mark a channel+head combination as checked.
    ///
    /// Tiger Style: Enforces MAX_PENDING_CHANGES limit.
    /// Returns true if marked, false if at capacity.
    pub(super) fn mark_channel_checked(&mut self, repo_id: &RepoId, channel: &str, head: &ChangeHash) -> bool {
        if !self.ensure_capacity() {
            return false;
        }
        let key = format!("{}:{}:{}", repo_id.to_hex(), channel, head);
        self.channel_checks.insert(key, Instant::now());
        true
    }

    /// Clear the channel check mark so it can be re-checked immediately.
    /// This should be called after a sync completes to allow retrying on failure.
    pub(super) fn clear_channel_check(&mut self, repo_id: &RepoId, channel: &str, head: &ChangeHash) {
        let key = format!("{}:{}:{}", repo_id.to_hex(), channel, head);
        self.channel_checks.remove(&key);
    }

    /// Record that a channel is waiting for a specific change.
    ///
    /// Tiger Style: Enforces MAX_AWAITING_UPDATES_PER_CHANGE limit per hash
    /// and MAX_PENDING_CHANGES limit for total awaiting_changes entries.
    /// Returns true if recorded, false if at capacity.
    pub(super) fn await_change(&mut self, hash: ChangeHash, repo_id: RepoId, channel: String) -> bool {
        // Check total awaiting_changes map size
        if self.awaiting_changes.len() >= MAX_PENDING_CHANGES {
            // Cleanup old entries
            let now = Instant::now();
            let timeout = Duration::from_secs(PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS);
            self.awaiting_changes.retain(|_, updates| {
                updates.retain(|u| now.duration_since(u.recorded_at) < timeout);
                !updates.is_empty()
            });

            if self.awaiting_changes.len() >= MAX_PENDING_CHANGES {
                warn!(
                    count = self.awaiting_changes.len(),
                    max = MAX_PENDING_CHANGES,
                    "awaiting_changes at capacity, rejecting new entry"
                );
                return false;
            }
        }

        let entry = self.awaiting_changes.entry(hash).or_default();

        // Check per-hash limit
        if entry.len() >= MAX_AWAITING_UPDATES_PER_CHANGE {
            warn!(
                count = entry.len(),
                max = MAX_AWAITING_UPDATES_PER_CHANGE,
                "too many updates waiting for change, rejecting"
            );
            return false;
        }

        entry.push(PendingChannelUpdate {
            repo_id,
            channel,
            recorded_at: Instant::now(),
        });
        true
    }

    /// Take all channel updates waiting for a specific change.
    pub(super) fn take_awaiting(&mut self, hash: &ChangeHash) -> Vec<PendingChannelUpdate> {
        self.awaiting_changes.remove(hash).unwrap_or_default()
    }

    /// Cleanup old entries.
    pub(super) fn cleanup(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS);
        let dedup = Duration::from_secs(PIJUL_SYNC_REQUEST_DEDUP_SECS);

        self.downloading.retain(|_, t| now.duration_since(*t) < timeout);
        self.requested.retain(|_, t| now.duration_since(*t) < dedup);
        self.channel_checks.retain(|_, t| now.duration_since(*t) < dedup);
        // Cleanup old awaiting entries
        self.awaiting_changes.retain(|_, updates| {
            updates.retain(|u| now.duration_since(u.recorded_at) < timeout);
            !updates.is_empty()
        });
    }
}
