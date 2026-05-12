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
    /// Appending the same prefix preserves sequence inequality.
    pub proof fn same_prefix_append_preserves_neq(prefix: Seq<u8>, left: Seq<u8>, right: Seq<u8>)
        requires left != right
        ensures prefix + left != prefix + right
    {
        if prefix + left == prefix + right {
            assert((prefix + left).len() == prefix.len() + left.len());
            assert((prefix + right).len() == prefix.len() + right.len());
            assert(left.len() == right.len());
            assert(left =~= right) by {
                assert forall |i: int| 0 <= i && i < left.len() implies left[i] == right[i] by {
                    assert(0 <= prefix.len() + i);
                    assert(prefix.len() + i < (prefix + left).len());
                    assert((prefix + left)[prefix.len() + i] == left[i]);
                    assert((prefix + right)[prefix.len() + i] == right[i]);
                }
            }
            assert(false);
        }
    }

    /// Appending the same suffix preserves sequence inequality.
    pub proof fn same_suffix_append_preserves_neq(left: Seq<u8>, right: Seq<u8>, suffix: Seq<u8>)
        requires left != right
        ensures left + suffix != right + suffix
    {
        if left + suffix == right + suffix {
            assert((left + suffix).len() == left.len() + suffix.len());
            assert((right + suffix).len() == right.len() + suffix.len());
            assert(left.len() == right.len());
            assert(left =~= right) by {
                assert forall |i: int| 0 <= i && i < left.len() implies left[i] == right[i] by {
                    assert(0 <= i);
                    assert(i < (left + suffix).len());
                    assert(i < (right + suffix).len());
                    assert((left + suffix)[i] == left[i]);
                    assert((right + suffix)[i] == right[i]);
                }
            }
            assert(false);
        }
    }

    // ========================================================================
    // INTEG-1: Tamper Detection
    // ========================================================================

    /// Trusted Blake3 collision-resistance assumption.
    ///
    /// If two different inputs produce the same hash, we have found a collision.
    /// In practice, this is computationally infeasible for a good hash function.
    /// Verus models `blake3_spec` as uninterpreted, so this is the named
    /// cryptographic trust boundary backed by the production `blake3` crate.
    #[verifier::external_body]
    pub proof fn blake3_collision_resistance(input1: Seq<u8>, input2: Seq<u8>)
        requires input1 != input2
        ensures blake3_spec(input1) != blake3_spec(input2)
    {
        // Assumed property of Blake3
    }

    /// Tamper detection: If data is modified, hash will differ.
    /// Trusted wrapper over `blake3_collision_resistance`; the remaining
    /// assumption is Blake3 collision resistance for the constructed byte input.
    ///
    /// INTEG-1: Given the same chain position, modifying entry data
    /// produces a different hash.
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
        let prefix = prev_hash + u64_to_le_bytes(index) + u64_to_le_bytes(term);
        let input1 = prefix + data1;
        let input2 = prefix + data2;
        same_prefix_append_preserves_neq(prefix, data1, data2);
        blake3_collision_resistance(input1, input2);
    }

    /// Trusted u64 little-endian encoding injectivity boundary.
    ///
    /// The byte-level LE model is opaque (`u64_to_le_byte` is uninterpreted), so
    /// injectivity is kept as the named encoding trust boundary while wrappers
    /// around constructed hash inputs are proved structurally.
    #[verifier(external_body)]
    pub proof fn u64_to_le_bytes_injective(left: u64, right: u64)
        requires
            left != right,
        ensures
            u64_to_le_bytes(left) != u64_to_le_bytes(right),
    {
    }

    /// Term modification detection. Proved structurally from the explicit u64
    /// encoding injectivity boundary plus Blake3 collision resistance.
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
        let prefix = prev_hash + u64_to_le_bytes(index);
        let left = u64_to_le_bytes(term1);
        let right = u64_to_le_bytes(term2);
        u64_to_le_bytes_injective(term1, term2);
        same_prefix_append_preserves_neq(prefix, left, right);
        same_suffix_append_preserves_neq(prefix + left, prefix + right, data);
        blake3_collision_resistance(prefix + left + data, prefix + right + data);
    }

    /// Index modification detection. Proved structurally from the explicit u64
    /// encoding injectivity boundary plus Blake3 collision resistance.
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
        let left = u64_to_le_bytes(index1);
        let right = u64_to_le_bytes(index2);
        let term_bytes = u64_to_le_bytes(term);
        u64_to_le_bytes_injective(index1, index2);
        same_prefix_append_preserves_neq(prev_hash, left, right);
        same_suffix_append_preserves_neq(prev_hash + left, prev_hash + right, term_bytes);
        same_suffix_append_preserves_neq(prev_hash + left + term_bytes, prev_hash + right + term_bytes, data);
        blake3_collision_resistance(prev_hash + left + term_bytes + data, prev_hash + right + term_bytes + data);
    }

    /// Previous hash modification detection (chain linking). Proved
    /// structurally from Blake3 collision resistance for the constructed entry
    /// hash inputs.
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
        let index_bytes = u64_to_le_bytes(index);
        let term_bytes = u64_to_le_bytes(term);
        same_suffix_append_preserves_neq(prev_hash1, prev_hash2, index_bytes);
        same_suffix_append_preserves_neq(prev_hash1 + index_bytes, prev_hash2 + index_bytes, term_bytes);
        same_suffix_append_preserves_neq(prev_hash1 + index_bytes + term_bytes, prev_hash2 + index_bytes + term_bytes, data);
        blake3_collision_resistance(prev_hash1 + index_bytes + term_bytes + data, prev_hash2 + index_bytes + term_bytes + data);
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

    /// Chain extension preserves validity by proving the inserted index directly
    /// and reducing all other indices to the pre-extension chain predicate.
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
        let new_prev = if new_index == 0 { genesis } else { pre_chain[sub1(new_index)] };
        let new_hash = compute_entry_hash_spec(new_prev, new_index, new_term, new_data);
        let post_chain = pre_chain.insert(new_index, new_hash);
        let new_log = log.insert(new_index, (new_term, new_data));

        assert forall |i: u64| entry_hash_valid(post_chain, new_log, genesis, i) by {
            if i == new_index {
                assert(new_log.contains_key(i));
                assert(post_chain.contains_key(i));
                assert(new_log[i] == (new_term, new_data));
                if i == 0 {
                    assert(post_chain[i] == compute_entry_hash_spec(genesis, i, new_term, new_data));
                } else {
                    assert(post_chain.contains_key(sub1(i)));
                    assert(post_chain[i] == compute_entry_hash_spec(post_chain[sub1(i)], i, new_term, new_data));
                }
            } else {
                if new_log.contains_key(i) {
                    assert(log.contains_key(i));
                    assert(entry_hash_valid(pre_chain, log, genesis, i));
                    assert(pre_chain.contains_key(i));
                    assert(post_chain.contains_key(i));
                    assert(new_log[i] == log[i]);
                    assert(post_chain[i] == pre_chain[i]);
                    if i != 0 {
                        assert(pre_chain.contains_key(sub1(i)));
                        if sub1(i) == new_index {
                            assert(!pre_chain.contains_key(new_index));
                            assert(false);
                        }
                        assert(post_chain[sub1(i)] == pre_chain[sub1(i)]);
                    }
                }
            }
        }
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

    /// INTEG-3: Snapshot binding - modifying either data or meta changes combined hash.
    /// Trusted wrapper over Blake3 collision resistance for concatenated
    /// snapshot data/meta hashes.
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
        same_suffix_append_preserves_neq(data_hash1, data_hash2, meta_hash);
        blake3_collision_resistance(input1, input2);
    }

    /// Snapshot binding for metadata. Trusted wrapper over Blake3 collision
    /// resistance for concatenated snapshot data/meta hashes.
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
        let input1 = data_hash + meta_hash1;
        let input2 = data_hash + meta_hash2;
        same_prefix_append_preserves_neq(data_hash, meta_hash1, meta_hash2);
        blake3_collision_resistance(input1, input2);
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
    /// If chains diverge at index i, the next linked hashes also differ because
    /// both valid chains compute the next entry from their diverged predecessor.
    /// The only remaining trust is the previous-hash Blake3 collision assumption
    /// called by `prev_hash_modification_detected`.
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
            log.contains_key((diverge_index + 1) as u64),
            chain1.contains_key((diverge_index + 1) as u64),
            chain2.contains_key((diverge_index + 1) as u64),
            chain1[diverge_index].len() == 32,
            chain2[diverge_index].len() == 32,
        ensures
            chain1[(diverge_index + 1) as u64] != chain2[(diverge_index + 1) as u64]
    {
        let next = (diverge_index + 1) as u64;
        assert(next != 0);
        assert(sub1(next) == diverge_index);
        assert(chains_diverge_at(chain1, chain2, diverge_index));
        assert(chain1[diverge_index] != chain2[diverge_index]);
        assert(entry_hash_valid(chain1, log, genesis, next));
        assert(entry_hash_valid(chain2, log, genesis, next));
        assert(log.contains_key(next));
        assert(chain1[next] == compute_entry_hash_spec(
            chain1[diverge_index],
            next,
            log[next].0,
            log[next].1,
        ));
        assert(chain2[next] == compute_entry_hash_spec(
            chain2[diverge_index],
            next,
            log[next].0,
            log[next].1,
        ));
        prev_hash_modification_detected(
            chain1[diverge_index],
            chain2[diverge_index],
            next,
            log[next].0,
            log[next].1,
        );
    }
}
