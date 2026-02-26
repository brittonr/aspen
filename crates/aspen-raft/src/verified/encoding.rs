//! Pure encoding/decoding functions for wire protocols.
//!
//! This module contains pure functions for encoding and decoding binary
//! data used in Raft RPC protocols. All functions are deterministic
//! and side-effect free.
//!
//! # Tiger Style
//!
//! - Fixed size encodings (4 bytes for shard prefix)
//! - Deterministic: same inputs always produce same outputs
//! - No allocations for fixed-size operations

/// Shard ID prefix size in bytes.
///
/// The shard ID is encoded as a 4-byte big-endian u32 at the start of each
/// sharded RPC message. This allows routing to the correct Raft core based
/// on the shard ID before deserializing the full message.
pub const SHARD_PREFIX_SIZE: usize = 4;

/// Encode a shard ID as a 4-byte big-endian prefix.
///
/// This is the wire format used for sharded RPC routing. The receiver
/// can read these 4 bytes to determine which shard should handle the
/// RPC before deserializing the full message.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::encode_shard_prefix;
///
/// let shard_id = 42u32;
/// let prefix = encode_shard_prefix(shard_id);
/// assert_eq!(prefix, [0, 0, 0, 42]);
/// ```
#[inline]
pub fn encode_shard_prefix(shard_id: u32) -> [u8; SHARD_PREFIX_SIZE] {
    shard_id.to_be_bytes()
}

/// Decode a shard ID from a 4-byte big-endian prefix.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::decode_shard_prefix;
///
/// let prefix = [0, 0, 0, 42];
/// let shard_id = decode_shard_prefix(&prefix);
/// assert_eq!(shard_id, 42);
/// ```
#[inline]
pub fn decode_shard_prefix(bytes: &[u8; SHARD_PREFIX_SIZE]) -> u32 {
    u32::from_be_bytes(*bytes)
}

/// Try to decode a shard ID from a byte slice.
///
/// Returns `None` if the slice is shorter than `SHARD_PREFIX_SIZE` bytes.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::try_decode_shard_prefix;
///
/// let data = vec![0, 0, 0, 42, 1, 2, 3, 4]; // shard_id followed by other data
/// let shard_id = try_decode_shard_prefix(&data);
/// assert_eq!(shard_id, Some(42));
///
/// let short_data = vec![0, 0, 0]; // only 3 bytes
/// assert_eq!(try_decode_shard_prefix(&short_data), None);
/// ```
#[inline]
pub fn try_decode_shard_prefix(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < SHARD_PREFIX_SIZE {
        return None;
    }
    let prefix: [u8; SHARD_PREFIX_SIZE] = bytes[..SHARD_PREFIX_SIZE].try_into().ok()?;
    Some(decode_shard_prefix(&prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Shard Prefix Encoding/Decoding Tests
    // =========================================================================

    #[test]
    fn test_shard_prefix_size() {
        assert_eq!(SHARD_PREFIX_SIZE, 4);
    }

    #[test]
    fn test_encode_shard_prefix_zero() {
        let prefix = encode_shard_prefix(0);
        assert_eq!(prefix, [0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_shard_prefix_small() {
        let prefix = encode_shard_prefix(42);
        assert_eq!(prefix, [0, 0, 0, 42]);
    }

    #[test]
    fn test_encode_shard_prefix_256() {
        let prefix = encode_shard_prefix(256);
        assert_eq!(prefix, [0, 0, 1, 0]);
    }

    #[test]
    fn test_encode_shard_prefix_large() {
        let prefix = encode_shard_prefix(0x12345678);
        assert_eq!(prefix, [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_encode_shard_prefix_max() {
        let prefix = encode_shard_prefix(u32::MAX);
        assert_eq!(prefix, [0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_decode_shard_prefix_zero() {
        let shard_id = decode_shard_prefix(&[0, 0, 0, 0]);
        assert_eq!(shard_id, 0);
    }

    #[test]
    fn test_decode_shard_prefix_small() {
        let shard_id = decode_shard_prefix(&[0, 0, 0, 42]);
        assert_eq!(shard_id, 42);
    }

    #[test]
    fn test_decode_shard_prefix_large() {
        let shard_id = decode_shard_prefix(&[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(shard_id, 0x12345678);
    }

    #[test]
    fn test_decode_shard_prefix_max() {
        let shard_id = decode_shard_prefix(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(shard_id, u32::MAX);
    }

    #[test]
    fn test_shard_prefix_roundtrip() {
        for shard_id in [0, 1, 42, 255, 256, 1000, 65535, 0x12345678, u32::MAX] {
            let encoded = encode_shard_prefix(shard_id);
            let decoded = decode_shard_prefix(&encoded);
            assert_eq!(decoded, shard_id);
        }
    }

    #[test]
    fn test_try_decode_shard_prefix_success() {
        let data = vec![0, 0, 0, 42, 1, 2, 3, 4]; // shard_id followed by other data
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_try_decode_shard_prefix_exact_size() {
        let data = vec![0, 0, 1, 0]; // exactly 4 bytes
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, Some(256));
    }

    #[test]
    fn test_try_decode_shard_prefix_too_short() {
        let data = vec![0, 0, 0]; // only 3 bytes
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_decode_shard_prefix_empty() {
        let data: Vec<u8> = vec![];
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_decode_shard_prefix_one_byte() {
        let data = vec![42];
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, None);
    }
}
