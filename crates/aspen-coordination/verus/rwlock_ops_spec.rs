//! RWLock Operation Specifications
//!
//! Proves that RWLock operations preserve all invariants.
//!
//! # Key Properties
//!
//! 1. **Acquire Read**: Adds reader if no writer and no pending writers
//! 2. **Acquire Write**: Acquires exclusive access, increments token
//! 3. **Release Read**: Removes reader, transitions to Free if last
//! 4. **Release Write**: Clears writer, transitions to Free
//! 5. **Downgrade**: Write -> Read atomically
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/rwlock_ops_spec.rs
//! ```

use vstd::prelude::*;

use super::rwlock_state_spec::*;

verus! {
    // ========================================================================
    // Acquire Read
    // ========================================================================

    /// Precondition for acquiring read lock
    pub open spec fn acquire_read_pre(state: RWLockStateSpec) -> bool {
        !is_write_mode(state) &&
        !has_writer(state) &&
        state.pending_writers == 0 &&
        state.reader_count < state.max_readers
    }

    /// Result of acquiring read lock
    pub open spec fn acquire_read_post(pre: RWLockStateSpec) -> RWLockStateSpec
        recommends acquire_read_pre(pre)
    {
        RWLockStateSpec {
            mode: RWLockModeSpec::Read,
            writer: None,
            reader_count: (pre.reader_count + 1) as u32,
            pending_writers: pre.pending_writers,
            fencing_token: pre.fencing_token,
            max_readers: pre.max_readers,
        }
    }

    // ========================================================================
    // Acquire Write
    // ========================================================================

    /// Precondition for acquiring write lock
    pub open spec fn acquire_write_pre(state: RWLockStateSpec) -> bool {
        is_free(state) ||
        (is_read_mode(state) && state.reader_count == 0)
    }

    /// Result of acquiring write lock
    pub open spec fn acquire_write_post(
        pre: RWLockStateSpec,
        holder_id: Seq<u8>,
        deadline_ms: u64,
    ) -> RWLockStateSpec
        recommends
            acquire_write_pre(pre),
            pre.fencing_token < 0xFFFF_FFFF_FFFF_FFFFu64,
    {
        let new_token = (pre.fencing_token + 1) as u64;
        RWLockStateSpec {
            mode: RWLockModeSpec::Write,
            writer: Some(WriterEntrySpec {
                holder_id,
                fencing_token: new_token,
                deadline_ms,
            }),
            reader_count: 0,
            pending_writers: if pre.pending_writers > 0 {
                (pre.pending_writers - 1) as u32
            } else {
                0
            },
            fencing_token: new_token,
            max_readers: pre.max_readers,
        }
    }

    // ========================================================================
    // Release Read
    // ========================================================================

    /// Precondition for releasing read lock
    pub open spec fn release_read_pre(state: RWLockStateSpec) -> bool {
        is_read_mode(state) && state.reader_count > 0
    }

    /// Result of releasing read lock
    pub open spec fn release_read_post(pre: RWLockStateSpec) -> RWLockStateSpec
        recommends release_read_pre(pre)
    {
        let new_count = (pre.reader_count - 1) as u32;
        RWLockStateSpec {
            mode: if new_count == 0 { RWLockModeSpec::Free } else { RWLockModeSpec::Read },
            writer: None,
            reader_count: new_count,
            pending_writers: pre.pending_writers,
            fencing_token: pre.fencing_token,
            max_readers: pre.max_readers,
        }
    }

    // ========================================================================
    // Release Write
    // ========================================================================

    /// Precondition for releasing write lock
    pub open spec fn release_write_pre(state: RWLockStateSpec, token: u64) -> bool {
        is_write_mode(state) &&
        state.writer.is_some() &&
        state.writer.unwrap().fencing_token == token
    }

    /// Result of releasing write lock
    pub open spec fn release_write_post(pre: RWLockStateSpec) -> RWLockStateSpec
        recommends release_write_pre(pre, pre.writer.unwrap().fencing_token)
    {
        RWLockStateSpec {
            mode: RWLockModeSpec::Free,
            writer: None,
            reader_count: 0,
            pending_writers: pre.pending_writers,
            fencing_token: pre.fencing_token,  // Token preserved for history
            max_readers: pre.max_readers,
        }
    }

    // ========================================================================
    // Downgrade (Write -> Read)
    // ========================================================================

    /// Precondition for downgrade
    pub open spec fn downgrade_pre(state: RWLockStateSpec, token: u64) -> bool {
        release_write_pre(state, token)
    }

    /// Result of downgrade
    pub open spec fn downgrade_post(
        pre: RWLockStateSpec,
        deadline_ms: u64,
    ) -> RWLockStateSpec
        recommends pre.writer.is_some()
    {
        RWLockStateSpec {
            mode: RWLockModeSpec::Read,
            writer: None,
            reader_count: 1,  // The downgrader becomes a reader
            pending_writers: pre.pending_writers,
            fencing_token: pre.fencing_token,  // Token preserved
            max_readers: pre.max_readers,
        }
    }

    // ========================================================================
    // Proofs: Acquire Read
    // ========================================================================

    /// Acquire read preserves mutual exclusion
    pub proof fn acquire_read_preserves_mutual_exclusion(
        pre: RWLockStateSpec,
    )
        requires
            acquire_read_pre(pre),
            mutual_exclusion_holds(pre),
        ensures
            mutual_exclusion_holds(acquire_read_post(pre))
    {
        let post = acquire_read_post(pre);
        // post.mode = Read, post.reader_count > 0, post.writer = None
    }

    /// Acquire read preserves fencing token
    pub proof fn acquire_read_preserves_token(
        pre: RWLockStateSpec,
    )
        requires acquire_read_pre(pre)
        ensures acquire_read_post(pre).fencing_token == pre.fencing_token
    {
        // By construction
    }

    /// Acquire read preserves invariant
    pub proof fn acquire_read_preserves_invariant(
        pre: RWLockStateSpec,
    )
        requires
            acquire_read_pre(pre),
            rwlock_invariant(pre),
        ensures
            rwlock_invariant(acquire_read_post(pre))
    {
        acquire_read_preserves_mutual_exclusion(pre);
        // writer_token_matches_global: no writer, trivially true
        // readers_bounded: pre.reader_count < max, so post <= max
    }

    // ========================================================================
    // Proofs: Acquire Write
    // ========================================================================

    /// Acquire write establishes mutual exclusion
    pub proof fn acquire_write_establishes_mutual_exclusion(
        pre: RWLockStateSpec,
        holder_id: Seq<u8>,
        deadline_ms: u64,
    )
        requires
            acquire_write_pre(pre),
            pre.fencing_token < 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures
            mutual_exclusion_holds(acquire_write_post(pre, holder_id, deadline_ms))
    {
        let post = acquire_write_post(pre, holder_id, deadline_ms);
        // post.mode = Write, post.reader_count = 0, post.writer = Some(...)
    }

    /// Acquire write increases fencing token
    pub proof fn acquire_write_increases_token(
        pre: RWLockStateSpec,
        holder_id: Seq<u8>,
        deadline_ms: u64,
    )
        requires
            acquire_write_pre(pre),
            pre.fencing_token < 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures
            acquire_write_post(pre, holder_id, deadline_ms).fencing_token > pre.fencing_token
    {
        // post.fencing_token = pre.fencing_token + 1
    }

    /// Acquire write preserves invariant
    pub proof fn acquire_write_preserves_invariant(
        pre: RWLockStateSpec,
        holder_id: Seq<u8>,
        deadline_ms: u64,
    )
        requires
            acquire_write_pre(pre),
            rwlock_invariant(pre),
            pre.fencing_token < 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures
            rwlock_invariant(acquire_write_post(pre, holder_id, deadline_ms))
    {
        acquire_write_establishes_mutual_exclusion(pre, holder_id, deadline_ms);
        let post = acquire_write_post(pre, holder_id, deadline_ms);
        // writer_token_matches_global: writer.fencing_token == post.fencing_token (both = pre + 1)
        // readers_bounded: post.reader_count = 0 <= max
    }

    // ========================================================================
    // Proofs: Release Read
    // ========================================================================

    /// Release read preserves mutual exclusion
    pub proof fn release_read_preserves_mutual_exclusion(
        pre: RWLockStateSpec,
    )
        requires
            release_read_pre(pre),
            mutual_exclusion_holds(pre),
        ensures
            mutual_exclusion_holds(release_read_post(pre))
    {
        let post = release_read_post(pre);
        // If new_count == 0: mode = Free, reader_count = 0, writer = None
        // If new_count > 0: mode = Read, reader_count > 0, writer = None
    }

    /// Release read preserves invariant
    pub proof fn release_read_preserves_invariant(
        pre: RWLockStateSpec,
    )
        requires
            release_read_pre(pre),
            rwlock_invariant(pre),
        ensures
            rwlock_invariant(release_read_post(pre))
    {
        release_read_preserves_mutual_exclusion(pre);
        // Token unchanged, no writer, readers decreased
    }

    // ========================================================================
    // Proofs: Release Write
    // ========================================================================

    /// Release write establishes free state
    pub proof fn release_write_makes_free(
        pre: RWLockStateSpec,
    )
        requires
            pre.writer.is_some(),
            release_write_pre(pre, pre.writer.unwrap().fencing_token),
        ensures
            is_free(release_write_post(pre))
    {
        // By construction: mode = Free
    }

    /// Release write preserves mutual exclusion
    pub proof fn release_write_preserves_mutual_exclusion(
        pre: RWLockStateSpec,
    )
        requires
            pre.writer.is_some(),
            release_write_pre(pre, pre.writer.unwrap().fencing_token),
            mutual_exclusion_holds(pre),
        ensures
            mutual_exclusion_holds(release_write_post(pre))
    {
        let post = release_write_post(pre);
        // mode = Free, reader_count = 0, writer = None
    }

    /// Release write preserves invariant
    pub proof fn release_write_preserves_invariant(
        pre: RWLockStateSpec,
    )
        requires
            pre.writer.is_some(),
            release_write_pre(pre, pre.writer.unwrap().fencing_token),
            rwlock_invariant(pre),
        ensures
            rwlock_invariant(release_write_post(pre))
    {
        release_write_preserves_mutual_exclusion(pre);
        // Token preserved (for history), no writer, readers = 0
    }

    // ========================================================================
    // Proofs: Downgrade
    // ========================================================================

    /// Downgrade preserves mutual exclusion
    pub proof fn downgrade_preserves_mutual_exclusion(
        pre: RWLockStateSpec,
        deadline_ms: u64,
    )
        requires
            pre.writer.is_some(),
            downgrade_pre(pre, pre.writer.unwrap().fencing_token),
            mutual_exclusion_holds(pre),
        ensures
            mutual_exclusion_holds(downgrade_post(pre, deadline_ms))
    {
        let post = downgrade_post(pre, deadline_ms);
        // mode = Read, reader_count = 1 > 0, writer = None
    }

    /// Downgrade preserves fencing token
    pub proof fn downgrade_preserves_token(
        pre: RWLockStateSpec,
        deadline_ms: u64,
    )
        requires pre.writer.is_some()
        ensures downgrade_post(pre, deadline_ms).fencing_token == pre.fencing_token
    {
        // Token preserved during downgrade
    }

    /// Downgrade preserves invariant
    pub proof fn downgrade_preserves_invariant(
        pre: RWLockStateSpec,
        deadline_ms: u64,
    )
        requires
            pre.writer.is_some(),
            downgrade_pre(pre, pre.writer.unwrap().fencing_token),
            rwlock_invariant(pre),
            pre.max_readers >= 1,  // Need at least 1 reader allowed
        ensures
            rwlock_invariant(downgrade_post(pre, deadline_ms))
    {
        downgrade_preserves_mutual_exclusion(pre, deadline_ms);
        let post = downgrade_post(pre, deadline_ms);
        // Token preserved, no writer (now reader), readers = 1
        // readers_bounded: 1 <= max_readers (by precondition)
        assert(post.reader_count == 1);
        assert(post.reader_count <= post.max_readers);
    }
}
