//! FoundationDB-compatible tuple encoding layer.
//!
//! This module provides order-preserving serialization of composite keys following
//! the [FoundationDB Tuple Layer specification](
//! https://github.com/apple/foundationdb/blob/main/design/tuple.md).
//!
//! # Design Goals
//!
//! 1. **Lexicographic ordering**: Packed bytes sort in the same order as the original tuple
//!    elements, enabling efficient range scans.
//!
//! 2. **Type-tagged encoding**: Each element is prefixed with a type code, allowing heterogeneous
//!    tuples and unambiguous decoding.
//!
//! 3. **Null-safe**: Embedded null bytes are escaped to preserve ordering.
//!
//! 4. **FoundationDB compatible**: Binary-compatible with FDB's tuple layer for interoperability.
//!
//! # Type Codes (FoundationDB spec)
//!
//! | Code | Type | Description |
//! |------|------|-------------|
//! | 0x00 | Null | Null/None value |
//! | 0x01 | Bytes | Byte string with null escaping |
//! | 0x02 | String | UTF-8 string with null escaping |
//! | 0x14 | IntZero | Integer zero (special case) |
//! | 0x0C-0x13 | NegInt | Negative integers (size = 0x14 - code) |
//! | 0x15-0x1C | PosInt | Positive integers (size = code - 0x14) |
//! | 0x05 | Nested | Nested tuple start |
//!
//! # Integer Encoding
//!
//! Integers are encoded using a variable-length scheme that preserves ordering:
//!
//! - Zero: Single byte 0x14
//! - Positive: 0x14 + size_in_bytes, then big-endian bytes
//! - Negative: 0x14 - size_in_bytes, then one's complement big-endian
//!
//! This ensures: INT_MIN < -1 < 0 < 1 < INT_MAX in lexicographic order.
//!
//! # Example
//!
//! ```ignore
//! use aspen::layer::{Tuple, Element};
//!
//! let tuple = Tuple::new()
//!     .push("users")
//!     .push(42i64)
//!     .push("profile");
//!
//! let packed = tuple.pack();
//! let unpacked = Tuple::unpack(&packed)?;
//!
//! assert_eq!(tuple, unpacked);
//! ```

use std::cmp::Ordering;

use snafu::ResultExt;
use snafu::Snafu;

// =============================================================================
// Type Codes (FoundationDB Tuple Layer Specification)
// =============================================================================

/// Null value type code.
const NULL_CODE: u8 = 0x00;

/// Byte string type code.
const BYTES_CODE: u8 = 0x01;

/// UTF-8 string type code.
const STRING_CODE: u8 = 0x02;

/// Nested tuple start type code.
const NESTED_CODE: u8 = 0x05;

/// Integer zero type code (pivot point for integer encoding).
const INT_ZERO_CODE: u8 = 0x14;

/// False boolean type code.
const FALSE_CODE: u8 = 0x26;

/// True boolean type code.
const TRUE_CODE: u8 = 0x27;

/// 32-bit float type code.
const FLOAT_CODE: u8 = 0x20;

/// 64-bit double type code.
const DOUBLE_CODE: u8 = 0x21;

/// Escape sequence for null bytes within strings.
const NULL_ESCAPE: u8 = 0xFF;

/// Size limits for integer encoding (number of bytes needed).
/// Index 0 = 1 byte max, Index 7 = 8 bytes max.
const INT_SIZE_LIMITS: [u64; 8] = [
    0xFF,
    0xFFFF,
    0xFF_FFFF,
    0xFFFF_FFFF,
    0xFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF,
    0xFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
];

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during tuple encoding/decoding.
#[derive(Debug, Snafu)]
pub enum TupleError {
    /// Unexpected end of input while decoding.
    #[snafu(display("unexpected end of input at offset {offset}"))]
    UnexpectedEnd {
        /// Byte offset where the error occurred.
        offset: usize,
    },

    /// Unknown type code encountered.
    #[snafu(display("unknown type code 0x{code:02X} at offset {offset}"))]
    UnknownTypeCode {
        /// The unknown type code.
        code: u8,
        /// Byte offset where the error occurred.
        offset: usize,
    },

    /// Invalid UTF-8 string data.
    #[snafu(display("invalid UTF-8 at offset {offset}: {source}"))]
    InvalidUtf8 {
        /// Byte offset where the error occurred.
        offset: usize,
        /// The underlying UTF-8 error.
        source: std::str::Utf8Error,
    },

    /// Missing null terminator for byte/string element.
    #[snafu(display("missing null terminator at offset {offset}"))]
    MissingTerminator {
        /// Byte offset where the error occurred.
        offset: usize,
    },

    /// Invalid escape sequence.
    #[snafu(display("invalid escape sequence at offset {offset}"))]
    InvalidEscape {
        /// Byte offset where the error occurred.
        offset: usize,
    },

    /// Integer overflow during decoding.
    #[snafu(display("integer overflow at offset {offset}"))]
    IntegerOverflow {
        /// Byte offset where the error occurred.
        offset: usize,
    },

    /// Nested tuple not properly terminated.
    #[snafu(display("unterminated nested tuple at offset {offset}"))]
    UnterminatedNested {
        /// Byte offset where the error occurred.
        offset: usize,
    },
}

// =============================================================================
// Element Type
// =============================================================================

/// A single element within a tuple.
///
/// Elements are typed and can be compared for ordering. The ordering matches
/// the lexicographic ordering of the packed bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// Null value (sorts first).
    Null,

    /// Byte string.
    Bytes(Vec<u8>),

    /// UTF-8 string.
    String(String),

    /// Signed 64-bit integer.
    Int(i64),

    /// Boolean value.
    Bool(bool),

    /// 32-bit floating point.
    Float(f32),

    /// 64-bit floating point.
    Double(f64),

    /// Nested tuple.
    Tuple(Tuple),
}

impl Eq for Element {}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by packing - this ensures correct FDB ordering
        let self_packed = self.pack();
        let other_packed = other.pack();
        self_packed.cmp(&other_packed)
    }
}

impl Element {
    /// Pack this element into bytes.
    fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.pack_into(&mut buf);
        buf
    }

    /// Pack this element into an existing buffer.
    fn pack_into(&self, buf: &mut Vec<u8>) {
        match self {
            Element::Null => {
                buf.push(NULL_CODE);
            }
            Element::Bytes(bytes) => {
                buf.push(BYTES_CODE);
                encode_bytes_with_null_escaping(bytes, buf);
                buf.push(0x00); // Terminator
            }
            Element::String(s) => {
                buf.push(STRING_CODE);
                encode_bytes_with_null_escaping(s.as_bytes(), buf);
                buf.push(0x00); // Terminator
            }
            Element::Int(n) => {
                encode_int(*n, buf);
            }
            Element::Bool(b) => {
                buf.push(if *b { TRUE_CODE } else { FALSE_CODE });
            }
            Element::Float(f) => {
                buf.push(FLOAT_CODE);
                encode_float(*f, buf);
            }
            Element::Double(d) => {
                buf.push(DOUBLE_CODE);
                encode_double(*d, buf);
            }
            Element::Tuple(t) => {
                buf.push(NESTED_CODE);
                for elem in &t.elements {
                    // Nested elements need special handling for null
                    if matches!(elem, Element::Null) {
                        buf.push(NULL_CODE);
                        buf.push(NULL_ESCAPE); // Escape null in nested context
                    } else {
                        elem.pack_into(buf);
                    }
                }
                buf.push(0x00); // Terminator
            }
        }
    }
}

impl From<()> for Element {
    fn from(_: ()) -> Self {
        Element::Null
    }
}

impl From<Vec<u8>> for Element {
    fn from(v: Vec<u8>) -> Self {
        Element::Bytes(v)
    }
}

impl From<&[u8]> for Element {
    fn from(v: &[u8]) -> Self {
        Element::Bytes(v.to_vec())
    }
}

impl From<String> for Element {
    fn from(s: String) -> Self {
        Element::String(s)
    }
}

impl From<&str> for Element {
    fn from(s: &str) -> Self {
        Element::String(s.to_string())
    }
}

impl From<i64> for Element {
    fn from(n: i64) -> Self {
        Element::Int(n)
    }
}

impl From<i32> for Element {
    fn from(n: i32) -> Self {
        Element::Int(n as i64)
    }
}

impl From<u64> for Element {
    fn from(n: u64) -> Self {
        // Handle overflow for very large u64 values
        if n > i64::MAX as u64 {
            // For values > i64::MAX, store as bytes
            // This maintains ordering within the type but loses semantic meaning
            Element::Bytes(n.to_be_bytes().to_vec())
        } else {
            Element::Int(n as i64)
        }
    }
}

impl From<u32> for Element {
    fn from(n: u32) -> Self {
        Element::Int(n as i64)
    }
}

impl From<u8> for Element {
    fn from(n: u8) -> Self {
        Element::Int(n as i64)
    }
}

impl From<bool> for Element {
    fn from(b: bool) -> Self {
        Element::Bool(b)
    }
}

impl From<f32> for Element {
    fn from(f: f32) -> Self {
        Element::Float(f)
    }
}

impl From<f64> for Element {
    fn from(f: f64) -> Self {
        Element::Double(f)
    }
}

impl From<Tuple> for Element {
    fn from(t: Tuple) -> Self {
        Element::Tuple(t)
    }
}

// =============================================================================
// Tuple Type
// =============================================================================

/// An ordered collection of typed elements that can be packed into bytes.
///
/// Tuples are the fundamental building block for structured keys. When packed,
/// they produce bytes that sort lexicographically in the same order as the
/// original tuple elements.
///
/// # Example
///
/// ```ignore
/// let t1 = Tuple::new().push("users").push(1i64);
/// let t2 = Tuple::new().push("users").push(2i64);
///
/// assert!(t1.pack() < t2.pack()); // Lexicographic ordering preserved
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tuple {
    elements: Vec<Element>,
}

impl Tuple {
    /// Create a new empty tuple.
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    /// Create a tuple with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
        }
    }

    /// Push an element onto the tuple (builder pattern).
    pub fn push<E: Into<Element>>(mut self, element: E) -> Self {
        self.elements.push(element.into());
        self
    }

    /// Push an element onto the tuple (mutating).
    pub fn push_mut<E: Into<Element>>(&mut self, element: E) {
        self.elements.push(element.into());
    }

    /// Get the number of elements in the tuple.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the tuple is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get an element by index.
    pub fn get(&self, index: usize) -> Option<&Element> {
        self.elements.get(index)
    }

    /// Get an iterator over the elements.
    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter()
    }

    /// Pack the tuple into bytes.
    ///
    /// The resulting bytes will sort lexicographically in the same order
    /// as the original tuple elements.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.elements.len() * 8);
        self.pack_into(&mut buf);
        buf
    }

    /// Pack the tuple into an existing buffer.
    pub fn pack_into(&self, buf: &mut Vec<u8>) {
        for elem in &self.elements {
            elem.pack_into(buf);
        }
    }

    /// Unpack a tuple from bytes.
    ///
    /// Returns the decoded tuple and ensures all bytes were consumed.
    pub fn unpack(data: &[u8]) -> Result<Self, TupleError> {
        let (tuple, consumed) = Self::unpack_partial(data)?;
        if consumed != data.len() {
            // Extra bytes after tuple - this is valid, return what we got
        }
        Ok(tuple)
    }

    /// Unpack a tuple from bytes, returning how many bytes were consumed.
    ///
    /// This is useful for parsing nested tuples or concatenated data.
    pub fn unpack_partial(data: &[u8]) -> Result<(Self, usize), TupleError> {
        let mut tuple = Tuple::new();
        let mut offset = 0;

        while offset < data.len() {
            let (elem, consumed) = decode_element(data, offset)?;
            tuple.elements.push(elem);
            offset += consumed;
        }

        Ok((tuple, offset))
    }

    /// Get the range of keys that have this tuple as a prefix.
    ///
    /// Returns `(start_key, end_key)` where:
    /// - `start_key` is the packed tuple
    /// - `end_key` is the packed tuple with 0xFF appended (exclusive bound)
    ///
    /// This is useful for range scans over all keys with a given prefix.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let prefix = Tuple::new().push("users");
    /// let (start, end) = prefix.range();
    ///
    /// // Scan all keys starting with ("users", ...)
    /// storage.scan_range(start, end);
    /// ```
    pub fn range(&self) -> (Vec<u8>, Vec<u8>) {
        let start = self.pack();
        let mut end = start.clone();
        end.push(0xFF); // 0xFF is the highest byte, so this is an exclusive upper bound
        (start, end)
    }

    /// Increment the last byte of the packed tuple to get a strict upper bound.
    ///
    /// This is the FDB `strinc` operation - useful for getting an exclusive
    /// end key for range queries.
    pub fn strinc(&self) -> Option<Vec<u8>> {
        let mut packed = self.pack();
        strinc(&mut packed).then_some(packed)
    }
}

impl PartialOrd for Tuple {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tuple {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by packing for correct FDB ordering
        self.pack().cmp(&other.pack())
    }
}

// =============================================================================
// Encoding Functions
// =============================================================================

/// Encode bytes with null byte escaping.
///
/// Null bytes (0x00) are encoded as (0x00, 0xFF) to preserve ordering.
fn encode_bytes_with_null_escaping(bytes: &[u8], buf: &mut Vec<u8>) {
    for &b in bytes {
        if b == 0x00 {
            buf.push(0x00);
            buf.push(NULL_ESCAPE);
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
fn encode_int(n: i64, buf: &mut Vec<u8>) {
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
fn encode_float(f: f32, buf: &mut Vec<u8>) {
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
fn encode_double(f: f64, buf: &mut Vec<u8>) {
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

// =============================================================================
// Decoding Functions
// =============================================================================

/// Decode a single element from bytes at the given offset.
///
/// Returns the decoded element and the number of bytes consumed.
fn decode_element(data: &[u8], offset: usize) -> Result<(Element, usize), TupleError> {
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

/// Increment a byte string to get a strict upper bound (FDB strinc).
///
/// Returns true if the increment was successful, false if the string was all 0xFF.
fn strinc(data: &mut Vec<u8>) -> bool {
    // Find the last non-0xFF byte and increment it
    while let Some(&last) = data.last() {
        if last < 0xFF {
            *data.last_mut().unwrap() = last + 1;
            return true;
        }
        data.pop();
    }
    false
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tuple() {
        let t = Tuple::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);

        let packed = t.pack();
        assert!(packed.is_empty());

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(t, unpacked);
    }

    #[test]
    fn test_null_element() {
        let t = Tuple::new().push(());
        let packed = t.pack();
        assert_eq!(packed, vec![NULL_CODE]);

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Null));
    }

    #[test]
    fn test_string_element() {
        let t = Tuple::new().push("hello");
        let packed = t.pack();

        // STRING_CODE + "hello" + NULL terminator
        assert_eq!(packed[0], STRING_CODE);
        assert_eq!(&packed[1..6], b"hello");
        assert_eq!(packed[6], 0x00);

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::String("hello".to_string())));
    }

    #[test]
    fn test_string_with_null_bytes() {
        let t = Tuple::new().push("foo\x00bar");
        let packed = t.pack();

        // Should escape the null byte
        assert!(packed.contains(&NULL_ESCAPE));

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::String("foo\x00bar".to_string())));
    }

    #[test]
    fn test_bytes_element() {
        let t = Tuple::new().push(vec![1u8, 2, 3, 4]);
        let packed = t.pack();

        assert_eq!(packed[0], BYTES_CODE);

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Bytes(vec![1, 2, 3, 4])));
    }

    #[test]
    fn test_integer_zero() {
        let t = Tuple::new().push(0i64);
        let packed = t.pack();
        assert_eq!(packed, vec![INT_ZERO_CODE]);

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Int(0)));
    }

    #[test]
    fn test_positive_integers() {
        for n in [1i64, 127, 128, 255, 256, 65535, 65536, i64::MAX] {
            let t = Tuple::new().push(n);
            let packed = t.pack();
            let unpacked = Tuple::unpack(&packed).unwrap();
            assert_eq!(unpacked.get(0), Some(&Element::Int(n)), "failed for n={}", n);
        }
    }

    #[test]
    fn test_negative_integers() {
        for n in [-1i64, -127, -128, -255, -256, -65535, -65536, i64::MIN] {
            let t = Tuple::new().push(n);
            let packed = t.pack();
            let unpacked = Tuple::unpack(&packed).unwrap();
            assert_eq!(unpacked.get(0), Some(&Element::Int(n)), "failed for n={}", n);
        }
    }

    #[test]
    fn test_integer_ordering() {
        // Verify that packed integers sort correctly
        let values: Vec<i64> = vec![i64::MIN, -1000, -1, 0, 1, 1000, i64::MAX];
        let packed: Vec<Vec<u8>> = values.iter().map(|&n| Tuple::new().push(n).pack()).collect();

        for i in 1..packed.len() {
            assert!(packed[i - 1] < packed[i], "ordering failed: {:?} should be < {:?}", values[i - 1], values[i]);
        }
    }

    #[test]
    fn test_string_ordering() {
        let values = ["", "a", "aa", "ab", "b", "ba"];
        let packed: Vec<Vec<u8>> = values.iter().map(|s| Tuple::new().push(*s).pack()).collect();

        for i in 1..packed.len() {
            assert!(packed[i - 1] < packed[i], "ordering failed: {:?} should be < {:?}", values[i - 1], values[i]);
        }
    }

    #[test]
    fn test_bool_element() {
        let t = Tuple::new().push(true).push(false);
        let packed = t.pack();

        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Bool(true)));
        assert_eq!(unpacked.get(1), Some(&Element::Bool(false)));
    }

    #[test]
    fn test_float_element() {
        let t = Tuple::new().push(1.234f32);
        let packed = t.pack();

        let unpacked = Tuple::unpack(&packed).unwrap();
        if let Some(Element::Float(f)) = unpacked.get(0) {
            assert!((f - 1.234f32).abs() < 1e-6);
        } else {
            panic!("expected Float element");
        }
    }

    #[test]
    fn test_double_element() {
        let t = Tuple::new().push(1.23456789f64);
        let packed = t.pack();

        let unpacked = Tuple::unpack(&packed).unwrap();
        if let Some(Element::Double(d)) = unpacked.get(0) {
            assert!((d - 1.23456789f64).abs() < 1e-10);
        } else {
            panic!("expected Double element");
        }
    }

    #[test]
    fn test_nested_tuple() {
        let inner = Tuple::new().push("inner").push(42i64);
        let outer = Tuple::new().push("outer").push(inner.clone());

        let packed = outer.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();

        assert_eq!(unpacked.get(0), Some(&Element::String("outer".to_string())));
        assert_eq!(unpacked.get(1), Some(&Element::Tuple(inner)));
    }

    #[test]
    fn test_nested_tuple_with_null() {
        let inner = Tuple::new().push(()).push("after_null");
        let outer = Tuple::new().push("outer").push(inner.clone());

        let packed = outer.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();

        if let Some(Element::Tuple(t)) = unpacked.get(1) {
            assert_eq!(t.get(0), Some(&Element::Null));
            assert_eq!(t.get(1), Some(&Element::String("after_null".to_string())));
        } else {
            panic!("expected nested tuple");
        }
    }

    #[test]
    fn test_composite_tuple() {
        let t = Tuple::new().push("users").push(12345i64).push("profile").push(true);

        let packed = t.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();

        assert_eq!(t, unpacked);
    }

    #[test]
    fn test_range() {
        let prefix = Tuple::new().push("users").push(1i64);
        let (start, end) = prefix.range();

        assert_eq!(start, prefix.pack());
        assert_eq!(end, {
            let mut v = prefix.pack();
            v.push(0xFF);
            v
        });

        // Any key with a longer suffix should be in range
        let key = Tuple::new().push("users").push(1i64).push("profile").pack();
        assert!(key >= start && key < end);
    }

    #[test]
    fn test_strinc() {
        let t = Tuple::new().push("abc");
        let incremented = t.strinc().unwrap();

        // Should be strictly greater than original
        assert!(incremented > t.pack());

        // But less than "abd"
        let next = Tuple::new().push("abd").pack();
        assert!(incremented <= next);
    }

    #[test]
    fn test_type_ordering() {
        // Verify cross-type ordering: Null < Bytes < String < Int < ...
        let null_packed = Tuple::new().push(()).pack();
        let bytes_packed = Tuple::new().push(vec![0u8]).pack();
        let string_packed = Tuple::new().push("a").pack();
        // Note: integers have different type codes based on value

        assert!(null_packed < bytes_packed);
        assert!(bytes_packed < string_packed);
    }

    #[test]
    fn test_roundtrip_stress() {
        // Test many different values
        for i in -1000i64..1000 {
            let t = Tuple::new().push(i);
            let packed = t.pack();
            let unpacked = Tuple::unpack(&packed).unwrap();
            assert_eq!(t, unpacked, "roundtrip failed for i={}", i);
        }
    }
}
