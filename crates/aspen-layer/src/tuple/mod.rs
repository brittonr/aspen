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
//! ```
//! use aspen_layer::{Tuple, Element};
//!
//! let tuple = Tuple::new()
//!     .push("users")
//!     .push(42i64)
//!     .push("profile");
//!
//! let packed = tuple.pack();
//! let unpacked = Tuple::unpack(&packed).unwrap();
//!
//! assert_eq!(tuple, unpacked);
//! ```

mod decoding;
mod element;
mod encoding;
mod tuple_type;

#[cfg(test)]
mod tests;

// Re-export all public types
pub use element::Element;
use snafu::Snafu;
pub use tuple_type::Tuple;

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
