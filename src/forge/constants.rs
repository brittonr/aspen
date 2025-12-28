//! Constants for the Forge module.
//!
//! Tiger Style: All constants are explicitly typed with fixed limits
//! to prevent unbounded resource usage.

use std::time::Duration;

// ============================================================================
// Git Object Limits
// ============================================================================

/// Maximum size of a single Git blob object.
///
/// Tiger Style: Prevents memory exhaustion from malicious or oversized files.
/// Large files should be stored via Git LFS or external blob references.
///
/// Used in:
/// - `GitBlobStore::store_blob`: Rejects blobs exceeding this limit
/// - `GitBlobStore::fetch_blob`: Validates received blob size
pub const MAX_BLOB_SIZE_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

/// Maximum size of a tree object (directory listing).
///
/// Tiger Style: Limits directory entries to prevent pathological cases.
/// A tree with 10,000 entries at ~100 bytes each = ~1MB.
pub const MAX_TREE_SIZE_BYTES: u64 = 2 * 1024 * 1024; // 2 MB

/// Maximum number of entries in a single tree object.
///
/// Tiger Style: Prevents excessive memory allocation during tree parsing.
pub const MAX_TREE_ENTRIES: u32 = 10_000;

/// Maximum size of a commit message.
///
/// Tiger Style: Prevents abuse via excessively long commit messages.
pub const MAX_COMMIT_MESSAGE_BYTES: u32 = 64 * 1024; // 64 KB

/// Maximum number of parents for a merge commit.
///
/// Tiger Style: Limits octopus merges to reasonable sizes.
/// Git itself has no hard limit, but we enforce one for sanity.
pub const MAX_COMMIT_PARENTS: u32 = 64;

// ============================================================================
// Collaborative Object Limits
// ============================================================================

/// Maximum size of a COB change payload.
///
/// Tiger Style: Prevents memory exhaustion from large issue bodies or comments.
pub const MAX_COB_CHANGE_SIZE_BYTES: u64 = 1024 * 1024; // 1 MB

/// Maximum number of parents for a COB change.
///
/// Tiger Style: Limits merge complexity in COB DAGs.
pub const MAX_COB_PARENTS: u32 = 32;

/// Maximum number of labels on an issue or patch.
///
/// Tiger Style: Prevents label explosion.
pub const MAX_LABELS: u32 = 100;

/// Maximum length of a label string.
pub const MAX_LABEL_LENGTH_BYTES: u32 = 256;

/// Maximum length of an issue/patch title.
pub const MAX_TITLE_LENGTH_BYTES: u32 = 512;

/// Maximum number of changes to walk when resolving COB state.
///
/// Tiger Style: Prevents runaway resolution on pathological DAGs.
pub const MAX_COB_CHANGES_TO_RESOLVE: u32 = 100_000;

// ============================================================================
// Repository Limits
// ============================================================================

/// Maximum number of delegates for a repository.
///
/// Tiger Style: Limits signature verification overhead.
pub const MAX_DELEGATES: u32 = 64;

/// Maximum threshold for delegate signatures (must be <= MAX_DELEGATES).
pub const MAX_THRESHOLD: u32 = MAX_DELEGATES;

/// Maximum length of a repository name.
pub const MAX_REPO_NAME_LENGTH_BYTES: u32 = 256;

/// Maximum length of a repository description.
pub const MAX_REPO_DESCRIPTION_LENGTH_BYTES: u32 = 4096;

/// Maximum number of refs (branches + tags) per repository.
///
/// Tiger Style: Prevents unbounded ref storage.
pub const MAX_REFS_PER_REPO: u32 = 10_000;

/// Maximum length of a ref name (e.g., "heads/feature/long-name").
pub const MAX_REF_NAME_LENGTH_BYTES: u32 = 512;

// ============================================================================
// Sync and Network Limits
// ============================================================================

/// Maximum number of objects to request in a single fetch batch.
///
/// Tiger Style: Limits memory usage during sync operations.
pub const MAX_FETCH_BATCH_SIZE: u32 = 1_000;

/// Maximum number of concurrent object fetches.
///
/// Tiger Style: Prevents connection exhaustion.
pub const MAX_CONCURRENT_FETCHES: u32 = 64;

/// Timeout for fetching a single object from a peer.
pub const FETCH_OBJECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for resolving COB state.
pub const COB_RESOLVE_TIMEOUT: Duration = Duration::from_secs(60);

// ============================================================================
// Gossip Limits
// ============================================================================

/// Minimum interval between gossip announcements for the same ref.
///
/// Tiger Style: Rate limiting to prevent gossip storms.
pub const MIN_GOSSIP_INTERVAL: Duration = Duration::from_secs(1);

/// Maximum number of refs to announce in a single gossip message.
pub const MAX_GOSSIP_REFS: u32 = 100;

/// Maximum size of a gossip message payload.
pub const MAX_GOSSIP_MESSAGE_SIZE_BYTES: u32 = 64 * 1024; // 64 KB

// ============================================================================
// Key Prefixes
// ============================================================================

/// KV key prefix for repository metadata.
pub const KV_PREFIX_REPOS: &str = "forge:repos:";

/// KV key prefix for refs.
pub const KV_PREFIX_REFS: &str = "forge:refs:";

/// KV key prefix for COB heads.
pub const KV_PREFIX_COB_HEADS: &str = "forge:cob:heads:";

/// KV key prefix for seeding configuration.
pub const KV_PREFIX_SEEDING: &str = "forge:seeding:";
