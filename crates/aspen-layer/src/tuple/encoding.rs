use super::INT_SIZE_LIMITS;
use super::INT_ZERO_CODE;

// =============================================================================
// Encoding Functions
// =============================================================================

/// Encode bytes with null byte escaping.
///
/// Null bytes (0x00) are encoded as (0x00, 0xFF) to preserve ordering.
pub(super) fn encode_bytes_with_null_escaping(bytes: &[u8], buf: &mut Vec<u8>) {
    for &b in bytes {
        if b == 0x00 {
            buf.push(0x00);
            buf.push(super::NULL_ESCAPE);
        } else {
            buf.push(b);
        }
    }
}

/// Encode an integer using FDB's variable-length scheme.
///
/// - Zero: 0x14
/// - Positive: 0x14 + size, then big-endian bytes
/// - Negative: 0x14 - size, then one's complement big-endian bytes
pub(super) fn encode_int(n: i64, buf: &mut Vec<u8>) {
    if n == 0 {
        buf.push(INT_ZERO_CODE);
        return;
    }

    if n > 0 {
        let n = n as u64;
        let size = int_size(n);
        buf.push(INT_ZERO_CODE + size);
        encode_uint_be(n, size, buf);
    } else {
        // Negative: use one's complement encoding
        let abs = if n == i64::MIN {
            // Special case: i64::MIN has no positive counterpart
            (i64::MAX as u64) + 1
        } else {
            (-n) as u64
        };
        let size = int_size(abs);
        buf.push(INT_ZERO_CODE - size);
        // One's complement: flip all bits
        let complement = !abs;
        // We need to mask to the correct number of bytes
        let mask = if size == 8 { u64::MAX } else { (1u64 << (size * 8)) - 1 };
        encode_uint_be(complement & mask, size, buf);
    }
}

/// Determine the number of bytes needed to encode a positive integer.
fn int_size(n: u64) -> u8 {
    for (i, &limit) in INT_SIZE_LIMITS.iter().enumerate() {
        if n <= limit {
            return (i + 1) as u8;
        }
    }
    8
}

/// Encode an unsigned integer in big-endian format with specified size.
fn encode_uint_be(n: u64, size: u8, buf: &mut Vec<u8>) {
    let bytes = n.to_be_bytes();
    let start = 8 - size as usize;
    buf.extend_from_slice(&bytes[start..]);
}

/// Encode a 32-bit float using FDB's ordering scheme.
///
/// Floats are transformed so that lexicographic byte ordering matches
/// numeric ordering. This involves flipping bits based on the sign.
pub(super) fn encode_float(f: f32, buf: &mut Vec<u8>) {
    let bits = f.to_bits();
    let transformed = if (bits & 0x8000_0000) != 0 {
        // Negative: flip all bits
        !bits
    } else {
        // Positive: flip sign bit only
        bits ^ 0x8000_0000
    };
    buf.extend_from_slice(&transformed.to_be_bytes());
}

/// Encode a 64-bit double using FDB's ordering scheme.
pub(super) fn encode_double(f: f64, buf: &mut Vec<u8>) {
    let bits = f.to_bits();
    let transformed = if (bits & 0x8000_0000_0000_0000) != 0 {
        // Negative: flip all bits
        !bits
    } else {
        // Positive: flip sign bit only
        bits ^ 0x8000_0000_0000_0000
    };
    buf.extend_from_slice(&transformed.to_be_bytes());
}
