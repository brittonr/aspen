//! Event encoding/decoding using MessagePack.
//!
//! Events are encoded as MessagePack for efficient binary serialization.
//! This module provides helpers for encoding events to bytes and decoding
//! them from bytes retrieved from the KV store.

use super::error::Result;
use super::event::Event;

/// Encode an event to MessagePack bytes.
///
/// The event is serialized including its topic, cursor, timestamp, payload,
/// and headers. This is the format stored in the KV store.
pub fn encode_event(event: &Event) -> Result<Vec<u8>> {
    let bytes = rmp_serde::to_vec(event)?;
    Ok(bytes)
}

/// Decode an event from MessagePack bytes.
///
/// The bytes should be in the format produced by `encode_event`.
pub fn decode_event(bytes: &[u8]) -> Result<Event> {
    let event = rmp_serde::from_slice(bytes)?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::super::cursor::Cursor;
    use super::super::topic::Topic;
    use super::*;

    #[test]
    fn test_roundtrip_encoding() {
        let topic = Topic::new("orders.created").unwrap();
        let event = Event {
            topic,
            cursor: Cursor::from_index(12345),
            timestamp_ms: 1704067200000,
            payload: b"test payload".to_vec(),
            headers: Vec::new(),
        };

        let encoded = encode_event(&event).unwrap();
        let decoded = decode_event(&encoded).unwrap();

        assert_eq!(decoded.topic, event.topic);
        assert_eq!(decoded.cursor, event.cursor);
        assert_eq!(decoded.timestamp_ms, event.timestamp_ms);
        assert_eq!(decoded.payload, event.payload);
        assert_eq!(decoded.headers, event.headers);
    }

    #[test]
    fn test_roundtrip_with_headers() {
        let topic = Topic::new("orders.created").unwrap();
        let event = Event {
            topic,
            cursor: Cursor::from_index(100),
            timestamp_ms: 1704067200000,
            payload: b"data".to_vec(),
            headers: vec![
                ("trace-id".to_string(), b"abc123".to_vec()),
                ("version".to_string(), b"1".to_vec()),
            ],
        };

        let encoded = encode_event(&event).unwrap();
        let decoded = decode_event(&encoded).unwrap();

        assert_eq!(decoded.headers.len(), 2);
        assert_eq!(decoded.headers[0].0, "trace-id");
        assert_eq!(decoded.headers[0].1, b"abc123");
    }
}
