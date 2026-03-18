//! DAG sync wire protocol types.
//!
//! Defines the request/response types for DAG synchronization over QUIC.
//! The protocol uses a single bidirectional stream:
//!
//! 1. Receiver sends a postcard-encoded [`DagSyncRequest`]
//! 2. Sender streams back [`ResponseFrame`]s in traversal order
//! 3. Sender calls `finish()` on the send stream when done
//!
//! # Frame Format
//!
//! Each response frame is a fixed 33-byte header:
//!
//! ```text
//! HashOnly:   [0x00] [32-byte BLAKE3 hash]
//! DataInline: [0x01] [32-byte BLAKE3 hash] → followed by [u64 LE size] + BAO4 data
//! ```

use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::constants::MAX_DAG_SYNC_REQUEST_SIZE;
use crate::constants::MAX_KNOWN_HEADS;

/// ALPN protocol identifier for DAG sync.
pub const DAG_SYNC_ALPN: &[u8] = b"/aspen/dag-sync/1";

// ============================================================================
// Request Types
// ============================================================================

/// A request to sync a DAG from a remote node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagSyncRequest {
    /// How to traverse the DAG.
    pub traversal: TraversalOpts,
    /// Which data to send inline vs. hash-only.
    pub inline: InlinePolicy,
}

/// Traversal configuration.
///
/// Determines which nodes the sender walks and in what order.
/// Both sender and receiver execute the same traversal, so these
/// options must produce a deterministic sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalOpts {
    /// Full DAG traversal from a root hash.
    Full(FullTraversalOpts),
    /// A fixed sequence of individual hashes.
    Sequence(Vec<[u8; 32]>),
}

/// Options for a full DAG traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullTraversalOpts {
    /// Root hash to start traversal from.
    pub root: [u8; 32],
    /// Known heads — traversal stops when hitting these.
    /// Enables incremental sync by skipping already-synced sub-DAGs.
    pub known_heads: BTreeSet<[u8; 32]>,
    /// Traversal order.
    #[serde(default)]
    pub order: TraversalOrder,
    /// Filter to apply during traversal.
    #[serde(default)]
    pub filter: TraversalFilter,
}

/// Traversal ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TraversalOrder {
    /// Depth-first, pre-order, left-to-right.
    /// Children are visited before siblings; leftmost child first.
    #[default]
    DepthFirstPreOrder,
}

/// Filter applied during traversal to include/exclude node types.
///
/// Filters control which nodes appear in the sync stream. A filtered-out
/// node is still traversed (its children are discovered) but its data
/// is not sent. Use this for stem/leaf split sync.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TraversalFilter {
    /// Include all nodes.
    #[default]
    All,
    /// Exclude nodes with the given type tags.
    ///
    /// Type tags are application-defined u32 values. For Forge:
    /// - 0 = Commit, 1 = Tree, 2 = Blob, 3 = Tag
    Exclude(BTreeSet<u32>),
    /// Include only nodes with the given type tags.
    Only(BTreeSet<u32>),
}

/// Policy for inlining data in the sync stream.
///
/// For inlined data, the frame contains the full BAO4-encoded content.
/// For non-inlined data, only the hash is sent; the receiver fetches
/// the actual bytes separately (e.g., via iroh-blobs).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InlinePolicy {
    /// Inline all data.
    #[default]
    All,
    /// Inline data smaller than this threshold (bytes).
    SizeThreshold(u32),
    /// Inline only metadata nodes (non-leaf).
    MetadataOnly,
    /// Never inline — send hash references only.
    None,
}

// ============================================================================
// Response Types
// ============================================================================

/// Discriminator byte for response frames.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Hash-only reference (no data follows).
    HashOnly = 0x00,
    /// Data follows: u64 LE size + BAO4-encoded bytes.
    DataInline = 0x01,
}

impl FrameType {
    /// Parse a discriminator byte.
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Self::HashOnly),
            0x01 => Some(Self::DataInline),
            _ => None,
        }
    }
}

/// Fixed-size response frame header (33 bytes).
///
/// ```text
/// [1 byte discriminator] [32 byte BLAKE3 hash]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseFrameHeader {
    pub frame_type: FrameType,
    pub hash: [u8; 32],
}

/// Total size of a response frame header in bytes.
pub const RESPONSE_FRAME_HEADER_SIZE: usize = 33;

impl ResponseFrameHeader {
    /// Create a hash-only frame header.
    pub fn hash_only(hash: [u8; 32]) -> Self {
        Self {
            frame_type: FrameType::HashOnly,
            hash,
        }
    }

    /// Create a data-inline frame header.
    pub fn data_inline(hash: [u8; 32]) -> Self {
        Self {
            frame_type: FrameType::DataInline,
            hash,
        }
    }

    /// Serialize to a 33-byte array.
    pub fn to_bytes(&self) -> [u8; RESPONSE_FRAME_HEADER_SIZE] {
        let mut buf = [0u8; RESPONSE_FRAME_HEADER_SIZE];
        buf[0] = self.frame_type as u8;
        buf[1..33].copy_from_slice(&self.hash);
        buf
    }

    /// Deserialize from a 33-byte array.
    pub fn from_bytes(buf: &[u8; RESPONSE_FRAME_HEADER_SIZE]) -> Option<Self> {
        let frame_type = FrameType::from_byte(buf[0])?;
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&buf[1..33]);
        Some(Self { frame_type, hash })
    }
}

// ============================================================================
// Validation
// ============================================================================

impl DagSyncRequest {
    /// Validate request bounds per Tiger Style limits.
    pub fn validate(&self) -> Result<(), &'static str> {
        match &self.traversal {
            TraversalOpts::Full(opts) => {
                if opts.known_heads.len() as u32 > MAX_KNOWN_HEADS {
                    return Err("known_heads exceeds MAX_KNOWN_HEADS");
                }
            }
            TraversalOpts::Sequence(hashes) => {
                if hashes.len() as u32 > MAX_KNOWN_HEADS {
                    return Err("sequence length exceeds MAX_KNOWN_HEADS");
                }
            }
        }
        Ok(())
    }

    /// Serialize to postcard bytes, checking size limit.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate().map_err(|msg| ProtocolError::InvalidRequest {
            message: msg.to_string(),
        })?;
        let bytes = postcard::to_allocvec(self).map_err(|e| ProtocolError::Serialization { message: e.to_string() })?;
        if bytes.len() as u32 > MAX_DAG_SYNC_REQUEST_SIZE {
            return Err(ProtocolError::RequestTooLarge {
                size: bytes.len() as u32,
                max: MAX_DAG_SYNC_REQUEST_SIZE,
            });
        }
        Ok(bytes)
    }

    /// Deserialize from postcard bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() as u32 > MAX_DAG_SYNC_REQUEST_SIZE {
            return Err(ProtocolError::RequestTooLarge {
                size: bytes.len() as u32,
                max: MAX_DAG_SYNC_REQUEST_SIZE,
            });
        }
        let request: Self =
            postcard::from_bytes(bytes).map_err(|e| ProtocolError::Deserialization { message: e.to_string() })?;
        request.validate().map_err(|msg| ProtocolError::InvalidRequest {
            message: msg.to_string(),
        })?;
        Ok(request)
    }
}

// ============================================================================
// Protocol Errors
// ============================================================================

/// Errors from the DAG sync protocol layer.
#[derive(Debug, snafu::Snafu)]
pub enum ProtocolError {
    /// Request failed validation.
    #[snafu(display("invalid request: {message}"))]
    InvalidRequest { message: String },

    /// Request exceeds size limit.
    #[snafu(display("request too large: {size} bytes (max {max})"))]
    RequestTooLarge { size: u32, max: u32 },

    /// Serialization failed.
    #[snafu(display("serialization error: {message}"))]
    Serialization { message: String },

    /// Deserialization failed.
    #[snafu(display("deserialization error: {message}"))]
    Deserialization { message: String },

    /// Invalid frame header.
    #[snafu(display("invalid frame header: {message}"))]
    InvalidFrame { message: String },

    /// I/O error during sync.
    #[snafu(display("sync I/O error: {source}"))]
    Io { source: std::io::Error },

    /// Unexpected end of stream.
    #[snafu(display("unexpected end of stream"))]
    UnexpectedEof,

    /// Hash mismatch during verification.
    #[snafu(display("hash mismatch: expected {expected}, got {actual}"))]
    HashMismatch { expected: String, actual: String },

    /// Transfer size limit exceeded.
    #[snafu(display("transfer size {size} exceeds max {max}"))]
    TransferLimitExceeded { size: u64, max: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_header_hash_only_roundtrip() {
        let hash = blake3::hash(b"test").as_bytes().to_owned();
        let header = ResponseFrameHeader::hash_only(hash);
        let bytes = header.to_bytes();
        let decoded = ResponseFrameHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, decoded);
        assert_eq!(decoded.frame_type, FrameType::HashOnly);
    }

    #[test]
    fn frame_header_data_inline_roundtrip() {
        let hash = blake3::hash(b"test").as_bytes().to_owned();
        let header = ResponseFrameHeader::data_inline(hash);
        let bytes = header.to_bytes();
        let decoded = ResponseFrameHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, decoded);
        assert_eq!(decoded.frame_type, FrameType::DataInline);
    }

    #[test]
    fn frame_header_invalid_discriminator() {
        let mut bytes = [0u8; RESPONSE_FRAME_HEADER_SIZE];
        bytes[0] = 0xFF;
        assert!(ResponseFrameHeader::from_bytes(&bytes).is_none());
    }

    #[test]
    fn request_postcard_roundtrip_full() {
        let root = *blake3::hash(b"root").as_bytes();
        let request = DagSyncRequest {
            traversal: TraversalOpts::Full(FullTraversalOpts {
                root,
                known_heads: BTreeSet::new(),
                order: TraversalOrder::DepthFirstPreOrder,
                filter: TraversalFilter::All,
            }),
            inline: InlinePolicy::All,
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
        assert_eq!(request, decoded);
    }

    #[test]
    fn request_postcard_roundtrip_sequence() {
        let h1 = *blake3::hash(b"h1").as_bytes();
        let h2 = *blake3::hash(b"h2").as_bytes();
        let request = DagSyncRequest {
            traversal: TraversalOpts::Sequence(vec![h1, h2]),
            inline: InlinePolicy::SizeThreshold(16384),
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
        assert_eq!(request, decoded);
    }

    #[test]
    fn request_with_known_heads_roundtrip() {
        let root = *blake3::hash(b"root").as_bytes();
        let head1 = *blake3::hash(b"head1").as_bytes();
        let head2 = *blake3::hash(b"head2").as_bytes();
        let request = DagSyncRequest {
            traversal: TraversalOpts::Full(FullTraversalOpts {
                root,
                known_heads: BTreeSet::from([head1, head2]),
                order: TraversalOrder::DepthFirstPreOrder,
                filter: TraversalFilter::Exclude(BTreeSet::from([2])), // Exclude blobs
            }),
            inline: InlinePolicy::MetadataOnly,
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
        assert_eq!(request, decoded);
    }

    #[test]
    fn request_with_only_filter_roundtrip() {
        let root = *blake3::hash(b"root").as_bytes();
        let request = DagSyncRequest {
            traversal: TraversalOpts::Full(FullTraversalOpts {
                root,
                known_heads: BTreeSet::new(),
                order: TraversalOrder::DepthFirstPreOrder,
                filter: TraversalFilter::Only(BTreeSet::from([0, 1, 3])), // Commits, Trees, Tags
            }),
            inline: InlinePolicy::None,
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
        assert_eq!(request, decoded);
    }
}
