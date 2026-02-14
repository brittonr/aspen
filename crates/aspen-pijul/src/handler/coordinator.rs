//! Download coordinator for bounded concurrent change downloads.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use tokio::sync::Semaphore;

use crate::constants::MAX_CONCURRENT_CHANGE_FETCHES;
use crate::types::ChangeHash;

/// Coordinates concurrent downloads with bounded parallelism.
///
/// Tiger Style: Uses a semaphore to limit concurrent downloads to
/// `MAX_CONCURRENT_CHANGE_FETCHES`, preventing connection exhaustion.
pub(super) struct DownloadCoordinator {
    /// Semaphore to limit concurrent downloads.
    semaphore: Arc<Semaphore>,
    /// Currently active downloads (hash -> start time).
    active: RwLock<HashMap<ChangeHash, Instant>>,
}

impl DownloadCoordinator {
    /// Create a new download coordinator.
    pub(super) fn new() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CHANGE_FETCHES as usize)),
            active: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a download is already active for this hash.
    pub(super) fn is_active(&self, hash: &ChangeHash) -> bool {
        self.active.read().contains_key(hash)
    }

    /// Mark a download as started.
    pub(super) fn start(&self, hash: ChangeHash) {
        self.active.write().insert(hash, Instant::now());
    }

    /// Mark a download as finished.
    pub(super) fn finish(&self, hash: &ChangeHash) {
        self.active.write().remove(hash);
    }

    /// Get the semaphore for acquiring permits.
    pub(super) fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }
}
