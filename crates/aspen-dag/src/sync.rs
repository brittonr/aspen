//! DAG sync sender and receiver over QUIC streams.
//!
//! The sender executes a traversal and writes response frames.
//! The receiver executes the same traversal and reads frames as they arrive,
//! inserting data into its local store between steps.
//!
//! # Wire Protocol
//!
//! ```text
//! Receiver → Sender: [postcard-encoded DagSyncRequest]
//! Sender → Receiver: [ResponseFrameHeader] [optional: u64 LE size + raw data]*
//! Sender: finish()
//! ```

use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tracing::debug;
use tracing::trace;

use crate::constants::MAX_DAG_SYNC_TRANSFER_SIZE;
use crate::protocol::FrameType;
use crate::protocol::InlinePolicy;
use crate::protocol::ProtocolError;
use crate::protocol::RESPONSE_FRAME_HEADER_SIZE;
use crate::protocol::ResponseFrameHeader;

// ============================================================================
// Sender
// ============================================================================

/// Context for deciding whether to inline a node's data.
pub struct InlineContext {
    /// Size of the node's data in bytes.
    pub data_size: u64,
    /// Application-defined type tag (e.g., 0=Commit, 1=Tree, 2=Blob).
    pub type_tag: u32,
    /// Whether this node is a leaf (has no children).
    pub is_leaf: bool,
}

impl InlinePolicy {
    /// Evaluate whether a node's data should be inlined.
    pub fn should_inline(&self, ctx: &InlineContext) -> bool {
        match self {
            InlinePolicy::All => true,
            InlinePolicy::SizeThreshold(max) => ctx.data_size <= *max as u64,
            InlinePolicy::MetadataOnly => !ctx.is_leaf,
            InlinePolicy::None => false,
        }
    }
}

/// Write a hash-only frame to the output stream.
pub async fn write_hash_only<W: AsyncWrite + Unpin>(writer: &mut W, hash: [u8; 32]) -> Result<u64, ProtocolError> {
    let header = ResponseFrameHeader::hash_only(hash);
    writer.write_all(&header.to_bytes()).await.map_err(|e| ProtocolError::Io { source: e })?;
    Ok(RESPONSE_FRAME_HEADER_SIZE as u64)
}

/// Write a data-inline frame to the output stream.
///
/// Writes the 33-byte header, then u64 LE size, then raw data bytes.
/// For simplicity, this sends raw data without BAO encoding.
/// Full BAO4 framing can be added when iroh-blobs integration lands.
pub async fn write_data_inline<W: AsyncWrite + Unpin>(
    writer: &mut W,
    hash: [u8; 32],
    data: &[u8],
) -> Result<u64, ProtocolError> {
    let header = ResponseFrameHeader::data_inline(hash);
    writer.write_all(&header.to_bytes()).await.map_err(|e| ProtocolError::Io { source: e })?;

    let data_size_bytes = data.len() as u64;
    writer
        .write_all(&data_size_bytes.to_le_bytes())
        .await
        .map_err(|e| ProtocolError::Io { source: e })?;

    writer.write_all(data).await.map_err(|e| ProtocolError::Io { source: e })?;

    // Header + size field + data
    Ok((RESPONSE_FRAME_HEADER_SIZE as u64).saturating_add(8).saturating_add(data_size_bytes))
}

// ============================================================================
// Receiver
// ============================================================================

/// A frame received during sync.
#[derive(Debug)]
pub enum ReceivedFrame {
    /// Hash-only reference — data must be fetched separately.
    HashOnly { hash: [u8; 32] },
    /// Inline data received and verified.
    Data { hash: [u8; 32], data: Vec<u8> },
}

/// Read the next response frame from a stream.
///
/// Returns `Ok(None)` at end-of-stream (traversal complete).
pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    bytes_received: &mut u64,
) -> Result<Option<ReceivedFrame>, ProtocolError> {
    const { assert!(RESPONSE_FRAME_HEADER_SIZE > 0, "frame header must have nonzero size") };
    const { assert!(MAX_DAG_SYNC_TRANSFER_SIZE > 0, "sync transfer limit must be positive") };
    // Read 33-byte header
    let mut header_buf = [0u8; RESPONSE_FRAME_HEADER_SIZE];
    match reader.read_exact(&mut header_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(ProtocolError::Io { source: e }),
    }

    *bytes_received = bytes_received.saturating_add(RESPONSE_FRAME_HEADER_SIZE as u64);

    let header = ResponseFrameHeader::from_bytes(&header_buf).ok_or_else(|| ProtocolError::InvalidFrame {
        message: format!("invalid discriminator: 0x{:02x}", header_buf[0]),
    })?;

    match header.frame_type {
        FrameType::HashOnly => {
            trace!(hash = hex::encode(header.hash), "received hash-only frame");
            Ok(Some(ReceivedFrame::HashOnly { hash: header.hash }))
        }
        FrameType::DataInline => {
            // Read u64 LE size
            let data_size_bytes = reader.read_u64_le().await.map_err(|e| ProtocolError::Io { source: e })?;
            *bytes_received = bytes_received.saturating_add(8);

            // Transfer limit check
            let projected = bytes_received.saturating_add(data_size_bytes);
            if projected > MAX_DAG_SYNC_TRANSFER_SIZE {
                return Err(ProtocolError::TransferLimitExceeded {
                    size: projected,
                    max: MAX_DAG_SYNC_TRANSFER_SIZE,
                });
            }

            // Read data
            let data_len = usize::try_from(data_size_bytes).map_err(|_| ProtocolError::TransferLimitExceeded {
                size: data_size_bytes,
                max: MAX_DAG_SYNC_TRANSFER_SIZE,
            })?;
            let mut data = vec![0u8; data_len];
            reader.read_exact(&mut data).await.map_err(|e| ProtocolError::Io { source: e })?;
            *bytes_received = bytes_received.saturating_add(data_size_bytes);

            // Verify BLAKE3 hash
            let actual_hash = blake3::hash(&data);
            if actual_hash.as_bytes() != &header.hash {
                return Err(ProtocolError::HashMismatch {
                    expected: hex::encode(header.hash),
                    actual: actual_hash.to_hex().to_string(),
                });
            }

            debug!(hash = hex::encode(header.hash), data_size_bytes, "received data-inline frame");
            Ok(Some(ReceivedFrame::Data {
                hash: header.hash,
                data,
            }))
        }
    }
}

// ============================================================================
// High-Level Sync Functions
// ============================================================================

/// Result of a DAG sync operation.
#[derive(Debug, Default)]
pub struct SyncStats {
    /// Number of data-inline frames received.
    pub data_frames: u32,
    /// Number of hash-only frames received.
    pub hash_only_frames: u32,
    /// Total bytes transferred (headers + data).
    pub bytes_transferred: u64,
}

/// Run the sender side of a DAG sync.
///
/// Takes an iterator of `(hash, Option<data>)` pairs and writes them
/// as response frames. If data is `Some`, writes a data-inline frame;
/// if `None`, writes hash-only.
pub async fn send_sync<W, I>(writer: &mut W, frames: I) -> Result<SyncStats, ProtocolError>
where
    W: AsyncWrite + Unpin,
    I: IntoIterator<Item = ([u8; 32], Option<Vec<u8>>)>,
{
    const { assert!(RESPONSE_FRAME_HEADER_SIZE == 33, "protocol uses 33-byte frame headers") };
    const { assert!(MAX_DAG_SYNC_TRANSFER_SIZE > 0, "sync transfer limit must be positive") };
    let mut stats = SyncStats::default();

    for (hash, data) in frames {
        let written = match data {
            Some(ref bytes) => {
                stats.data_frames = stats.data_frames.saturating_add(1);
                write_data_inline(writer, hash, bytes).await?
            }
            None => {
                stats.hash_only_frames = stats.hash_only_frames.saturating_add(1);
                write_hash_only(writer, hash).await?
            }
        };
        stats.bytes_transferred = stats.bytes_transferred.saturating_add(written);

        if stats.bytes_transferred > MAX_DAG_SYNC_TRANSFER_SIZE {
            return Err(ProtocolError::TransferLimitExceeded {
                size: stats.bytes_transferred,
                max: MAX_DAG_SYNC_TRANSFER_SIZE,
            });
        }
    }

    writer.flush().await.map_err(|e| ProtocolError::Io { source: e })?;

    Ok(stats)
}

/// Run the receiver side of a DAG sync.
///
/// Reads frames from the stream and passes each to the callback.
/// The callback receives a `ReceivedFrame` and should insert data
/// into the local store. Returns when the stream ends.
pub async fn recv_sync<R, F>(reader: &mut R, mut on_frame: F) -> Result<SyncStats, ProtocolError>
where
    R: AsyncRead + Unpin,
    F: FnMut(ReceivedFrame) -> Result<(), ProtocolError>,
{
    let mut stats = SyncStats::default();
    let mut bytes_received: u64 = 0;

    while bytes_received < MAX_DAG_SYNC_TRANSFER_SIZE {
        match read_frame(reader, &mut bytes_received).await? {
            Some(ReceivedFrame::HashOnly { hash }) => {
                stats.hash_only_frames = stats.hash_only_frames.saturating_add(1);
                on_frame(ReceivedFrame::HashOnly { hash })?;
            }
            Some(ReceivedFrame::Data { hash, data }) => {
                stats.data_frames = stats.data_frames.saturating_add(1);
                on_frame(ReceivedFrame::Data { hash, data })?;
            }
            None => break,
        }
    }

    stats.bytes_transferred = bytes_received;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_policy_all() {
        let policy = InlinePolicy::All;
        let ctx = InlineContext {
            data_size: 1_000_000,
            type_tag: 0,
            is_leaf: true,
        };
        assert!(policy.should_inline(&ctx));
    }

    #[test]
    fn inline_policy_none() {
        let policy = InlinePolicy::None;
        let ctx = InlineContext {
            data_size: 1,
            type_tag: 0,
            is_leaf: false,
        };
        assert!(!policy.should_inline(&ctx));
    }

    #[test]
    fn inline_policy_size_threshold() {
        let policy = InlinePolicy::SizeThreshold(16384);

        let small = InlineContext {
            data_size: 100,
            type_tag: 0,
            is_leaf: true,
        };
        assert!(policy.should_inline(&small));

        let large = InlineContext {
            data_size: 100_000,
            type_tag: 0,
            is_leaf: true,
        };
        assert!(!policy.should_inline(&large));
    }

    #[test]
    fn inline_policy_metadata_only() {
        let policy = InlinePolicy::MetadataOnly;

        let interior = InlineContext {
            data_size: 500,
            type_tag: 0,
            is_leaf: false,
        };
        assert!(policy.should_inline(&interior));

        let leaf = InlineContext {
            data_size: 500,
            type_tag: 2,
            is_leaf: true,
        };
        assert!(!policy.should_inline(&leaf));
    }

    #[tokio::test]
    async fn send_recv_roundtrip_data_inline() {
        let data = b"hello world";
        let hash = *blake3::hash(data).as_bytes();

        let frames = vec![(hash, Some(data.to_vec()))];

        // Send
        let mut buf = Vec::new();
        let send_stats = send_sync(&mut buf, frames).await.unwrap();
        assert_eq!(send_stats.data_frames, 1);
        assert_eq!(send_stats.hash_only_frames, 0);

        // Receive
        let mut received = Vec::new();
        let recv_stats = recv_sync(&mut buf.as_slice(), |frame| {
            received.push(frame);
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(recv_stats.data_frames, 1);
        assert_eq!(received.len(), 1);
        match &received[0] {
            ReceivedFrame::Data { hash: h, data: d } => {
                assert_eq!(h, &hash);
                assert_eq!(d, data);
            }
            _ => panic!("expected Data frame"),
        }
    }

    #[tokio::test]
    async fn send_recv_roundtrip_hash_only() {
        let hash = *blake3::hash(b"reference").as_bytes();
        let frames = vec![(hash, None)];

        let mut buf = Vec::new();
        send_sync(&mut buf, frames).await.unwrap();

        let mut received = Vec::new();
        recv_sync(&mut buf.as_slice(), |frame| {
            received.push(frame);
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(received.len(), 1);
        match &received[0] {
            ReceivedFrame::HashOnly { hash: h } => assert_eq!(h, &hash),
            _ => panic!("expected HashOnly frame"),
        }
    }

    #[tokio::test]
    async fn send_recv_mixed_frames() {
        let data1 = b"commit object";
        let hash1 = *blake3::hash(data1).as_bytes();

        let hash2 = *blake3::hash(b"big blob ref").as_bytes();

        let data3 = b"tree object";
        let hash3 = *blake3::hash(data3).as_bytes();

        let frames = vec![
            (hash1, Some(data1.to_vec())),
            (hash2, None),
            (hash3, Some(data3.to_vec())),
        ];

        let mut buf = Vec::new();
        let stats = send_sync(&mut buf, frames).await.unwrap();
        assert_eq!(stats.data_frames, 2);
        assert_eq!(stats.hash_only_frames, 1);

        let mut received = Vec::new();
        recv_sync(&mut buf.as_slice(), |frame| {
            received.push(frame);
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(received.len(), 3);
        assert!(matches!(&received[0], ReceivedFrame::Data { .. }));
        assert!(matches!(&received[1], ReceivedFrame::HashOnly { .. }));
        assert!(matches!(&received[2], ReceivedFrame::Data { .. }));
    }

    #[tokio::test]
    async fn recv_detects_hash_mismatch() {
        // Manually craft a frame with wrong hash
        let data = b"correct data";
        let wrong_hash = *blake3::hash(b"wrong").as_bytes();

        let mut buf = Vec::new();
        // Write header with wrong hash
        let header = ResponseFrameHeader::data_inline(wrong_hash);
        buf.extend_from_slice(&header.to_bytes());
        // Write size
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes());
        // Write actual data (hash won't match)
        buf.extend_from_slice(data);

        let result = recv_sync(&mut buf.as_slice(), |_| Ok(())).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProtocolError::HashMismatch { .. }), "expected HashMismatch, got {err:?}");
    }

    #[tokio::test]
    async fn recv_empty_stream() {
        let buf: Vec<u8> = vec![];
        let stats = recv_sync(&mut buf.as_slice(), |_| Ok(())).await.unwrap();
        assert_eq!(stats.data_frames, 0);
        assert_eq!(stats.hash_only_frames, 0);
    }
}
