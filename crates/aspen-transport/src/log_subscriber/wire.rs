//! Wire format for log subscriber protocol messages.
//!
//! Messages use postcard serialization with length-prefixed framing (4-byte big-endian).
//! This ensures messages are never split across QUIC packets and prevents partial reads.

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

/// Read a length-prefixed message from a QUIC receive stream.
///
/// Format: `[u32 big-endian length][postcard-encoded body]`
pub async fn read_message<T: for<'de> Deserialize<'de>>(
    recv: &mut iroh::endpoint::RecvStream,
    max_size: usize,
) -> Result<T> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await.context("failed to read message length")?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > max_size {
        anyhow::bail!("message too large: {} > {}", len, max_size);
    }

    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.context("failed to read message body")?;

    postcard::from_bytes(&buf).context("failed to deserialize message")
}

/// Write a length-prefixed message to a QUIC send stream.
///
/// Format: `[u32 big-endian length][postcard-encoded body]`
pub async fn write_message<T: Serialize>(
    send: &mut iroh::endpoint::SendStream,
    message: &T,
    max_size: usize,
) -> Result<()> {
    let buf = postcard::to_allocvec(message).context("failed to serialize message")?;

    if buf.len() > max_size {
        anyhow::bail!("message too large: {} > {}", buf.len(), max_size);
    }

    let len = buf.len() as u32;
    send.write_all(&len.to_be_bytes()).await.context("failed to write message length")?;
    send.write_all(&buf).await.context("failed to write message body")?;

    Ok(())
}
