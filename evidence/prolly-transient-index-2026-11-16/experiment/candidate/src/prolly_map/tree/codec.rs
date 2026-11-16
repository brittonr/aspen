#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "codec writers require the growable bounded byte vector that they extend"
)]

use super::super::ProllyIssue;

pub(super) const NODE_MAGIC: [u8; 4] = *b"MPL1";
pub(super) const LEAF_KIND: u8 = 1;
pub(super) const INTERNAL_KIND: u8 = 2;

pub(super) fn put_bytes_u16(target: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ProllyIssue> {
    let length = u16::try_from(bytes.len()).map_err(|_| ProllyIssue::NodeEncodingMalformed("u16-length"))?;
    target.extend_from_slice(&length.to_le_bytes());
    target.extend_from_slice(bytes);
    Ok(())
}

pub(super) fn put_bytes_u32(target: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ProllyIssue> {
    let length = u32::try_from(bytes.len()).map_err(|_| ProllyIssue::NodeEncodingMalformed("u32-length"))?;
    target.extend_from_slice(&length.to_le_bytes());
    target.extend_from_slice(bytes);
    Ok(())
}

pub(super) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(super) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub(super) fn take(&mut self, length: usize) -> Result<&'a [u8], ProllyIssue> {
        let end = self.offset.checked_add(length).ok_or(ProllyIssue::NodeEncodingMalformed("offset-overflow"))?;
        let value = self.bytes.get(self.offset..end).ok_or(ProllyIssue::NodeEncodingMalformed("truncated"))?;
        self.offset = end;
        Ok(value)
    }

    pub(super) fn u8(&mut self) -> Result<u8, ProllyIssue> {
        self.take(1)?.first().copied().ok_or(ProllyIssue::NodeEncodingMalformed("u8"))
    }

    pub(super) fn u16(&mut self) -> Result<u16, ProllyIssue> {
        let bytes = self.take(core::mem::size_of::<u16>())?;
        let array = <[u8; core::mem::size_of::<u16>()]>::try_from(bytes)
            .map_err(|_| ProllyIssue::NodeEncodingMalformed("u16"))?;
        Ok(u16::from_le_bytes(array))
    }

    pub(super) fn u32(&mut self) -> Result<u32, ProllyIssue> {
        let bytes = self.take(core::mem::size_of::<u32>())?;
        let array = <[u8; core::mem::size_of::<u32>()]>::try_from(bytes)
            .map_err(|_| ProllyIssue::NodeEncodingMalformed("u32"))?;
        Ok(u32::from_le_bytes(array))
    }

    pub(super) fn bytes_u16(&mut self) -> Result<Vec<u8>, ProllyIssue> {
        let length = usize::from(self.u16()?);
        self.take(length).map(<[u8]>::to_vec)
    }

    pub(super) fn bytes_u32(&mut self) -> Result<Vec<u8>, ProllyIssue> {
        let length = usize::try_from(self.u32()?).map_err(|_| ProllyIssue::NodeEncodingMalformed("usize"))?;
        self.take(length).map(<[u8]>::to_vec)
    }

    pub(super) fn string_u16(&mut self) -> Result<String, ProllyIssue> {
        String::from_utf8(self.bytes_u16()?).map_err(|_| ProllyIssue::NodeEncodingMalformed("utf8"))
    }
}
