//! Download metrics tracking for Pijul sync operations.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Metrics for download operations.
///
/// All counters use `Relaxed` ordering since they're informational only
/// and don't need synchronization guarantees.
#[derive(Default)]
pub struct DownloadMetrics {
    /// Total number of downloads attempted.
    pub downloads_attempted: AtomicU64,
    /// Number of successful downloads.
    pub downloads_succeeded: AtomicU64,
    /// Number of failed downloads (after all retries exhausted).
    pub downloads_failed: AtomicU64,
    /// Number of downloads skipped (already had change).
    pub downloads_skipped: AtomicU64,
    /// Total bytes downloaded (approximate, from change sizes).
    pub bytes_downloaded: AtomicU64,
    /// Total download time in milliseconds.
    pub download_time_ms: AtomicU64,
    /// Number of retries performed.
    pub retries_performed: AtomicU64,
}

impl DownloadMetrics {
    /// Record a successful download.
    pub(super) fn record_success(&self, bytes: u64, duration_ms: u64) {
        self.downloads_succeeded.fetch_add(1, Ordering::Relaxed);
        self.bytes_downloaded.fetch_add(bytes, Ordering::Relaxed);
        self.download_time_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Record a failed download.
    pub(super) fn record_failure(&self) {
        self.downloads_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a skipped download.
    pub(super) fn record_skipped(&self) {
        self.downloads_skipped.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an attempt.
    pub(super) fn record_attempt(&self) {
        self.downloads_attempted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a retry.
    pub(super) fn record_retry(&self) {
        self.retries_performed.fetch_add(1, Ordering::Relaxed);
    }
}
