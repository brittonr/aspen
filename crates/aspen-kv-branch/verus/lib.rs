//! Verus formal specifications for KV branch overlay.
//!
//! # Invariants Verified
//!
//! ## Scan Merge
//!
//! 1. **MERGE-1: Tombstone Exclusion**: No key in the output has a corresponding tombstone in the
//!    branch dirty map.
//! 2. **MERGE-2: Sort Order**: Output is strictly lexicographically sorted — for all adjacent pairs
//!    (i, i+1), output[i].key < output[i+1].key.
//! 3. **MERGE-3: Branch Precedence**: If a key exists in both the branch dirty writes and the
//!    parent scan, the output contains the branch value.

mod scan_merge_spec;
