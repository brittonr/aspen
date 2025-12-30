//! Constants for the Pijul module.
//!
//! Tiger Style: All constants are explicitly typed with fixed limits
//! to prevent unbounded resource usage.

use std::time::Duration;

// ============================================================================
// Change Limits
// ============================================================================

/// Maximum size of a single Pijul change (compressed).
///
/// Tiger Style: Prevents memory exhaustion from oversized changes.
/// Changes are stored as gzipped bincode in iroh-blobs.
pub const MAX_CHANGE_SIZE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

/// Maximum number of hunks in a single change.
///
/// Tiger Style: Limits complexity of individual changes.
pub const MAX_HUNKS_PER_CHANGE: u32 = 10_000;

/// Maximum number of dependencies a change can have.
///
/// Tiger Style: Limits DAG complexity.
pub const MAX_CHANGE_DEPENDENCIES: u32 = 1_000;

/// Maximum depth when traversing change DAG.
///
/// Tiger Style: Prevents stack overflow on deep histories.
pub const MAX_CHANGE_DAG_DEPTH: u32 = 100_000;

// ============================================================================
// Repository Limits
// ============================================================================

/// Maximum file path length in a Pijul repository.
///
/// Tiger Style: Prevents pathological path handling.
pub const MAX_PATH_LENGTH_BYTES: u32 = 4096;

/// Maximum number of channels (branches) per repository.
///
/// Tiger Style: Limits channel management overhead.
pub const MAX_CHANNELS: u32 = 1_000;

/// Maximum length of a channel name.
pub const MAX_CHANNEL_NAME_LENGTH_BYTES: u32 = 256;

/// Maximum length of a change description/message.
pub const MAX_CHANGE_MESSAGE_BYTES: u32 = 64 * 1024; // 64 KB

/// Maximum number of authors per change.
pub const MAX_AUTHORS_PER_CHANGE: u32 = 16;

// ============================================================================
// Pristine Limits
// ============================================================================

/// Maximum size of the pristine database.
///
/// Tiger Style: Prevents disk exhaustion from pathological repositories.
/// Can be configured per-repository if needed.
pub const MAX_PRISTINE_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024; // 10 GB

/// Maximum number of vertices in the file graph.
///
/// Tiger Style: Limits memory usage during graph operations.
pub const MAX_PRISTINE_VERTICES: u64 = 100_000_000;

// ============================================================================
// Sync and Network Limits
// ============================================================================

/// Maximum number of changes to request in a single sync batch.
///
/// Tiger Style: Limits memory usage during sync operations.
pub const MAX_SYNC_BATCH_SIZE: u32 = 1_000;

/// Maximum number of concurrent change fetches.
///
/// Tiger Style: Prevents connection exhaustion.
pub const MAX_CONCURRENT_CHANGE_FETCHES: u32 = 64;

/// Timeout for fetching a single change from a peer.
pub const FETCH_CHANGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for applying a change to the pristine.
pub const APPLY_CHANGE_TIMEOUT: Duration = Duration::from_secs(60);

// ============================================================================
// Cache Configuration
// ============================================================================

/// Maximum number of changes to cache in memory.
///
/// Tiger Style: Bounds memory usage for change caching.
pub const CHANGE_CACHE_SIZE: usize = 1_000;

/// Maximum number of pristine handles to cache.
///
/// Tiger Style: Limits open file handles.
pub const PRISTINE_CACHE_SIZE: usize = 16;

// ============================================================================
// Gossip Configuration
// ============================================================================

/// Maximum number of repositories to subscribe to for gossip.
///
/// Tiger Style: Limits gossip connection overhead.
pub const PIJUL_GOSSIP_MAX_SUBSCRIBED_REPOS: u32 = 100;

/// Timeout for subscribing to a gossip topic.
pub const PIJUL_GOSSIP_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum retries for gossip stream errors.
pub const PIJUL_GOSSIP_MAX_STREAM_RETRIES: u32 = 10;

/// Backoff durations for gossip stream retries (in seconds).
pub const PIJUL_GOSSIP_STREAM_BACKOFF_SECS: &[u64] = &[1, 2, 4, 8, 16];

/// Maximum number of change hashes in a WantChanges or HaveChanges announcement.
///
/// Tiger Style: Limits announcement message size.
pub const PIJUL_GOSSIP_MAX_CHANGE_HASHES: u32 = 100;

// ============================================================================
// Sync Handler Configuration
// ============================================================================

/// Maximum number of changes to request or respond with in a single batch.
///
/// Tiger Style: Limits memory usage during sync operations.
pub const MAX_CHANGES_PER_REQUEST: u32 = 50;

/// Timeout in seconds for downloading a change from a peer.
///
/// After this timeout, the download is considered failed and can be retried.
pub const PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS: u64 = 60;

/// Deduplication window in seconds for sync requests.
///
/// Prevents repeatedly requesting the same change within this window.
pub const PIJUL_SYNC_REQUEST_DEDUP_SECS: u64 = 30;

// ============================================================================
// Key Prefixes
// ============================================================================

/// KV key prefix for Pijul repository metadata.
pub const KV_PREFIX_PIJUL_REPOS: &str = "pijul:repos:";

/// KV key prefix for Pijul channel heads.
pub const KV_PREFIX_PIJUL_CHANNELS: &str = "pijul:channels:";

/// KV key prefix for Pijul seeding configuration.
pub const KV_PREFIX_PIJUL_SEEDING: &str = "pijul:seeding:";

/// KV key prefix for change metadata (not the change itself, just metadata).
pub const KV_PREFIX_PIJUL_CHANGE_META: &str = "pijul:change:meta:";

// ============================================================================
// Working Directory Configuration
// ============================================================================

/// Working directory metadata directory name.
///
/// This directory is created inside the working directory root and contains
/// configuration, staged files list, and the local pristine cache.
pub const WORKING_DIR_METADATA_DIR: &str = ".aspen/pijul";

/// Working directory configuration file name.
pub const WORKING_DIR_CONFIG_FILE: &str = "config.toml";

/// Working directory staged files list.
pub const WORKING_DIR_STAGED_FILE: &str = "staged";

/// Working directory local pristine subdirectory.
pub const WORKING_DIR_PRISTINE_DIR: &str = "pristine";

/// Maximum number of staged files.
///
/// Tiger Style: Limits memory usage for tracking staged files.
pub const MAX_STAGED_FILES: u32 = 10_000;
