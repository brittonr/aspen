//! Directory Layer State Model
//!
//! Abstract state model for formal verification of directory layer operations.
//!
//! # State Model
//!
//! The `DirectoryState` captures:
//! - Mapping of paths to directory entries
//! - Prefix allocation state
//!
//! # Key Invariants
//!
//! 1. **DIR-1: Namespace Isolation**: Different paths have disjoint key ranges
//! 2. **DIR-2: Prefix Uniqueness**: Each prefix maps to exactly one directory
//! 3. **DIR-3: Hierarchy Consistency**: Child directories only exist if parent exists
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-core/verus/directory_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// A directory path is a sequence of path components
    pub struct PathSpec {
        /// Path components (e.g., ["app", "users", "data"])
        pub components: Seq<Seq<u8>>,
    }

    /// Directory entry storing allocated prefix
    pub struct DirectoryEntrySpec {
        /// Path to this directory
        pub path: PathSpec,
        /// Unique prefix allocated by HCA
        pub prefix: u64,
        /// Layer identifier (optional)
        pub layer: Option<Seq<u8>>,
    }

    /// Complete directory state for verification
    pub struct DirectoryState {
        /// All registered directories (path -> entry)
        pub directories: Map<Seq<Seq<u8>>, DirectoryEntrySpec>,
        /// Set of all allocated prefixes
        pub allocated_prefixes: Set<u64>,
        /// Reverse mapping (prefix -> path) for uniqueness checking
        pub prefix_to_path: Map<u64, Seq<Seq<u8>>>,
    }

    // ========================================================================
    // Path Operations
    // ========================================================================

    /// Check if path a is a prefix of path b
    pub open spec fn is_path_prefix(a: PathSpec, b: PathSpec) -> bool {
        a.components.len() <= b.components.len() &&
        forall |i: int| 0 <= i < a.components.len() ==>
            a.components[i] =~= b.components[i]
    }

    /// Check if path a is a strict prefix of path b (not equal)
    pub open spec fn is_strict_path_prefix(a: PathSpec, b: PathSpec) -> bool {
        is_path_prefix(a, b) && a.components.len() < b.components.len()
    }

    /// Get parent path (remove last component)
    pub open spec fn parent_path(p: PathSpec) -> PathSpec {
        PathSpec { components: p.components.take(p.components.len() - 1) }
    }

    /// Check if two paths are equal
    pub open spec fn paths_equal(a: PathSpec, b: PathSpec) -> bool {
        a.components =~= b.components
    }

    /// Root path (empty)
    pub open spec fn root_path() -> PathSpec {
        PathSpec { components: Seq::empty() }
    }

    // ========================================================================
    // Invariant 1: Namespace Isolation
    // ========================================================================

    /// DIR-1: Different paths have disjoint key ranges
    ///
    /// Two directories with different paths have different prefixes,
    /// ensuring their key spaces don't overlap.
    pub open spec fn namespace_isolation(state: DirectoryState) -> bool {
        forall |p1: Seq<Seq<u8>>, p2: Seq<Seq<u8>>|
            state.directories.contains_key(p1) &&
            state.directories.contains_key(p2) &&
            !(p1 =~= p2) ==>
            state.directories[p1].prefix != state.directories[p2].prefix
    }

    /// Key ranges derived from prefixes don't overlap
    pub open spec fn key_ranges_disjoint(prefix1: u64, prefix2: u64) -> bool {
        prefix1 != prefix2
        // In the actual encoding, different prefixes produce
        // non-overlapping key ranges
    }

    // ========================================================================
    // Invariant 2: Prefix Uniqueness
    // ========================================================================

    /// DIR-2: Each prefix maps to exactly one directory
    ///
    /// The prefix_to_path mapping is the inverse of directories[path].prefix
    pub open spec fn prefix_uniqueness(state: DirectoryState) -> bool {
        // Every prefix in allocated_prefixes maps to exactly one path
        forall |prefix: u64| state.allocated_prefixes.contains(prefix) ==>
            state.prefix_to_path.contains_key(prefix) &&
            state.directories.contains_key(state.prefix_to_path[prefix]) &&
            state.directories[state.prefix_to_path[prefix]].prefix == prefix
    }

    /// Reverse direction: every directory's prefix is in allocated set
    pub open spec fn prefix_allocation_complete(state: DirectoryState) -> bool {
        forall |path: Seq<Seq<u8>>| state.directories.contains_key(path) ==>
            state.allocated_prefixes.contains(state.directories[path].prefix)
    }

    // ========================================================================
    // Invariant 3: Hierarchy Consistency
    // ========================================================================

    /// DIR-3: Child directories only exist if parent exists
    ///
    /// For any non-root directory, its parent must also exist.
    pub open spec fn hierarchy_consistency(state: DirectoryState) -> bool {
        forall |path: Seq<Seq<u8>>|
            state.directories.contains_key(path) &&
            path.len() > 0 ==>
            state.directories.contains_key(path.take(path.len() - 1))
    }

    /// Alternative: all ancestor paths must exist
    pub open spec fn all_ancestors_exist(state: DirectoryState, path: Seq<Seq<u8>>) -> bool
        decreases path.len()
    {
        if path.len() == 0 {
            true // Root always "exists" conceptually
        } else {
            let parent = path.take(path.len() - 1);
            state.directories.contains_key(parent) &&
            all_ancestors_exist(state, parent)
        }
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined directory invariant
    pub open spec fn directory_invariant(state: DirectoryState) -> bool {
        namespace_isolation(state) &&
        prefix_uniqueness(state) &&
        prefix_allocation_complete(state) &&
        hierarchy_consistency(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial directory state (empty)
    pub open spec fn initial_directory_state() -> DirectoryState {
        DirectoryState {
            directories: Map::empty(),
            allocated_prefixes: Set::empty(),
            prefix_to_path: Map::empty(),
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant()
        ensures directory_invariant(initial_directory_state())
    {
        // Empty maps trivially satisfy all forall properties
    }

    // ========================================================================
    // Helper Predicates
    // ========================================================================

    /// Check if a directory exists
    pub open spec fn directory_exists(state: DirectoryState, path: Seq<Seq<u8>>) -> bool {
        state.directories.contains_key(path)
    }

    /// Check if a prefix is already allocated
    pub open spec fn prefix_allocated(state: DirectoryState, prefix: u64) -> bool {
        state.allocated_prefixes.contains(prefix)
    }

    /// Get directory entry by path
    pub open spec fn get_directory(state: DirectoryState, path: Seq<Seq<u8>>) -> Option<DirectoryEntrySpec> {
        if state.directories.contains_key(path) {
            Some(state.directories[path])
        } else {
            None
        }
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Proof: Namespace isolation implies no prefix collision
    pub proof fn isolation_implies_no_collision(
        state: DirectoryState,
        p1: Seq<Seq<u8>>,
        p2: Seq<Seq<u8>>,
    )
        requires
            namespace_isolation(state),
            state.directories.contains_key(p1),
            state.directories.contains_key(p2),
            !(p1 =~= p2),
        ensures state.directories[p1].prefix != state.directories[p2].prefix
    {
        // Directly from namespace_isolation definition
    }

    /// Proof: Prefix uniqueness implies bidirectional mapping
    pub proof fn uniqueness_implies_bijection(state: DirectoryState, prefix: u64)
        requires
            prefix_uniqueness(state),
            state.allocated_prefixes.contains(prefix),
        ensures ({
            let path = state.prefix_to_path[prefix];
            state.directories.contains_key(path) &&
            state.directories[path].prefix == prefix
        })
    {
        // Directly from prefix_uniqueness definition
    }

    /// Proof: Hierarchy consistency implies parent exists for any child
    pub proof fn hierarchy_parent_exists(
        state: DirectoryState,
        child_path: Seq<Seq<u8>>,
    )
        requires
            hierarchy_consistency(state),
            state.directories.contains_key(child_path),
            child_path.len() > 0,
        ensures state.directories.contains_key(child_path.take(child_path.len() - 1))
    {
        // Directly from hierarchy_consistency definition
    }

    /// Proof: Creating directory with fresh prefix preserves isolation
    pub proof fn create_preserves_isolation(
        pre: DirectoryState,
        path: Seq<Seq<u8>>,
        new_prefix: u64,
    )
        requires
            namespace_isolation(pre),
            !pre.allocated_prefixes.contains(new_prefix),
            !pre.directories.contains_key(path),
        ensures ({
            let new_entry = DirectoryEntrySpec {
                path: PathSpec { components: path },
                prefix: new_prefix,
                layer: None,
            };
            let post = DirectoryState {
                directories: pre.directories.insert(path, new_entry),
                allocated_prefixes: pre.allocated_prefixes.insert(new_prefix),
                prefix_to_path: pre.prefix_to_path.insert(new_prefix, path),
            };
            namespace_isolation(post)
        })
    {
        // New prefix is fresh, so no collision with existing directories
        // Existing directories maintain their isolation property
    }
}
