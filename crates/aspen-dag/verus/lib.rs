//! Verus formal specifications for aspen-dag.
//!
//! # Invariants Verified
//!
//! ## DAG Traversal
//!
//! 1. **TRAV-1: Visit Uniqueness**: Each node is yielded at most once per traversal
//! 2. **TRAV-2: Depth Boundedness**: Traversal depth never exceeds MAX_DAG_TRAVERSAL_DEPTH
//! 3. **TRAV-3: Count Boundedness**: Traversal count never exceeds MAX_TRAVERSAL_NODES
//! 4. **TRAV-4: Visited Set Monotonicity**: The visited set only grows, never shrinks
//! 5. **TRAV-5: Known Head Termination**: Traversal never descends below known heads
//! 6. **TRAV-6: Determinism**: Same root + same DAG + same config = same sequence

mod traversal_spec;
