//! Directory Layer Operations Specification
//!
//! Formal specifications for directory layer operations.
//!
//! # Operations
//!
//! - `create`: Create a new directory at path with allocated prefix
//! - `open`: Open existing directory or create if not exists
//! - `remove`: Remove a directory
//! - `list`: List subdirectories
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-core/verus/directory_ops_spec.rs
//! ```

use vstd::prelude::*;

// Import from directory_state_spec
use crate::directory_state_spec::*;

verus! {
    // ========================================================================
    // Create Operation
    // ========================================================================

    /// Precondition for creating a directory
    pub open spec fn create_pre(state: DirectoryState, path: Seq<Seq<u8>>, new_prefix: u64) -> bool {
        // Path must not already exist
        !state.directories.contains_key(path) &&
        // Prefix must not be allocated
        !state.allocated_prefixes.contains(new_prefix) &&
        // Parent must exist (for non-root)
        (path.len() == 0 || state.directories.contains_key(path.take(path.len() - 1)))
    }

    /// Result of creating a directory
    pub open spec fn create_post(pre: DirectoryState, path: Seq<Seq<u8>>, new_prefix: u64) -> DirectoryState {
        let new_entry = DirectoryEntrySpec {
            path: PathSpec { components: path },
            prefix: new_prefix,
            layer: None,
        };
        DirectoryState {
            directories: pre.directories.insert(path, new_entry),
            allocated_prefixes: pre.allocated_prefixes.insert(new_prefix),
            prefix_to_path: pre.prefix_to_path.insert(new_prefix, path),
        }
    }

    /// Proof: Create preserves namespace isolation
    pub proof fn create_preserves_isolation(
        pre: DirectoryState,
        path: Seq<Seq<u8>>,
        new_prefix: u64,
    )
        requires
            create_pre(pre, path, new_prefix),
            namespace_isolation(pre),
        ensures namespace_isolation(create_post(pre, path, new_prefix))
    {
        let post = create_post(pre, path, new_prefix);
        // For any two different paths in post:
        // - If both are from pre, isolation holds by pre's invariant
        // - If one is new (path), its prefix is fresh (not in pre)
        assert forall |p1: Seq<Seq<u8>>, p2: Seq<Seq<u8>>|
            post.directories.contains_key(p1) &&
            post.directories.contains_key(p2) &&
            !(p1 =~= p2)
            implies post.directories[p1].prefix != post.directories[p2].prefix
        by {
            if p1 =~= path && !(p2 =~= path) {
                // p1 is new, p2 is old
                // new_prefix not in pre.allocated_prefixes
                // So post.directories[p2].prefix != new_prefix
                assert(pre.directories.contains_key(p2));
                assert(pre.allocated_prefixes.contains(pre.directories[p2].prefix));
                assert(!pre.allocated_prefixes.contains(new_prefix));
            } else if !(p1 =~= path) && p2 =~= path {
                // p1 is old, p2 is new
                assert(pre.directories.contains_key(p1));
                assert(pre.allocated_prefixes.contains(pre.directories[p1].prefix));
                assert(!pre.allocated_prefixes.contains(new_prefix));
            } else {
                // Both old
                assert(pre.directories.contains_key(p1));
                assert(pre.directories.contains_key(p2));
            }
        }
    }

    /// Proof: Create preserves prefix uniqueness
    pub proof fn create_preserves_uniqueness(
        pre: DirectoryState,
        path: Seq<Seq<u8>>,
        new_prefix: u64,
    )
        requires
            create_pre(pre, path, new_prefix),
            prefix_uniqueness(pre),
            prefix_allocation_complete(pre),
        ensures ({
            let post = create_post(pre, path, new_prefix);
            prefix_uniqueness(post) && prefix_allocation_complete(post)
        })
    {
        let post = create_post(pre, path, new_prefix);
        // New prefix maps to new path
        // Old prefixes still map to old paths
    }

    /// Proof: Create preserves hierarchy consistency
    pub proof fn create_preserves_hierarchy(
        pre: DirectoryState,
        path: Seq<Seq<u8>>,
        new_prefix: u64,
    )
        requires
            create_pre(pre, path, new_prefix),
            hierarchy_consistency(pre),
        ensures hierarchy_consistency(create_post(pre, path, new_prefix))
    {
        let post = create_post(pre, path, new_prefix);
        // create_pre requires parent exists
        // All other paths' parents still exist
    }

    /// Proof: Create preserves full invariant
    pub proof fn create_preserves_invariant(
        pre: DirectoryState,
        path: Seq<Seq<u8>>,
        new_prefix: u64,
    )
        requires
            create_pre(pre, path, new_prefix),
            directory_invariant(pre),
        ensures directory_invariant(create_post(pre, path, new_prefix))
    {
        create_preserves_isolation(pre, path, new_prefix);
        create_preserves_uniqueness(pre, path, new_prefix);
        create_preserves_hierarchy(pre, path, new_prefix);
    }

    // ========================================================================
    // Open Operation
    // ========================================================================

    /// Precondition for opening a directory (either exists or can be created)
    pub open spec fn open_pre(state: DirectoryState, path: Seq<Seq<u8>>) -> bool {
        // Either:
        // 1. Directory already exists (open existing), OR
        // 2. Directory doesn't exist but parent does (create new)
        state.directories.contains_key(path) ||
        (path.len() == 0 || state.directories.contains_key(path.take(path.len() - 1)))
    }

    /// Precondition for opening an existing directory (must exist)
    pub open spec fn open_existing_pre(state: DirectoryState, path: Seq<Seq<u8>>) -> bool {
        state.directories.contains_key(path)
    }

    /// Precondition for creating a new directory via open (doesn't exist, parent does)
    pub open spec fn open_create_pre(state: DirectoryState, path: Seq<Seq<u8>>) -> bool {
        !state.directories.contains_key(path) &&
        (path.len() == 0 || state.directories.contains_key(path.take(path.len() - 1)))
    }

    /// Check if open would create (directory doesn't exist)
    pub open spec fn open_creates(state: DirectoryState, path: Seq<Seq<u8>>) -> bool {
        !state.directories.contains_key(path)
    }

    /// Result of opening an existing directory (no state change)
    pub open spec fn open_existing_post(pre: DirectoryState, path: Seq<Seq<u8>>) -> DirectoryState {
        pre // No change
    }

    // ========================================================================
    // Remove Operation
    // ========================================================================

    /// Precondition for removing a directory
    pub open spec fn remove_pre(state: DirectoryState, path: Seq<Seq<u8>>) -> bool {
        // Directory must exist
        state.directories.contains_key(path) &&
        // Must not have children
        !has_children(state, path)
    }

    /// Check if directory has children
    pub open spec fn has_children(state: DirectoryState, parent_path: Seq<Seq<u8>>) -> bool {
        exists |child_path: Seq<Seq<u8>>|
            state.directories.contains_key(child_path) &&
            child_path.len() == parent_path.len() + 1 &&
            child_path.take(parent_path.len() as int) =~= parent_path
    }

    /// Result of removing a directory
    pub open spec fn remove_post(pre: DirectoryState, path: Seq<Seq<u8>>) -> DirectoryState {
        let entry = pre.directories[path];
        DirectoryState {
            directories: pre.directories.remove(path),
            allocated_prefixes: pre.allocated_prefixes.remove(entry.prefix),
            prefix_to_path: pre.prefix_to_path.remove(entry.prefix),
        }
    }

    /// Proof: Remove preserves namespace isolation
    pub proof fn remove_preserves_isolation(pre: DirectoryState, path: Seq<Seq<u8>>)
        requires
            remove_pre(pre, path),
            namespace_isolation(pre),
        ensures namespace_isolation(remove_post(pre, path))
    {
        let post = remove_post(pre, path);
        // Removing a directory only removes one entry
        // Remaining entries maintain isolation
    }

    /// Proof: Remove preserves prefix uniqueness
    pub proof fn remove_preserves_uniqueness(pre: DirectoryState, path: Seq<Seq<u8>>)
        requires
            remove_pre(pre, path),
            prefix_uniqueness(pre),
            prefix_allocation_complete(pre),
        ensures ({
            let post = remove_post(pre, path);
            prefix_uniqueness(post) && prefix_allocation_complete(post)
        })
    {
        // Prefix and path are removed together
        // Remaining mappings stay consistent
    }

    /// Proof: Remove preserves hierarchy consistency
    pub proof fn remove_preserves_hierarchy(pre: DirectoryState, path: Seq<Seq<u8>>)
        requires
            remove_pre(pre, path),
            hierarchy_consistency(pre),
        ensures hierarchy_consistency(remove_post(pre, path))
    {
        let post = remove_post(pre, path);
        // remove_pre ensures no children
        // Remaining directories still have their parents
        // (removing path doesn't affect other parent relationships)
    }

    /// Proof: Remove preserves full invariant
    pub proof fn remove_preserves_invariant(pre: DirectoryState, path: Seq<Seq<u8>>)
        requires
            remove_pre(pre, path),
            directory_invariant(pre),
        ensures directory_invariant(remove_post(pre, path))
    {
        remove_preserves_isolation(pre, path);
        remove_preserves_uniqueness(pre, path);
        remove_preserves_hierarchy(pre, path);
    }

    // ========================================================================
    // List Operation
    // ========================================================================

    /// List direct children of a directory
    pub open spec fn list_children(state: DirectoryState, parent_path: Seq<Seq<u8>>) -> Set<Seq<Seq<u8>>> {
        Set::new(|child_path: Seq<Seq<u8>>|
            state.directories.contains_key(child_path) &&
            child_path.len() == parent_path.len() + 1 &&
            child_path.take(parent_path.len() as int) =~= parent_path
        )
    }

    /// Proof: Listed children are all directories
    pub proof fn list_returns_directories(
        state: DirectoryState,
        parent_path: Seq<Seq<u8>>,
        child_path: Seq<Seq<u8>>,
    )
        requires list_children(state, parent_path).contains(child_path)
        ensures state.directories.contains_key(child_path)
    {
        // Directly from list_children definition
    }

    // ========================================================================
    // Move Operation
    // ========================================================================

    /// Precondition for moving a directory
    pub open spec fn move_pre(
        state: DirectoryState,
        old_path: Seq<Seq<u8>>,
        new_path: Seq<Seq<u8>>,
    ) -> bool {
        // Source must exist
        state.directories.contains_key(old_path) &&
        // Destination must not exist
        !state.directories.contains_key(new_path) &&
        // New parent must exist
        (new_path.len() == 0 || state.directories.contains_key(new_path.take(new_path.len() - 1))) &&
        // Can't move a directory into itself
        !(new_path.len() > old_path.len() && new_path.take(old_path.len() as int) =~= old_path)
    }

    /// Result of moving a directory (keeps same prefix)
    pub open spec fn move_post(
        pre: DirectoryState,
        old_path: Seq<Seq<u8>>,
        new_path: Seq<Seq<u8>>,
    ) -> DirectoryState {
        let entry = pre.directories[old_path];
        let updated_entry = DirectoryEntrySpec {
            path: PathSpec { components: new_path },
            prefix: entry.prefix, // Keep same prefix
            layer: entry.layer,
        };
        DirectoryState {
            directories: pre.directories.remove(old_path).insert(new_path, updated_entry),
            allocated_prefixes: pre.allocated_prefixes, // Unchanged
            prefix_to_path: pre.prefix_to_path.insert(entry.prefix, new_path),
        }
    }

    /// Proof: Move preserves namespace isolation
    pub proof fn move_preserves_isolation(
        pre: DirectoryState,
        old_path: Seq<Seq<u8>>,
        new_path: Seq<Seq<u8>>,
    )
        requires
            move_pre(pre, old_path, new_path),
            namespace_isolation(pre),
        ensures namespace_isolation(move_post(pre, old_path, new_path))
    {
        // Same prefix, just different path
        // Prefix uniqueness still holds
    }

    // ========================================================================
    // Subspace Properties
    // ========================================================================

    /// Check if a prefix is valid for subspace allocation
    ///
    /// The prefix u64::MAX cannot be used because it would create an empty
    /// subspace range [MAX, MAX). The allocator should never allocate this value.
    pub open spec fn valid_prefix(prefix: u64) -> bool {
        prefix < u64::MAX
    }

    /// A directory's subspace key range
    ///
    /// In FDB's directory layer, each directory gets a unique prefix that defines
    /// its key range. Keys in this directory start with the prefix, and the range
    /// extends to (but excludes) the next prefix value.
    ///
    /// The actual key encoding uses a variable-length tuple encoding where the
    /// prefix is prepended. This means all keys in a directory share the same
    /// prefix bytes, creating a contiguous range in the key space.
    ///
    /// Returns (start, end) where start is inclusive and end is exclusive.
    ///
    /// IMPORTANT: If prefix == u64::MAX, the range [MAX, MAX) is empty, meaning
    /// no keys can be stored in this directory. This is a degenerate case that
    /// should be prevented by the allocator (see `valid_prefix`).
    pub open spec fn directory_subspace(entry: DirectoryEntrySpec) -> (u64, u64) {
        // The subspace spans from prefix to the next prefix value
        let start = entry.prefix;
        let end = if entry.prefix == u64::MAX {
            // Edge case: prefix == MAX creates empty range [MAX, MAX)
            // This prefix should never be allocated; callers should check valid_prefix
            u64::MAX
        } else {
            (entry.prefix + 1) as u64
        };
        (start, end)
    }

    /// Check if a subspace has valid (non-empty) key range
    pub open spec fn subspace_non_empty(entry: DirectoryEntrySpec) -> bool {
        let (start, end) = directory_subspace(entry);
        start < end
    }

    /// Proof: Valid prefix implies non-empty subspace
    pub proof fn valid_prefix_implies_non_empty(entry: DirectoryEntrySpec)
        requires valid_prefix(entry.prefix)
        ensures subspace_non_empty(entry)
    {
        // If prefix < u64::MAX, then end = prefix + 1 > prefix = start
        let (start, end) = directory_subspace(entry);
        assert(entry.prefix < u64::MAX);
        assert(end == entry.prefix + 1);
        assert(start == entry.prefix);
        assert(start < end);
    }

    /// Two directories have non-overlapping key ranges
    pub open spec fn subspaces_disjoint(e1: DirectoryEntrySpec, e2: DirectoryEntrySpec) -> bool {
        let (start1, end1) = directory_subspace(e1);
        let (start2, end2) = directory_subspace(e2);
        // Non-overlapping if one ends before other starts
        end1 <= start2 || end2 <= start1
    }

    /// Proof: Namespace isolation implies disjoint subspaces
    pub proof fn isolation_implies_disjoint_subspaces(
        state: DirectoryState,
        p1: Seq<Seq<u8>>,
        p2: Seq<Seq<u8>>,
    )
        requires
            namespace_isolation(state),
            state.directories.contains_key(p1),
            state.directories.contains_key(p2),
            !(p1 =~= p2),
        ensures subspaces_disjoint(state.directories[p1], state.directories[p2])
    {
        // Different prefixes => disjoint key ranges
        let e1 = state.directories[p1];
        let e2 = state.directories[p2];
        assert(e1.prefix != e2.prefix);
        // With different prefixes, the ranges [prefix, prefix+1) don't overlap
    }
}
