//! Allocation-free packed-value shape admission before recursive decoding.

use preserves::reader::Reader;

/// Maximum packed nesting admitted by bounded external-data readers.
pub const MAX_DEPTH: usize = 32;

/// Token ceiling shared by emitted external receipts and bounded ledger reads.
pub const MAX_VALUE_TOKENS: u64 = 65_536;

#[derive(Clone, Copy)]
enum Frame {
    Compound,
    Prefix(u8),
}

/// Check one complete packed value without allocating its decoded tree.
/// Callers separately bound input bytes and enforce canonical encoding.
pub fn preflight(bytes: &[u8], maximum_tokens: u64) -> crate::error::Result<()> {
    let mut frames = [Frame::Compound; MAX_DEPTH];
    let mut depth = 0;
    let mut nodes: u64 = 0;
    let mut reader = preserves::PackedReader::new(preserves::BytesBinarySource::new(bytes));
    let mut is_root_complete = false;
    while !is_root_complete {
        nodes = nodes.checked_add(1).ok_or_else(denied)?;
        if nodes > maximum_tokens {
            return Err(denied());
        }
        let token = reader.peek_class().map_err(|_| denied())?.ok_or_else(denied)?;
        let mut is_complete = false;
        let frame = match token {
            preserves::reader::NextToken::Value(preserves::ValueClass::Atomic(_)) => {
                reader.skip_atom().map_err(|_| denied())?;
                is_complete = true;
                None
            }
            preserves::reader::NextToken::Value(preserves::ValueClass::Compound(class)) => {
                match class {
                    preserves::CompoundClass::Record => reader.open_record(),
                    preserves::CompoundClass::Sequence => reader.open_sequence(),
                    preserves::CompoundClass::Set => reader.open_set(),
                    preserves::CompoundClass::Dictionary => reader.open_dictionary(),
                }
                .map_err(|_| denied())?;
                Some(Frame::Compound)
            }
            preserves::reader::NextToken::Value(preserves::ValueClass::Embedded) => {
                reader.open_embedded().map_err(|_| denied())?;
                Some(Frame::Prefix(1))
            }
            preserves::reader::NextToken::Annotation => {
                reader.open_annotation().map_err(|_| denied())?;
                Some(Frame::Prefix(2))
            }
            preserves::reader::NextToken::End => {
                if depth == 0 || !matches!(frames[depth - 1], Frame::Compound) {
                    return Err(denied());
                }
                reader.peekend().map_err(|_| denied())?;
                depth -= 1;
                is_complete = true;
                None
            }
        };
        if let Some(frame) = frame {
            if depth == MAX_DEPTH {
                return Err(denied());
            }
            frames[depth] = frame;
            depth += 1;
        }
        if is_complete {
            is_root_complete = finish_value(&mut frames, &mut depth);
        }
    }
    if reader.peek_class().map_err(|_| denied())?.is_some() {
        return Err(denied());
    }
    Ok(())
}

fn finish_value(frames: &mut [Frame; MAX_DEPTH], depth: &mut usize) -> bool {
    while *depth > 0 {
        match frames[*depth - 1] {
            Frame::Compound => return false,
            Frame::Prefix(1) => *depth -= 1,
            Frame::Prefix(n) => {
                frames[*depth - 1] = Frame::Prefix(n - 1);
                return false;
            }
        }
    }
    true
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("packed input is malformed or exceeds structural bounds")
}
