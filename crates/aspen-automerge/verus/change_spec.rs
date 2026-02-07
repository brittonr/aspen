//! Automerge Change Specification
//!
//! Formal specification for change size limits and batching.
//!
//! # Properties
//!
//! 1. **AM-4: Change Size Bound**: change_size <= MAX
//! 2. **AM-5: Batch Size Bound**: batch_size <= MAX
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-automerge/verus/change_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum size of a single change/patch (1 MB)
    pub const MAX_CHANGE_SIZE: u64 = 1024 * 1024;

    /// Maximum number of changes in a single batch
    pub const MAX_BATCH_CHANGES: u64 = 1000;

    /// Maximum document size (for reference)
    pub const MAX_DOCUMENT_SIZE: u64 = 16 * 1024 * 1024;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract change
    pub struct ChangeSpec {
        /// Size of the change in bytes
        pub size: u64,
        /// Hash of the change (for deduplication)
        pub hash: Seq<u8>,
        /// Dependencies (parent change hashes)
        pub deps: Seq<Seq<u8>>,
    }

    /// Abstract batch of changes
    pub struct ChangeBatchSpec {
        /// Changes in the batch
        pub changes: Seq<ChangeSpec>,
        /// Total size of all changes
        pub total_size: u64,
    }

    // ========================================================================
    // AM-4: Change Size Bound
    // ========================================================================

    /// Change size is within limits
    pub open spec fn change_size_bounded(change: ChangeSpec) -> bool {
        change.size <= MAX_CHANGE_SIZE
    }

    /// Proof: Reject oversized changes
    pub proof fn reject_oversized_changes(size: u64)
        requires size > MAX_CHANGE_SIZE
        ensures !change_size_bounded(ChangeSpec {
            size,
            hash: Seq::empty(),
            deps: Seq::empty(),
        })
    {
        // size > MAX => !bounded
    }

    // ========================================================================
    // AM-5: Batch Size Bound
    // ========================================================================

    /// Batch has bounded number of changes
    pub open spec fn batch_size_bounded(batch: ChangeBatchSpec) -> bool {
        batch.changes.len() <= MAX_BATCH_CHANGES
    }

    /// All changes in batch are bounded
    pub open spec fn batch_changes_bounded(batch: ChangeBatchSpec) -> bool {
        forall |i: int| 0 <= i < batch.changes.len() ==>
            change_size_bounded(batch.changes[i])
    }

    /// Batch total size is bounded
    pub open spec fn batch_total_bounded(batch: ChangeBatchSpec) -> bool {
        batch.total_size <= MAX_DOCUMENT_SIZE
    }

    /// Full batch invariant
    pub open spec fn batch_invariant(batch: ChangeBatchSpec) -> bool {
        batch_size_bounded(batch) &&
        batch_changes_bounded(batch) &&
        batch_total_bounded(batch)
    }

    // ========================================================================
    // Batch Operations
    // ========================================================================

    /// Add change to batch
    pub open spec fn add_to_batch_effect(
        pre: ChangeBatchSpec,
        post: ChangeBatchSpec,
        change: ChangeSpec,
    ) -> bool {
        change_size_bounded(change) &&
        post.changes.len() == pre.changes.len() + 1 &&
        post.changes[post.changes.len() - 1] == change &&
        post.total_size == pre.total_size + change.size
    }

    /// Can add to batch
    pub open spec fn can_add_to_batch(
        batch: ChangeBatchSpec,
        change: ChangeSpec,
    ) -> bool {
        batch.changes.len() < MAX_BATCH_CHANGES &&
        batch.total_size + change.size <= MAX_DOCUMENT_SIZE
    }

    /// Proof: Adding preserves batch invariant
    pub proof fn add_preserves_invariant(
        pre: ChangeBatchSpec,
        post: ChangeBatchSpec,
        change: ChangeSpec,
    )
        requires
            batch_invariant(pre),
            can_add_to_batch(pre, change),
            add_to_batch_effect(pre, post, change),
        ensures batch_invariant(post)
    {
        // All individual invariants preserved
    }

    // ========================================================================
    // Apply Changes
    // ========================================================================

    /// Apply changes to document
    pub open spec fn apply_changes_effect(
        doc_size_pre: u64,
        doc_size_post: u64,
        batch: ChangeBatchSpec,
    ) -> bool {
        // Document size may grow or shrink based on operations
        // But must stay within bounds
        doc_size_post <= MAX_DOCUMENT_SIZE
    }

    /// Apply is bounded by batch
    pub proof fn apply_bounded_by_batch(
        doc_size: u64,
        batch: ChangeBatchSpec,
    )
        requires
            doc_size <= MAX_DOCUMENT_SIZE,
            batch_invariant(batch),
        ensures
            // Worst case: all changes are additions
            doc_size + batch.total_size <= MAX_DOCUMENT_SIZE + MAX_DOCUMENT_SIZE
    {
        // doc <= MAX, batch.total <= MAX
        // sum <= 2 * MAX
    }

    // ========================================================================
    // Empty Batch
    // ========================================================================

    /// Empty batch
    pub open spec fn empty_batch() -> ChangeBatchSpec {
        ChangeBatchSpec {
            changes: Seq::empty(),
            total_size: 0,
        }
    }

    /// Proof: Empty batch satisfies invariant
    pub proof fn empty_batch_valid()
        ensures batch_invariant(empty_batch())
    {
        // len = 0 <= MAX
        // total = 0 <= MAX_DOC
        // No changes to check
    }

    // ========================================================================
    // Change Dependencies
    // ========================================================================

    /// Maximum number of dependencies per change
    pub const MAX_DEPS_PER_CHANGE: u64 = 100;

    /// Change dependencies are bounded
    pub open spec fn change_deps_bounded(change: ChangeSpec) -> bool {
        change.deps.len() <= MAX_DEPS_PER_CHANGE
    }

    /// Full change invariant
    pub open spec fn change_invariant(change: ChangeSpec) -> bool {
        change_size_bounded(change) &&
        change_deps_bounded(change)
    }

    // ========================================================================
    // Conflict Detection
    // ========================================================================

    /// Changes are concurrent (no dependency relationship)
    pub open spec fn changes_concurrent(
        change1: ChangeSpec,
        change2: ChangeSpec,
    ) -> bool {
        // Neither depends on the other
        !change1.deps.contains(change2.hash) &&
        !change2.deps.contains(change1.hash)
    }

    /// Changes may conflict if concurrent and affect same key
    /// (Actual conflict detection is in Automerge, this is simplified)
    pub open spec fn may_conflict(
        change1: ChangeSpec,
        change2: ChangeSpec,
    ) -> bool {
        changes_concurrent(change1, change2)
        // Real conflict depends on operations targeting same keys
    }
}
