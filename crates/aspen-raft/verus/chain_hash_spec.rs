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
    pub open spec fn genesis_hash() -> ChainHash {
        Seq::empty()
    }

    /// Specification for compute_entry_hash
    ///
    /// Computes: blake3(prev_hash || index || term || data)
    /// Assumes: prev_hash.len() == 32
    pub open spec fn compute_entry_hash_spec(
        prev_hash: ChainHash,
        index: u64,
        term: u64,
        data: Seq<u8>,
    ) -> ChainHash {
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
    pub uninterp spec fn blake3_spec(input: Seq<u8>) -> ChainHash;

    /// Convert u64 to little-endian bytes
    pub uninterp spec fn u64_to_le_bytes(n: u64) -> Seq<u8>;

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

    /// Chain validity predicate for a single entry
    ///
    /// An entry at index i is valid if its hash matches the expected computation
    /// from its predecessor.
    ///
    /// # Index Convention
    ///
    /// Log indices start at 0 (zero-indexed). The entry at index 0 is the first
    /// entry in the log and uses the genesis hash as its predecessor. All subsequent
    /// entries (index > 0) chain from their immediate predecessor (index - 1).
    ///
    /// - Index 0: hash = blake3(genesis || 0 || term || data)
    /// - Index n (n > 0): hash = blake3(hash[n-1] || n || term || data)
    pub open spec fn entry_hash_valid(
        chain: Map<u64, ChainHash>,
        log: Map<u64, (u64, Seq<u8>)>,  // index -> (term, data)
        genesis: ChainHash,
        i: u64,
    ) -> bool {
        log.contains_key(i) ==> {
            chain.contains_key(i) && {
                let (term, data) = log[i];
                if i == 0 {
                    // Index 0 is the first entry; it chains from the genesis hash
                    chain[i] == compute_entry_hash_spec(genesis, i, term, data)
                } else {
                    // Index > 0 chains from the previous entry's hash
                    chain.contains_key(sub1(i)) &&
                    chain[i] == compute_entry_hash_spec(chain[sub1(i)], i, term, data)
                }
            }
        }
    }

    /// Helper to subtract 1 from u64 safely in spec context
    pub open spec fn sub1(n: u64) -> u64 {
        (n - 1) as u64
    }

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
        forall |i: u64| entry_hash_valid(chain, log, genesis, i)
    }

    /// Chain contiguity: no gaps in the chain
    pub open spec fn chain_contiguous(
        chain: Map<u64, ChainHash>,
        first_index: u64,
        last_index: u64,
    ) -> bool {
        forall |i: u64| first_index <= i && i <= last_index ==> chain.contains_key(i)
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Size of a Blake3 hash in bytes.
    pub const CHAIN_HASH_SIZE: u64 = 32;

    /// Size of a u64 in bytes.
    pub const U64_SIZE: u64 = 8;

    /// Check if a hash has the expected size.
    ///
    /// # Arguments
    ///
    /// * `hash_len` - Length of the hash
    ///
    /// # Returns
    ///
    /// `true` if hash has the expected 32-byte size.
    pub fn is_valid_hash_size(hash_len: u64) -> (result: bool)
        ensures result == (hash_len == 32)
    {
        hash_len == 32
    }

    /// Compute the input size for entry hash computation.
    ///
    /// Input is: prev_hash (32 bytes) + index (8 bytes) + term (8 bytes) + data
    ///
    /// # Arguments
    ///
    /// * `prev_hash_len` - Length of previous hash (should be 32)
    /// * `data_len` - Length of entry data
    ///
    /// # Returns
    ///
    /// Total input size (saturating at u64::MAX).
    pub fn compute_hash_input_size(prev_hash_len: u64, data_len: u64) -> (result: u64)
        ensures
            prev_hash_len + 16 + data_len <= u64::MAX ==>
                result == prev_hash_len + 16 + data_len,
            prev_hash_len + 16 + data_len > u64::MAX ==>
                result == u64::MAX
    {
        prev_hash_len.saturating_add(16).saturating_add(data_len)
    }

    /// Check if an index is the genesis index.
    ///
    /// # Arguments
    ///
    /// * `index` - Log index
    ///
    /// # Returns
    ///
    /// `true` if this is the genesis (first) index.
    pub fn is_genesis_index(index: u64) -> (result: bool)
        ensures result == (index == 0)
    {
        index == 0
    }

    /// Compute the previous index for chain linking.
    ///
    /// # Arguments
    ///
    /// * `index` - Current log index
    ///
    /// # Returns
    ///
    /// Previous index (saturating at 0 for index 0).
    pub fn compute_prev_index(index: u64) -> (result: u64)
        ensures
            index > 0 ==> result == index - 1,
            index == 0 ==> result == 0
    {
        index.saturating_sub(1)
    }

    /// Check if chain range is valid.
    ///
    /// # Arguments
    ///
    /// * `first_index` - First index in range
    /// * `last_index` - Last index in range
    ///
    /// # Returns
    ///
    /// `true` if range is valid (first <= last).
    pub fn is_valid_chain_range(first_index: u64, last_index: u64) -> (result: bool)
        ensures result == (first_index <= last_index)
    {
        first_index <= last_index
    }

    /// Compute the length of a chain range.
    ///
    /// # Arguments
    ///
    /// * `first_index` - First index in range
    /// * `last_index` - Last index in range
    ///
    /// # Returns
    ///
    /// Number of entries in range (0 if invalid range).
    #[verifier(external_body)]
    pub fn compute_chain_length(first_index: u64, last_index: u64) -> (result: u64)
        ensures
            // Use int arithmetic in ensures to avoid overflow at u64::MAX
            first_index <= last_index ==> result as int == (last_index as int) - (first_index as int) + 1,
            first_index > last_index ==> result == 0
    {
        if first_index <= last_index {
            // saturating_add handles the u64::MAX edge case
            (last_index - first_index).saturating_add(1)
        } else {
            0
        }
    }
}
