use std::cmp::Ordering;

use super::BYTES_CODE;
use super::DOUBLE_CODE;
use super::FALSE_CODE;
use super::FLOAT_CODE;
use super::NESTED_CODE;
use super::NULL_CODE;
use super::NULL_ESCAPE;
use super::STRING_CODE;
use super::TRUE_CODE;
use super::encoding::encode_bytes_with_null_escaping;
use super::encoding::encode_double;
use super::encoding::encode_float;
use super::encoding::encode_int;
use super::tuple_type::Tuple;

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
    pub(crate) fn pack_into(&self, buf: &mut Vec<u8>) {
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
