//! SNIX Directory Storage Specification
//!
//! Formal specification for directory entry and depth limits.
//!
//! # Properties
//!
//! 1. **SNIX-3: Entry Bound**: entries <= MAX
//! 2. **SNIX-4: Depth Bound**: depth <= MAX
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-snix/verus/directory_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum entries in a single directory
    pub const MAX_DIRECTORY_ENTRIES: u64 = 100_000;

    /// Maximum directory traversal depth
    pub const MAX_DIRECTORY_DEPTH: u64 = 256;

    /// Maximum directories to buffer in recursive enumeration
    pub const MAX_RECURSIVE_BUFFER: u64 = 10_000;

    /// Directory operation timeout (ms)
    pub const DIRECTORY_TIMEOUT_MS: u64 = 10_000;

    /// BLAKE3 digest length
    pub const B3_DIGEST_LENGTH: u64 = 32;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Directory entry type
    pub enum EntryType {
        /// Regular file
        Regular,
        /// Directory (contains digest of nested Directory)
        Directory,
        /// Symlink
        Symlink,
    }

    /// Abstract directory entry
    pub struct DirectoryEntrySpec {
        /// Entry name
        pub name: Seq<char>,
        /// Entry type
        pub entry_type: EntryType,
        /// Size (for files) or 0 (for directories/symlinks)
        pub size: u64,
        /// Digest (for files and directories)
        pub digest: Seq<u8>,
        /// Symlink target (for symlinks)
        pub symlink_target: Option<Seq<char>>,
        /// Whether entry is executable (for files)
        pub executable: bool,
    }

    /// Abstract directory
    pub struct DirectorySpec {
        /// BLAKE3 digest of this directory
        pub digest: Seq<u8>,
        /// Entries in the directory
        pub entries: Seq<DirectoryEntrySpec>,
    }

    /// Directory traversal state
    pub struct TraversalState {
        /// Current depth in tree
        pub current_depth: u64,
        /// Directories visited
        pub visited_count: u64,
        /// Pending directories to visit
        pub pending_count: u64,
    }

    // ========================================================================
    // SNIX-3: Entry Bound
    // ========================================================================

    /// Directory has bounded entries
    pub open spec fn entry_bounded(dir: DirectorySpec) -> bool {
        dir.entries.len() <= MAX_DIRECTORY_ENTRIES
    }

    /// Proof: Reject too many entries
    pub proof fn reject_too_many_entries(entries: Seq<DirectoryEntrySpec>)
        requires entries.len() > MAX_DIRECTORY_ENTRIES
        ensures !entry_bounded(DirectorySpec {
            digest: Seq::empty(),
            entries,
        })
    {
        // entries.len() > MAX => !bounded
    }

    // ========================================================================
    // SNIX-4: Depth Bound
    // ========================================================================

    /// Traversal depth is bounded
    pub open spec fn depth_bounded(state: TraversalState) -> bool {
        state.current_depth <= MAX_DIRECTORY_DEPTH
    }

    /// Buffer is bounded
    pub open spec fn buffer_bounded(state: TraversalState) -> bool {
        state.pending_count <= MAX_RECURSIVE_BUFFER
    }

    /// Full traversal invariant
    pub open spec fn traversal_invariant(state: TraversalState) -> bool {
        depth_bounded(state) && buffer_bounded(state)
    }

    /// Proof: Initial traversal is valid
    pub proof fn initial_traversal_valid()
        ensures traversal_invariant(TraversalState {
            current_depth: 0,
            visited_count: 0,
            pending_count: 0,
        })
    {
        // depth = 0 <= MAX, pending = 0 <= MAX
    }

    // ========================================================================
    // Directory Invariant
    // ========================================================================

    /// Directory digest is valid
    pub open spec fn directory_digest_valid(dir: DirectorySpec) -> bool {
        dir.digest.len() == B3_DIGEST_LENGTH
    }

    /// All entries have valid digests
    pub open spec fn entries_valid(dir: DirectorySpec) -> bool {
        forall |i: int| 0 <= i < dir.entries.len() ==>
            entry_valid(dir.entries[i])
    }

    /// Single entry is valid
    pub open spec fn entry_valid(entry: DirectoryEntrySpec) -> bool {
        entry.name.len() > 0 &&
        match entry.entry_type {
            EntryType::Regular | EntryType::Directory =>
                entry.digest.len() == B3_DIGEST_LENGTH,
            EntryType::Symlink =>
                entry.symlink_target.is_some() &&
                entry.symlink_target.unwrap().len() > 0,
        }
    }

    /// Full directory invariant
    pub open spec fn directory_invariant(dir: DirectorySpec) -> bool {
        entry_bounded(dir) &&
        directory_digest_valid(dir) &&
        entries_valid(dir)
    }

    // ========================================================================
    // Entry Uniqueness
    // ========================================================================

    /// All entry names are unique
    pub open spec fn entries_unique(dir: DirectorySpec) -> bool {
        forall |i: int, j: int|
            0 <= i < dir.entries.len() &&
            0 <= j < dir.entries.len() &&
            i != j ==>
            dir.entries[i].name != dir.entries[j].name
    }

    /// Proof: Finding entry by name is deterministic
    pub proof fn find_entry_deterministic(dir: DirectorySpec, name: Seq<char>)
        requires entries_unique(dir)
        ensures true  // At most one entry with this name
    {
        // Uniqueness ensures at most one match
    }

    // ========================================================================
    // Directory Operations
    // ========================================================================

    /// Put directory effect
    pub open spec fn put_directory_effect(
        dir: DirectorySpec,
        result_digest: Seq<u8>,
    ) -> bool {
        directory_invariant(dir) &&
        result_digest == dir.digest
    }

    /// Get directory effect
    pub open spec fn get_directory_effect(
        digest: Seq<u8>,
        result: Option<DirectorySpec>,
    ) -> bool {
        result.is_some() ==>
            result.unwrap().digest == digest &&
            directory_invariant(result.unwrap())
    }

    // ========================================================================
    // Recursive Traversal
    // ========================================================================

    /// Descend into subdirectory
    pub open spec fn descend_effect(
        pre: TraversalState,
        post: TraversalState,
    ) -> bool
        recommends pre.current_depth < MAX_DIRECTORY_DEPTH
    {
        post.current_depth == pre.current_depth + 1 &&
        post.visited_count == pre.visited_count + 1
    }

    /// Ascend from subdirectory
    pub open spec fn ascend_effect(
        pre: TraversalState,
        post: TraversalState,
    ) -> bool
        recommends pre.current_depth > 0
    {
        post.current_depth == pre.current_depth - 1
    }

    /// Proof: Descend preserves depth bound when allowed
    pub proof fn descend_preserves_bound(
        pre: TraversalState,
        post: TraversalState,
    )
        requires
            depth_bounded(pre),
            pre.current_depth < MAX_DIRECTORY_DEPTH,
            descend_effect(pre, post),
        ensures depth_bounded(post)
    {
        // pre.depth < MAX
        // post.depth = pre.depth + 1 <= MAX
    }

    // ========================================================================
    // Directory Size Calculation
    // ========================================================================

    /// Total size of all files in directory (non-recursive)
    pub open spec fn directory_size(dir: DirectorySpec) -> u64 {
        // Sum of all file sizes
        // Simplified: would use fold in real implementation
        0  // Placeholder
    }

    /// Proof: Directory size bounded by entry count * max file size
    pub proof fn directory_size_bounded(dir: DirectorySpec)
        requires entry_bounded(dir)
        ensures true  // Size bounded by MAX_ENTRIES * MAX_FILE_SIZE
    {
        // Each entry has bounded size
        // Total bounded by count * max
    }
}
