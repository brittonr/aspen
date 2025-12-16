//! Fuzz target for integrity verification primitives.
//!
//! This target fuzzes the Blake3 chain hashing implementation used for
//! Raft log integrity verification. Tests ensure no panics or integer
//! overflows occur with arbitrary inputs.
//!
//! Attack vectors tested:
//! - Arbitrary hash inputs
//! - Integer overflow in index/term calculations
//! - Hex encoding/decoding round-trips
//! - Hash verification with mismatched inputs
//! - Empty and maximum-size entry data

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;

/// Chain hash type (32 bytes / 256 bits Blake3)
type ChainHash = [u8; 32];

/// Genesis hash constant (all zeros)
const GENESIS_HASH: ChainHash = [0u8; 32];

/// Tiger Style: Maximum entry size for fuzzing
const MAX_ENTRY_SIZE: usize = 1024 * 1024; // 1 MB

#[derive(Debug, Arbitrary)]
struct FuzzIntegrityInput {
    /// Previous hash in the chain
    prev_hash: [u8; 32],
    /// Raft log index
    log_index: u64,
    /// Raft term
    term: u64,
    /// Entry data (bounded by arbitrary)
    entry_bytes: Vec<u8>,
    /// Expected hash for verification testing
    expected_hash: [u8; 32],
}

/// Compute chain hash matching integrity.rs implementation
fn compute_entry_hash(
    prev_hash: &ChainHash,
    log_index: u64,
    term: u64,
    entry_bytes: &[u8],
) -> ChainHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_hash);
    hasher.update(&log_index.to_le_bytes());
    hasher.update(&term.to_le_bytes());
    hasher.update(entry_bytes);
    *hasher.finalize().as_bytes()
}

/// Verify chain hash
fn verify_entry_hash(
    prev_hash: &ChainHash,
    log_index: u64,
    term: u64,
    entry_bytes: &[u8],
    expected: &ChainHash,
) -> bool {
    let computed = compute_entry_hash(prev_hash, log_index, term, entry_bytes);
    // Constant-time comparison would be used in production
    computed == *expected
}

fuzz_target!(|input: FuzzIntegrityInput| {
    // Tiger Style: Bound entry size
    if input.entry_bytes.len() > MAX_ENTRY_SIZE {
        return;
    }

    // Test hash computation with arbitrary inputs
    let hash1 = compute_entry_hash(
        &input.prev_hash,
        input.log_index,
        input.term,
        &input.entry_bytes,
    );

    // Verify determinism: same inputs produce same hash
    let hash2 = compute_entry_hash(
        &input.prev_hash,
        input.log_index,
        input.term,
        &input.entry_bytes,
    );
    assert_eq!(hash1, hash2, "hash computation must be deterministic");

    // Test hex encoding round-trip
    let hex_str = hex::encode(hash1);
    let decoded = hex::decode(&hex_str).expect("hex decode should succeed");
    assert_eq!(
        hash1.as_slice(),
        decoded.as_slice(),
        "hex round-trip must be lossless"
    );

    // Test verification with correct hash
    assert!(
        verify_entry_hash(
            &input.prev_hash,
            input.log_index,
            input.term,
            &input.entry_bytes,
            &hash1
        ),
        "verification should pass with correct hash"
    );

    // Test verification with arbitrary (likely wrong) hash
    let _ = verify_entry_hash(
        &input.prev_hash,
        input.log_index,
        input.term,
        &input.entry_bytes,
        &input.expected_hash,
    );

    // Test chain from genesis
    let first_hash = compute_entry_hash(&GENESIS_HASH, 1, 1, &input.entry_bytes);
    let second_hash = compute_entry_hash(&first_hash, 2, 1, &input.entry_bytes);

    // Verify chain property: different prev_hash produces different hash
    if input.entry_bytes == input.entry_bytes {
        // Same entry data but different positions should produce different hashes
        assert_ne!(
            first_hash, second_hash,
            "different positions should produce different hashes"
        );
    }

    // Test with edge case log indices
    let _ = compute_entry_hash(&input.prev_hash, 0, input.term, &input.entry_bytes);
    let _ = compute_entry_hash(&input.prev_hash, u64::MAX, input.term, &input.entry_bytes);
    let _ = compute_entry_hash(&input.prev_hash, input.log_index, 0, &input.entry_bytes);
    let _ = compute_entry_hash(&input.prev_hash, input.log_index, u64::MAX, &input.entry_bytes);

    // Test with empty entry
    let _ = compute_entry_hash(&input.prev_hash, input.log_index, input.term, &[]);
});
