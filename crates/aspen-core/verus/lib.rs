//! Verus Formal Specifications for Aspen Core
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of core primitives in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-core          # Verify all specs
//! nix run .#verify-verus-core -- quick # Syntax check only
//! nix run .#verify-verus               # Verify all (Core + Raft + Coordination)
//! ```
//!
//! # Module Overview
//!
//! ## Hybrid Logical Clock (HLC)
//! - `hlc_spec`: HLC state model and invariants for distributed timestamps
//!
//! ## High-Contention Allocator
//! - `allocator_spec`: HCA state model and uniqueness guarantees
//!
//! ## Tuple Layer
//! - `tuple_spec`: Tuple encoding invariants and ordering proofs
//!
//! ## Directory Layer
//! - `directory_state_spec`: Directory state model
//! - `directory_ops_spec`: Directory operation correctness
//!
//! ## Index Layer
//! - `index_spec`: Secondary index consistency invariants
//!
//! # Invariants Verified
//!
//! ## Hybrid Logical Clock
//!
//! 1. **HLC-1: Monotonicity**: Timestamps strictly increase within a node
//!    - Every new_timestamp() call returns a value greater than the previous
//!    - The logical counter increments when wall clock doesn't advance
//!
//! 2. **HLC-2: Causality**: Received timestamps advance the local clock
//!    - update_from_timestamp() ensures local clock >= received
//!    - Maintains happened-before relationships across nodes
//!
//! 3. **HLC-3: Total Order**: Any two timestamps are comparable
//!    - Ordering is defined by (wall_time, logical_counter, node_id)
//!    - No two different timestamps compare equal
//!
//! 4. **HLC-4: Bounded Drift**: Logical counter is bounded
//!    - Counter resets when wall clock advances
//!    - Prevents unbounded growth in logical component
//!
//! ## High-Contention Allocator
//!
//! 1. **ALLOC-1: Uniqueness**: No prefix allocated twice
//!    - Each successful allocate() returns a globally unique value
//!    - Proved via candidate claim atomicity (CAS semantics)
//!
//! 2. **ALLOC-2: Monotonicity**: window_start only increases
//!    - Window advances forward, never backward
//!    - Ensures progress under contention
//!
//! 3. **ALLOC-3: Counter Bounded**: counter <= window_start + window_size
//!    - Counter tracks maximum allocated value
//!    - Window size grows with allocations
//!
//! ## Tuple Layer
//!
//! 1. **TUPLE-1: Order Preservation**: Encoded bytes preserve tuple ordering
//!    - Lexicographic order of packed tuples matches semantic order
//!    - Critical for range scans and prefix matching
//!
//! 2. **TUPLE-2: Roundtrip Correctness**: decode(encode(t)) == t
//!    - No information lost during serialization
//!    - Every valid tuple survives pack/unpack cycle
//!
//! 3. **TUPLE-3: Prefix Property**: Tuple prefixes encode to byte prefixes
//!    - prefix(pack(t)) == pack(prefix(t))
//!    - Enables efficient key range operations
//!
//! 4. **TUPLE-4: Null Escaping**: Null bytes properly escaped
//!    - Embedded nulls don't corrupt encoding
//!    - Maintains unambiguous decoding
//!
//! ## Directory Layer
//!
//! 1. **DIR-1: Namespace Isolation**: Different paths have disjoint key ranges
//!    - No key collision between directories
//!    - Prefix allocation ensures separation
//!
//! 2. **DIR-2: Prefix Uniqueness**: Each prefix maps to exactly one directory
//!    - Bijection between prefixes and directory paths
//!    - HCA guarantees allocation uniqueness
//!
//! 3. **DIR-3: Hierarchy Consistency**: Child directories only exist if parent exists
//!    - No orphan directories
//!    - Parent creation is prerequisite
//!
//! ## Index Layer
//!
//! 1. **IDX-1: Consistency**: Index entries match primary data versions
//!    - Index value includes correct version from primary
//!    - Stale reads return matching versions
//!
//! 2. **IDX-2: Atomic Update**: Index updates atomic with primary updates
//!    - Transaction includes both primary and index writes
//!    - No partial updates visible
//!
//! 3. **IDX-3: No Stale Entries**: No references to non-existent primaries
//!    - Delete cascades to index entries
//!    - Or soft-delete with version tracking
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - System clock advances monotonically (standard OS assumption)
//! - CAS operations are linearizable (provided by Raft consensus)
//! - blake3 hash produces uniformly distributed outputs

use vstd::prelude::*;

verus! {
    // Re-export HLC specifications
    pub use hlc_spec::HlcTimestampSpec;
    pub use hlc_spec::HlcState;
    pub use hlc_spec::hlc_monotonicity;
    pub use hlc_spec::hlc_causality;
    pub use hlc_spec::hlc_total_order;
    pub use hlc_spec::hlc_bounded_drift;
    pub use hlc_spec::hlc_invariant;

    // Re-export Allocator specifications
    pub use allocator_spec::HcaState;
    pub use allocator_spec::alloc_uniqueness;
    pub use allocator_spec::alloc_monotonicity;
    pub use allocator_spec::alloc_counter_bounded;
    pub use allocator_spec::allocator_invariant;

    // Re-export Tuple specifications
    pub use tuple_spec::TupleSpec;
    pub use tuple_spec::tuple_order_preservation;
    pub use tuple_spec::tuple_roundtrip;
    pub use tuple_spec::tuple_prefix_property;
    pub use tuple_spec::tuple_invariant;

    // Re-export Directory specifications
    pub use directory_state_spec::DirectoryState;
    pub use directory_state_spec::DirectoryEntrySpec;
    pub use directory_state_spec::namespace_isolation;
    pub use directory_state_spec::prefix_uniqueness;
    pub use directory_state_spec::hierarchy_consistency;
    pub use directory_state_spec::directory_invariant;

    pub use directory_ops_spec::create_pre;
    pub use directory_ops_spec::create_post;
    pub use directory_ops_spec::open_pre;
    pub use directory_ops_spec::remove_pre;
    pub use directory_ops_spec::remove_post;

    // Re-export Index specifications
    pub use index_spec::IndexState;
    pub use index_spec::IndexEntrySpec;
    pub use index_spec::index_consistency;
    pub use index_spec::index_atomic_update;
    pub use index_spec::index_no_stale_entries;
    pub use index_spec::index_invariant;
}

mod allocator_spec;
mod directory_ops_spec;
mod directory_state_spec;
mod hlc_spec;
mod index_spec;
mod tuple_spec;
