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
    assert!(buf.len() <= buf.capacity(), "buffer length must stay within capacity");
    if n == 0 {
        buf.push(INT_ZERO_CODE);
        return;
    }

    if n > 0 {
        let value = n as u64;
        let size_bytes = int_size(value);
        assert!(size_bytes > 0, "positive integers must use at least one byte");
        buf.push(INT_ZERO_CODE.saturating_add(size_bytes));
        encode_uint_be(value, size_bytes, buf);
        return;
    }

    let abs = if n == i64::MIN {
        // Special case: i64::MIN has no positive counterpart.
        (i64::MAX as u64).saturating_add(1)
    } else {
        (-n) as u64
    };
    let size_bytes = int_size(abs);
    assert!(size_bytes > 0, "negative integers must use at least one byte");
    buf.push(INT_ZERO_CODE.saturating_sub(size_bytes));
    let complement = !abs;
    let shift_bits = usize::from(size_bytes).saturating_mul(8);
    let mask = if size_bytes == 8 {
        u64::MAX
    } else {
        (1u64 << shift_bits).saturating_sub(1)
    };
    encode_uint_be(complement & mask, size_bytes, buf);
}

/// Determine the number of bytes needed to encode a positive integer.
fn int_size(n: u64) -> u8 {
    for (index, &limit) in INT_SIZE_LIMITS.iter().enumerate() {
        if n <= limit {
            let size_bytes = u8::try_from(index).ok().and_then(|value| value.checked_add(1)).unwrap_or(8);
            return size_bytes;
        }
    }
    8
}

/// Encode an unsigned integer in big-endian format with specified size.
fn encode_uint_be(n: u64, size_bytes: u8, buf: &mut Vec<u8>) {
    let bytes = n.to_be_bytes();
    let start_index = 8_usize.saturating_sub(usize::from(size_bytes));
    buf.extend_from_slice(&bytes[start_index..]);
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
