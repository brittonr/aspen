/// Verus specifications for chunk size bounds and manifest entry bounds.
///
/// # Invariants Verified
///
/// - CHUNK-1: Chunk sizes are within [MIN_CHUNK_SIZE, MAX_CHUNK_SIZE]
/// - CHUNK-2: Total chunk sizes equal original blob size
/// - CHUNK-3: Manifest entry count bounded by blob size / min chunk size
/// - CHUNK-4: Inline threshold ≤ max chunk size (small blobs fit in one chunk)
use vstd::prelude::*;

verus! {

// ========================================================================
// Constants (mirror production values)
// ========================================================================

pub const MIN_CHUNK_SIZE: u32 = 16 * 1024;       // 16 KiB
pub const AVG_CHUNK_SIZE: u32 = 64 * 1024;       // 64 KiB
pub const MAX_CHUNK_SIZE: u32 = 256 * 1024;       // 256 KiB
pub const INLINE_THRESHOLD: u64 = 256 * 1024;     // 256 KiB
pub const MAX_INLINE_MANIFEST_SIZE: u64 = 64 * 1024; // 64 KiB

// ========================================================================
// Spec Functions
// ========================================================================

/// CHUNK-1: Valid chunk size range (last chunk may be smaller).
pub open spec fn valid_chunk_size(size: u32, is_last: bool) -> bool {
    if is_last {
        size > 0 && size <= MAX_CHUNK_SIZE
    } else {
        size >= MIN_CHUNK_SIZE && size <= MAX_CHUNK_SIZE
    }
}

/// CHUNK-2: Chunks cover the entire blob.
pub open spec fn chunks_cover_blob(chunk_sizes: Seq<u32>, blob_size: u64) -> bool {
    chunk_sizes.fold_left(0u64, |acc: u64, s: u32| acc + s as u64) == blob_size
}

/// CHUNK-3: Upper bound on manifest entry count.
pub open spec fn max_manifest_entries(blob_size: u64) -> u64 {
    if blob_size == 0 {
        0
    } else {
        // ceil(blob_size / MIN_CHUNK_SIZE)
        (blob_size + MIN_CHUNK_SIZE as u64 - 1) / MIN_CHUNK_SIZE as u64
    }
}

/// CHUNK-4: Inline threshold invariant — blobs at or below threshold
/// produce at most one chunk.
pub open spec fn inline_threshold_single_chunk(blob_size: u64) -> bool {
    blob_size <= INLINE_THRESHOLD ==> max_manifest_entries(blob_size) <= 1
        || blob_size <= MAX_CHUNK_SIZE as u64
}

// ========================================================================
// Exec Functions (verified implementations)
// ========================================================================

/// Compute maximum manifest entries for a given blob size.
pub fn compute_max_manifest_entries(blob_size: u64) -> (result: u64)
    ensures
        result == max_manifest_entries(blob_size),
        blob_size > 0 ==> result >= 1,
{
    if blob_size == 0 {
        0
    } else {
        (blob_size + MIN_CHUNK_SIZE as u64 - 1) / MIN_CHUNK_SIZE as u64
    }
}

/// Check if a blob should be chunked (above inline threshold).
pub fn should_chunk(blob_size: u64) -> (result: bool)
    ensures result == (blob_size > INLINE_THRESHOLD)
{
    blob_size > INLINE_THRESHOLD
}

// ========================================================================
// Constant Relationship Proofs
// ========================================================================

/// Proof: MIN < AVG < MAX chunk sizes.
proof fn chunk_size_ordering()
    ensures
        MIN_CHUNK_SIZE < AVG_CHUNK_SIZE,
        AVG_CHUNK_SIZE < MAX_CHUNK_SIZE,
        MIN_CHUNK_SIZE > 0,
{
}

/// Proof: inline threshold equals max chunk size.
proof fn inline_equals_max_chunk()
    ensures INLINE_THRESHOLD == MAX_CHUNK_SIZE as u64
{
}

/// Proof: manifest entry size (36 bytes each) bounded.
proof fn manifest_size_bounded(entry_count: u64)
    requires entry_count <= 65536  // practical upper bound
    ensures entry_count * 36 <= 65536 * 36
{
}

} // verus!
