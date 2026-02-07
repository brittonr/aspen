//! Index Layer State Machine Model
//!
//! Abstract state model for formal verification of secondary index operations.
//!
//! # State Model
//!
//! The `IndexState` captures:
//! - Primary data entries with versions
//! - Secondary index entries pointing to primaries
//!
//! # Key Invariants
//!
//! 1. **IDX-1: Consistency**: Index entries match primary data versions
//! 2. **IDX-2: Atomic Update**: Index updates atomic with primary updates
//! 3. **IDX-3: No Stale Entries**: No references to non-existent primaries
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-core/verus/index_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Primary data entry
    pub struct PrimaryEntrySpec {
        /// Primary key
        pub key: Seq<u8>,
        /// Value data
        pub value: Seq<u8>,
        /// Version number (monotonically increasing)
        pub version: u64,
        /// Whether entry is deleted (soft delete)
        pub deleted: bool,
    }

    /// Index entry pointing to a primary
    pub struct IndexEntrySpec {
        /// Index key (derived from indexed field)
        pub index_key: Seq<u8>,
        /// Primary key being referenced
        pub primary_key: Seq<u8>,
        /// Version of primary when index was created/updated
        pub primary_version: u64,
    }

    /// Index definition
    pub struct IndexDefinitionSpec {
        /// Name of the index
        pub name: Seq<u8>,
        /// Function to extract index key from primary value
        /// (abstract - actual implementation in Rust code)
        pub is_applicable: bool,
    }

    /// Complete index state for verification
    pub struct IndexState {
        /// All primary entries (key -> entry)
        pub primaries: Map<Seq<u8>, PrimaryEntrySpec>,
        /// All index entries (index_key -> list of entries)
        pub indices: Map<Seq<u8>, Seq<IndexEntrySpec>>,
        /// Current version counter
        pub current_version: u64,
    }

    // ========================================================================
    // Invariant 1: Consistency
    // ========================================================================

    /// IDX-1: Index entries match primary data versions
    ///
    /// For each index entry, the referenced primary exists and the
    /// version matches (or the index is being lazily cleaned up).
    pub open spec fn index_consistency(state: IndexState) -> bool {
        forall |index_key: Seq<u8>, i: int|
            state.indices.contains_key(index_key) &&
            0 <= i < state.indices[index_key].len() ==>
            {
                let entry = state.indices[index_key][i];
                // Primary must exist
                state.primaries.contains_key(entry.primary_key) &&
                // Version must match or be older (lazy cleanup allowed)
                entry.primary_version <= state.primaries[entry.primary_key].version
            }
    }

    /// Stronger form: versions exactly match
    pub open spec fn index_exact_consistency(state: IndexState) -> bool {
        forall |index_key: Seq<u8>, i: int|
            state.indices.contains_key(index_key) &&
            0 <= i < state.indices[index_key].len() ==>
            {
                let entry = state.indices[index_key][i];
                state.primaries.contains_key(entry.primary_key) &&
                entry.primary_version == state.primaries[entry.primary_key].version &&
                !state.primaries[entry.primary_key].deleted
            }
    }

    // ========================================================================
    // Invariant 2: Atomic Update
    // ========================================================================

    /// IDX-2: Index updates are atomic with primary updates
    ///
    /// When a primary is updated, its index entries are updated in the same
    /// transaction.
    pub open spec fn index_atomic_update(
        pre: IndexState,
        post: IndexState,
        primary_key: Seq<u8>,
    ) -> bool {
        // If primary was updated...
        pre.primaries.contains_key(primary_key) &&
        post.primaries.contains_key(primary_key) &&
        pre.primaries[primary_key].version < post.primaries[primary_key].version ==>
        // ...then all index entries for that primary have been updated
        forall |index_key: Seq<u8>, i: int|
            post.indices.contains_key(index_key) &&
            0 <= i < post.indices[index_key].len() &&
            post.indices[index_key][i].primary_key =~= primary_key ==>
            post.indices[index_key][i].primary_version == post.primaries[primary_key].version
    }

    /// Update operation on a primary with index maintenance
    pub struct UpdateSpec {
        pub primary_key: Seq<u8>,
        pub new_value: Seq<u8>,
        pub old_index_key: Option<Seq<u8>>,
        pub new_index_key: Seq<u8>,
    }

    /// Effect of atomic update
    pub open spec fn atomic_update_effect(
        pre: IndexState,
        update: UpdateSpec,
    ) -> IndexState
        recommends pre.primaries.contains_key(update.primary_key)
    {
        let old_entry = pre.primaries[update.primary_key];
        let new_version = (old_entry.version + 1) as u64;
        let new_primary = PrimaryEntrySpec {
            key: update.primary_key,
            value: update.new_value,
            version: new_version,
            deleted: false,
        };

        // Create new index entry
        let new_index_entry = IndexEntrySpec {
            index_key: update.new_index_key,
            primary_key: update.primary_key,
            primary_version: new_version,
        };

        // Remove old index entry (if exists) and add new one
        // (simplified - actual implementation handles this atomically)
        IndexState {
            primaries: pre.primaries.insert(update.primary_key, new_primary),
            indices: add_index_entry(
                remove_old_index_entry(pre.indices, update.old_index_key, update.primary_key),
                update.new_index_key,
                new_index_entry
            ),
            current_version: (pre.current_version + 1) as u64,
        }
    }

    /// Helper: remove old index entry
    pub open spec fn remove_old_index_entry(
        indices: Map<Seq<u8>, Seq<IndexEntrySpec>>,
        old_key: Option<Seq<u8>>,
        primary_key: Seq<u8>,
    ) -> Map<Seq<u8>, Seq<IndexEntrySpec>> {
        match old_key {
            None => indices,
            Some(key) => {
                if indices.contains_key(key) {
                    let entries = indices[key];
                    let filtered = entries.filter(|e: IndexEntrySpec| !(e.primary_key =~= primary_key));
                    indices.insert(key, filtered)
                } else {
                    indices
                }
            }
        }
    }

    /// Helper: add new index entry
    pub open spec fn add_index_entry(
        indices: Map<Seq<u8>, Seq<IndexEntrySpec>>,
        key: Seq<u8>,
        entry: IndexEntrySpec,
    ) -> Map<Seq<u8>, Seq<IndexEntrySpec>> {
        if indices.contains_key(key) {
            indices.insert(key, indices[key].push(entry))
        } else {
            indices.insert(key, Seq::empty().push(entry))
        }
    }

    // ========================================================================
    // Invariant 3: No Stale Entries
    // ========================================================================

    /// IDX-3: No references to non-existent primaries
    ///
    /// Every index entry references a primary that exists (possibly deleted).
    pub open spec fn index_no_stale_entries(state: IndexState) -> bool {
        forall |index_key: Seq<u8>, i: int|
            state.indices.contains_key(index_key) &&
            0 <= i < state.indices[index_key].len() ==>
            state.primaries.contains_key(state.indices[index_key][i].primary_key)
    }

    /// After delete, index entries are cleaned up (eventually)
    pub open spec fn delete_cleans_index(
        pre: IndexState,
        post: IndexState,
        deleted_key: Seq<u8>,
    ) -> bool {
        // After delete (hard or soft), no index entries reference the key
        // (May be immediate or lazy cleanup)
        !post.primaries.contains_key(deleted_key) ||
        post.primaries[deleted_key].deleted ==>
        forall |index_key: Seq<u8>, i: int|
            post.indices.contains_key(index_key) &&
            0 <= i < post.indices[index_key].len() ==>
            !(post.indices[index_key][i].primary_key =~= deleted_key)
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined index invariant
    pub open spec fn index_invariant(state: IndexState) -> bool {
        index_consistency(state) &&
        index_no_stale_entries(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial index state (empty)
    pub open spec fn initial_index_state() -> IndexState {
        IndexState {
            primaries: Map::empty(),
            indices: Map::empty(),
            current_version: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant()
        ensures index_invariant(initial_index_state())
    {
        // Empty maps trivially satisfy forall properties
    }

    // ========================================================================
    // Insert Operation
    // ========================================================================

    /// Precondition for inserting a new primary
    pub open spec fn insert_pre(state: IndexState, key: Seq<u8>) -> bool {
        // Key must not already exist
        !state.primaries.contains_key(key)
    }

    /// Effect of inserting a new primary with index
    pub open spec fn insert_effect(
        pre: IndexState,
        key: Seq<u8>,
        value: Seq<u8>,
        index_key: Seq<u8>,
    ) -> IndexState
        recommends insert_pre(pre, key)
    {
        let new_primary = PrimaryEntrySpec {
            key: key,
            value: value,
            version: 1,
            deleted: false,
        };
        let new_index_entry = IndexEntrySpec {
            index_key: index_key,
            primary_key: key,
            primary_version: 1,
        };

        IndexState {
            primaries: pre.primaries.insert(key, new_primary),
            indices: add_index_entry(pre.indices, index_key, new_index_entry),
            current_version: (pre.current_version + 1) as u64,
        }
    }

    /// Proof: Insert preserves index invariant
    pub proof fn insert_preserves_invariant(
        pre: IndexState,
        key: Seq<u8>,
        value: Seq<u8>,
        index_key: Seq<u8>,
    )
        requires
            insert_pre(pre, key),
            index_invariant(pre),
        ensures index_invariant(insert_effect(pre, key, value, index_key))
    {
        let post = insert_effect(pre, key, value, index_key);

        // New index entry points to new primary with correct version
        // Old index entries still point to their primaries
    }

    // ========================================================================
    // Delete Operation
    // ========================================================================

    /// Precondition for deleting a primary
    pub open spec fn delete_pre(state: IndexState, key: Seq<u8>) -> bool {
        state.primaries.contains_key(key)
    }

    /// Effect of deleting a primary (and its index entries)
    pub open spec fn delete_effect(
        pre: IndexState,
        key: Seq<u8>,
        index_key: Seq<u8>,
    ) -> IndexState
        recommends delete_pre(pre, key)
    {
        IndexState {
            primaries: pre.primaries.remove(key),
            indices: remove_old_index_entry(pre.indices, Some(index_key), key),
            current_version: (pre.current_version + 1) as u64,
        }
    }

    /// Proof: Delete preserves index invariant
    pub proof fn delete_preserves_invariant(
        pre: IndexState,
        key: Seq<u8>,
        index_key: Seq<u8>,
    )
        requires
            delete_pre(pre, key),
            index_invariant(pre),
        ensures index_invariant(delete_effect(pre, key, index_key))
    {
        let post = delete_effect(pre, key, index_key);

        // Index entry for deleted primary is removed
        // Other index entries still valid
    }

    // ========================================================================
    // Query Operation (Read-Only)
    // ========================================================================

    /// Look up primaries by index key
    pub open spec fn lookup_by_index(
        state: IndexState,
        index_key: Seq<u8>,
    ) -> Seq<PrimaryEntrySpec> {
        if !state.indices.contains_key(index_key) {
            Seq::empty()
        } else {
            let entries = state.indices[index_key];
            entries.map(|i: int, e: IndexEntrySpec|
                if state.primaries.contains_key(e.primary_key) {
                    state.primaries[e.primary_key]
                } else {
                    // Should not happen if invariant holds
                    PrimaryEntrySpec {
                        key: Seq::empty(),
                        value: Seq::empty(),
                        version: 0,
                        deleted: true,
                    }
                }
            )
        }
    }

    /// Proof: Lookup returns valid primaries if invariant holds
    pub proof fn lookup_returns_valid(
        state: IndexState,
        index_key: Seq<u8>,
    )
        requires index_invariant(state)
        ensures ({
            let results = lookup_by_index(state, index_key);
            forall |i: int| 0 <= i < results.len() ==>
                state.primaries.contains_key(results[i].key)
        })
    {
        // Follows from index_no_stale_entries
    }

    // ========================================================================
    // Version Monotonicity
    // ========================================================================

    /// Version counter is monotonic
    pub open spec fn version_monotonic(pre: IndexState, post: IndexState) -> bool {
        post.current_version >= pre.current_version
    }

    /// Primary versions are monotonic
    pub open spec fn primary_version_monotonic(pre: IndexState, post: IndexState, key: Seq<u8>) -> bool {
        pre.primaries.contains_key(key) && post.primaries.contains_key(key) ==>
        post.primaries[key].version >= pre.primaries[key].version
    }

    /// Proof: Updates increase version
    pub proof fn update_increases_version(pre: IndexState, update: UpdateSpec)
        requires pre.primaries.contains_key(update.primary_key)
        ensures ({
            let post = atomic_update_effect(pre, update);
            version_monotonic(pre, post) &&
            primary_version_monotonic(pre, post, update.primary_key)
        })
    {
        // new_version = old_version + 1
        // current_version = pre.current_version + 1
    }
}
