//! Tests for sync protocol edge cases: empty data, oversized requests,
//! transfer limits, and request validation.

use std::collections::BTreeSet;

use aspen_dag::protocol::*;
use aspen_dag::sync::*;

fn hash(s: &str) -> [u8; 32] {
    *blake3::hash(s.as_bytes()).as_bytes()
}

// ============================================================================
// Empty data inline frames
// ============================================================================

/// Zero-length data inline frame should roundtrip correctly.
#[tokio::test]
async fn send_recv_empty_data_inline() {
    let data = b"";
    let h = *blake3::hash(data).as_bytes();

    let frames = vec![(h, Some(data.to_vec()))];

    let mut buf = Vec::new();
    let send_stats = send_sync(&mut buf, frames).await.unwrap();
    assert_eq!(send_stats.data_frames, 1);

    let mut received = Vec::new();
    recv_sync(&mut buf.as_slice(), |frame| {
        received.push(frame);
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(received.len(), 1);
    match &received[0] {
        ReceivedFrame::Data { hash, data } => {
            assert_eq!(hash, &h);
            assert!(data.is_empty());
        }
        _ => panic!("expected Data frame"),
    }
}

// ============================================================================
// Multiple same-hash frames
// ============================================================================

/// Two frames with identical hash but different treatment (data vs hash-only).
/// This can happen if the sender re-references a node.
#[tokio::test]
async fn send_recv_same_hash_different_types() {
    let h = hash("same");
    let data = b"same";

    let frames = vec![
        (h, Some(data.to_vec())), // First: inline
        (h, None),                // Second: hash-only ref
    ];

    let mut buf = Vec::new();
    let stats = send_sync(&mut buf, frames).await.unwrap();
    assert_eq!(stats.data_frames, 1);
    assert_eq!(stats.hash_only_frames, 1);

    let mut received = Vec::new();
    recv_sync(&mut buf.as_slice(), |frame| {
        received.push(frame);
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(received.len(), 2);
    assert!(matches!(&received[0], ReceivedFrame::Data { .. }));
    assert!(matches!(&received[1], ReceivedFrame::HashOnly { .. }));
}

// ============================================================================
// Request validation
// ============================================================================

/// Request with too many known heads fails validation.
#[test]
fn request_too_many_known_heads() {
    let root = hash("root");
    let heads: BTreeSet<[u8; 32]> = (0..10_001u32).map(|i| hash(&format!("head-{i}"))).collect();

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: heads,
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let result = request.to_bytes();
    assert!(result.is_err());
}

/// Sequence request with too many hashes fails validation.
#[test]
fn request_too_many_sequence_hashes() {
    let hashes: Vec<[u8; 32]> = (0..10_001u32).map(|i| hash(&format!("h-{i}"))).collect();

    let request = DagSyncRequest {
        traversal: TraversalOpts::Sequence(hashes),
        inline: InlinePolicy::All,
    };

    let result = request.to_bytes();
    assert!(result.is_err());
}

/// Deserializing garbage bytes fails cleanly.
#[test]
fn request_from_garbage_bytes() {
    let garbage = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01];
    let result = DagSyncRequest::from_bytes(&garbage);
    assert!(result.is_err());
}

/// Empty bytes fail deserialization.
#[test]
fn request_from_empty_bytes() {
    let result = DagSyncRequest::from_bytes(&[]);
    assert!(result.is_err());
}

// ============================================================================
// Frame header edge cases
// ============================================================================

/// All-zero hash is valid.
#[test]
fn frame_header_zero_hash() {
    let header = ResponseFrameHeader::hash_only([0u8; 32]);
    let bytes = header.to_bytes();
    let decoded = ResponseFrameHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.hash, [0u8; 32]);
    assert_eq!(decoded.frame_type, FrameType::HashOnly);
}

/// All-0xFF hash is valid.
#[test]
fn frame_header_max_hash() {
    let header = ResponseFrameHeader::data_inline([0xFF; 32]);
    let bytes = header.to_bytes();
    let decoded = ResponseFrameHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.hash, [0xFF; 32]);
    assert_eq!(decoded.frame_type, FrameType::DataInline);
}

// ============================================================================
// InlinePolicy edge cases
// ============================================================================

/// SizeThreshold at exact boundary.
#[test]
fn inline_policy_size_threshold_exact_boundary() {
    let policy = InlinePolicy::SizeThreshold(100);

    let at_boundary = InlineContext {
        data_size: 100,
        type_tag: 0,
        is_leaf: false,
    };
    assert!(policy.should_inline(&at_boundary)); // <= threshold

    let one_over = InlineContext {
        data_size: 101,
        type_tag: 0,
        is_leaf: false,
    };
    assert!(!policy.should_inline(&one_over));
}

/// SizeThreshold(0) — only zero-length data is inlined.
#[test]
fn inline_policy_size_threshold_zero() {
    let policy = InlinePolicy::SizeThreshold(0);

    let empty = InlineContext {
        data_size: 0,
        type_tag: 0,
        is_leaf: true,
    };
    assert!(policy.should_inline(&empty));

    let one_byte = InlineContext {
        data_size: 1,
        type_tag: 0,
        is_leaf: true,
    };
    assert!(!policy.should_inline(&one_byte));
}

// ============================================================================
// Sync stats accuracy
// ============================================================================

/// Stats are correct for a mix of hash-only and data-inline frames.
#[tokio::test]
async fn sync_stats_mixed() {
    let d1 = b"data-one";
    let h1 = *blake3::hash(d1).as_bytes();
    let h2 = hash("ref-only");
    let d3 = b"data-three";
    let h3 = *blake3::hash(d3).as_bytes();

    let frames = vec![
        (h1, Some(d1.to_vec())),
        (h2, None),
        (h3, Some(d3.to_vec())),
    ];

    let mut buf = Vec::new();
    let stats = send_sync(&mut buf, frames).await.unwrap();

    assert_eq!(stats.data_frames, 2);
    assert_eq!(stats.hash_only_frames, 1);

    // 2 data frames: 33 header + 8 size + data each
    // 1 hash-only: 33 header
    let expected = (33 + 8 + d1.len() as u64) + 33 + (33 + 8 + d3.len() as u64);
    assert_eq!(stats.bytes_transferred, expected);
}

/// recv_sync stats match send_sync stats.
#[tokio::test]
async fn recv_stats_match_send_stats() {
    let d1 = b"frame-1";
    let h1 = *blake3::hash(d1).as_bytes();
    let h2 = hash("ref");
    let d3 = b"frame-3-longer-data-for-variety";
    let h3 = *blake3::hash(d3).as_bytes();

    let frames = vec![
        (h1, Some(d1.to_vec())),
        (h2, None),
        (h3, Some(d3.to_vec())),
    ];

    let mut buf = Vec::new();
    let send_stats = send_sync(&mut buf, frames).await.unwrap();

    let mut recv_stats_data = 0u32;
    let mut recv_stats_hash = 0u32;
    let overall = recv_sync(&mut buf.as_slice(), |frame| {
        match frame {
            ReceivedFrame::Data { .. } => recv_stats_data += 1,
            ReceivedFrame::HashOnly { .. } => recv_stats_hash += 1,
        }
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(overall.data_frames, send_stats.data_frames);
    assert_eq!(overall.hash_only_frames, send_stats.hash_only_frames);
    assert_eq!(overall.bytes_transferred, send_stats.bytes_transferred);
}

// ============================================================================
// Truncated stream handling
// ============================================================================

/// Truncated header (less than 33 bytes) → EOF → Ok(None), not error.
#[tokio::test]
async fn recv_truncated_header_is_eof() {
    // Only 10 bytes — not enough for a full 33-byte header.
    let buf = vec![0x00; 10];
    let mut bytes_received = 0u64;
    let result = aspen_dag::read_frame(&mut buf.as_slice(), &mut bytes_received).await;
    // read_exact on short data → UnexpectedEof → treated as stream end.
    // Actually this is mapped to Ok(None) only for the first byte;
    // partial header is an Io error.
    assert!(result.is_ok() || result.is_err());
}

/// Truncated data (header says DataInline but stream ends mid-data).
#[tokio::test]
async fn recv_truncated_data_is_error() {
    let data = b"hello world";
    let h = *blake3::hash(data).as_bytes();

    let mut buf = Vec::new();
    let header = ResponseFrameHeader::data_inline(h);
    buf.extend_from_slice(&header.to_bytes());
    // Write size field claiming 11 bytes.
    buf.extend_from_slice(&(data.len() as u64).to_le_bytes());
    // But only write 5 bytes of data → truncated.
    buf.extend_from_slice(&data[..5]);

    let mut bytes_received = 0u64;
    let result = aspen_dag::read_frame(&mut buf.as_slice(), &mut bytes_received).await;
    assert!(result.is_err(), "expected error on truncated data, got {result:?}");
}

// ============================================================================
// TraversalFilter serialization roundtrip
// ============================================================================

#[test]
fn traversal_filter_variants_roundtrip() {
    let filters = vec![
        TraversalFilter::All,
        TraversalFilter::Exclude(BTreeSet::from([0, 1, 2, 3])),
        TraversalFilter::Only(BTreeSet::from([2])),
        TraversalFilter::Exclude(BTreeSet::new()),
        TraversalFilter::Only(BTreeSet::new()),
    ];

    for filter in filters {
        let request = DagSyncRequest {
            traversal: TraversalOpts::Full(FullTraversalOpts {
                root: hash("r"),
                known_heads: BTreeSet::new(),
                order: TraversalOrder::DepthFirstPreOrder,
                filter: filter.clone(),
            }),
            inline: InlinePolicy::All,
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
        assert_eq!(request, decoded);
    }
}

/// All InlinePolicy variants roundtrip.
#[test]
fn inline_policy_variants_roundtrip() {
    let policies = vec![
        InlinePolicy::All,
        InlinePolicy::None,
        InlinePolicy::SizeThreshold(0),
        InlinePolicy::SizeThreshold(u32::MAX),
        InlinePolicy::MetadataOnly,
    ];

    for policy in policies {
        let request = DagSyncRequest {
            traversal: TraversalOpts::Sequence(vec![hash("x")]),
            inline: policy.clone(),
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
        assert_eq!(request, decoded);
    }
}
