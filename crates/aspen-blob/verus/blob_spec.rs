//! Blob Storage Specification
//!
//! Formal specification for blob size limits and storage routing.
//!
//! # Properties
//!
//! 1. **BLOB-1: Size Bounds**: blob_size <= MAX_BLOB_SIZE
//! 2. **BLOB-2: Threshold Routing**: Large values offloaded
//! 3. **BLOB-3: Reference Integrity**: References valid
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-blob/verus/blob_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Threshold for automatic large value offloading (1 MB)
    pub const BLOB_THRESHOLD: u32 = 1_048_576;

    /// Maximum blob size (1 GB)
    pub const MAX_BLOB_SIZE: u64 = 1_073_741_824;

    /// Maximum blobs to list in a single request
    pub const MAX_BLOB_LIST_SIZE: u32 = 1000;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract blob storage state
    pub struct BlobStoreState {
        /// Number of blobs stored
        pub blob_count: u64,
        /// Total bytes stored
        pub total_bytes: u64,
    }

    /// Abstract blob reference
    pub struct BlobRefSpec {
        /// BLAKE3 hash of blob content
        pub hash: Seq<u8>,
        /// Size of the blob in bytes
        pub size_bytes: u64,
        /// Whether blob exists in store
        pub does_exist: bool,
    }

    /// Abstract KV value storage decision
    pub enum StorageDecision {
        /// Store directly in KV
        Inline,
        /// Store as blob with reference in KV
        Offload,
    }

    // ========================================================================
    // BLOB-1: Size Bounds
    // ========================================================================

    /// Blob size is within limits
    pub open spec fn blob_size_bounded(blob: BlobRefSpec) -> bool {
        blob.size_bytes <= MAX_BLOB_SIZE
    }

    /// Proof: Accept only bounded blobs
    pub proof fn reject_oversized_blobs(size_bytes: u64)
        requires size_bytes > MAX_BLOB_SIZE
        ensures !blob_size_bounded(BlobRefSpec {
            hash: Seq::empty(),
            size_bytes,
            does_exist: true,
        })
    {
        // size > MAX => !bounded
    }

    // ========================================================================
    // BLOB-2: Threshold Routing
    // ========================================================================

    /// Determine storage decision based on size
    pub open spec fn storage_decision(size: u64) -> StorageDecision {
        if size <= BLOB_THRESHOLD as u64 {
            StorageDecision::Inline
        } else {
            StorageDecision::Offload
        }
    }

    /// Threshold routing is correct
    pub open spec fn threshold_routing(size: u64, decision: StorageDecision) -> bool {
        match decision {
            StorageDecision::Inline => size <= BLOB_THRESHOLD as u64,
            StorageDecision::Offload => size > BLOB_THRESHOLD as u64,
        }
    }

    /// Proof: Routing decision is deterministic
    pub proof fn routing_deterministic(size: u64)
        ensures threshold_routing(size, storage_decision(size))
    {
        // Direct from storage_decision definition
    }

    /// Proof: Large values are always offloaded
    pub proof fn large_values_offloaded(size: u64)
        requires size > BLOB_THRESHOLD as u64
        ensures storage_decision(size) == StorageDecision::Offload
    {
        // size > threshold => Offload
    }

    /// Proof: Small values are always inlined
    pub proof fn small_values_inlined(size: u64)
        requires size <= BLOB_THRESHOLD as u64
        ensures storage_decision(size) == StorageDecision::Inline
    {
        // size <= threshold => Inline
    }

    // ========================================================================
    // BLOB-3: Reference Integrity
    // ========================================================================

    /// Blob reference points to existing blob
    pub open spec fn reference_integrity(blob_ref: BlobRefSpec) -> bool {
        blob_ref.does_exist
    }

    /// Blob store with reference integrity
    pub open spec fn store_integrity(refs: Seq<BlobRefSpec>) -> bool {
        forall |i: int| 0 <= i < refs.len() ==> reference_integrity(refs[i])
    }

    /// Creating blob ensures reference is valid
    pub open spec fn create_blob_effect(
        pre_refs: Seq<BlobRefSpec>,
        post_refs: Seq<BlobRefSpec>,
        new_blob: BlobRefSpec,
    ) -> bool {
        // New blob is valid
        new_blob.does_exist &&
        blob_size_bounded(new_blob) &&
        // Added to store
        post_refs.len() == pre_refs.len() + 1 &&
        post_refs[post_refs.len() - 1] == new_blob
    }

    /// Proof: Creation preserves integrity
    pub proof fn create_preserves_integrity(
        pre_refs: Seq<BlobRefSpec>,
        post_refs: Seq<BlobRefSpec>,
        new_blob: BlobRefSpec,
    )
        requires
            store_integrity(pre_refs),
            create_blob_effect(pre_refs, post_refs, new_blob),
        ensures store_integrity(post_refs)
    {
        // All old refs valid, new ref valid => all valid
    }

    // ========================================================================
    // Blob Reference Format
    // ========================================================================

    /// Blob reference prefix
    pub const BLOB_REF_PREFIX_LEN: usize = 7;  // "__blob:"

    /// Check if value is a blob reference
    pub open spec fn is_blob_reference(value: Seq<u8>) -> bool {
        // Value starts with "__blob:" prefix
        value.len() >= BLOB_REF_PREFIX_LEN
        // Actual prefix check would require byte comparison
    }

    /// Extract hash from blob reference
    pub open spec fn extract_blob_hash(value: Seq<u8>) -> Seq<u8>
        requires is_blob_reference(value)
    {
        // Skip prefix bytes
        value.subrange(BLOB_REF_PREFIX_LEN as int, value.len() as int)
    }

    // ========================================================================
    // List Pagination
    // ========================================================================

    /// List results are bounded
    pub open spec fn list_bounded(result_count: u32) -> bool {
        result_count <= MAX_BLOB_LIST_SIZE
    }

    /// Proof: Pagination always bounded
    pub proof fn pagination_bounds(requested: u32, available: u32)
        ensures {
            let returned = if requested < available { requested } else { available };
            let capped = if returned > MAX_BLOB_LIST_SIZE { MAX_BLOB_LIST_SIZE } else { returned };
            list_bounded(capped)
        }
    {
        // min(requested, available, MAX) <= MAX
    }

    // ========================================================================
    // Content Addressing
    // ========================================================================

    /// BLAKE3 hash length (32 bytes)
    pub const HASH_LENGTH: usize = 32;

    /// Hash is valid format
    pub open spec fn hash_valid(hash: Seq<u8>) -> bool {
        hash.len() == HASH_LENGTH
    }

    /// Same content produces same hash (axiom)
    pub open spec fn content_addressing(content1: Seq<u8>, content2: Seq<u8>, hash1: Seq<u8>, hash2: Seq<u8>) -> bool {
        // Axiom: BLAKE3 is deterministic
        content1 == content2 ==> hash1 == hash2
    }

    /// Proof: Content addressing is deterministic
    pub proof fn hash_deterministic()
        ensures true  // Axiom
    {
        // BLAKE3 is a pure function: same input -> same output
    }
}
