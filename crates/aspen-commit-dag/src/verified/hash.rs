//! Pure chain-hash helpers owned by the branch/DAG family.
//!
//! These helpers intentionally mirror the BLAKE3/hex helper surface that the
//! commit DAG needs without depending on Aspen's Raft compatibility shell.

/// Number of bytes in a BLAKE3 chain hash.
pub const CHAIN_HASH_BYTES: usize = 32;

/// BLAKE3 chain hash used for commit IDs and mutation hashes.
pub type ChainHash = [u8; CHAIN_HASH_BYTES];

/// Genesis hash used when a commit has no parent.
pub const GENESIS_HASH: ChainHash = [0u8; CHAIN_HASH_BYTES];

/// Compare two chain hashes without early-exit on the first differing byte.
#[inline]
pub fn constant_time_compare(left: &ChainHash, right: &ChainHash) -> bool {
    let mut difference = 0u8;
    for (&left_byte, &right_byte) in left.iter().zip(right.iter()) {
        difference |= left_byte ^ right_byte;
    }
    difference == 0
}

/// Convert a chain hash to lowercase hexadecimal.
#[inline]
pub fn hash_to_hex(hash: &ChainHash) -> String {
    hex::encode(hash)
}

/// Parse a hexadecimal chain hash.
#[inline]
pub fn hash_from_hex(hex_str: &str) -> Option<ChainHash> {
    let bytes = hex::decode(hex_str).ok()?;
    if bytes.len() != CHAIN_HASH_BYTES {
        return None;
    }

    let mut hash = [0u8; CHAIN_HASH_BYTES];
    hash.copy_from_slice(&bytes);
    Some(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_compare_accepts_equal_hashes() {
        let left = [0xAB; CHAIN_HASH_BYTES];
        let right = [0xAB; CHAIN_HASH_BYTES];

        assert!(constant_time_compare(&left, &right));
    }

    #[test]
    fn constant_time_compare_rejects_different_hashes() {
        let left = [0xAB; CHAIN_HASH_BYTES];
        let right = [0xAC; CHAIN_HASH_BYTES];

        assert!(!constant_time_compare(&left, &right));
    }

    #[test]
    fn hash_hex_roundtrip_accepts_valid_hash() {
        let hash = [0xCD; CHAIN_HASH_BYTES];
        let encoded = hash_to_hex(&hash);

        assert_eq!(hash_from_hex(&encoded), Some(hash));
    }

    #[test]
    fn hash_from_hex_rejects_malformed_input() {
        assert_eq!(hash_from_hex("not hex"), None);
    }

    #[test]
    fn hash_from_hex_rejects_wrong_length_input() {
        let too_short = "ab";
        let too_long = "ab".repeat(CHAIN_HASH_BYTES + 1);

        assert_eq!(hash_from_hex(too_short), None);
        assert_eq!(hash_from_hex(&too_long), None);
    }
}
