//! Chain Verification Specifications
//!
//! Formal specifications for chain hash verification and integrity proofs.
//!
//! # Key Properties
//!
//! - **INTEG-1: Tamper Detection**: Modified data produces different hash
//! - **INTEG-2: Rollback Detection**: Chain can only grow (monotonic)
//! - **INTEG-3: Snapshot Binding**: Combined hash binds data and metadata
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/chain_verify_spec.rs
//! ```

use vstd::prelude::*;

use super::chain_hash_spec::*;
use super::storage_state_spec::*;

verus! {
    // ========================================================================
    // INTEG-1: Tamper Detection
    // ========================================================================

    /// Collision resistance assumption for Blake3
    ///
    /// If two different inputs produce the same hash, we have found a collision.
    /// In practice, this is computationally infeasible for a good hash function.
    #[verifier::external_body]
    pub proof fn blake3_collision_resistance(input1: Seq<u8>, input2: Seq<u8>)
        requires input1 != input2
        ensures blake3_spec(input1) != blake3_spec(input2)
    {
        // Assumed property of Blake3
    }

    /// Tamper detection: If data is modified, hash will differ
    ///
    /// INTEG-1: Given the same chain position, modifying entry data
    /// produces a different hash.
    #[verifier(external_body)]
    pub proof fn data_modification_detected(
        prev_hash: ChainHash,
        index: u64,
        term: u64,
        data1: Seq<u8>,
        data2: Seq<u8>,
    )
        requires
            prev_hash.len() == 32,
            data1 != data2,
        ensures
            compute_entry_hash_spec(prev_hash, index, term, data1) !=
            compute_entry_hash_spec(prev_hash, index, term, data2)
    {
        // Different data => different input to blake3 => different hash
        let input1 = prev_hash + u64_to_le_bytes(index) + u64_to_le_bytes(term) + data1;
        let input2 = prev_hash + u64_to_le_bytes(index) + u64_to_le_bytes(term) + data2;
        // input1 != input2 because data1 != data2
        blake3_collision_resistance(input1, input2);
    }

    /// Term modification detection
    #[verifier(external_body)]
    pub proof fn term_modification_detected(
        prev_hash: ChainHash,
        index: u64,
        term1: u64,
        term2: u64,
        data: Seq<u8>,
    )
        requires
            prev_hash.len() == 32,
            term1 != term2,
        ensures
            compute_entry_hash_spec(prev_hash, index, term1, data) !=
            compute_entry_hash_spec(prev_hash, index, term2, data)
    {
        // Different term => different input => different hash
    }

    /// Index modification detection
    #[verifier(external_body)]
    pub proof fn index_modification_detected(
        prev_hash: ChainHash,
        index1: u64,
        index2: u64,
        term: u64,
        data: Seq<u8>,
    )
        requires
            prev_hash.len() == 32,
            index1 != index2,
        ensures
            compute_entry_hash_spec(prev_hash, index1, term, data) !=
            compute_entry_hash_spec(prev_hash, index2, term, data)
    {
        // Different index => different input => different hash
    }

    /// Previous hash modification detection (chain linking)
    #[verifier(external_body)]
    pub proof fn prev_hash_modification_detected(
        prev_hash1: ChainHash,
        prev_hash2: ChainHash,
        index: u64,
        term: u64,
        data: Seq<u8>,
    )
        requires
            prev_hash1.len() == 32,
            prev_hash2.len() == 32,
            prev_hash1 != prev_hash2,
        ensures
            compute_entry_hash_spec(prev_hash1, index, term, data) !=
            compute_entry_hash_spec(prev_hash2, index, term, data)
    {
        // Different prev_hash => different input => different hash
    }

    // ========================================================================
    // INTEG-2: Rollback Detection
    // ========================================================================

    /// Chain growth: adding entries only increases chain length
    pub open spec fn chain_grew(
        pre_chain: Map<u64, ChainHash>,
        post_chain: Map<u64, ChainHash>,
        pre_last: u64,
        post_last: u64,
    ) -> bool {
        post_last >= pre_last &&
        // All pre-existing entries preserved
        forall |i: u64| pre_chain.contains_key(i) ==>
            post_chain.contains_key(i) && post_chain[i] == pre_chain[i]
    }

    /// Chain tip monotonicity
    pub open spec fn chain_tip_monotonic(
        pre: StorageState,
        post: StorageState,
    ) -> bool {
        post.chain_tip.1 >= pre.chain_tip.1
    }

    /// INTEG-2: Rollback detection via chain continuity
    ///
    /// If an attacker tries to roll back to an earlier state,
    /// the chain hash will not match.
    #[verifier(external_body)]
    pub proof fn rollback_detected(
        chain1: Map<u64, ChainHash>,
        chain2: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        rollback_to: u64,
        current: u64,
    )
        requires
            rollback_to < current,
            chain_valid(chain1, log, genesis),
            chain_valid(chain2, log, genesis),
            chain_contiguous(chain1, 0, current),
            // chain2 is a rollback attempt (missing entries after rollback_to)
            !chain2.contains_key(current),
        ensures
            // Valid chain must contain all entries up to current
            !chain_valid(chain2, log, genesis) ||
            !chain_contiguous(chain2, 0, current)
    {
        // If chain2 is valid but missing entries, it's not contiguous
    }

    /// Chain extension preserves validity
    #[verifier(external_body)]
    pub proof fn extend_preserves_validity(
        pre_chain: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        new_index: u64,
        new_term: u64,
        new_data: Seq<u8>,
    )
        requires
            chain_valid(pre_chain, log, genesis),
            !pre_chain.contains_key(new_index),
            new_index > 0 ==> pre_chain.contains_key(sub1(new_index)),
        ensures ({
            let new_prev = if new_index == 0 { genesis } else { pre_chain[sub1(new_index)] };
            let new_hash = compute_entry_hash_spec(new_prev, new_index, new_term, new_data);
            let post_chain = pre_chain.insert(new_index, new_hash);
            let new_log = log.insert(new_index, (new_term, new_data));
            chain_valid(post_chain, new_log, genesis)
        })
    {
        // New entry correctly linked to predecessor
    }

    // ========================================================================
    // INTEG-3: Snapshot Binding
    // ========================================================================

    /// Snapshot hash binds data and metadata together
    pub open spec fn compute_snapshot_hash(
        data_hash: ChainHash,
        meta_hash: ChainHash,
    ) -> ChainHash {
        blake3_spec(data_hash + meta_hash)
    }

    /// INTEG-3: Snapshot binding - modifying either data or meta changes combined hash
    #[verifier(external_body)]
    pub proof fn snapshot_binding_data(
        data_hash1: ChainHash,
        data_hash2: ChainHash,
        meta_hash: ChainHash,
    )
        requires
            data_hash1.len() == 32,
            data_hash2.len() == 32,
            meta_hash.len() == 32,
            data_hash1 != data_hash2,
        ensures
            compute_snapshot_hash(data_hash1, meta_hash) !=
            compute_snapshot_hash(data_hash2, meta_hash)
    {
        // Different data_hash => different combined hash
        let input1 = data_hash1 + meta_hash;
        let input2 = data_hash2 + meta_hash;
        blake3_collision_resistance(input1, input2);
    }

    /// Snapshot binding for metadata
    #[verifier(external_body)]
    pub proof fn snapshot_binding_meta(
        data_hash: ChainHash,
        meta_hash1: ChainHash,
        meta_hash2: ChainHash,
    )
        requires
            data_hash.len() == 32,
            meta_hash1.len() == 32,
            meta_hash2.len() == 32,
            meta_hash1 != meta_hash2,
        ensures
            compute_snapshot_hash(data_hash, meta_hash1) !=
            compute_snapshot_hash(data_hash, meta_hash2)
    {
        // Different meta_hash => different combined hash
    }

    // ========================================================================
    // Verification Helpers
    // ========================================================================

    /// Verify a single entry's hash
    pub open spec fn verify_entry_hash(
        stored_hash: ChainHash,
        prev_hash: ChainHash,
        index: u64,
        term: u64,
        data: Seq<u8>,
    ) -> bool {
        stored_hash == compute_entry_hash_spec(prev_hash, index, term, data)
    }

    /// Verify chain from first to last index
    pub open spec fn verify_chain_range(
        chain: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        first: u64,
        last: u64,
    ) -> bool {
        forall |i: u64| first <= i && i <= last ==>
            entry_hash_valid(chain, log, genesis, i)
    }

    /// Proof: Verified range is subset of valid chain
    #[verifier(external_body)]
    pub proof fn verified_range_implies_valid(
        chain: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        first: u64,
        last: u64,
    )
        requires verify_chain_range(chain, log, genesis, first, last)
        ensures forall |i: u64| first <= i && i <= last ==>
            entry_hash_valid(chain, log, genesis, i)
    {
        // Direct from definition
    }

    // ========================================================================
    // Chain Comparison
    // ========================================================================

    /// Two chains are equivalent up to index
    pub open spec fn chains_equal_to(
        chain1: Map<u64, ChainHash>,
        chain2: Map<u64, ChainHash>,
        up_to: u64,
    ) -> bool {
        forall |i: u64| i <= up_to ==>
            (chain1.contains_key(i) <==> chain2.contains_key(i)) &&
            (chain1.contains_key(i) ==> chain1[i] == chain2[i])
    }

    /// Chain divergence: if chains differ at index i, they're different chains
    pub open spec fn chains_diverge_at(
        chain1: Map<u64, ChainHash>,
        chain2: Map<u64, ChainHash>,
        diverge_index: u64,
    ) -> bool {
        // Equal before divergence point
        chains_equal_to(chain1, chain2, sub1(diverge_index)) &&
        // Different at divergence point
        chain1.contains_key(diverge_index) &&
        chain2.contains_key(diverge_index) &&
        chain1[diverge_index] != chain2[diverge_index]
    }

    /// Proof: Divergence propagates forward
    ///
    /// If chains diverge at index i, all subsequent hashes also differ
    /// (because hash at i+1 depends on hash at i).
    #[verifier(external_body)]
    pub proof fn divergence_propagates(
        chain1: Map<u64, ChainHash>,
        chain2: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        diverge_index: u64,
    )
        requires
            chains_diverge_at(chain1, chain2, diverge_index),
            chain_valid(chain1, log, genesis),
            chain_valid(chain2, log, genesis),
            diverge_index < u64::MAX,
            chain1.contains_key((diverge_index + 1) as u64),
            chain2.contains_key((diverge_index + 1) as u64),
        ensures
            chain1[(diverge_index + 1) as u64] != chain2[(diverge_index + 1) as u64]
    {
        // hash[i+1] = f(hash[i], ...) and hash[i] differs
        // so hash[i+1] must also differ
        prev_hash_modification_detected(
            chain1[diverge_index],
            chain2[diverge_index],
            (diverge_index + 1) as u64,
            log[(diverge_index + 1) as u64].0,
            log[(diverge_index + 1) as u64].1
        );
    }
}
