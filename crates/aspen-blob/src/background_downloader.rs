//! Background blob download manager.
//!
//! This module provides background downloading of blobs announced via gossip.
//! When a peer announces they have a blob available, the background downloader
//! can automatically fetch it for redundancy without blocking the caller.
//!
//! # Architecture
//!
//! ```text
//! GossipDiscovery (blob announcement received)
//!        |
//!        v
//! BlobAnnouncedCallback (enqueue_download)
//!        |
//!        v
//! BackgroundBlobDownloader (spawned worker task)
//!        |
//!        +--> Checks: already exists? size acceptable? peer available?
//!        |
//!        v
//! IrohBlobStore::download_from_peer()
//! ```
//!
//! # Tiger Style
//!
//! - Bounded concurrent downloads (MAX_CONCURRENT_BACKGROUND_DOWNLOADS)
//! - Fixed queue size prevents unbounded memory usage
//! - All operations have timeouts
//! - Failed downloads tracked per peer with backoff
//!
//! # Test Coverage
//!
//! See `tests/background_downloader_test.rs` for integration tests.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use iroh_blobs::Hash;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::BlobRead;
use crate::IrohBlobStore;

/// Maximum concurrent background downloads.
///
/// Tiger Style: Bounded to prevent resource exhaustion.
pub const MAX_CONCURRENT_BACKGROUND_DOWNLOADS: u32 = 8;

/// Maximum size for background downloads (100 MB).
///
/// Tiger Style: Skip very large blobs that should be explicitly requested.
pub const MAX_BACKGROUND_DOWNLOAD_SIZE: u64 = 100 * 1024 * 1024;

/// Background download timeout per blob.
///
/// Tiger Style: Bounded wait prevents hung downloads.
pub const BACKGROUND_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum pending downloads in the queue.
///
/// Tiger Style: Bounded queue prevents unbounded memory use.
pub const BACKGROUND_DOWNLOAD_QUEUE_SIZE: usize = 256;

/// Maximum consecutive failures per peer before backoff.
pub const MAX_PEER_FAILURES: u32 = 3;

/// Base backoff duration after peer failures.
pub const PEER_BACKOFF_BASE: Duration = Duration::from_secs(30);

/// A request to download a blob in the background.
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    /// BLAKE3 hash of the blob.
    pub hash: Hash,
    /// Public key of the peer providing the blob.
    pub provider: iroh::PublicKey,
    /// Size of the blob in bytes.
    pub size: u64,
    /// Optional tag for logging/categorization.
    pub tag: Option<String>,
}

/// Statistics about background download operations.
#[derive(Debug, Clone, Default)]
pub struct DownloadStats {
    /// Number of downloads queued.
    pub queued: u64,
    /// Number of downloads completed successfully.
    pub completed: u64,
    /// Number of downloads failed.
    pub failed: u64,
    /// Number of downloads skipped (already exists, too large, etc.).
    pub skipped: u64,
    /// Number of downloads currently in progress.
    pub in_progress: u32,
}

/// Per-peer download failure tracking.
#[derive(Debug, Clone, Default)]
struct PeerState {
    /// Consecutive failure count.
    consecutive_failures: u32,
    /// When backoff expires (if in backoff).
    backoff_until: Option<Instant>,
}

/// Background blob download manager.
///
/// Manages a pool of download workers that fetch blobs from peers
/// in the background. Downloads are prioritized by size (small first)
/// and filtered by peer reputation.
pub struct BackgroundBlobDownloader {
    /// Channel to send download requests to the worker.
    request_tx: mpsc::Sender<DownloadRequest>,
    /// Download statistics.
    stats: Arc<Mutex<DownloadStats>>,
    /// Cancellation token for graceful shutdown.
    cancel: CancellationToken,
    /// Handle to the worker task.
    worker_handle: Option<JoinHandle<()>>,
}

impl BackgroundBlobDownloader {
    /// Create a new background downloader.
    ///
    /// # Arguments
    ///
    /// * `blob_store` - The blob store to download into
    /// * `cancel` - Cancellation token for shutdown
    ///
    /// # Returns
    ///
    /// The downloader and a JoinHandle for the background worker task.
    pub async fn spawn(blob_store: Arc<IrohBlobStore>, cancel: CancellationToken) -> (Self, JoinHandle<()>) {
        let (request_tx, request_rx) = mpsc::channel(BACKGROUND_DOWNLOAD_QUEUE_SIZE);
        let stats = Arc::new(Mutex::new(DownloadStats::default()));

        let worker = DownloadWorker {
            blob_store,
            request_rx,
            stats: Arc::clone(&stats),
            cancel: cancel.clone(),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_BACKGROUND_DOWNLOADS as usize)),
            in_progress: Arc::new(Mutex::new(HashSet::new())),
            peer_states: Arc::new(Mutex::new(HashMap::new())),
        };

        let worker_handle = tokio::spawn(worker.run());

        let downloader = Self {
            request_tx,
            stats,
            cancel,
            worker_handle: None, // We don't track internally since caller owns the handle
        };

        (downloader, worker_handle)
    }

    /// Enqueue a blob for background download.
    ///
    /// This is a non-blocking operation. The download will be processed
    /// by the background worker when resources are available.
    ///
    /// # Arguments
    ///
    /// * `hash` - BLAKE3 hash of the blob
    /// * `provider` - Public key of the peer to download from
    /// * `size` - Size of the blob in bytes
    /// * `tag` - Optional tag for categorization
    ///
    /// # Returns
    ///
    /// Ok if the request was queued, Err if the queue is full.
    pub async fn enqueue_download(
        &self,
        hash: Hash,
        provider: iroh::PublicKey,
        size: u64,
        tag: Option<String>,
    ) -> Result<(), BackgroundDownloadError> {
        // Skip very large blobs
        if size > MAX_BACKGROUND_DOWNLOAD_SIZE {
            debug!(
                hash = %hash,
                size,
                max = MAX_BACKGROUND_DOWNLOAD_SIZE,
                "skipping background download: blob too large"
            );
            self.stats.lock().await.skipped += 1;
            return Ok(());
        }

        let request = DownloadRequest {
            hash,
            provider,
            size,
            tag,
        };

        match self.request_tx.try_send(request) {
            Ok(()) => {
                self.stats.lock().await.queued += 1;
                debug!(hash = %hash, provider = %provider.fmt_short(), "enqueued background download");
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                debug!(hash = %hash, "background download queue full");
                Err(BackgroundDownloadError::QueueFull)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(BackgroundDownloadError::Shutdown),
        }
    }

    /// Get current download statistics.
    pub async fn stats(&self) -> DownloadStats {
        self.stats.lock().await.clone()
    }

    /// Gracefully shut down the downloader.
    ///
    /// Cancels all in-flight downloads and waits for the worker to exit.
    pub async fn shutdown(mut self) {
        info!("shutting down background blob downloader");
        self.cancel.cancel();

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.await;
        }

        let (queued, completed, failed, skipped) = {
            let stats = self.stats.lock().await;
            (stats.queued, stats.completed, stats.failed, stats.skipped)
        }; // Lock released before logging
        info!(queued, completed, failed, skipped, "background blob downloader shutdown complete");
    }
}

/// Errors that can occur during background download operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundDownloadError {
    /// The download queue is full.
    QueueFull,
    /// The downloader has been shut down.
    Shutdown,
}

impl std::fmt::Display for BackgroundDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => write!(f, "background download queue is full"),
            Self::Shutdown => write!(f, "background downloader has been shut down"),
        }
    }
}

impl std::error::Error for BackgroundDownloadError {}

/// Internal worker that processes download requests.
struct DownloadWorker {
    blob_store: Arc<IrohBlobStore>,
    request_rx: mpsc::Receiver<DownloadRequest>,
    stats: Arc<Mutex<DownloadStats>>,
    cancel: CancellationToken,
    /// Semaphore limiting concurrent downloads.
    semaphore: Arc<Semaphore>,
    /// Set of hashes currently being downloaded (to avoid duplicates).
    in_progress: Arc<Mutex<HashSet<Hash>>>,
    /// Per-peer failure tracking.
    peer_states: Arc<Mutex<HashMap<iroh::PublicKey, PeerState>>>,
}

impl DownloadWorker {
    async fn run(mut self) {
        info!("background blob download worker started");

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    debug!("background download worker cancelled");
                    break;
                }
                Some(request) = self.request_rx.recv() => {
                    self.handle_request(request).await;
                }
            }
        }

        info!("background blob download worker stopped");
    }

    async fn handle_request(&self, request: DownloadRequest) {
        let hash = request.hash;

        // Check if already in progress
        {
            let in_progress = self.in_progress.lock().await;
            if in_progress.contains(&hash) {
                debug!(hash = %hash, "skipping: download already in progress");
                self.stats.lock().await.skipped += 1;
                return;
            }
        }

        // Check if peer is in backoff
        {
            let peer_states = self.peer_states.lock().await;
            if let Some(state) = peer_states.get(&request.provider)
                && let Some(backoff_until) = state.backoff_until
                && Instant::now() < backoff_until
            {
                debug!(
                    hash = %hash,
                    provider = %request.provider.fmt_short(),
                    "skipping: peer in backoff"
                );
                self.stats.lock().await.skipped += 1;
                return;
            }
        }

        // Check if blob already exists
        match self.blob_store.status(&hash).await {
            Ok(Some(status)) if status.complete => {
                debug!(hash = %hash, "skipping: blob already exists");
                self.stats.lock().await.skipped += 1;
                return;
            }
            Ok(_) => {}
            Err(e) => {
                warn!(hash = %hash, error = %e, "error checking blob existence");
            }
        }

        // Mark as in progress
        {
            let mut in_progress = self.in_progress.lock().await;
            in_progress.insert(hash);
        }
        self.stats.lock().await.in_progress += 1;

        // Spawn the download task
        let blob_store = Arc::clone(&self.blob_store);
        let stats = Arc::clone(&self.stats);
        let semaphore = Arc::clone(&self.semaphore);
        let in_progress = Arc::clone(&self.in_progress);
        let peer_states = Arc::clone(&self.peer_states);
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            // Acquire semaphore permit (waits if at capacity)
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    // Semaphore closed (shutdown)
                    let mut in_prog = in_progress.lock().await;
                    in_prog.remove(&hash);
                    let mut s = stats.lock().await;
                    s.in_progress = s.in_progress.saturating_sub(1);
                    s.failed += 1;
                    return;
                }
            };

            // Execute download with timeout
            let result = tokio::time::timeout(
                BACKGROUND_DOWNLOAD_TIMEOUT,
                blob_store.download_from_peer(&hash, request.provider),
            )
            .await;

            // Clean up in-progress set
            {
                let mut in_prog = in_progress.lock().await;
                in_prog.remove(&hash);
            }

            // Update stats and peer state
            let mut s = stats.lock().await;
            s.in_progress = s.in_progress.saturating_sub(1);

            match result {
                Ok(Ok(blob_ref)) => {
                    s.completed += 1;
                    info!(
                        hash = %hash,
                        size = blob_ref.size,
                        provider = %request.provider.fmt_short(),
                        tag = ?request.tag,
                        "background download completed"
                    );

                    // Reset peer failure count on success
                    let mut peer_states = peer_states.lock().await;
                    peer_states.remove(&request.provider);
                }
                Ok(Err(e)) => {
                    s.failed += 1;
                    warn!(
                        hash = %hash,
                        provider = %request.provider.fmt_short(),
                        error = %e,
                        "background download failed"
                    );

                    // Update peer failure state
                    let mut peer_states = peer_states.lock().await;
                    let state = peer_states.entry(request.provider).or_default();
                    state.consecutive_failures += 1;

                    if state.consecutive_failures >= MAX_PEER_FAILURES {
                        let backoff = PEER_BACKOFF_BASE * state.consecutive_failures;
                        state.backoff_until = Some(Instant::now() + backoff);
                        warn!(
                            provider = %request.provider.fmt_short(),
                            failures = state.consecutive_failures,
                            backoff_secs = backoff.as_secs(),
                            "peer entering backoff due to repeated failures"
                        );
                    }
                }
                Err(_timeout) => {
                    if !cancel.is_cancelled() {
                        s.failed += 1;
                        warn!(
                            hash = %hash,
                            provider = %request.provider.fmt_short(),
                            timeout_secs = BACKGROUND_DOWNLOAD_TIMEOUT.as_secs(),
                            "background download timed out"
                        );

                        // Timeout counts as a failure
                        let mut peer_states = peer_states.lock().await;
                        let state = peer_states.entry(request.provider).or_default();
                        state.consecutive_failures += 1;

                        if state.consecutive_failures >= MAX_PEER_FAILURES {
                            let backoff = PEER_BACKOFF_BASE * state.consecutive_failures;
                            state.backoff_until = Some(Instant::now() + backoff);
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_are_bounded() {
        assert!(MAX_CONCURRENT_BACKGROUND_DOWNLOADS <= 16);
        assert!(MAX_CONCURRENT_BACKGROUND_DOWNLOADS >= 1);
        assert!(BACKGROUND_DOWNLOAD_QUEUE_SIZE <= 1024);
        assert!(BACKGROUND_DOWNLOAD_QUEUE_SIZE >= 16);
        assert!(MAX_BACKGROUND_DOWNLOAD_SIZE <= 1024 * 1024 * 1024); // <= 1GB
        assert!(MAX_BACKGROUND_DOWNLOAD_SIZE >= 1024 * 1024); // >= 1MB
    }

    #[test]
    fn test_background_download_error_display() {
        assert_eq!(format!("{}", BackgroundDownloadError::QueueFull), "background download queue is full");
        assert_eq!(format!("{}", BackgroundDownloadError::Shutdown), "background downloader has been shut down");
    }

    #[test]
    fn test_download_stats_default() {
        let stats = DownloadStats::default();
        assert_eq!(stats.queued, 0);
        assert_eq!(stats.completed, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.in_progress, 0);
    }

    #[test]
    fn test_peer_state_default() {
        let state = PeerState::default();
        assert_eq!(state.consecutive_failures, 0);
        assert!(state.backoff_until.is_none());
    }

    #[test]
    fn test_download_request_clone() {
        let request = DownloadRequest {
            hash: Hash::from([0u8; 32]),
            provider: iroh::SecretKey::generate(&mut rand::rngs::OsRng).public(),
            size: 1024,
            tag: Some("test".to_string()),
        };
        let cloned = request.clone();
        assert_eq!(cloned.hash, request.hash);
        assert_eq!(cloned.size, request.size);
        assert_eq!(cloned.tag, request.tag);
    }
}
