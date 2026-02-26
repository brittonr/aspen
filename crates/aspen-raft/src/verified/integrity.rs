//! Pure chain hashing functions for Raft log integrity verification.
//!
//! Provides cryptographic integrity verification for Raft log entries using Blake3
//! chain hashing, where each entry's hash depends on all previous entries. This enables
//! detection of:
//!
//! - Hardware corruption (bit flips, disk errors, misdirected writes)
//! - Byzantine modification attempts (tampered log entries)
//! - Log truncation/tampering (chain discontinuity)
//! - Rollback attacks (detected via chain mismatch)
//!
//! # Chain Hash Design
//!
//! Each entry's hash is computed as:
//! ```text
//! entry_hash = blake3(prev_hash || log_index || term || entry_data)
//! ```
//!
//! This creates an unbreakable chain where modifying any entry invalidates all
//! subsequent hashes, making tampering detectable.
//!
//! # Industry Reference
//!
//! This design follows TigerBeetle's approach to chain hashing:
//! - Every log entry includes a checksum pointer to the previous entry
//! - Uses Blake3 for high performance (~3GB/s on modern CPUs)
//! - Enables "physical repair" by identifying corrupted entries via hash mismatch
//!
//! # Tiger Style Compliance
//!
//! - Fixed 32-byte Blake3 hashes (256-bit security)
//! - Constant-time comparison to prevent timing attacks
//! - Deterministic: same inputs always produce same hash
//! - No allocations after initial hasher creation

use serde::Deserialize;
use serde::Serialize;

/// Blake3 hash (32 bytes / 256 bits).
///
/// Tiger Style: Fixed size type prevents unbounded allocation.
pub type ChainHash = [u8; 32];

/// Genesis hash for the start of the chain (all zeros).
///
/// Used as the `prev_hash` for the first entry in the log. This is a well-known
/// constant that all nodes agree on, establishing a common chain root.
pub const GENESIS_HASH: ChainHash = [0u8; 32];

/// Compute chain hash for a log entry.
///
/// The hash includes:
/// - Previous hash (chain linkage, prevents reordering)
/// - Log index (prevents index substitution attacks)
/// - Term (prevents term substitution attacks)
/// - Entry data (actual payload)
///
/// # Arguments
///
/// * `prev_hash` - Hash of the previous entry (or GENESIS_HASH for first entry)
/// * `log_index` - The Raft log index of this entry
/// * `term` - The Raft term when this entry was created
/// * `entry_bytes` - Serialized entry data
///
/// # Returns
///
/// A 32-byte Blake3 hash.
///
/// # Tiger Style
///
/// - Deterministic: same inputs always produce same hash
/// - No allocations after initial hasher creation
/// - Fixed output size (32 bytes)
///
/// # Verus Verification
///
/// This function is marked with `#[verifier::external_body]` when the `verus`
/// feature is enabled. The `ensures` clause links this exec function to its
/// spec counterpart `compute_entry_hash_spec`:
///
/// ```text
/// ensures result@ == compute_entry_hash_spec(prev_hash@, log_index, term, entry_bytes@)
/// ```
///
/// This tells Verus: "Trust this implementation, but verify callers against the spec."
/// The spec proves INVARIANT 2 (Chain Continuity) - each hash correctly depends on
/// its predecessor's hash.
//
// When verus is enabled, this would have:
// #[cfg_attr(feature = "verus", verifier::external_body)]
// ensures result@ == compute_entry_hash_spec(prev_hash@, log_index, term, entry_bytes@)
pub fn compute_entry_hash(prev_hash: &ChainHash, log_index: u64, term: u64, entry_bytes: &[u8]) -> ChainHash {
    // PROOF: Chain Continuity (INVARIANT 2)
    //
    // This function implements the chain hash computation specified in
    // crates/aspen-raft/verus/chain_hash.rs:
    //
    //   compute_entry_hash_spec(prev_hash, index, term, data) =
    //       blake3_spec(prev_hash ++ u64_to_le_bytes(index) ++ u64_to_le_bytes(term) ++ data)
    //
    // The implementation below matches this spec exactly:
    // 1. Concatenates prev_hash (32 bytes) - chain linkage
    // 2. Concatenates log_index as little-endian bytes (8 bytes) - index binding
    // 3. Concatenates term as little-endian bytes (8 bytes) - term binding
    // 4. Concatenates entry_bytes - payload binding
    // 5. Returns blake3 hash of the concatenation
    //
    // This ensures each entry's hash depends on ALL previous entries (via prev_hash),
    // making any modification to the chain detectable.

    let mut hasher = blake3::Hasher::new();

    // Include previous hash (chain linkage)
    hasher.update(prev_hash);

    // Include log index and term (prevent reordering/substitution)
    hasher.update(&log_index.to_le_bytes());
    hasher.update(&term.to_le_bytes());

    // Include entry data
    hasher.update(entry_bytes);

    *hasher.finalize().as_bytes()
}

/// Verify that an entry hash matches the expected value.
///
/// Recomputes the hash and compares it to the expected hash using
/// constant-time comparison to prevent timing attacks.
///
/// # Arguments
///
/// * `prev_hash` - Hash of the previous entry
/// * `log_index` - The Raft log index of this entry
/// * `term` - The Raft term when this entry was created
/// * `entry_bytes` - Serialized entry data
/// * `expected` - The expected hash to verify against
///
/// # Returns
///
/// `true` if the computed hash matches the expected hash, `false` otherwise.
pub fn verify_entry_hash(
    prev_hash: &ChainHash,
    log_index: u64,
    term: u64,
    entry_bytes: &[u8],
    expected: &ChainHash,
) -> bool {
    let computed = compute_entry_hash(prev_hash, log_index, term, entry_bytes);
    constant_time_compare(&computed, expected)
}

/// Constant-time comparison of two hashes.
///
/// Prevents timing attacks by ensuring comparison takes the same time
/// regardless of where (if anywhere) the hashes differ.
///
/// # Tiger Style
///
/// - Fixed iteration count (always 32 iterations)
/// - No early exit on mismatch
/// - XOR accumulation prevents branch prediction leakage
#[inline]
pub fn constant_time_compare(a: &ChainHash, b: &ChainHash) -> bool {
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Cached chain tip state for efficient appends.
///
/// Maintains the current chain tip (hash and index) in memory to avoid
/// database lookups on every append operation.
#[derive(Debug, Clone)]
pub struct ChainTipState {
    /// Hash of the most recent entry (or GENESIS_HASH if empty).
    pub hash: ChainHash,
    /// Index of the most recent entry (or 0 if empty).
    pub index: u64,
}

impl Default for ChainTipState {
    fn default() -> Self {
        Self {
            hash: GENESIS_HASH,
            index: 0,
        }
    }
}

/// Snapshot integrity metadata.
///
/// Stores cryptographic hashes of snapshot data and metadata for verification
/// during snapshot installation. This prevents installing corrupted or
/// tampered snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotIntegrity {
    /// Blake3 hash of snapshot data.
    pub data_hash: ChainHash,
    /// Blake3 hash of snapshot metadata (last_log_id, membership).
    pub meta_hash: ChainHash,
    /// Combined integrity hash: blake3(data_hash || meta_hash).
    pub combined_hash: ChainHash,
    /// Log chain hash at snapshot point (for chain continuity).
    pub chain_hash_at_snapshot: ChainHash,
}

impl SnapshotIntegrity {
    /// Compute integrity hashes for a snapshot.
    ///
    /// # Arguments
    ///
    /// * `meta_bytes` - Serialized snapshot metadata
    /// * `data` - Snapshot data bytes
    /// * `chain_hash` - Current chain hash at snapshot point
    ///
    /// # Returns
    ///
    /// A `SnapshotIntegrity` struct containing all computed hashes.
    pub fn compute(meta_bytes: &[u8], data: &[u8], chain_hash: ChainHash) -> Self {
        let data_hash = *blake3::hash(data).as_bytes();
        let meta_hash = *blake3::hash(meta_bytes).as_bytes();

        let mut combined_hasher = blake3::Hasher::new();
        combined_hasher.update(&data_hash);
        combined_hasher.update(&meta_hash);
        let combined_hash = *combined_hasher.finalize().as_bytes();

        Self {
            data_hash,
            meta_hash,
            combined_hash,
            chain_hash_at_snapshot: chain_hash,
        }
    }

    /// Verify snapshot integrity.
    ///
    /// Recomputes hashes from the provided data and compares them to the
    /// stored hashes using constant-time comparison.
    ///
    /// # Arguments
    ///
    /// * `meta_bytes` - Serialized snapshot metadata
    /// * `data` - Snapshot data bytes
    ///
    /// # Returns
    ///
    /// `true` if both data and metadata hashes match, `false` otherwise.
    pub fn verify(&self, meta_bytes: &[u8], data: &[u8]) -> bool {
        self.verify_with_chain_hash(meta_bytes, data, None)
    }

    /// Verify snapshot integrity with optional chain hash verification.
    ///
    /// Recomputes hashes from the provided data and compares them to the
    /// stored hashes using constant-time comparison. Optionally verifies
    /// the chain hash at snapshot point for chain continuity.
    ///
    /// # Arguments
    ///
    /// * `meta_bytes` - Serialized snapshot metadata
    /// * `data` - Snapshot data bytes
    /// * `expected_chain_hash` - Optional chain hash for chain continuity verification
    ///
    /// # Returns
    ///
    /// `true` if all verified hashes match, `false` otherwise.
    pub fn verify_with_chain_hash(
        &self,
        meta_bytes: &[u8],
        data: &[u8],
        expected_chain_hash: Option<&ChainHash>,
    ) -> bool {
        let data_hash = *blake3::hash(data).as_bytes();
        let meta_hash = *blake3::hash(meta_bytes).as_bytes();

        let basic_valid =
            constant_time_compare(&self.data_hash, &data_hash) && constant_time_compare(&self.meta_hash, &meta_hash);

        match expected_chain_hash {
            Some(expected) => basic_valid && constant_time_compare(&self.chain_hash_at_snapshot, expected),
            None => basic_valid,
        }
    }

    /// Get the combined hash as a hex string for logging.
    pub fn combined_hash_hex(&self) -> String {
        hex::encode(self.combined_hash)
    }

    /// Get the chain hash at snapshot point.
    pub fn chain_hash_at_snapshot(&self) -> &ChainHash {
        &self.chain_hash_at_snapshot
    }
}

/// Chain corruption details for error reporting.
#[derive(Debug, Clone)]
pub struct ChainCorruption {
    /// Log index where corruption was detected.
    pub index: u64,
    /// Expected hash (from stored chain).
    pub expected: ChainHash,
    /// Found hash (recomputed from entry data).
    pub found: ChainHash,
}

impl ChainCorruption {
    /// Format corruption details as a human-readable string.
    pub fn display(&self) -> String {
        format!(
            "chain integrity violation at index {}: expected {}, found {}",
            self.index,
            hex::encode(self.expected),
            hex::encode(self.found)
        )
    }
}

/// Convert a chain hash to a hex string for display/logging.
#[inline]
pub fn hash_to_hex(hash: &ChainHash) -> String {
    hex::encode(hash)
}

/// Parse a hex string back to a chain hash.
///
/// Returns `None` if the hex string is invalid or wrong length.
pub fn hash_from_hex(hex_str: &str) -> Option<ChainHash> {
    let bytes = hex::decode(hex_str).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Some(hash)
}

// ============================================================================
// Chain Hash Helper Functions (ported from verus specs for sync parity)
// ============================================================================

/// Check if a hash has the expected size.
///
/// # Arguments
///
/// * `hash_len` - Length of the hash
///
/// # Returns
///
/// `true` if hash has the expected 32-byte size.
#[inline]
#[allow(dead_code)]
pub const fn is_valid_hash_size(hash_len: u64) -> bool {
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
#[inline]
#[allow(dead_code)]
pub const fn compute_hash_input_size(prev_hash_len: u64, data_len: u64) -> u64 {
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
#[inline]
#[allow(dead_code)]
pub const fn is_genesis_index(index: u64) -> bool {
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
#[inline]
#[allow(dead_code)]
pub const fn compute_prev_index(index: u64) -> u64 {
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
#[inline]
#[allow(dead_code)]
pub const fn is_valid_chain_range(first_index: u64, last_index: u64) -> bool {
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
#[inline]
#[allow(dead_code)]
pub fn compute_chain_length(first_index: u64, last_index: u64) -> u64 {
    if first_index <= last_index {
        (last_index - first_index).saturating_add(1)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_hash_is_zeroes() {
        assert_eq!(GENESIS_HASH, [0u8; 32]);
    }

    #[test]
    fn test_hash_computation_is_deterministic() {
        let prev_hash = GENESIS_HASH;
        let entry_data = b"test entry data";

        let hash1 = compute_entry_hash(&prev_hash, 1, 1, entry_data);
        let hash2 = compute_entry_hash(&prev_hash, 1, 1, entry_data);

        assert_eq!(hash1, hash2, "hashing must be deterministic");
    }

    #[test]
    fn test_different_inputs_produce_different_hashes() {
        let prev_hash = GENESIS_HASH;

        let hash1 = compute_entry_hash(&prev_hash, 1, 1, b"entry 1");
        let hash2 = compute_entry_hash(&prev_hash, 1, 1, b"entry 2");

        assert_ne!(hash1, hash2, "different data should produce different hashes");
    }

    #[test]
    fn test_chain_linkage() {
        let hash1 = compute_entry_hash(&GENESIS_HASH, 1, 1, b"entry 1");
        let hash2 = compute_entry_hash(&hash1, 2, 1, b"entry 2");
        let hash3 = compute_entry_hash(&hash2, 3, 1, b"entry 3");

        // Each hash should be unique
        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);

        // Verify chain
        assert!(verify_entry_hash(&GENESIS_HASH, 1, 1, b"entry 1", &hash1));
        assert!(verify_entry_hash(&hash1, 2, 1, b"entry 2", &hash2));
        assert!(verify_entry_hash(&hash2, 3, 1, b"entry 3", &hash3));
    }

    #[test]
    fn test_corruption_detection() {
        let hash1 = compute_entry_hash(&GENESIS_HASH, 1, 1, b"original data");

        // Verify corruption is detected
        assert!(!verify_entry_hash(&GENESIS_HASH, 1, 1, b"corrupted data", &hash1));
    }

    #[test]
    fn test_index_matters() {
        let prev_hash = GENESIS_HASH;
        let data = b"same data";

        let hash1 = compute_entry_hash(&prev_hash, 1, 1, data);
        let hash2 = compute_entry_hash(&prev_hash, 2, 1, data);

        assert_ne!(hash1, hash2, "different indices should produce different hashes");
    }

    #[test]
    fn test_term_matters() {
        let prev_hash = GENESIS_HASH;
        let data = b"same data";

        let hash1 = compute_entry_hash(&prev_hash, 1, 1, data);
        let hash2 = compute_entry_hash(&prev_hash, 1, 2, data);

        assert_ne!(hash1, hash2, "different terms should produce different hashes");
    }

    #[test]
    fn test_prev_hash_matters() {
        let data = b"same data";
        let hash1 = compute_entry_hash(&GENESIS_HASH, 1, 1, data);

        let different_prev = [1u8; 32];
        let hash2 = compute_entry_hash(&different_prev, 1, 1, data);

        assert_ne!(hash1, hash2, "different prev_hash should produce different hashes");
    }

    #[test]
    fn test_constant_time_compare_equal() {
        let a = [1u8; 32];
        let b = [1u8; 32];
        assert!(constant_time_compare(&a, &b));
    }

    #[test]
    fn test_constant_time_compare_not_equal() {
        let a = [1u8; 32];
        let mut b = [1u8; 32];
        b[31] = 2; // Differ at last byte
        assert!(!constant_time_compare(&a, &b));
    }

    #[test]
    fn test_chain_tip_state_default() {
        let tip = ChainTipState::default();
        assert_eq!(tip.hash, GENESIS_HASH);
        assert_eq!(tip.index, 0);
    }

    #[test]
    fn test_snapshot_integrity_compute_and_verify() {
        let meta_bytes = b"snapshot metadata";
        let data = b"snapshot data content";
        let chain_hash = [42u8; 32];

        let integrity = SnapshotIntegrity::compute(meta_bytes, data, chain_hash);

        assert!(integrity.verify(meta_bytes, data));
        assert!(!integrity.verify(b"wrong metadata", data));
        assert!(!integrity.verify(meta_bytes, b"wrong data"));
    }

    #[test]
    fn test_snapshot_integrity_verify_with_chain_hash() {
        let meta_bytes = b"snapshot metadata";
        let data = b"snapshot data content";
        let chain_hash = [42u8; 32];
        let wrong_chain_hash = [99u8; 32];

        let integrity = SnapshotIntegrity::compute(meta_bytes, data, chain_hash);

        // Basic verify still works (backward compatible)
        assert!(integrity.verify(meta_bytes, data));

        // Verify with correct chain hash
        assert!(integrity.verify_with_chain_hash(meta_bytes, data, Some(&chain_hash)));

        // Verify with wrong chain hash fails
        assert!(!integrity.verify_with_chain_hash(meta_bytes, data, Some(&wrong_chain_hash)));

        // Verify with None chain hash (backward compatible)
        assert!(integrity.verify_with_chain_hash(meta_bytes, data, None));

        // Chain hash accessor
        assert_eq!(integrity.chain_hash_at_snapshot(), &chain_hash);
    }

    #[test]
    fn test_hash_hex_roundtrip() {
        let original = [0xAB; 32];
        let hex_str = hash_to_hex(&original);
        let recovered = hash_from_hex(&hex_str).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_hash_from_hex_invalid() {
        assert!(hash_from_hex("not hex").is_none());
        assert!(hash_from_hex("ab").is_none()); // Too short
        assert!(hash_from_hex(&"ab".repeat(33)).is_none()); // Too long
    }

    #[test]
    fn test_chain_corruption_display() {
        let corruption = ChainCorruption {
            index: 42,
            expected: [0xAA; 32],
            found: [0xBB; 32],
        };
        let msg = corruption.display();
        assert!(msg.contains("42"));
        assert!(msg.contains("aa".repeat(32).as_str()));
        assert!(msg.contains("bb".repeat(32).as_str()));
    }
}
