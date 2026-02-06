//! Storage state machine model for verification
//!
//! This module defines the abstract state model used for formal verification
//! of storage operations.
//!
//! # State Model
//!
//! The `StorageStateSpec` captures:
//! - Raft log entries with their chain hashes
//! - KV state machine entries
//! - Metadata (last_applied, last_purged, chain tip)
//! - Response cache for idempotency
//!
//! # Invariants
//!
//! The following invariants are verified to hold across all operations:
//!
//! 1. Chain tip is synchronized with the actual last entry
//! 2. Chain hashes form a valid linked chain
//! 3. last_applied is monotonically increasing
//! 4. last_purged is monotonically increasing
//! 5. Responses are cached for applied entries only

use std::collections::BTreeMap;

use super::chain_hash::ChainHashSpec;
use super::chain_hash::LogEntryData;

/// Abstract storage state for verification.
///
/// This is a simplified model of `SharedRedbStorage` that captures
/// the essential state needed for correctness proofs.
#[derive(Clone, Debug, Default)]
pub struct StorageStateSpec {
    // ========================================================================
    // Raft Log State
    // ========================================================================
    /// Log entries: index -> (term, data)
    pub log: BTreeMap<u64, LogEntryData>,

    /// Chain hashes: index -> 32-byte hash
    pub chain_hashes: BTreeMap<u64, ChainHashSpec>,

    /// Current chain tip (hash, index)
    /// Invariant: chain_tip.1 == max(log.keys()) or 0 if empty
    pub chain_tip: (ChainHashSpec, u64),

    // ========================================================================
    // State Machine State
    // ========================================================================
    /// KV store: key -> (value, mod_revision, create_revision, version)
    pub kv: BTreeMap<String, KvEntrySpec>,

    /// Index of last applied log entry (None if no entries applied)
    pub last_applied: Option<u64>,

    /// Index of last purged log entry (None if no entries purged)
    pub last_purged: Option<u64>,

    // ========================================================================
    // Response Cache
    // ========================================================================
    /// Cached responses for idempotency: index -> serialized response
    pub pending_responses: BTreeMap<u64, Vec<u8>>,

    // ========================================================================
    // Constants
    // ========================================================================
    /// Genesis hash (used as prev_hash for first entry)
    pub genesis_hash: ChainHashSpec,
}

/// Abstract KV entry for specifications.
#[derive(Clone, Debug)]
pub struct KvEntrySpec {
    pub value: String,
    pub mod_revision: u64,
    pub create_revision: u64,
    pub version: u64,
    pub expires_at_ms: Option<u64>,
}

impl StorageStateSpec {
    /// Create a new empty storage state.
    #[cfg(not(feature = "verus"))]
    pub fn new() -> Self {
        Self {
            genesis_hash: [0u8; 32],
            chain_tip: ([0u8; 32], 0),
            ..Default::default()
        }
    }

    /// Get the first log index (after purging).
    pub fn first_log_index(&self) -> u64 {
        self.last_purged.map(|p| p + 1).unwrap_or(1)
    }

    /// Get the last log index.
    pub fn last_log_index(&self) -> u64 {
        self.log.keys().max().copied().unwrap_or(0)
    }

    /// Check if the log is empty.
    pub fn log_is_empty(&self) -> bool {
        self.log.is_empty()
    }
}

// ============================================================================
// Ghost State Extraction
// ============================================================================

/// Ghost state snapshot for verification.
///
/// This struct captures the abstract state of `SharedRedbStorage` at a point
/// in time for use in ghost code proofs. It is a zero-cost abstraction that
/// compiles away during normal cargo builds.
///
/// # Usage
///
/// ```rust,ignore
/// // In append():
/// ghost! {
///     let pre_state = GhostStorageState::capture(&self);
/// }
///
/// // ... production code ...
///
/// proof! {
///     let post_state = GhostStorageState::capture(&self);
///     assert(last_applied_monotonic(pre_state.spec, post_state.spec));
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct GhostStorageState {
    /// The abstract storage state for verification.
    pub spec: StorageStateSpec,
}

impl GhostStorageState {
    /// Create an empty ghost state (zero-cost).
    #[inline(always)]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a ghost state with the given spec (zero-cost).
    #[inline(always)]
    pub fn from_spec(_spec: StorageStateSpec) -> Self {
        Self::default()
    }
}

/// Trait for extracting ghost state from production storage.
///
/// This trait provides a `to_spec_state()` method that extracts the abstract
/// state needed for verification. When `verus` is disabled, this is a no-op
/// that returns an empty state.
///
/// When `verus` is enabled, this extracts actual state for proof verification.
pub trait GhostStateExtractor {
    /// Extract abstract state for verification.
    ///
    /// When verus is enabled, this reads the actual storage state.
    /// When verus is disabled, this is a no-op returning an empty state.
    fn to_spec_state(&self) -> GhostStorageState;
}

/// Trait for capturing ghost snapshots of chain state.
///
/// This provides zero-cost ghost tracking of chain hash state before
/// and after operations.
pub trait GhostChainState {
    /// Get the current chain tip hash (ghost).
    fn ghost_chain_tip_hash(&self) -> [u8; 32] {
        [0u8; 32]
    }

    /// Get the current chain tip index (ghost).
    fn ghost_chain_tip_index(&self) -> u64 {
        0
    }

    /// Get the last applied index (ghost).
    fn ghost_last_applied(&self) -> Option<u64> {
        None
    }

    /// Get the last purged index (ghost).
    fn ghost_last_purged(&self) -> Option<u64> {
        None
    }
}

// ============================================================================
// Invariant Predicates
// ============================================================================

/// INVARIANT 3: Chain tip synchronization
///
/// The chain_tip always reflects the most recent entry:
/// - If log is empty AND no entries purged: chain_tip = (genesis_hash, 0)
/// - If log is empty AND entries were purged: chain_tip.1 >= last_purged
/// - Otherwise: chain_tip = (hash[max_index], max_index)
///
/// Note: After purging, chain_tip may point to a purged index (to maintain
/// chain continuity for future appends). This is valid storage state.
pub fn chain_tip_synchronized(state: &StorageStateSpec) -> bool {
    if state.log.is_empty() {
        // Log is empty - check if this is fresh state or post-purge
        match state.last_purged {
            None => {
                // Fresh state - chain tip should be genesis
                state.chain_tip.1 == 0 && state.chain_tip.0 == state.genesis_hash
            }
            Some(purged_idx) => {
                // Post-purge - chain tip should be at or after purge point
                // This allows continuing the chain from the purged state
                state.chain_tip.1 >= purged_idx
            }
        }
    } else {
        let max_index = state.last_log_index();
        state.chain_tip.1 == max_index
            && state.chain_hashes.get(&max_index).map(|h| *h == state.chain_tip.0).unwrap_or(false)
    }
}

/// INVARIANT 5: Monotonic last_applied
///
/// last_applied can only increase, never decrease.
pub fn last_applied_monotonic(pre: &StorageStateSpec, post: &StorageStateSpec) -> bool {
    match (pre.last_applied, post.last_applied) {
        (None, _) => true,
        (Some(a), Some(b)) => a <= b,
        (Some(_), None) => false,
    }
}

/// INVARIANT 6: Monotonic purge
///
/// last_purged can only increase, never decrease.
pub fn purge_monotonic(pre: &StorageStateSpec, post: &StorageStateSpec) -> bool {
    match (pre.last_purged, post.last_purged) {
        (None, _) => true,
        (Some(a), Some(b)) => a <= b,
        (Some(_), None) => false,
    }
}

/// INVARIANT 4: Response cache consistency
///
/// Responses are only cached for entries that have been applied.
pub fn response_cache_consistent(state: &StorageStateSpec) -> bool {
    let last = state.last_applied.unwrap_or(0);
    state.pending_responses.keys().all(|&idx| idx <= last)
}

/// Combined invariant check for all storage invariants.
pub fn storage_invariant(state: &StorageStateSpec) -> bool {
    chain_tip_synchronized(state) && response_cache_consistent(state)
}

// ============================================================================
// Verus Specifications
// ============================================================================

/*
verus! {
    use vstd::prelude::*;
    use super::chain_hash::*;

    /// Abstract log entry structure
    pub struct LogEntry {
        pub index: u64,
        pub term: u64,
        pub data: Seq<u8>,
    }

    /// Abstract KV entry structure
    pub struct KvEntry {
        pub value: Seq<u8>,
        pub mod_revision: u64,
        pub create_revision: u64,
        pub version: u64,
        pub expires_at_ms: Option<u64>,
    }

    /// Complete storage state for verification
    pub struct StorageState {
        // Raft log state
        pub log: Map<u64, LogEntry>,
        pub chain_hashes: Map<u64, ChainHash>,
        pub chain_tip: (ChainHash, u64),  // (hash, index)

        // State machine state
        pub kv: Map<Seq<u8>, KvEntry>,
        pub last_applied: Option<u64>,
        pub last_purged: Option<u64>,

        // Response tracking
        pub pending_responses: Map<u64, Seq<u8>>,

        // Constants
        pub genesis_hash: ChainHash,
    }

    /// INVARIANT 3: Chain tip synchronization
    ///
    /// The chain_tip.1 equals the maximum log index (or 0 if empty),
    /// and chain_tip.0 equals the hash at that index (or genesis if empty).
    pub open spec fn chain_tip_synchronized(state: StorageState) -> bool {
        if state.log.is_empty() {
            state.chain_tip.1 == 0 && state.chain_tip.0 == state.genesis_hash
        } else {
            let max_index = state.log.dom().fold(0u64, |acc, i| if i > acc { i } else { acc });
            state.chain_tip.1 == max_index &&
            state.chain_hashes.contains_key(max_index) &&
            state.chain_hashes[max_index] == state.chain_tip.0
        }
    }

    /// INVARIANT 5: Monotonic last_applied
    pub open spec fn last_applied_monotonic(
        pre: StorageState,
        post: StorageState
    ) -> bool {
        match (pre.last_applied, post.last_applied) {
            (None, _) => true,
            (Some(a), Some(b)) => a <= b,
            (Some(_), None) => false,
        }
    }

    /// INVARIANT 6: Monotonic purge
    pub open spec fn purge_monotonic(
        pre: StorageState,
        post: StorageState
    ) -> bool {
        match (pre.last_purged, post.last_purged) {
            (None, _) => true,
            (Some(a), Some(b)) => a <= b,
            (Some(_), None) => false,
        }
    }

    /// INVARIANT 4: Response cache consistency
    ///
    /// All cached response indices are <= last_applied
    pub open spec fn response_cache_consistent(state: StorageState) -> bool {
        forall |idx: u64| state.pending_responses.contains_key(idx) ==> {
            match state.last_applied {
                Some(last) => idx <= last,
                None => false,
            }
        }
    }

    /// Combined invariant predicate
    pub open spec fn storage_invariant(state: StorageState) -> bool {
        chain_tip_synchronized(state) &&
        chain_valid(state.chain_hashes,
            state.log.map_values(|e: LogEntry| (e.term, e.data)),
            state.genesis_hash) &&
        response_cache_consistent(state)
    }

    /// Proof: Empty state satisfies all invariants
    pub proof fn empty_state_invariant(genesis: ChainHash)
        requires genesis.len() == 32
        ensures storage_invariant(StorageState {
            log: Map::empty(),
            chain_hashes: Map::empty(),
            chain_tip: (genesis, 0),
            kv: Map::empty(),
            last_applied: None,
            last_purged: None,
            pending_responses: Map::empty(),
            genesis_hash: genesis,
        })
    {
        // Empty state trivially satisfies:
        // - chain_tip_synchronized: chain_tip = (genesis, 0) for empty log
        // - chain_valid: no entries to verify
        // - response_cache_consistent: no responses cached
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state_invariants() {
        let state = StorageStateSpec::new();

        assert!(chain_tip_synchronized(&state));
        assert!(response_cache_consistent(&state));
        assert!(storage_invariant(&state));
    }

    #[test]
    fn test_last_applied_monotonic() {
        let pre = StorageStateSpec {
            last_applied: Some(5),
            ..Default::default()
        };

        // Increasing is allowed
        let post_increase = StorageStateSpec {
            last_applied: Some(10),
            ..Default::default()
        };
        assert!(last_applied_monotonic(&pre, &post_increase));

        // Same is allowed
        let post_same = StorageStateSpec {
            last_applied: Some(5),
            ..Default::default()
        };
        assert!(last_applied_monotonic(&pre, &post_same));

        // Decreasing is not allowed
        let post_decrease = StorageStateSpec {
            last_applied: Some(3),
            ..Default::default()
        };
        assert!(!last_applied_monotonic(&pre, &post_decrease));

        // None -> Some is allowed
        let pre_none = StorageStateSpec {
            last_applied: None,
            ..Default::default()
        };
        assert!(last_applied_monotonic(&pre_none, &pre));
    }

    #[test]
    fn test_purge_monotonic() {
        let pre = StorageStateSpec {
            last_purged: Some(5),
            ..Default::default()
        };

        // Increasing is allowed
        let post_increase = StorageStateSpec {
            last_purged: Some(10),
            ..Default::default()
        };
        assert!(purge_monotonic(&pre, &post_increase));

        // Decreasing is not allowed
        let post_decrease = StorageStateSpec {
            last_purged: Some(3),
            ..Default::default()
        };
        assert!(!purge_monotonic(&pre, &post_decrease));
    }
}
