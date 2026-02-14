//! Wire format for federation protocol messages.
//!
//! Messages use postcard serialization with length-prefixed framing (4-byte big-endian).

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use super::MAX_MESSAGE_SIZE;

/// Read a length-prefixed message.
pub async fn read_message<T: for<'de> Deserialize<'de>>(recv: &mut iroh::endpoint::RecvStream) -> Result<T> {
    // Read length prefix (4 bytes, big-endian)
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await.context("failed to read message length")?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Check size limit
    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!("message too large: {} > {}", len, MAX_MESSAGE_SIZE);
    }

    // Read message body
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.context("failed to read message body")?;

    // Deserialize
    let message = postcard::from_bytes(&buf).context("failed to deserialize message")?;

    Ok(message)
}

/// Write a length-prefixed message.
pub async fn write_message<T: Serialize>(send: &mut iroh::endpoint::SendStream, message: &T) -> Result<()> {
    // Serialize
    let buf = postcard::to_allocvec(message).context("failed to serialize message")?;

    // Check size limit
    if buf.len() > MAX_MESSAGE_SIZE {
        anyhow::bail!("message too large: {} > {}", buf.len(), MAX_MESSAGE_SIZE);
    }

    // Write length prefix
    let len = buf.len() as u32;
    send.write_all(&len.to_be_bytes()).await.context("failed to write message length")?;

    // Write message body
    send.write_all(&buf).await.context("failed to write message body")?;

    Ok(())
}
