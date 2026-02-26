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

    let code = data[offset];

    match code {
        NULL_CODE => Ok((Element::Null, 1)),

        BYTES_CODE => {
            let (bytes, consumed) = decode_bytes_with_null_escaping(data, offset + 1)?;
            Ok((Element::Bytes(bytes), consumed + 1))
        }

        STRING_CODE => {
            let (bytes, consumed) = decode_bytes_with_null_escaping(data, offset + 1)?;
            let s = std::str::from_utf8(&bytes).context(InvalidUtf8Snafu { offset })?;
            Ok((Element::String(s.to_string()), consumed + 1))
        }

        NESTED_CODE => {
            let (tuple, consumed) = decode_nested_tuple(data, offset + 1)?;
            Ok((Element::Tuple(tuple), consumed + 1))
        }

        FALSE_CODE => Ok((Element::Bool(false), 1)),
        TRUE_CODE => Ok((Element::Bool(true), 1)),

        FLOAT_CODE => {
            if offset + 5 > data.len() {
                return Err(TupleError::UnexpectedEnd { offset });
            }
            let f = decode_float(&data[offset + 1..offset + 5]);
            Ok((Element::Float(f), 5))
        }

        DOUBLE_CODE => {
            if offset + 9 > data.len() {
                return Err(TupleError::UnexpectedEnd { offset });
            }
            let d = decode_double(&data[offset + 1..offset + 9]);
            Ok((Element::Double(d), 9))
        }

        // Integer codes: 0x0C-0x1C (excluding 0x14 which is zero)
        code if (0x0C..=0x1C).contains(&code) => {
            let (n, consumed) = decode_int(data, offset)?;
            Ok((Element::Int(n), consumed))
        }

        _ => Err(TupleError::UnknownTypeCode { code, offset }),
    }
}

/// Decode bytes with null escaping (0x00, 0xFF -> 0x00).
///
/// Returns the decoded bytes and the number of bytes consumed (including terminator).
fn decode_bytes_with_null_escaping(data: &[u8], start: usize) -> Result<(Vec<u8>, usize), TupleError> {
    let mut result = Vec::new();
    let mut i = start;

    while i < data.len() {
        let b = data[i];

        if b == 0x00 {
            // Check for escape sequence or terminator
            if i + 1 < data.len() && data[i + 1] == NULL_ESCAPE {
                // Escaped null
                result.push(0x00);
                i += 2;
            } else {
                // Terminator
                return Ok((result, i - start + 1));
            }
        } else {
            result.push(b);
            i += 1;
        }
    }

    Err(TupleError::MissingTerminator { offset: start })
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

    if code > INT_ZERO_CODE {
        // Positive integer
        let size = (code - INT_ZERO_CODE) as usize;
        if offset + 1 + size > data.len() {
            return Err(TupleError::UnexpectedEnd { offset });
        }

        let n = decode_uint_be(&data[offset + 1..offset + 1 + size]);
        if n > i64::MAX as u64 {
            return Err(TupleError::IntegerOverflow { offset });
        }
        Ok((n as i64, 1 + size))
    } else {
        // Negative integer
        let size = (INT_ZERO_CODE - code) as usize;
        if offset + 1 + size > data.len() {
            return Err(TupleError::UnexpectedEnd { offset });
        }

        let complement = decode_uint_be(&data[offset + 1..offset + 1 + size]);
        // Undo one's complement
        let mask = if size == 8 { u64::MAX } else { (1u64 << (size * 8)) - 1 };
        let abs = (!complement) & mask;

        if abs == 0 {
            return Ok((0, 1 + size));
        }

        // Convert to negative
        if abs > (i64::MAX as u64) + 1 {
            return Err(TupleError::IntegerOverflow { offset });
        }

        let n = if abs == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(abs as i64)
        };

        Ok((n, 1 + size))
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
fn decode_nested_tuple(data: &[u8], start: usize) -> Result<(Tuple, usize), TupleError> {
    let mut tuple = Tuple::new();
    let mut i = start;

    while i < data.len() {
        if data[i] == 0x00 {
            // Check for escaped null or terminator
            if i + 1 < data.len() && data[i + 1] == NULL_ESCAPE {
                // Escaped null - this represents Element::Null in nested context
                tuple.elements.push(Element::Null);
                i += 2;
            } else {
                // Terminator
                return Ok((tuple, i - start + 1));
            }
        } else {
            let (elem, consumed) = decode_element(data, i)?;
            tuple.elements.push(elem);
            i += consumed;
        }
    }

    Err(TupleError::UnterminatedNested { offset: start })
}
