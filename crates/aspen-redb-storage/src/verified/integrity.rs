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
pub fn compute_entry_hash(prev_hash: &ChainHash, log_index: u64, term: u64, entry_bytes: &[u8]) -> ChainHash {
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
        let log_index = 1;
        let term = 1;
        let entry_bytes = b"test entry";

        let hash1 = compute_entry_hash(&prev_hash, log_index, term, entry_bytes);
        let hash2 = compute_entry_hash(&prev_hash, log_index, term, entry_bytes);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_inputs_produce_different_hashes() {
        let prev_hash = GENESIS_HASH;
        let term = 1;
        let entry_bytes = b"test entry";

        let hash1 = compute_entry_hash(&prev_hash, 1, term, entry_bytes);
        let hash2 = compute_entry_hash(&prev_hash, 2, term, entry_bytes);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_entry_hash_success() {
        let prev_hash = GENESIS_HASH;
        let log_index = 1;
        let term = 1;
        let entry_bytes = b"test entry";

        let hash = compute_entry_hash(&prev_hash, log_index, term, entry_bytes);
        assert!(verify_entry_hash(&prev_hash, log_index, term, entry_bytes, &hash));
    }

    #[test]
    fn test_verify_entry_hash_failure() {
        let prev_hash = GENESIS_HASH;
        let log_index = 1;
        let term = 1;
        let entry_bytes = b"test entry";
        let wrong_hash = [1u8; 32];

        assert!(!verify_entry_hash(&prev_hash, log_index, term, entry_bytes, &wrong_hash));
    }

    #[test]
    fn test_constant_time_compare_equal() {
        let hash1 = [1u8; 32];
        let hash2 = [1u8; 32];
        assert!(constant_time_compare(&hash1, &hash2));
    }

    #[test]
    fn test_constant_time_compare_different() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        assert!(!constant_time_compare(&hash1, &hash2));
    }

    #[test]
    fn test_chain_tip_state_default() {
        let state = ChainTipState::default();
        assert_eq!(state.hash, GENESIS_HASH);
        assert_eq!(state.index, 0);
    }

    #[test]
    fn test_hash_to_hex() {
        let hash = GENESIS_HASH;
        let hex = hash_to_hex(&hash);
        assert_eq!(hex, "0000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_hash_from_hex_valid() {
        let hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let hash = hash_from_hex(hex);
        assert_eq!(hash, Some(GENESIS_HASH));
    }

    #[test]
    fn test_hash_from_hex_invalid() {
        let hex = "not a valid hex string";
        let hash = hash_from_hex(hex);
        assert_eq!(hash, None);
    }

    #[test]
    fn test_hash_from_hex_wrong_length() {
        let hex = "0000000000"; // Too short
        let hash = hash_from_hex(hex);
        assert_eq!(hash, None);
    }

    #[test]
    fn test_snapshot_integrity_compute_and_verify() {
        let meta_bytes = b"snapshot metadata";
        let data = b"snapshot data";
        let chain_hash = [42u8; 32];

        let integrity = SnapshotIntegrity::compute(meta_bytes, data, chain_hash);
        assert!(integrity.verify(meta_bytes, data));
    }

    #[test]
    fn test_snapshot_integrity_verify_failure() {
        let meta_bytes = b"snapshot metadata";
        let data = b"snapshot data";
        let chain_hash = [42u8; 32];

        let integrity = SnapshotIntegrity::compute(meta_bytes, data, chain_hash);
        assert!(!integrity.verify(meta_bytes, b"different data"));
    }

    #[test]
    fn test_snapshot_integrity_verify_with_chain_hash() {
        let meta_bytes = b"snapshot metadata";
        let data = b"snapshot data";
        let chain_hash = [42u8; 32];

        let integrity = SnapshotIntegrity::compute(meta_bytes, data, chain_hash);
        assert!(integrity.verify_with_chain_hash(meta_bytes, data, Some(&chain_hash)));
        assert!(!integrity.verify_with_chain_hash(meta_bytes, data, Some(&[0u8; 32])));
    }

    #[test]
    fn test_chain_corruption_display() {
        let corruption = ChainCorruption {
            index: 42,
            expected: [1u8; 32],
            found: [2u8; 32],
        };
        let msg = corruption.display();
        assert!(msg.contains("42"));
        assert!(msg.contains("chain integrity violation"));
    }
}
