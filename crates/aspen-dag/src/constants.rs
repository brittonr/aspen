//! Tiger Style resource bounds for DAG traversal and sync.
//!
//! All operations are bounded to prevent resource exhaustion.

/// Maximum depth for DAG traversal.
///
/// Limits stack growth during depth-first traversal. Git repos rarely
/// exceed 1,000 levels of tree nesting; 10,000 provides headroom for
/// pathological DAGs without unbounded memory use.
pub const MAX_DAG_TRAVERSAL_DEPTH: u32 = 10_000;

/// Maximum number of entries in the visited set.
///
/// Bounds memory for the HashSet tracking already-visited nodes.
/// 1M entries at 32 bytes each ≈ 32 MB of hash storage.
pub const MAX_VISITED_SET_SIZE: u32 = 1_000_000;

/// Maximum number of nodes a single traversal will yield.
///
/// Prevents runaway traversals on unexpectedly large DAGs.
pub const MAX_TRAVERSAL_NODES: u32 = 5_000_000;

/// Maximum serialized size of a DagSyncRequest (16 MiB).
///
/// Bounds the request payload. A visited set of 100K hashes at 32 bytes
/// each is ~3.2 MB, well within this limit.
pub const MAX_DAG_SYNC_REQUEST_SIZE: u32 = 16 * 1024 * 1024;

/// Maximum total bytes transferred in a single DAG sync response (10 GiB).
///
/// Prevents a single sync from consuming unbounded bandwidth.
pub const MAX_DAG_SYNC_TRANSFER_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// Maximum number of known heads in a DagSyncRequest.
///
/// Limits the size of the set sent by the receiver to short-circuit
/// traversal at already-synced boundaries.
pub const MAX_KNOWN_HEADS: u32 = 10_000;

/// Maximum number of children a single node may declare.
///
/// Bounds the fan-out per node during link extraction. A Git tree
/// with 100K entries is extreme; 1M is the safety ceiling.
pub const MAX_CHILDREN_PER_NODE: u32 = 1_000_000;

/// BAO chunk group size exponent. 2^4 * 1024 = 16 KiB chunk groups.
pub const BAO_CHUNK_GROUP_LOG: u32 = 4;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

const _: () = assert!(MAX_DAG_TRAVERSAL_DEPTH > 0);
const _: () = assert!(MAX_VISITED_SET_SIZE > 0);
const _: () = assert!(MAX_TRAVERSAL_NODES > 0);
const _: () = assert!(MAX_DAG_SYNC_REQUEST_SIZE > 0);
const _: () = assert!(MAX_DAG_SYNC_TRANSFER_SIZE > 0);
const _: () = assert!(MAX_KNOWN_HEADS > 0);
const _: () = assert!(MAX_CHILDREN_PER_NODE > 0);
const _: () = assert!(MAX_VISITED_SET_SIZE >= MAX_KNOWN_HEADS);
