//! Chain Hash Specifications
//!
//! Specifications for chain hash computation used in Raft log integrity.
//!
//! # Chain Hash Design
//!
//! Each entry's hash is computed as:
//! ```text
//! entry_hash = blake3(prev_hash || log_index || term || entry_data)
//! ```
//!
//! This creates an unbreakable chain where modifying any entry invalidates
//! all subsequent hashes.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/chain_hash_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    /// 32-byte Blake3 hash
    pub type ChainHash = Seq<u8>;

    /// Genesis hash for the start of the chain (all zeros)
    pub spec const GENESIS_HASH: ChainHash;

    /// Specification for compute_entry_hash
    ///
    /// Computes: blake3(prev_hash || index || term || data)
    pub open spec fn compute_entry_hash_spec(
        prev_hash: ChainHash,
        index: u64,
        term: u64,
        data: Seq<u8>,
    ) -> ChainHash
        recommends prev_hash.len() == 32
    {
        // Abstract spec - actual blake3 computation modeled
        // as an uninterpreted function with known properties
        blake3_spec(
            prev_hash + u64_to_le_bytes(index) + u64_to_le_bytes(term) + data
        )
    }

    /// Blake3 is modeled as uninterpreted with collision resistance
    ///
    /// We don't prove properties of blake3 itself - we assume:
    /// 1. Output is always 32 bytes
    /// 2. The function is deterministic
    pub closed spec fn blake3_spec(input: Seq<u8>) -> ChainHash;

    /// Convert u64 to little-endian bytes
    pub closed spec fn u64_to_le_bytes(n: u64) -> Seq<u8>;

    /// Axiom: u64_to_le_bytes produces exactly 8 bytes
    #[verifier::external_body]
    pub proof fn u64_to_le_bytes_length(n: u64)
        ensures u64_to_le_bytes(n).len() == 8
    {}

    /// Axiom: blake3 produces 32-byte output
    #[verifier::external_body]
    pub proof fn blake3_output_length(input: Seq<u8>)
        ensures blake3_spec(input).len() == 32
    {}

    /// Axiom: blake3 is deterministic
    #[verifier::external_body]
    pub proof fn blake3_deterministic(a: Seq<u8>, b: Seq<u8>)
        requires a == b
        ensures blake3_spec(a) == blake3_spec(b)
    {}

    // ========================================================================
    // INVARIANT 2: Chain Continuity
    // ========================================================================

    /// Chain validity predicate
    ///
    /// For a valid chain, each hash depends on its predecessor:
    /// - Entry 1's hash = blake3(GENESIS_HASH || 1 || term_1 || data_1)
    /// - Entry n's hash = blake3(hash_{n-1} || n || term_n || data_n)
    pub open spec fn chain_valid(
        chain: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,  // index -> (term, data)
        genesis: ChainHash,
    ) -> bool {
        forall |i: u64| log.contains_key(i) ==> {
            let prev = if i == 0 { genesis } else { chain[i - 1] };
            let (term, data) = log[i];
            chain.contains_key(i) &&
            chain[i] == compute_entry_hash_spec(prev, i, term, data)
        }
    }

    /// Chain contiguity: no gaps in the chain
    pub open spec fn chain_contiguous(
        chain: Map<u64, ChainHash>,
        first_index: u64,
        last_index: u64,
    ) -> bool {
        forall |i: u64| first_index <= i <= last_index ==> chain.contains_key(i)
    }

    // ========================================================================
    // Chain Preservation Proofs
    // ========================================================================

    /// Proof: Appending preserves chain validity
    ///
    /// When we append a new entry with correctly computed hash,
    /// the chain remains valid.
    pub proof fn append_preserves_chain(
        pre_chain: Map<u64, ChainHash>,
        pre_log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        new_index: u64,
        new_term: u64,
        new_data: Seq<u8>,
    )
        requires
            genesis.len() == 32,
            chain_valid(pre_chain, pre_log, genesis),
            !pre_log.contains_key(new_index),
            // New entry continues from chain tip
            new_index == 0 || pre_chain.contains_key(new_index - 1),
        ensures
            chain_valid(
                pre_chain.insert(new_index, compute_entry_hash_spec(
                    if new_index == 0 { genesis } else { pre_chain[new_index - 1] },
                    new_index,
                    new_term,
                    new_data
                )),
                pre_log.insert(new_index, (new_term, new_data)),
                genesis
            )
    {
        // The new chain includes one additional entry
        let prev_hash = if new_index == 0 { genesis } else { pre_chain[new_index - 1] };
        let new_hash = compute_entry_hash_spec(prev_hash, new_index, new_term, new_data);
        let post_chain = pre_chain.insert(new_index, new_hash);
        let post_log = pre_log.insert(new_index, (new_term, new_data));

        // For all existing entries, validity is preserved (unchanged)
        // For the new entry, validity holds by construction
        assert forall |i: u64| post_log.contains_key(i) implies {
            let prev = if i == 0 { genesis } else { post_chain[i - 1] };
            let (term, data) = post_log[i];
            post_chain.contains_key(i) &&
            post_chain[i] == compute_entry_hash_spec(prev, i, term, data)
        } by {
            if i == new_index {
                // New entry: holds by construction
                assert(post_chain[i] == new_hash);
                assert(post_log[i] == (new_term, new_data));
            } else {
                // Existing entry: preserved from pre-state
                assert(pre_log.contains_key(i));
                assert(pre_chain.contains_key(i));
            }
        }
    }

    /// Proof: Truncating tail preserves chain validity for remaining entries
    ///
    /// When we remove entries from index `truncate_at` onwards,
    /// entries before `truncate_at` remain valid.
    pub proof fn truncate_preserves_chain(
        chain: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,
        genesis: ChainHash,
        truncate_at: u64,
    )
        requires
            genesis.len() == 32,
            chain_valid(chain, log, genesis),
        ensures
            chain_valid(
                chain.restrict(Set::new(|i: u64| i < truncate_at)),
                log.restrict(Set::new(|i: u64| i < truncate_at)),
                genesis
            )
    {
        // Restriction preserves validity for remaining entries
        // because their hashes only depend on predecessors (all retained)
        let restricted_chain = chain.restrict(Set::new(|i: u64| i < truncate_at));
        let restricted_log = log.restrict(Set::new(|i: u64| i < truncate_at));

        assert forall |i: u64| restricted_log.contains_key(i) implies {
            let prev = if i == 0 { genesis } else { restricted_chain[i - 1] };
            let (term, data) = restricted_log[i];
            restricted_chain.contains_key(i) &&
            restricted_chain[i] == compute_entry_hash_spec(prev, i, term, data)
        } by {
            // i < truncate_at, so entry and its predecessor are retained
            assert(log.contains_key(i));
            assert(chain.contains_key(i));
            if i > 0 {
                assert(i - 1 < truncate_at);
                assert(restricted_chain.contains_key(i - 1));
            }
        }
    }
}
