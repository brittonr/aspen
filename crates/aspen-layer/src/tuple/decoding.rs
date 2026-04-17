use snafu::ResultExt;

use super::BYTES_CODE;
use super::DOUBLE_CODE;
use super::FALSE_CODE;
use super::FLOAT_CODE;
use super::INT_ZERO_CODE;
use super::InvalidUtf8Snafu;
use super::NESTED_CODE;
use super::NULL_CODE;
use super::NULL_ESCAPE;
use super::STRING_CODE;
use super::TRUE_CODE;
use super::TupleError;
use super::element::Element;
use super::tuple_type::Tuple;

// =============================================================================
// Decoding Functions
// =============================================================================

/// Decode a single element from bytes at the given offset.
///
/// Returns the decoded element and the number of bytes consumed.
pub(super) fn decode_element(data: &[u8], offset: usize) -> Result<(Element, usize), TupleError> {
    if offset >= data.len() {
        return Err(TupleError::UnexpectedEnd { offset });
    }
    assert!(offset < data.len(), "offset must be in bounds after early return");

    let code = data[offset];
    let decode_ctx = ElementOffsets::new(offset);
    assert!(decode_ctx.payload_offset_bytes > offset, "payload offset must advance past type code");

    match code {
        NULL_CODE => Ok((Element::Null, 1)),
        BYTES_CODE => decode_bytes_element(data, decode_ctx.payload_offset_bytes),
        STRING_CODE => decode_string_element(data, decode_ctx),
        NESTED_CODE => decode_tuple_element(data, decode_ctx.payload_offset_bytes),
        FALSE_CODE => Ok((Element::Bool(false), 1)),
        TRUE_CODE => Ok((Element::Bool(true), 1)),
        FLOAT_CODE => decode_float_element(data, decode_ctx),
        DOUBLE_CODE => decode_double_element(data, decode_ctx),
        code if (0x0C..=0x1C).contains(&code) => {
            let (n, consumed) = decode_int(data, offset)?;
            Ok((Element::Int(n), consumed))
        }
        _ => Err(TupleError::UnknownTypeCode { code, offset }),
    }
}

fn decode_bytes_element(data: &[u8], payload_offset_bytes: usize) -> Result<(Element, usize), TupleError> {
    let (bytes, consumed) = decode_bytes_with_null_escaping(data, payload_offset_bytes)?;
    let total_consumed = consumed.saturating_add(1);
    assert!(total_consumed > consumed, "element must include type code byte");
    Ok((Element::Bytes(bytes), total_consumed))
}

fn decode_string_element(data: &[u8], offsets: ElementOffsets) -> Result<(Element, usize), TupleError> {
    let (bytes, consumed) = decode_bytes_with_null_escaping(data, offsets.payload_offset_bytes)?;
    let value = std::str::from_utf8(&bytes)
        .context(InvalidUtf8Snafu {
            offset: offsets.element_offset_bytes,
        })?
        .to_string();
    let total_consumed = consumed.saturating_add(1);
    assert!(total_consumed > consumed, "string element must include type code byte");
    Ok((Element::String(value), total_consumed))
}

fn decode_tuple_element(data: &[u8], payload_offset: usize) -> Result<(Element, usize), TupleError> {
    let (tuple, consumed) = decode_nested_tuple(data, payload_offset)?;
    let total_consumed = consumed.saturating_add(1);
    assert!(total_consumed > consumed, "tuple element must include type code byte");
    Ok((Element::Tuple(tuple), total_consumed))
}

fn decode_float_element(data: &[u8], offsets: ElementOffsets) -> Result<(Element, usize), TupleError> {
    let bytes =
        read_exact_bytes(data, DecodeRange::new(offsets.payload_offset_bytes, 4, offsets.element_offset_bytes))?;
    Ok((Element::Float(decode_float(bytes)), 5))
}

fn decode_double_element(data: &[u8], offsets: ElementOffsets) -> Result<(Element, usize), TupleError> {
    let bytes =
        read_exact_bytes(data, DecodeRange::new(offsets.payload_offset_bytes, 8, offsets.element_offset_bytes))?;
    Ok((Element::Double(decode_double(bytes)), 9))
}

fn read_exact_bytes(data: &[u8], range: DecodeRange) -> Result<&[u8], TupleError> {
    let end = range.start_offset_bytes.saturating_add(range.len_bytes);
    if end > data.len() {
        return Err(TupleError::UnexpectedEnd {
            offset: range.error_offset_bytes,
        });
    }
    assert!(end <= data.len(), "validated slice end must stay in bounds");
    Ok(&data[range.start_offset_bytes..end])
}

/// Decode bytes with null escaping (0x00, 0xFF -> 0x00).
///
/// Returns the decoded bytes and the number of bytes consumed (including terminator).
fn decode_bytes_with_null_escaping(data: &[u8], start_offset_bytes: usize) -> Result<(Vec<u8>, usize), TupleError> {
    let mut result = Vec::with_capacity(data.len().saturating_sub(start_offset_bytes));
    let mut i = start_offset_bytes;
    assert!(i == start_offset_bytes, "cursor must start at requested offset");

    while i < data.len() {
        let b = data[i];

        if b == 0x00 {
            let next_index = i.saturating_add(1);
            let has_escape = next_index < data.len() && data[next_index] == NULL_ESCAPE;
            if has_escape {
                result.push(0x00);
                i = i.saturating_add(2);
            } else {
                let consumed = i.saturating_sub(start_offset_bytes).saturating_add(1);
                assert!(consumed > 0, "terminator must consume at least one byte");
                assert!(consumed <= data.len().saturating_sub(start_offset_bytes).saturating_add(1));
                return Ok((result, consumed));
            }
        } else {
            result.push(b);
            i = i.saturating_add(1);
        }
    }

    Err(TupleError::MissingTerminator {
        offset: start_offset_bytes,
    })
}

/// Decode an integer from the FDB encoding.
fn decode_int(data: &[u8], offset: usize) -> Result<(i64, usize), TupleError> {
    if offset >= data.len() {
        return Err(TupleError::UnexpectedEnd { offset });
    }

    let code = data[offset];
    if code == INT_ZERO_CODE {
        return Ok((0, 1));
    }

    let payload_offset_bytes = offset.saturating_add(1);
    assert!(payload_offset_bytes > offset, "integer payload must start after type code");
    if code > INT_ZERO_CODE {
        let size_bytes = usize::from(code.saturating_sub(INT_ZERO_CODE));
        assert!(size_bytes <= 8, "FDB integer width must fit u64");
        let bytes = read_exact_bytes(data, DecodeRange::new(payload_offset_bytes, size_bytes, offset))?;
        let n = decode_uint_be(bytes);
        if n > i64::MAX as u64 {
            return Err(TupleError::IntegerOverflow { offset });
        }
        return Ok((n as i64, size_bytes.saturating_add(1)));
    }

    let size_bytes = usize::from(INT_ZERO_CODE.saturating_sub(code));
    assert!(size_bytes <= 8, "FDB integer width must fit u64");
    let bytes = read_exact_bytes(data, DecodeRange::new(payload_offset_bytes, size_bytes, offset))?;
    let complement = decode_uint_be(bytes);
    let shift_bits = size_bytes.saturating_mul(8);
    let mask = if size_bytes == 8 {
        u64::MAX
    } else {
        (1u64 << shift_bits).saturating_sub(1)
    };
    let abs = (!complement) & mask;
    if abs == 0 {
        return Ok((0, size_bytes.saturating_add(1)));
    }

    let min_i64_abs = (i64::MAX as u64).saturating_add(1);
    if abs > min_i64_abs {
        return Err(TupleError::IntegerOverflow { offset });
    }

    let n = if abs == min_i64_abs { i64::MIN } else { -(abs as i64) };
    Ok((n, size_bytes.saturating_add(1)))
}

#[derive(Clone, Copy)]
struct ElementOffsets {
    element_offset_bytes: usize,
    payload_offset_bytes: usize,
}

impl ElementOffsets {
    fn new(element_offset_bytes: usize) -> Self {
        Self {
            element_offset_bytes,
            payload_offset_bytes: element_offset_bytes.saturating_add(1),
        }
    }
}

#[derive(Clone, Copy)]
struct DecodeRange {
    start_offset_bytes: usize,
    len_bytes: usize,
    error_offset_bytes: usize,
}

impl DecodeRange {
    fn new(start_offset_bytes: usize, len_bytes: usize, error_offset_bytes: usize) -> Self {
        Self {
            start_offset_bytes,
            len_bytes,
            error_offset_bytes,
        }
    }
}

/// Decode an unsigned integer from big-endian bytes.
fn decode_uint_be(data: &[u8]) -> u64 {
    let mut result = 0u64;
    for &b in data {
        result = (result << 8) | (b as u64);
    }
    result
}

/// Decode a 32-bit float from FDB encoding.
fn decode_float(data: &[u8]) -> f32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(data);
    let transformed = u32::from_be_bytes(bytes);

    let bits = if (transformed & 0x8000_0000) != 0 {
        // Was positive (sign bit is now set from XOR)
        transformed ^ 0x8000_0000
    } else {
        // Was negative (all bits were flipped)
        !transformed
    };

    f32::from_bits(bits)
}

/// Decode a 64-bit double from FDB encoding.
fn decode_double(data: &[u8]) -> f64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(data);
    let transformed = u64::from_be_bytes(bytes);

    let bits = if (transformed & 0x8000_0000_0000_0000) != 0 {
        // Was positive
        transformed ^ 0x8000_0000_0000_0000
    } else {
        // Was negative
        !transformed
    };

    f64::from_bits(bits)
}

/// Decode a nested tuple.
fn decode_nested_tuple(data: &[u8], start_offset_bytes: usize) -> Result<(Tuple, usize), TupleError> {
    let mut tuple = Tuple::new();
    let mut i = start_offset_bytes;
    assert!(i == start_offset_bytes, "nested tuple cursor must start at requested offset");

    while i < data.len() {
        if data[i] == 0x00 {
            let next_index = i.saturating_add(1);
            let has_escape = next_index < data.len() && data[next_index] == NULL_ESCAPE;
            if has_escape {
                tuple.elements.push(Element::Null);
                i = i.saturating_add(2);
            } else {
                let consumed = i.saturating_sub(start_offset_bytes).saturating_add(1);
                assert!(consumed > 0, "nested tuple terminator must consume bytes");
                assert!(consumed <= data.len().saturating_sub(start_offset_bytes).saturating_add(1));
                return Ok((tuple, consumed));
            }
        } else {
            let (elem, consumed) = decode_element(data, i)?;
            tuple.elements.push(elem);
            i = i.saturating_add(consumed);
        }
    }

    Err(TupleError::UnterminatedNested {
        offset: start_offset_bytes,
    })
}
