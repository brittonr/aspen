//! Snapshot Operation Specifications
//!
//! Proves that snapshot creation and installation preserve storage invariants.
//!
//! # Snapshots in Raft
//!
//! Snapshots capture the state machine at a specific log index, allowing:
//! - Log compaction (purging old entries)
//! - Fast follower catch-up (install snapshot instead of replaying log)
//!
//! # INVARIANT 7: Snapshot Integrity
//!
//! Snapshots must include:
//! - Valid chain hash at the snapshot point
//! - Cryptographic hashes of snapshot data
//! - Consistent state machine representation
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/snapshot_spec.rs
//! ```

use vstd::prelude::*;

use super::chain_hash_spec::*;
use super::storage_state_spec::*;

verus! {
    /// Snapshot metadata
    pub struct SnapshotMeta {
        pub last_log_index: u64,
        pub last_log_term: u64,
        pub chain_hash_at_snapshot: ChainHash,
    }

    /// Snapshot integrity data
    pub struct SnapshotIntegrity {
        pub data_hash: ChainHash,
        pub meta_hash: ChainHash,
        pub combined_hash: ChainHash,
        pub chain_hash_at_snapshot: ChainHash,
    }

    /// INVARIANT 7: Snapshot integrity verification
    ///
    /// A snapshot is valid if:
    /// 1. Data hash matches actual data
    /// 2. Chain hash matches the chain at snapshot index
    /// 3. Combined hash is correctly computed
    pub open spec fn snapshot_valid(
        state: StorageState,
        meta: SnapshotMeta,
        data: Seq<u8>,
        integrity: SnapshotIntegrity,
    ) -> bool {
        // Data hash correct
        blake3_spec(data) == integrity.data_hash &&
        // Chain hash matches stored hash (if present)
        (meta.last_log_index == 0 ||
            (state.chain_hashes.contains_key(meta.last_log_index) &&
             state.chain_hashes[meta.last_log_index] == integrity.chain_hash_at_snapshot)) &&
        // Combined hash correctly computed
        blake3_spec(integrity.data_hash + integrity.meta_hash) == integrity.combined_hash
    }

    /// Specification of snapshot creation (read-only)
    pub open spec fn create_snapshot_post(
        pre: StorageState,
        _snapshot_index: u64,
    ) -> StorageState {
        // Snapshot creation doesn't modify state
        pre
    }

    /// Specification of snapshot installation
    ///
    /// # Preconditions
    ///
    /// Snapshot installation requires:
    ///
    /// 1. **Chain hash validity**: `meta.chain_hash_at_snapshot` must be a valid 32-byte hash.
    ///    This ensures the chain tip remains valid after installation.
    ///
    /// 2. **Index monotonicity**: `meta.last_log_index` must be greater than or equal to the
    ///    current `last_applied` (if any). Snapshots represent committed state at a specific
    ///    index, so installing an older snapshot would violate monotonicity.
    ///
    /// 3. **Purge monotonicity**: `meta.last_log_index` must be greater than or equal to the
    ///    current `last_purged` (if any). We cannot "un-purge" already purged entries.
    pub open spec fn install_snapshot_post(
        pre: StorageState,
        meta: SnapshotMeta,
        new_kv: Map<Seq<u8>, KvEntry>,
    ) -> StorageState
        requires
            // Chain hash validity: must be 32 bytes (standard Blake3 output)
            meta.chain_hash_at_snapshot.len() == 32,
            // Index monotonicity: snapshot index must not regress from last_applied
            match pre.last_applied {
                Some(last) => meta.last_log_index >= last,
                None => true,
            },
            // Purge monotonicity: snapshot index must not regress from last_purged
            match pre.last_purged {
                Some(purged) => meta.last_log_index >= purged,
                None => true,
            }
    {
        let retained_log = pre.log.restrict(Set::new(|i: u64| i > meta.last_log_index));
        let retained_hashes = pre.chain_hashes.restrict(Set::new(|i: u64| i > meta.last_log_index));

        let new_tip = if retained_log.is_empty() {
            // No remaining log entries; chain tip is the snapshot point
            // Precondition ensures meta.chain_hash_at_snapshot is valid (32 bytes)
            (meta.chain_hash_at_snapshot, meta.last_log_index)
        } else {
            // Log entries remain after the snapshot; preserve existing chain tip
            // (these entries chain from the snapshot point)
            pre.chain_tip
        };

        StorageState {
            log: retained_log,
            chain_hashes: retained_hashes,
            chain_tip: new_tip,
            kv: new_kv,
            last_applied: Some(meta.last_log_index),
            last_purged: Some(meta.last_log_index),
            pending_responses: Map::empty(),  // Clear responses
            genesis_hash: pre.genesis_hash,
        }
    }

    /// Snapshot creation is pure/read-only
    pub proof fn create_is_readonly(
        pre: StorageState,
        snapshot_index: u64,
    )
        ensures create_snapshot_post(pre, snapshot_index) == pre
    {
        // Snapshot creation doesn't modify state
    }

    /// Snapshot installation updates last_applied
    pub proof fn install_updates_last_applied(
        pre: StorageState,
        meta: SnapshotMeta,
        new_kv: Map<Seq<u8>, KvEntry>,
    )
        requires
            meta.chain_hash_at_snapshot.len() == 32,
            match pre.last_applied {
                Some(last) => meta.last_log_index >= last,
                None => true,
            },
            match pre.last_purged {
                Some(purged) => meta.last_log_index >= purged,
                None => true,
            }
        ensures
            install_snapshot_post(pre, meta, new_kv).last_applied == Some(meta.last_log_index),
    {
        // By construction
    }

    /// Snapshot installation clears old entries
    pub proof fn install_clears_old_entries(
        pre: StorageState,
        meta: SnapshotMeta,
        new_kv: Map<Seq<u8>, KvEntry>,
    )
        requires
            meta.chain_hash_at_snapshot.len() == 32,
            match pre.last_applied {
                Some(last) => meta.last_log_index >= last,
                None => true,
            },
            match pre.last_purged {
                Some(purged) => meta.last_log_index >= purged,
                None => true,
            }
        ensures
            forall |i: u64| i <= meta.last_log_index ==>
                !install_snapshot_post(pre, meta, new_kv).log.contains_key(i),
    {
        // By construction: restrict keeps only i > meta.last_log_index
    }

    /// Snapshot installation preserves monotonicity of last_applied
    pub proof fn install_preserves_last_applied_monotonicity(
        pre: StorageState,
        meta: SnapshotMeta,
        new_kv: Map<Seq<u8>, KvEntry>,
    )
        requires
            meta.chain_hash_at_snapshot.len() == 32,
            match pre.last_applied {
                Some(last) => meta.last_log_index >= last,
                None => true,
            },
            match pre.last_purged {
                Some(purged) => meta.last_log_index >= purged,
                None => true,
            }
        ensures
            last_applied_monotonic(pre, install_snapshot_post(pre, meta, new_kv)),
    {
        // By precondition: meta.last_log_index >= pre.last_applied (if Some)
        // Post state has last_applied = Some(meta.last_log_index)
        // Therefore monotonicity preserved
    }

    /// Snapshot installation preserves monotonicity of last_purged
    pub proof fn install_preserves_purge_monotonicity(
        pre: StorageState,
        meta: SnapshotMeta,
        new_kv: Map<Seq<u8>, KvEntry>,
    )
        requires
            meta.chain_hash_at_snapshot.len() == 32,
            match pre.last_applied {
                Some(last) => meta.last_log_index >= last,
                None => true,
            },
            match pre.last_purged {
                Some(purged) => meta.last_log_index >= purged,
                None => true,
            }
        ensures
            purge_monotonic(pre, install_snapshot_post(pre, meta, new_kv)),
    {
        // By precondition: meta.last_log_index >= pre.last_purged (if Some)
        // Post state has last_purged = Some(meta.last_log_index)
        // Therefore monotonicity preserved
    }
}
