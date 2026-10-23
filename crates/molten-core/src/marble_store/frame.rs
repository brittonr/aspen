use super::*;
use crate::prolly_map::NodeRef;

/// The version byte of every block frame.
pub const BLOCK_FRAME_VERSION: u8 = 1;
/// Bytes before a frame's payload: the version byte and the digest.
pub const FRAME_HEADER_BYTES: u32 = 33;
const NODE_REF_PREFIX: &str = "blake3:";

/// One decoded block frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockFrame<'a> {
    pub digest: Digest,
    pub payload: &'a [u8],
}

/// Parse a Prolly node reference into its fixed-length digest.
///
/// Only the canonical form, `blake3:` followed by 64 lowercase hexadecimal
/// digits, is accepted, so the digest renders back to the identical reference.
// r[impl aspen.marble_store.art_index]
pub fn digest_of_node_ref(node_ref: &NodeRef) -> Result<Digest, MarbleStoreIssue> {
    let hex = node_ref.as_str().strip_prefix(NODE_REF_PREFIX).ok_or(MarbleStoreIssue::InvalidNodeRef)?;
    if hex.len() != DIGEST_BYTES * 2 {
        return Err(MarbleStoreIssue::InvalidNodeRef);
    }
    let mut digest = [0_u8; DIGEST_BYTES];
    for (slot, pair) in digest.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        let [high, low] = pair else {
            return Err(MarbleStoreIssue::InvalidNodeRef);
        };
        *slot = (hex_value(*high)? << 4) | hex_value(*low)?;
    }
    Ok(digest)
}

/// Render a digest as its canonical Prolly node reference.
pub fn node_ref_of_digest(digest: &Digest) -> NodeRef {
    let mut text = String::with_capacity(NODE_REF_PREFIX.len() + DIGEST_BYTES * 2);
    text.push_str(NODE_REF_PREFIX);
    for byte in digest {
        text.push(hex_digit(byte >> 4));
        text.push(hex_digit(byte & 0x0f));
    }
    NodeRef::new(text)
}

/// Frame a block payload under its digest for the marble heap.
///
/// The digest travels with the payload so a crash recovery can rebuild the
/// index from the heap alone.
pub fn encode_block_frame(digest: &Digest, payload: &[u8], max_object_bytes: u32) -> Result<Vec<u8>, MarbleStoreIssue> {
    let length = u32::try_from(payload.len())
        .ok()
        .and_then(|length| length.checked_add(FRAME_HEADER_BYTES))
        .filter(|length| *length <= max_object_bytes)
        .ok_or(MarbleStoreIssue::PayloadTooLarge)?;
    let capacity = usize::try_from(length).map_err(|_| MarbleStoreIssue::PayloadTooLarge)?;
    let mut frame = Vec::with_capacity(capacity);
    frame.push(BLOCK_FRAME_VERSION);
    frame.extend_from_slice(digest);
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Decode one block frame read back from the heap.
pub fn decode_block_frame(frame: &[u8], max_object_bytes: u32) -> Result<BlockFrame<'_>, MarbleStoreIssue> {
    let within_bound = u32::try_from(frame.len()).is_ok_and(|length| length <= max_object_bytes);
    if !within_bound {
        return Err(MarbleStoreIssue::PayloadTooLarge);
    }
    let (&version, rest) = frame.split_first().ok_or(MarbleStoreIssue::TruncatedFrame)?;
    if version != BLOCK_FRAME_VERSION {
        return Err(MarbleStoreIssue::UnknownFrameVersion(version));
    }
    let (digest, payload) = rest.split_first_chunk::<DIGEST_BYTES>().ok_or(MarbleStoreIssue::TruncatedFrame)?;
    Ok(BlockFrame {
        digest: *digest,
        payload,
    })
}

const fn hex_value(digit: u8) -> Result<u8, MarbleStoreIssue> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        _ => Err(MarbleStoreIssue::InvalidNodeRef),
    }
}

const fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + nibble - 10) as char,
    }
}
