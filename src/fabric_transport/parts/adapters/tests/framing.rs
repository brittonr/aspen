//! The Iroh loopback server reads and echoes one length-prefixed bounded frame: a valid frame
//! round-trips, while an over-bound, zero, short or trailing frame is refused.

use super::*;

const TEST_MAX_FRAME_BYTES: u64 = 64;
const TEST_ALPN: &[u8] = b"molten-loopback-frame-bound-test";

fn prefixed(declared_bytes: u64, payload: &[u8]) -> Vec<u8> {
    let mut raw = declared_bytes.to_be_bytes().to_vec();
    raw.extend_from_slice(payload);
    raw
}

/// Await one client step under the loopback timeout, so a stalled peer fails the test.
async fn within<T, E: std::fmt::Display>(
    timeout: std::time::Duration,
    phase: &str,
    step: impl Future<Output = std::result::Result<T, E>>,
) -> crate::error::Result<T> {
    tokio::time::timeout(timeout, step)
        .await
        .map_err(|_| crate::error::Failure::invalid_harness(format!("test client {phase} timed out")))?
        .map_err(iroh_error)
}

/// Write `raw` bytes to the loopback server's stream and return the server's outcome.
async fn serve_raw(raw: &[u8]) -> crate::error::Result<Vec<u8>> {
    let timeout = std::time::Duration::from_secs(LIVE_LOOPBACK_TIMEOUT_SECONDS);
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let server = bind_loopback_endpoint(&lookup, TEST_ALPN).await?;
    let client = bind_loopback_endpoint(&lookup, TEST_ALPN).await?;
    lookup.add_endpoint_info(server.addr());
    lookup.add_endpoint_info(client.addr());
    let server_task = tokio::spawn(echo_one_stream(server.clone(), timeout, TEST_MAX_FRAME_BYTES));

    let connection = within(timeout, "connect", client.connect(server.addr(), TEST_ALPN)).await?;
    let (mut send, mut receive) = within(timeout, "stream open", connection.open_bi()).await?;
    within(timeout, "write", send.write_all(raw)).await?;
    send.finish().map_err(iroh_error)?;
    // A refused frame ends the server's connection, so this read fails instead of waiting.
    let _echo = read_bounded_frame(&mut receive, TEST_MAX_FRAME_BYTES, timeout).await;
    connection.close(IROH_CLOSE_CODE.into(), IROH_CLOSE_REASON);

    let served = server_task.await.expect("loopback server task");
    client.close().await;
    server.close().await;
    served.map(|(received, _remote)| received)
}

// r[verify molten.fabric_transport.flow_control]
#[tokio::test]
async fn loopback_echo_accepts_exactly_one_bounded_prefixed_frame() {
    let payload = vec![7_u8; usize::try_from(TEST_MAX_FRAME_BYTES).expect("bound")];
    let received = serve_raw(&prefixed(TEST_MAX_FRAME_BYTES, &payload)).await.expect("a full-bound frame echoes");
    assert_eq!(received, payload);
    let refusals = [
        ("over-bound", prefixed(TEST_MAX_FRAME_BYTES + 1, &[7; 65]), Some("exceeds bound")),
        ("zero-length", prefixed(0, &[]), Some("exceeds bound")),
        ("unprefixed", b"raw-unprefixed-frame".to_vec(), Some("exceeds bound")),
        // The zero-byte terminal read refuses any trailing byte as an over-long stream.
        ("trailing", prefixed(3, b"three+extra"), Some("stream too long")),
        ("short", prefixed(10, b"four"), None),
    ];
    for (label, raw, expected) in refusals {
        let refused = serve_raw(&raw).await.expect_err(label).to_string();
        if let Some(expected) = expected {
            assert!(refused.contains(expected), "{label}: {refused}");
        }
    }
}
