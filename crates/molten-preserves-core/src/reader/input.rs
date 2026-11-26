//! Byte cursor for canonical lengths and atom payloads.

/// The last 7-bit group of a `u64` varint, which may only carry the top bit.
const LAST_VARINT_SHIFT: u32 = 63;

pub(super) struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub(super) fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub(super) fn position(&self) -> usize {
        self.position
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.position == self.bytes.len()
    }

    pub(super) fn next_byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.position)?;
        self.position += 1;
        Some(byte)
    }

    /// Takes `length` bytes, checked against the remaining input before anything is copied.
    fn take(&mut self, length: usize) -> Result<&'a [u8], super::failure::DecodeError> {
        let end = self.position.checked_add(length).ok_or(super::failure::DecodeError::LengthOverflow)?;
        let payload = self.bytes.get(self.position..end).ok_or(super::failure::DecodeError::Truncated)?;
        self.position = end;
        Ok(payload)
    }

    /// Reads a minimal length varint.
    fn length(&mut self) -> Result<usize, super::failure::DecodeError> {
        let mut value: u64 = 0;
        for (index, shift) in (0..=LAST_VARINT_SHIFT).step_by(7).enumerate() {
            let byte = self.next_byte().ok_or(super::failure::DecodeError::Truncated)?;
            if shift == LAST_VARINT_SHIFT && byte > 1 {
                return Err(super::failure::DecodeError::LengthOverflow);
            }
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                if byte == 0 && index > 0 {
                    return Err(super::failure::DecodeError::NonCanonicalLength);
                }
                return usize::try_from(value).map_err(|_| super::failure::DecodeError::LengthOverflow);
            }
        }
        Err(super::failure::DecodeError::LengthOverflow)
    }

    /// Reads the length and payload of a length-prefixed atom tagged `tag`.
    pub(super) fn atom(
        &mut self,
        tag: crate::tags::Tag,
    ) -> Result<crate::model::PackedValue, super::failure::DecodeError> {
        let length = self.length()?;
        if tag == crate::tags::Tag::Ieee754 && length != crate::tags::IEEE754_WIDTH {
            return Err(super::failure::DecodeError::InvalidFloatWidth);
        }
        let payload = self.take(length)?;
        match tag {
            crate::tags::Tag::Ieee754 => Ok(double(payload)),
            crate::tags::Tag::SignedInteger => crate::model::signed::PackedInteger::from_be_bytes(payload)
                .map(crate::model::PackedValue::SignedInteger)
                .map_err(|_| super::failure::DecodeError::NonCanonicalInteger),
            crate::tags::Tag::String => text(payload).map(crate::model::PackedValue::String),
            crate::tags::Tag::ByteString => Ok(crate::model::PackedValue::ByteString(payload.to_vec())),
            crate::tags::Tag::Symbol => text(payload).map(crate::model::PackedValue::Symbol),
            crate::tags::Tag::False
            | crate::tags::Tag::True
            | crate::tags::Tag::End
            | crate::tags::Tag::Annotation
            | crate::tags::Tag::Embedded
            | crate::tags::Tag::Record
            | crate::tags::Tag::Sequence
            | crate::tags::Tag::Set
            | crate::tags::Tag::Dictionary => Err(super::failure::DecodeError::InvalidTag(tag.byte())),
        }
    }
}

fn double(payload: &[u8]) -> crate::model::PackedValue {
    let mut bits = [0; crate::tags::IEEE754_WIDTH];
    bits.copy_from_slice(payload);
    crate::model::PackedValue::Double(crate::model::PackedDouble::from_bits(u64::from_be_bytes(bits)))
}

fn text(payload: &[u8]) -> Result<alloc::string::String, super::failure::DecodeError> {
    core::str::from_utf8(payload)
        .map(alloc::string::String::from)
        .map_err(|_| super::failure::DecodeError::InvalidUtf8)
}
