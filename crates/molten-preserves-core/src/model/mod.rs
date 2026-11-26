//! The canonical packed value tree.
//!
//! `PackedValue` models exactly the values a canonical packed encoding can carry. Annotations are
//! not part of the model: Aspen identity is computed without them, so the strict decoder rejects
//! them and the annotation-skipping decoder drops them. Sets and dictionaries are kept in canonical
//! packed byte order of their elements and keys, the order `preserves::PackedWriter` emits.

pub mod collections;
pub mod signed;

/// An IEEE 754 binary64 value kept as its exact bit pattern, so every NaN payload and both zeros
/// keep their own canonical encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackedDouble(u64);

impl PackedDouble {
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub fn from_f64(value: f64) -> Self {
        Self(value.to_bits())
    }

    pub const fn to_bits(self) -> u64 {
        self.0
    }

    pub fn to_f64(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// A record: a label value followed by zero or more fields.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PackedRecord {
    pub label: alloc::boxed::Box<PackedValue>,
    pub fields: alloc::vec::Vec<PackedValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PackedValue {
    Boolean(bool),
    Double(PackedDouble),
    SignedInteger(signed::PackedInteger),
    String(alloc::string::String),
    ByteString(alloc::vec::Vec<u8>),
    Symbol(alloc::string::String),
    Record(PackedRecord),
    Sequence(alloc::vec::Vec<PackedValue>),
    Set(collections::PackedSet),
    Dictionary(collections::PackedDictionary),
    Embedded(alloc::boxed::Box<PackedValue>),
}

impl PackedValue {
    pub fn string(text: &str) -> Self {
        Self::String(alloc::string::String::from(text))
    }

    pub fn symbol(name: &str) -> Self {
        Self::Symbol(alloc::string::String::from(name))
    }

    pub fn record(label: PackedValue, fields: alloc::vec::Vec<PackedValue>) -> Self {
        Self::Record(PackedRecord {
            label: alloc::boxed::Box::new(label),
            fields,
        })
    }

    /// A record labelled by the symbol `label`.
    pub fn symbol_record(label: &str, fields: alloc::vec::Vec<PackedValue>) -> Self {
        Self::record(Self::symbol(label), fields)
    }

    pub fn embedded(inner: PackedValue) -> Self {
        Self::Embedded(alloc::boxed::Box::new(inner))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Self::Symbol(name) => Some(name),
            _ => None,
        }
    }

    /// The fields of a record labelled by the symbol `label` with exactly `arity` fields.
    pub fn symbol_record_fields(&self, label: &str, arity: u64) -> Option<&[PackedValue]> {
        let Self::Record(record) = self else {
            return None;
        };
        let has_arity = u64::try_from(record.fields.len()).is_ok_and(|count| count == arity);
        let is_match = record.label.as_symbol() == Some(label) && has_arity;
        is_match.then_some(record.fields.as_slice())
    }
}

impl From<bool> for PackedValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i128> for PackedValue {
    fn from(value: i128) -> Self {
        Self::SignedInteger(signed::PackedInteger::from(value))
    }
}

impl From<u128> for PackedValue {
    fn from(value: u128) -> Self {
        Self::SignedInteger(signed::PackedInteger::from(value))
    }
}

impl From<u64> for PackedValue {
    fn from(value: u64) -> Self {
        Self::from(u128::from(value))
    }
}

impl From<i64> for PackedValue {
    fn from(value: i64) -> Self {
        Self::from(i128::from(value))
    }
}

impl From<f64> for PackedValue {
    fn from(value: f64) -> Self {
        Self::Double(PackedDouble::from_f64(value))
    }
}
