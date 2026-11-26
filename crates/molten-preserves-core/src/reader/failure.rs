//! Decode failures and their classes.

/// Why a packed input was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    InputTooLarge {
        limit: u64,
    },
    DepthExceeded {
        limit: u64,
    },
    NodesExceeded {
        limit: u64,
    },
    ContainerExceeded {
        limit: u64,
    },
    Truncated,
    InvalidTag(u8),
    /// An end marker outside any compound.
    UnexpectedEnd,
    MissingRecordLabel,
    /// A dictionary key, `Embedded` prefix, or annotation closed without its value.
    MissingValue,
    LengthOverflow,
    InvalidUtf8,
    /// An IEEE 754 atom whose payload is not exactly 8 bytes.
    InvalidFloatWidth,
    DuplicateElement,
    NonCanonicalLength,
    NonCanonicalInteger,
    UnsortedElements,
    Annotation,
    TrailingBytes,
}

/// The kind of rejection: a configured ceiling, bytes no Preserves reader accepts, or parseable
/// bytes that differ from the canonical encoding of the value they denote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeErrorClass {
    Limit,
    Malformed,
    NonCanonical,
}

impl DecodeError {
    pub const fn class(self) -> DecodeErrorClass {
        match self {
            Self::InputTooLarge { .. }
            | Self::DepthExceeded { .. }
            | Self::NodesExceeded { .. }
            | Self::ContainerExceeded { .. } => DecodeErrorClass::Limit,
            Self::Truncated
            | Self::InvalidTag(_)
            | Self::UnexpectedEnd
            | Self::MissingRecordLabel
            | Self::MissingValue
            | Self::LengthOverflow
            | Self::InvalidUtf8
            | Self::InvalidFloatWidth
            | Self::DuplicateElement => DecodeErrorClass::Malformed,
            Self::NonCanonicalLength
            | Self::NonCanonicalInteger
            | Self::UnsortedElements
            | Self::Annotation
            | Self::TrailingBytes => DecodeErrorClass::NonCanonical,
        }
    }
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputTooLarge { limit } => write!(formatter, "packed input exceeds {limit} bytes"),
            Self::DepthExceeded { limit } => write!(formatter, "packed input exceeds nesting depth {limit}"),
            Self::NodesExceeded { limit } => write!(formatter, "packed input exceeds {limit} values"),
            Self::ContainerExceeded { limit } => write!(formatter, "packed compound exceeds {limit} children"),
            Self::Truncated => formatter.write_str("packed input ends inside a value"),
            Self::InvalidTag(byte) => write!(formatter, "invalid packed tag {byte:#04x}"),
            Self::UnexpectedEnd => formatter.write_str("packed end marker outside a compound"),
            Self::MissingRecordLabel => formatter.write_str("packed record has no label"),
            Self::MissingValue => formatter.write_str("packed key, embedded, or annotation prefix has no value"),
            Self::LengthOverflow => formatter.write_str("packed length does not fit"),
            Self::InvalidUtf8 => formatter.write_str("packed string or symbol is not UTF-8"),
            Self::InvalidFloatWidth => formatter.write_str("packed IEEE 754 value is not 8 bytes wide"),
            Self::DuplicateElement => formatter.write_str("packed set element or dictionary key is duplicated"),
            Self::NonCanonicalLength => formatter.write_str("packed length varint is not minimal"),
            Self::NonCanonicalInteger => formatter.write_str("packed integer is not minimal"),
            Self::UnsortedElements => {
                formatter.write_str("packed set elements or dictionary keys are not in canonical order")
            }
            Self::Annotation => formatter.write_str("packed annotations are not canonical"),
            Self::TrailingBytes => formatter.write_str("packed input has bytes after the value"),
        }
    }
}
