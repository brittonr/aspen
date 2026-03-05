//! Wire types for the ephemeral pub/sub QUIC protocol.
//!
//! Messages use postcard serialization with length-prefixed framing (4-byte big-endian).
//! Same framing as `aspen-transport::log_subscriber::wire`.

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

/// Maximum wire message size (64 KB).
///
/// Ephemeral events should be small (tokens, deltas, status updates).
/// This is intentionally much smaller than `MAX_PAYLOAD_SIZE` (1 MB)
/// to keep the streaming path fast.
pub const MAX_EPHEMERAL_WIRE_MESSAGE_SIZE: usize = 64 * 1024;

/// Subscribe request sent by a remote client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralSubscribeRequest {
    /// Topic pattern string (supports `*` and `>` wildcards).
    pub pattern: String,
    /// Requested buffer size (server may clamp to its own limits).
    pub buffer_size: u32,
}

/// An event sent from server to subscriber over the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralWireEvent {
    /// Topic name the event was published to.
    pub topic: String,
    /// Timestamp in milliseconds (publisher's clock).
    pub timestamp_ms: u64,
    /// Event payload.
    pub payload: Vec<u8>,
    /// Optional headers.
    pub headers: Vec<(String, Vec<u8>)>,
}

/// Read a length-prefixed postcard message from a QUIC receive stream.
///
/// Format: `[u32 big-endian length][postcard-encoded body]`
pub async fn read_frame<T: for<'de> Deserialize<'de>>(
    recv: &mut iroh::endpoint::RecvStream,
    max_size: usize,
) -> Result<T> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await.context("failed to read frame length")?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > max_size {
        anyhow::bail!("frame too large: {} > {}", len, max_size);
    }

    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.context("failed to read frame body")?;

    postcard::from_bytes(&buf).context("failed to deserialize frame")
}

/// Write a length-prefixed postcard message to a QUIC send stream.
///
/// Format: `[u32 big-endian length][postcard-encoded body]`
pub async fn write_frame<T: Serialize>(send: &mut iroh::endpoint::SendStream, message: &T) -> Result<()> {
    let buf = postcard::to_allocvec(message).context("failed to serialize frame")?;

    if buf.len() > MAX_EPHEMERAL_WIRE_MESSAGE_SIZE {
        anyhow::bail!("frame too large: {} > {}", buf.len(), MAX_EPHEMERAL_WIRE_MESSAGE_SIZE);
    }

    let len = buf.len() as u32;
    send.write_all(&len.to_be_bytes()).await.context("failed to write frame length")?;
    send.write_all(&buf).await.context("failed to write frame body")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_request_roundtrip() {
        let req = EphemeralSubscribeRequest {
            pattern: "orders.*".to_string(),
            buffer_size: 256,
        };
        let bytes = postcard::to_allocvec(&req).unwrap();
        let decoded: EphemeralSubscribeRequest = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.pattern, "orders.*");
        assert_eq!(decoded.buffer_size, 256);
    }

    #[test]
    fn test_wire_event_roundtrip() {
        let event = EphemeralWireEvent {
            topic: "orders.created".to_string(),
            timestamp_ms: 1704067200000,
            payload: b"hello world".to_vec(),
            headers: vec![("trace-id".to_string(), b"abc123".to_vec())],
        };
        let bytes = postcard::to_allocvec(&event).unwrap();
        let decoded: EphemeralWireEvent = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.topic, "orders.created");
        assert_eq!(decoded.timestamp_ms, 1704067200000);
        assert_eq!(decoded.payload, b"hello world");
        assert_eq!(decoded.headers.len(), 1);
        assert_eq!(decoded.headers[0].0, "trace-id");
    }

    #[test]
    fn test_wire_event_empty_payload() {
        let event = EphemeralWireEvent {
            topic: "ping".to_string(),
            timestamp_ms: 0,
            payload: vec![],
            headers: vec![],
        };
        let bytes = postcard::to_allocvec(&event).unwrap();
        let decoded: EphemeralWireEvent = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.topic, "ping");
        assert!(decoded.payload.is_empty());
        assert!(decoded.headers.is_empty());
    }

    #[test]
    fn test_length_prefix_encoding() {
        // Verify the 4-byte BE length prefix format
        let req = EphemeralSubscribeRequest {
            pattern: "test".to_string(),
            buffer_size: 128,
        };
        let body = postcard::to_allocvec(&req).unwrap();
        let len = body.len() as u32;
        let len_bytes = len.to_be_bytes();

        // First 4 bytes are length, rest is body
        let mut frame = Vec::with_capacity(4 + body.len());
        frame.extend_from_slice(&len_bytes);
        frame.extend_from_slice(&body);

        // Decode length
        let decoded_len = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        assert_eq!(decoded_len, body.len());

        // Decode body
        let decoded: EphemeralSubscribeRequest = postcard::from_bytes(&frame[4..]).unwrap();
        assert_eq!(decoded.pattern, "test");
    }
}
