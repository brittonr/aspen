//! Allocation-free packed-value shape admission before recursive decoding.
//!
//! `preflight` walks the tag stream of one packed value with a fixed-size frame stack. It checks
//! tags, atom lengths, compound nesting, annotation and `Embedded` prefixes, the token ceiling, and
//! trailing input. It does not check canonical form: callers bound input bytes separately and run
//! the strict decoder (`crate::reader`) when they need canonical identity.

/// Maximum packed nesting admitted by bounded external-data readers.
pub const MAX_DEPTH: usize = 32;

/// Token ceiling shared by emitted external receipts and bounded ledger reads.
pub const MAX_VALUE_TOKENS: u64 = 65_536;

/// The packed input is malformed or exceeds the structural bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreflightDenied;

impl core::fmt::Display for PreflightDenied {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("packed input is malformed or exceeds structural bounds")
    }
}

#[derive(Clone, Copy)]
enum Frame {
    Compound,
    /// A prefix that completes after this many following values (`Embedded`: 1, annotation: 2).
    Prefix(u8),
}

struct FrameStack {
    frames: [Frame; MAX_DEPTH],
    depth: usize,
}

impl FrameStack {
    fn open(&mut self, frame: Frame) -> Result<(), PreflightDenied> {
        let slot = self.frames.get_mut(self.depth).ok_or(PreflightDenied)?;
        *slot = frame;
        self.depth += 1;
        Ok(())
    }

    fn close_compound(&mut self) -> Result<(), PreflightDenied> {
        let top = self.depth.checked_sub(1).ok_or(PreflightDenied)?;
        match self.frames[top] {
            Frame::Compound => {
                self.depth = top;
                Ok(())
            }
            Frame::Prefix(_) => Err(PreflightDenied),
        }
    }

    /// Credits one completed value to the open prefixes; true once the root value is complete.
    fn finish_value(&mut self) -> bool {
        while let Some(top) = self.depth.checked_sub(1) {
            match self.frames[top] {
                Frame::Compound => return false,
                Frame::Prefix(1) => self.depth = top,
                Frame::Prefix(remaining) => {
                    self.frames[top] = Frame::Prefix(remaining - 1);
                    return false;
                }
            }
        }
        true
    }
}

/// Check one complete packed value without allocating its decoded tree.
/// Callers separately bound input bytes and enforce canonical encoding.
pub fn preflight(bytes: &[u8], maximum_tokens: u64) -> Result<(), PreflightDenied> {
    let mut stack = FrameStack {
        frames: [Frame::Compound; MAX_DEPTH],
        depth: 0,
    };
    let mut position = 0;
    let mut tokens: u64 = 0;
    let mut is_root_complete = false;
    while !is_root_complete {
        tokens = tokens.checked_add(1).ok_or(PreflightDenied)?;
        if tokens > maximum_tokens {
            return Err(PreflightDenied);
        }
        let tag = bytes.get(position).copied().and_then(crate::tags::Tag::from_byte).ok_or(PreflightDenied)?;
        position += 1;
        let is_complete = match tag {
            crate::tags::Tag::False | crate::tags::Tag::True => true,
            crate::tags::Tag::Ieee754
            | crate::tags::Tag::SignedInteger
            | crate::tags::Tag::String
            | crate::tags::Tag::ByteString
            | crate::tags::Tag::Symbol => {
                position = skip_atom_payload(bytes, position)?;
                true
            }
            crate::tags::Tag::Record
            | crate::tags::Tag::Sequence
            | crate::tags::Tag::Set
            | crate::tags::Tag::Dictionary => {
                stack.open(Frame::Compound)?;
                false
            }
            crate::tags::Tag::Embedded => {
                stack.open(Frame::Prefix(1))?;
                false
            }
            crate::tags::Tag::Annotation => {
                stack.open(Frame::Prefix(2))?;
                false
            }
            crate::tags::Tag::End => {
                stack.close_compound()?;
                true
            }
        };
        if is_complete {
            is_root_complete = stack.finish_value();
        }
    }
    if position == bytes.len() {
        Ok(())
    } else {
        Err(PreflightDenied)
    }
}

/// Skips one length-prefixed atom payload and returns the position after it.
fn skip_atom_payload(bytes: &[u8], position: usize) -> Result<usize, PreflightDenied> {
    let (length, after_length) = read_varint(bytes, position)?;
    let length = usize::try_from(length).map_err(|_| PreflightDenied)?;
    let end = after_length.checked_add(length).ok_or(PreflightDenied)?;
    if end > bytes.len() {
        return Err(PreflightDenied);
    }
    Ok(end)
}

/// Reads a length varint with the `preserves` reader's admission rule: at most ten 7-bit groups,
/// and the tenth group may only carry the final bit of a `u64`. Non-minimal groups are admitted
/// here; the strict decoder rejects them.
fn read_varint(bytes: &[u8], mut position: usize) -> Result<(u64, usize), PreflightDenied> {
    const LAST_SHIFT: u32 = 63;
    let mut shift: u32 = 0;
    let mut value: u64 = 0;
    loop {
        let byte = *bytes.get(position).ok_or(PreflightDenied)?;
        position += 1;
        if shift == LAST_SHIFT && byte > 1 {
            return Err(PreflightDenied);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, position));
        }
        shift += 7;
    }
}
