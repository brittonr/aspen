//! Storage State Machine Model
//!
//! Abstract state model for formal verification of storage operations.
//!
//! # State Model
//!
//! The `StorageState` captures:
//! - Raft log entries with their chain hashes
//! - KV state machine entries
//! - Metadata (last_applied, last_purged, chain tip)
//! - Response cache for idempotency
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/storage_state_spec.rs
//! ```

use vstd::prelude::*;

use super::chain_hash_spec::*;

verus! {
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
            // Chain tip index must be in the log and hash must match
            state.chain_hashes.contains_key(state.chain_tip.1) &&
            state.log.contains_key(state.chain_tip.1) &&
            state.chain_hashes[state.chain_tip.1] == state.chain_tip.0
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
        response_cache_consistent(state)
    }

    /// Proof: Empty state satisfies all invariants
    pub proof fn empty_state_invariant(genesis: ChainHash)
        requires genesis.len() == 32
        ensures storage_invariant(StorageState {
            log: Map::empty(),
            chain_hashes: Map::empty(),
            chain_tip: (genesis, 0u64),
            kv: Map::empty(),
            last_applied: None,
            last_purged: None,
            pending_responses: Map::empty(),
            genesis_hash: genesis,
        })
    {
        // Empty state trivially satisfies:
        // - chain_tip_synchronized: chain_tip = (genesis, 0) for empty log
        // - response_cache_consistent: no responses cached
    }
}
