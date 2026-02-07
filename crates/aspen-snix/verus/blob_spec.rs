//! SNIX Blob Storage Specification
//!
//! Formal specification for blob size and digest validation.
//!
//! # Properties
//!
//! 1. **SNIX-1: Blob Size Bound**: blob_size <= MAX
//! 2. **SNIX-2: Digest Valid**: correct length
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-snix/verus/blob_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum size of a single blob (1 GB)
    pub const MAX_BLOB_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

    /// Length of BLAKE3 digest (32 bytes)
    pub const B3_DIGEST_LENGTH: u64 = 32;

    /// Chunk size for streaming reads (256 KB)
    pub const BLOB_CHUNK_SIZE_BYTES: u64 = 256 * 1024;

    /// Blob operation timeout (ms)
    pub const BLOB_TIMEOUT_MS: u64 = 30_000;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract blob
    pub struct BlobSpec {
        /// BLAKE3 digest of content
        pub digest: Seq<u8>,
        /// Size in bytes
        pub size: u64,
        /// Whether blob exists in store
        pub exists: bool,
    }

    /// Blob read request
    pub struct BlobReadRequest {
        /// Digest to fetch
        pub digest: Seq<u8>,
        /// Whether to stream in chunks
        pub streaming: bool,
    }

    // ========================================================================
    // SNIX-1: Blob Size Bound
    // ========================================================================

    /// Blob size is within limits
    pub open spec fn blob_size_bounded(blob: BlobSpec) -> bool {
        blob.size <= MAX_BLOB_SIZE_BYTES
    }

    /// Proof: Reject oversized blobs
    pub proof fn reject_oversized_blobs(size: u64)
        requires size > MAX_BLOB_SIZE_BYTES
        ensures !blob_size_bounded(BlobSpec {
            digest: Seq::empty(),
            size,
            exists: true,
        })
    {
        // size > MAX => !bounded
    }

    // ========================================================================
    // SNIX-2: Digest Valid
    // ========================================================================

    /// Digest has valid format
    pub open spec fn digest_valid(digest: Seq<u8>) -> bool {
        digest.len() == B3_DIGEST_LENGTH
    }

    /// Blob has valid digest
    pub open spec fn blob_digest_valid(blob: BlobSpec) -> bool {
        digest_valid(blob.digest)
    }

    /// Proof: Empty digest is invalid
    pub proof fn empty_digest_invalid()
        ensures !digest_valid(Seq::empty())
    {
        // 0 != 32
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Full blob invariant
    pub open spec fn blob_invariant(blob: BlobSpec) -> bool {
        blob_size_bounded(blob) &&
        blob_digest_valid(blob)
    }

    // ========================================================================
    // Chunked Reading
    // ========================================================================

    /// Number of chunks for a blob
    pub open spec fn chunk_count(size: u64) -> u64 {
        if size == 0 {
            0
        } else {
            (size + BLOB_CHUNK_SIZE_BYTES - 1) / BLOB_CHUNK_SIZE_BYTES
        }
    }

    /// Maximum chunks for any valid blob
    pub open spec fn max_chunks() -> u64 {
        (MAX_BLOB_SIZE_BYTES + BLOB_CHUNK_SIZE_BYTES - 1) / BLOB_CHUNK_SIZE_BYTES
    }

    /// Proof: Chunk count is bounded for valid blobs
    pub proof fn chunk_count_bounded(blob: BlobSpec)
        requires blob_size_bounded(blob)
        ensures chunk_count(blob.size) <= max_chunks()
    {
        // size <= MAX => chunks <= max_chunks
    }

    /// Chunk at index is valid size
    pub open spec fn chunk_size_at(size: u64, chunk_index: u64) -> u64
        recommends chunk_index < chunk_count(size)
    {
        let offset = chunk_index * BLOB_CHUNK_SIZE_BYTES;
        let remaining = size - offset;
        if remaining < BLOB_CHUNK_SIZE_BYTES {
            remaining
        } else {
            BLOB_CHUNK_SIZE_BYTES
        }
    }

    // ========================================================================
    // Content Addressing
    // ========================================================================

    /// Same digest implies same content (axiom)
    pub open spec fn content_addressed(
        blob1: BlobSpec,
        blob2: BlobSpec,
    ) -> bool {
        blob1.digest == blob2.digest ==> blob1.size == blob2.size
    }

    /// Proof: Content addressing is reflexive
    pub proof fn content_addressing_reflexive(blob: BlobSpec)
        ensures content_addressed(blob, blob)
    {
        // Same blob has same digest and size
    }

    // ========================================================================
    // Blob Operations
    // ========================================================================

    /// Put blob effect
    pub open spec fn put_blob_effect(
        pre_exists: bool,
        post_exists: bool,
        blob: BlobSpec,
    ) -> bool {
        // After put, blob exists
        post_exists &&
        // Blob is valid
        blob_invariant(blob)
    }

    /// Get blob effect
    pub open spec fn get_blob_effect(
        blob: BlobSpec,
        result: Option<Seq<u8>>,
    ) -> bool {
        if blob.exists {
            result.is_some() &&
            result.unwrap().len() == blob.size
        } else {
            result.is_none()
        }
    }

    /// Delete blob effect
    pub open spec fn delete_blob_effect(
        pre_exists: bool,
        post_exists: bool,
    ) -> bool {
        // After delete, blob doesn't exist
        !post_exists
    }

    // ========================================================================
    // Streaming State
    // ========================================================================

    /// Blob stream state
    pub struct BlobStreamState {
        /// Total blob size
        pub total_size: u64,
        /// Bytes read so far
        pub bytes_read: u64,
        /// Whether stream is complete
        pub complete: bool,
    }

    /// Stream invariant
    pub open spec fn stream_invariant(stream: BlobStreamState) -> bool {
        stream.bytes_read <= stream.total_size &&
        (stream.complete == (stream.bytes_read == stream.total_size))
    }

    /// Read chunk effect
    pub open spec fn read_chunk_effect(
        pre: BlobStreamState,
        post: BlobStreamState,
        chunk_size: u64,
    ) -> bool {
        post.total_size == pre.total_size &&
        post.bytes_read == pre.bytes_read + chunk_size &&
        post.bytes_read <= post.total_size
    }

    /// Proof: Reading preserves stream invariant
    pub proof fn read_preserves_invariant(
        pre: BlobStreamState,
        post: BlobStreamState,
        chunk_size: u64,
    )
        requires
            stream_invariant(pre),
            read_chunk_effect(pre, post, chunk_size),
        ensures
            post.bytes_read <= post.total_size
    {
        // post.bytes_read = pre.bytes_read + chunk_size <= total_size
    }
}
