//! Pure shard prefix encoding functions.
//!
//! This module contains pure functions for encoding and decoding shard prefixes
//! in RPC wire format. All functions are deterministic and side-effect free.
//!
//! # Wire Format
//!
//! Shard prefixes are 4-byte big-endian u32 values prepended to RPC messages
//! to route them to the appropriate shard.
//!
//! # Tiger Style
//!
//! - Fixed-size encoding (4 bytes)
//! - Deterministic encoding/decoding
//! - No allocations

/// Size of shard prefix in bytes (4 bytes for u32 big-endian).
pub const SHARD_PREFIX_SIZE: u32 = 4;

// ============================================================================
// Encoding
// ============================================================================

/// Encode a shard ID as a 4-byte big-endian prefix.
///
/// # Arguments
///
/// * `shard_id` - The shard ID to encode
///
/// # Returns
///
/// A 4-byte array containing the big-endian encoding of the shard ID.
///
/// # Example
///
/// ```ignore
/// let prefix = encode_shard_prefix(256);
/// assert_eq!(prefix, [0, 0, 1, 0]); // 256 in big-endian
/// ```
///
/// # Tiger Style
///
/// - Fixed-size output (4 bytes)
/// - Deterministic (same input always produces same output)
/// - No allocations
#[inline]
pub fn encode_shard_prefix(shard_id: u32) -> [u8; 4] {
    shard_id.to_be_bytes()
}

// ============================================================================
// Decoding
// ============================================================================

/// Try to decode a shard ID from bytes.
///
/// # Arguments
///
/// * `bytes` - The byte slice to decode from (must be at least 4 bytes)
///
/// # Returns
///
/// `Some(shard_id)` if decoding succeeded, `None` if the slice is too short.
///
/// # Example
///
/// ```ignore
/// let bytes = [0, 0, 1, 0];
/// assert_eq!(try_decode_shard_prefix(&bytes), Some(256));
///
/// let short = [0, 0];
/// assert_eq!(try_decode_shard_prefix(&short), None);
/// ```
///
/// # Tiger Style
///
/// - Returns None for insufficient input (no panics)
/// - Deterministic decoding
/// - Only reads first 4 bytes
#[inline]
pub fn try_decode_shard_prefix(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < SHARD_PREFIX_SIZE as usize {
        return None;
    }
    // SAFETY: We've verified length is at least 4
    let prefix: [u8; 4] = bytes[..4].try_into().ok()?;
    Some(u32::from_be_bytes(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Encoding Tests
    // ========================================================================

    #[test]
    fn test_encode_zero() {
        assert_eq!(encode_shard_prefix(0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_one() {
        assert_eq!(encode_shard_prefix(1), [0, 0, 0, 1]);
    }

    #[test]
    fn test_encode_256() {
        assert_eq!(encode_shard_prefix(256), [0, 0, 1, 0]);
    }

    #[test]
    fn test_encode_max() {
        assert_eq!(encode_shard_prefix(u32::MAX), [0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_encode_typical_shard() {
        // Shard 42 - typical usage
        assert_eq!(encode_shard_prefix(42), [0, 0, 0, 42]);
    }

    #[test]
    fn test_encode_large_shard() {
        // 0x01020304 = 16909060
        assert_eq!(encode_shard_prefix(0x01020304), [0x01, 0x02, 0x03, 0x04]);
    }

    // ========================================================================
    // Decoding Tests
    // ========================================================================

    #[test]
    fn test_decode_zero() {
        assert_eq!(try_decode_shard_prefix(&[0, 0, 0, 0]), Some(0));
    }

    #[test]
    fn test_decode_one() {
        assert_eq!(try_decode_shard_prefix(&[0, 0, 0, 1]), Some(1));
    }

    #[test]
    fn test_decode_256() {
        assert_eq!(try_decode_shard_prefix(&[0, 0, 1, 0]), Some(256));
    }

    #[test]
    fn test_decode_max() {
        assert_eq!(try_decode_shard_prefix(&[0xFF, 0xFF, 0xFF, 0xFF]), Some(u32::MAX));
    }

    #[test]
    fn test_decode_too_short() {
        assert_eq!(try_decode_shard_prefix(&[]), None);
        assert_eq!(try_decode_shard_prefix(&[0]), None);
        assert_eq!(try_decode_shard_prefix(&[0, 0]), None);
        assert_eq!(try_decode_shard_prefix(&[0, 0, 0]), None);
    }

    #[test]
    fn test_decode_exact_length() {
        assert_eq!(try_decode_shard_prefix(&[0, 0, 0, 42]), Some(42));
    }

    #[test]
    fn test_decode_extra_bytes_ignored() {
        // Only first 4 bytes matter
        assert_eq!(try_decode_shard_prefix(&[0, 0, 0, 42, 0xFF, 0xFF, 0xFF]), Some(42));
    }

    // ========================================================================
    // Roundtrip Tests
    // ========================================================================

    #[test]
    fn test_roundtrip_zero() {
        let encoded = encode_shard_prefix(0);
        assert_eq!(try_decode_shard_prefix(&encoded), Some(0));
    }

    #[test]
    fn test_roundtrip_max() {
        let encoded = encode_shard_prefix(u32::MAX);
        assert_eq!(try_decode_shard_prefix(&encoded), Some(u32::MAX));
    }

    #[test]
    fn test_roundtrip_typical() {
        for shard_id in [0, 1, 42, 255, 256, 65535, 1000000, u32::MAX] {
            let encoded = encode_shard_prefix(shard_id);
            assert_eq!(try_decode_shard_prefix(&encoded), Some(shard_id), "roundtrip failed for {}", shard_id);
        }
    }
}
