//! Arbitrary-width signed integers in their canonical packed form.

/// Width of an `i128` in bytes.
const I128_WIDTH: usize = 16;

/// A signed integer kept as its minimal big-endian two's-complement bytes: zero is empty, and no
/// leading byte only repeats the sign of the next one. This is the exact payload of the packed
/// `0xB0` atom.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PackedInteger(alloc::vec::Vec<u8>);

/// The integer bytes carry a redundant sign-extension byte (or a lone zero byte).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonMinimalInteger;

impl PackedInteger {
    /// Admits only the minimal two's-complement encoding.
    pub fn from_be_bytes(bytes: &[u8]) -> Result<Self, NonMinimalInteger> {
        if is_minimal(bytes) {
            Ok(Self(bytes.to_vec()))
        } else {
            Err(NonMinimalInteger)
        }
    }

    pub fn as_be_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn is_negative(&self) -> bool {
        self.0.first().is_some_and(|byte| byte & 0x80 != 0)
    }

    pub fn to_i128(&self) -> Option<i128> {
        if self.0.len() > I128_WIDTH {
            return None;
        }
        let fill = if self.is_negative() { 0xff } else { 0x00 };
        let mut buffer = [fill; I128_WIDTH];
        buffer[I128_WIDTH - self.0.len()..].copy_from_slice(&self.0);
        Some(i128::from_be_bytes(buffer))
    }

    pub fn to_u128(&self) -> Option<u128> {
        if self.is_negative() {
            return None;
        }
        let magnitude = match self.0.split_first() {
            Some((0, rest)) => rest,
            _ => self.0.as_slice(),
        };
        if magnitude.len() > I128_WIDTH {
            return None;
        }
        let mut buffer = [0; I128_WIDTH];
        buffer[I128_WIDTH - magnitude.len()..].copy_from_slice(magnitude);
        Some(u128::from_be_bytes(buffer))
    }

    pub fn to_u64(&self) -> Option<u64> {
        self.to_u128().and_then(|value| u64::try_from(value).ok())
    }
}

impl From<i128> for PackedInteger {
    fn from(value: i128) -> Self {
        let bytes = value.to_be_bytes();
        Self(bytes[redundant_prefix_len(&bytes)..].to_vec())
    }
}

impl From<u128> for PackedInteger {
    fn from(value: u128) -> Self {
        if let Ok(signed) = i128::try_from(value) {
            return Self::from(signed);
        }
        let mut bytes = alloc::vec::Vec::with_capacity(I128_WIDTH + 1);
        bytes.push(0);
        bytes.extend_from_slice(&value.to_be_bytes());
        Self(bytes)
    }
}

fn is_redundant_pair(first: u8, second: u8) -> bool {
    let is_positive_padding = first == 0x00 && second & 0x80 == 0;
    let is_negative_padding = first == 0xff && second & 0x80 != 0;
    is_positive_padding || is_negative_padding
}

fn is_minimal(bytes: &[u8]) -> bool {
    match bytes {
        [] => true,
        [only] => *only != 0,
        [first, second, ..] => !is_redundant_pair(*first, *second),
    }
}

/// Leading bytes of a fixed-width big-endian integer that only repeat the sign; a zero value
/// drops every byte.
pub(crate) fn redundant_prefix_len(bytes: &[u8]) -> usize {
    let redundant = bytes.windows(2).take_while(|pair| is_redundant_pair(pair[0], pair[1])).count();
    if bytes[redundant..] == [0] {
        bytes.len()
    } else {
        redundant
    }
}
