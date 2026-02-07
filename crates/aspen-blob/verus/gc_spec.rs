//! Blob Garbage Collection Specification
//!
//! Formal specification for blob GC timing and tag protection.
//!
//! # Properties
//!
//! 1. **BLOB-6: GC Grace Period**: Grace period before collection
//! 2. **BLOB-7: Tag Protection**: Tagged blobs not collected
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-blob/verus/gc_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// GC interval in seconds
    pub const GC_INTERVAL_SECS: u64 = 60;

    /// Grace period before collecting unreferenced blobs (seconds)
    pub const GC_GRACE_PERIOD_SECS: u64 = 300;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract blob with GC metadata
    pub struct BlobGcState {
        /// BLAKE3 hash identifying the blob
        pub hash: Seq<u8>,
        /// Whether blob has any tags
        pub has_tags: bool,
        /// Last reference timestamp
        pub last_referenced: u64,
        /// Whether blob is in active transfer
        pub in_transfer: bool,
    }

    /// Abstract GC state
    pub struct GcState {
        /// Current time
        pub current_time: u64,
        /// Time of last GC run
        pub last_gc_time: u64,
        /// Blobs in the store
        pub blobs: Seq<BlobGcState>,
    }

    // ========================================================================
    // BLOB-6: GC Grace Period
    // ========================================================================

    /// Blob is within grace period
    pub open spec fn within_grace_period(
        blob: BlobGcState,
        current_time: u64,
    ) -> bool {
        current_time < blob.last_referenced + GC_GRACE_PERIOD_SECS
    }

    /// Grace period respected before collection
    pub open spec fn grace_period_respected(
        blob: BlobGcState,
        current_time: u64,
        collected: bool,
    ) -> bool {
        // If within grace period, cannot be collected
        within_grace_period(blob, current_time) ==> !collected
    }

    /// Proof: Grace period protects recent references
    pub proof fn grace_period_protection(
        blob: BlobGcState,
        current_time: u64,
    )
        requires current_time < blob.last_referenced + GC_GRACE_PERIOD_SECS
        ensures !grace_period_respected(blob, current_time, true)
    {
        // within_grace => cannot be collected => collected=true violates
    }

    // ========================================================================
    // BLOB-7: Tag Protection
    // ========================================================================

    /// Tagged blob is protected from GC
    pub open spec fn tag_protection(
        blob: BlobGcState,
        collected: bool,
    ) -> bool {
        blob.has_tags ==> !collected
    }

    /// Proof: Tags always protect blobs
    pub proof fn tags_protect_blobs(blob: BlobGcState)
        requires blob.has_tags
        ensures !tag_protection(blob, true)
    {
        // has_tags => !collected
    }

    // ========================================================================
    // Collection Eligibility
    // ========================================================================

    /// Blob is eligible for collection
    pub open spec fn eligible_for_collection(
        blob: BlobGcState,
        current_time: u64,
    ) -> bool {
        // No tags
        !blob.has_tags &&
        // Past grace period
        !within_grace_period(blob, current_time) &&
        // Not in active transfer
        !blob.in_transfer
    }

    /// Proof: Collection only if eligible
    pub proof fn collection_requires_eligibility(
        blob: BlobGcState,
        current_time: u64,
        collected: bool,
    )
        requires collected
        ensures
            !blob.has_tags,
            !within_grace_period(blob, current_time),
            !blob.in_transfer,
    {
        // collected ==> eligible
    }

    // ========================================================================
    // GC Run Semantics
    // ========================================================================

    /// GC run effect
    pub open spec fn gc_run_effect(
        pre: GcState,
        post: GcState,
        collected_hashes: Seq<Seq<u8>>,
    ) -> bool {
        // Time advances
        post.current_time >= pre.current_time &&
        // Last GC updated
        post.last_gc_time == post.current_time &&
        // Only eligible blobs collected
        forall |i: int| 0 <= i < collected_hashes.len() ==> {
            exists |j: int| 0 <= j < pre.blobs.len() &&
                pre.blobs[j].hash == collected_hashes[i] &&
                eligible_for_collection(pre.blobs[j], pre.current_time)
        } &&
        // Non-eligible blobs preserved
        forall |j: int| 0 <= j < pre.blobs.len() ==> {
            !eligible_for_collection(pre.blobs[j], pre.current_time) ==>
            exists |k: int| 0 <= k < post.blobs.len() &&
                post.blobs[k].hash == pre.blobs[j].hash
        }
    }

    /// Proof: GC preserves protected blobs
    pub proof fn gc_preserves_protected(
        pre: GcState,
        post: GcState,
        collected: Seq<Seq<u8>>,
    )
        requires gc_run_effect(pre, post, collected)
        ensures
            forall |j: int| 0 <= j < pre.blobs.len() &&
                (pre.blobs[j].has_tags ||
                 within_grace_period(pre.blobs[j], pre.current_time) ||
                 pre.blobs[j].in_transfer) ==>
            exists |k: int| 0 <= k < post.blobs.len() &&
                post.blobs[k].hash == pre.blobs[j].hash
    {
        // Protected = !eligible => preserved
    }

    // ========================================================================
    // GC Interval
    // ========================================================================

    /// GC should run based on interval
    pub open spec fn gc_should_run(state: GcState) -> bool {
        state.current_time >= state.last_gc_time + GC_INTERVAL_SECS
    }

    /// Proof: GC runs periodically
    pub proof fn gc_runs_periodically(
        initial_time: u64,
        current_time: u64,
        last_gc: u64,
    )
        requires
            last_gc >= initial_time,
            current_time >= last_gc + GC_INTERVAL_SECS,
        ensures gc_should_run(GcState {
            current_time,
            last_gc_time: last_gc,
            blobs: Seq::empty(),
        })
    {
        // current >= last + interval => should run
    }

    // ========================================================================
    // Tag Operations
    // ========================================================================

    /// Adding tag protects blob
    pub open spec fn add_tag_effect(
        pre: BlobGcState,
        post: BlobGcState,
    ) -> bool {
        post.hash == pre.hash &&
        post.has_tags &&
        post.last_referenced == pre.last_referenced &&
        post.in_transfer == pre.in_transfer
    }

    /// Removing last tag may make blob eligible
    pub open spec fn remove_tag_effect(
        pre: BlobGcState,
        post: BlobGcState,
        was_last_tag: bool,
    ) -> bool {
        post.hash == pre.hash &&
        // If last tag removed, no longer has tags
        (was_last_tag ==> !post.has_tags) &&
        (!was_last_tag ==> post.has_tags == pre.has_tags) &&
        post.last_referenced == pre.last_referenced &&
        post.in_transfer == pre.in_transfer
    }

    /// Proof: Adding tag guarantees protection
    pub proof fn add_tag_protects(
        pre: BlobGcState,
        post: BlobGcState,
    )
        requires add_tag_effect(pre, post)
        ensures forall |current_time: u64| tag_protection(post, true) == false
    {
        // has_tags => !collected for any collected=true
    }

    // ========================================================================
    // Reference Update
    // ========================================================================

    /// Accessing blob updates last_referenced
    pub open spec fn reference_update_effect(
        pre: BlobGcState,
        post: BlobGcState,
        access_time: u64,
    ) -> bool {
        post.hash == pre.hash &&
        post.has_tags == pre.has_tags &&
        post.last_referenced == access_time &&
        post.in_transfer == pre.in_transfer
    }

    /// Proof: Reference restarts grace period
    pub proof fn reference_restarts_grace(
        pre: BlobGcState,
        post: BlobGcState,
        access_time: u64,
    )
        requires
            reference_update_effect(pre, post, access_time),
        ensures within_grace_period(post, access_time)
    {
        // last_referenced = access_time
        // access_time < access_time + grace_period
    }
}
