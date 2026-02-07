//! Blob Concurrency Specification
//!
//! Formal specification for concurrent download/upload limits.
//!
//! # Properties
//!
//! 1. **BLOB-4: Download Limit**: concurrent_downloads <= MAX
//! 2. **BLOB-5: Upload Limit**: concurrent_uploads <= MAX
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-blob/verus/concurrency_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum concurrent blob downloads
    pub const MAX_CONCURRENT_DOWNLOADS: u32 = 10;

    /// Maximum concurrent blob uploads
    pub const MAX_CONCURRENT_UPLOADS: u32 = 10;

    /// Download timeout in seconds
    pub const DOWNLOAD_TIMEOUT_SECS: u64 = 300;

    /// Upload timeout in seconds
    pub const UPLOAD_TIMEOUT_SECS: u64 = 300;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract concurrent operation state
    pub struct ConcurrencyState {
        /// Active download count
        pub active_downloads: u32,
        /// Active upload count
        pub active_uploads: u32,
        /// Available download permits
        pub download_permits: u32,
        /// Available upload permits
        pub upload_permits: u32,
    }

    // ========================================================================
    // BLOB-4: Download Limit
    // ========================================================================

    /// Active downloads within limit
    pub open spec fn download_bounded(state: ConcurrencyState) -> bool {
        state.active_downloads <= MAX_CONCURRENT_DOWNLOADS
    }

    /// Download permit invariant
    pub open spec fn download_permit_invariant(state: ConcurrencyState) -> bool {
        state.active_downloads + state.download_permits == MAX_CONCURRENT_DOWNLOADS
    }

    /// Proof: Permit invariant implies bounds
    pub proof fn download_invariant_implies_bounds(state: ConcurrencyState)
        requires download_permit_invariant(state)
        ensures download_bounded(state)
    {
        // active + permits = MAX
        // permits >= 0
        // Therefore active <= MAX
    }

    // ========================================================================
    // BLOB-5: Upload Limit
    // ========================================================================

    /// Active uploads within limit
    pub open spec fn upload_bounded(state: ConcurrencyState) -> bool {
        state.active_uploads <= MAX_CONCURRENT_UPLOADS
    }

    /// Upload permit invariant
    pub open spec fn upload_permit_invariant(state: ConcurrencyState) -> bool {
        state.active_uploads + state.upload_permits == MAX_CONCURRENT_UPLOADS
    }

    /// Proof: Permit invariant implies bounds
    pub proof fn upload_invariant_implies_bounds(state: ConcurrencyState)
        requires upload_permit_invariant(state)
        ensures upload_bounded(state)
    {
        // active + permits = MAX
        // permits >= 0
        // Therefore active <= MAX
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Full concurrency invariant
    pub open spec fn concurrency_invariant(state: ConcurrencyState) -> bool {
        download_bounded(state) &&
        upload_bounded(state) &&
        download_permit_invariant(state) &&
        upload_permit_invariant(state)
    }

    // ========================================================================
    // Download Operations
    // ========================================================================

    /// Start download semantics
    pub open spec fn start_download_effect(
        pre: ConcurrencyState,
        post: ConcurrencyState,
        success: bool,
    ) -> bool {
        if pre.download_permits == 0 {
            // No permits available
            !success && post == pre
        } else {
            // Acquire permit
            success &&
            post.active_downloads == pre.active_downloads + 1 &&
            post.download_permits == pre.download_permits - 1 &&
            post.active_uploads == pre.active_uploads &&
            post.upload_permits == pre.upload_permits
        }
    }

    /// Complete download semantics
    pub open spec fn complete_download_effect(
        pre: ConcurrencyState,
        post: ConcurrencyState,
    ) -> bool
        recommends pre.active_downloads > 0
    {
        post.active_downloads == pre.active_downloads - 1 &&
        post.download_permits == pre.download_permits + 1 &&
        post.active_uploads == pre.active_uploads &&
        post.upload_permits == pre.upload_permits
    }

    /// Proof: Start download preserves invariant
    pub proof fn start_download_preserves_invariant(
        pre: ConcurrencyState,
        post: ConcurrencyState,
        success: bool,
    )
        requires
            concurrency_invariant(pre),
            start_download_effect(pre, post, success),
        ensures concurrency_invariant(post)
    {
        if pre.download_permits == 0 {
            // No change
        } else {
            // active + 1, permits - 1 => sum unchanged
        }
    }

    /// Proof: Complete download preserves invariant
    pub proof fn complete_download_preserves_invariant(
        pre: ConcurrencyState,
        post: ConcurrencyState,
    )
        requires
            concurrency_invariant(pre),
            pre.active_downloads > 0,
            complete_download_effect(pre, post),
        ensures concurrency_invariant(post)
    {
        // active - 1, permits + 1 => sum unchanged
    }

    // ========================================================================
    // Upload Operations
    // ========================================================================

    /// Start upload semantics
    pub open spec fn start_upload_effect(
        pre: ConcurrencyState,
        post: ConcurrencyState,
        success: bool,
    ) -> bool {
        if pre.upload_permits == 0 {
            !success && post == pre
        } else {
            success &&
            post.active_uploads == pre.active_uploads + 1 &&
            post.upload_permits == pre.upload_permits - 1 &&
            post.active_downloads == pre.active_downloads &&
            post.download_permits == pre.download_permits
        }
    }

    /// Complete upload semantics
    pub open spec fn complete_upload_effect(
        pre: ConcurrencyState,
        post: ConcurrencyState,
    ) -> bool
        recommends pre.active_uploads > 0
    {
        post.active_uploads == pre.active_uploads - 1 &&
        post.upload_permits == pre.upload_permits + 1 &&
        post.active_downloads == pre.active_downloads &&
        post.download_permits == pre.download_permits
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial concurrency state
    pub open spec fn initial_concurrency_state() -> ConcurrencyState {
        ConcurrencyState {
            active_downloads: 0,
            active_uploads: 0,
            download_permits: MAX_CONCURRENT_DOWNLOADS,
            upload_permits: MAX_CONCURRENT_UPLOADS,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_valid()
        ensures concurrency_invariant(initial_concurrency_state())
    {
        let state = initial_concurrency_state();
        // active = 0, permits = MAX => bounded and invariant
    }

    // ========================================================================
    // Timeout Properties
    // ========================================================================

    /// Operation must complete or timeout
    pub open spec fn operation_terminates(
        start_time: u64,
        current_time: u64,
        timeout_secs: u64,
        completed: bool,
    ) -> bool {
        completed || current_time >= start_time + timeout_secs
    }

    /// Download timeout check
    pub open spec fn download_timeout_check(
        start_time: u64,
        current_time: u64,
    ) -> bool {
        current_time >= start_time + DOWNLOAD_TIMEOUT_SECS
    }

    /// Upload timeout check
    pub open spec fn upload_timeout_check(
        start_time: u64,
        current_time: u64,
    ) -> bool {
        current_time >= start_time + UPLOAD_TIMEOUT_SECS
    }
}
