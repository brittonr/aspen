//! Packed Preserves wire tags (`0x80..0xB7`).

pub const FALSE: u8 = 0x80;
pub const TRUE: u8 = 0x81;
pub const END: u8 = 0x84;
pub const ANNOTATION: u8 = 0x85;
pub const EMBEDDED: u8 = 0x86;
pub const IEEE754: u8 = 0x87;
pub const SIGNED_INTEGER: u8 = 0xB0;
pub const STRING: u8 = 0xB1;
pub const BYTE_STRING: u8 = 0xB2;
pub const SYMBOL: u8 = 0xB3;
pub const RECORD: u8 = 0xB4;
pub const SEQUENCE: u8 = 0xB5;
pub const SET: u8 = 0xB6;
pub const DICTIONARY: u8 = 0xB7;

/// Width of the only admitted IEEE 754 payload (binary64).
pub const IEEE754_WIDTH: usize = 8;

/// One packed wire tag. Every other byte in tag position is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    False,
    True,
    End,
    Annotation,
    Embedded,
    Ieee754,
    SignedInteger,
    String,
    ByteString,
    Symbol,
    Record,
    Sequence,
    Set,
    Dictionary,
}

impl Tag {
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            FALSE => Some(Self::False),
            TRUE => Some(Self::True),
            END => Some(Self::End),
            ANNOTATION => Some(Self::Annotation),
            EMBEDDED => Some(Self::Embedded),
            IEEE754 => Some(Self::Ieee754),
            SIGNED_INTEGER => Some(Self::SignedInteger),
            STRING => Some(Self::String),
            BYTE_STRING => Some(Self::ByteString),
            SYMBOL => Some(Self::Symbol),
            RECORD => Some(Self::Record),
            SEQUENCE => Some(Self::Sequence),
            SET => Some(Self::Set),
            DICTIONARY => Some(Self::Dictionary),
            _ => None,
        }
    }

    pub const fn byte(self) -> u8 {
        match self {
            Self::False => FALSE,
            Self::True => TRUE,
            Self::End => END,
            Self::Annotation => ANNOTATION,
            Self::Embedded => EMBEDDED,
            Self::Ieee754 => IEEE754,
            Self::SignedInteger => SIGNED_INTEGER,
            Self::String => STRING,
            Self::ByteString => BYTE_STRING,
            Self::Symbol => SYMBOL,
            Self::Record => RECORD,
            Self::Sequence => SEQUENCE,
            Self::Set => SET,
            Self::Dictionary => DICTIONARY,
        }
    }

    /// Atoms carry a varint length and that many payload bytes.
    pub const fn has_length_prefix(self) -> bool {
        matches!(self, Self::Ieee754 | Self::SignedInteger | Self::String | Self::ByteString | Self::Symbol)
    }
}
