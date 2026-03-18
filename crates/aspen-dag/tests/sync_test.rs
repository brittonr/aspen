//! Integration tests for DAG sync protocol.
//!
//! Tests the full sender→receiver flow using in-memory streams.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;

use aspen_dag::protocol::DagSyncRequest;
use aspen_dag::protocol::FullTraversalOpts;
use aspen_dag::protocol::InlinePolicy;
use aspen_dag::protocol::TraversalFilter;
use aspen_dag::protocol::TraversalOpts;
use aspen_dag::protocol::TraversalOrder;
use aspen_dag::sync::ReceivedFrame;
use aspen_dag::sync::recv_sync;
use aspen_dag::sync::send_sync;

fn hash(s: &str) -> [u8; 32] {
    *blake3::hash(s.as_bytes()).as_bytes()
}

/// Build a test DAG and produce the send/receive frame sequence.
///
/// The sender generates frames from the dag in DFS order.
/// Data is the hash input string (so we can verify content).
fn build_sync_frames(root: &str, edges: &[(&str, &[&str])]) -> Vec<([u8; 32], Option<Vec<u8>>)> {
    let mut children_map: HashMap<[u8; 32], Vec<[u8; 32]>> = HashMap::new();
    let mut data_map: HashMap<[u8; 32], Vec<u8>> = HashMap::new();

    // Register all nodes
    for (parent, kids) in edges {
        let ph = hash(parent);
        data_map.entry(ph).or_insert_with(|| parent.as_bytes().to_vec());
        let child_hashes: Vec<[u8; 32]> = kids.iter().map(|k| hash(k)).collect();
        children_map.insert(ph, child_hashes);
        for kid in *kids {
            let kh = hash(kid);
            data_map.entry(kh).or_insert_with(|| kid.as_bytes().to_vec());
            children_map.entry(kh).or_default();
        }
    }

    // DFS traversal to generate frame order
    let root_h = hash(root);
    let mut stack = vec![root_h];
    let mut visited = HashSet::new();
    let mut frames = Vec::new();

    while let Some(h) = stack.pop() {
        if !visited.insert(h) {
            continue;
        }
        let data = data_map.get(&h).cloned();
        frames.push((h, data));
        if let Some(kids) = children_map.get(&h) {
            for kid in kids.iter().rev() {
                stack.push(*kid);
            }
        }
    }

    frames
}

// ============================================================================
// Full Sync Tests
// ============================================================================

/// Full sync of a linear chain: A → B → C → D.
#[tokio::test]
async fn full_sync_linear_chain() {
    let frames = build_sync_frames("A", &[("A", &["B"]), ("B", &["C"]), ("C", &["D"])]);

    assert_eq!(frames.len(), 4);

    // Send
    let mut buf = Vec::new();
    let send_stats = send_sync(&mut buf, frames).await.unwrap();
    assert_eq!(send_stats.data_frames, 4);

    // Receive
    let mut received: Vec<([u8; 32], Vec<u8>)> = Vec::new();
    let recv_stats = recv_sync(&mut buf.as_slice(), |frame| {
        if let ReceivedFrame::Data { hash, data } = frame {
            received.push((hash, data));
        }
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(recv_stats.data_frames, 4);
    assert_eq!(received.len(), 4);

    // Verify order: A, B, C, D
    assert_eq!(received[0].1, b"A");
    assert_eq!(received[1].1, b"B");
    assert_eq!(received[2].1, b"C");
    assert_eq!(received[3].1, b"D");
}

/// Full sync of a binary tree.
///
/// ```text
///       A
///      / \
///     B   C
///    / \
///   D   E
/// ```
#[tokio::test]
async fn full_sync_binary_tree() {
    let frames = build_sync_frames("A", &[("A", &["B", "C"]), ("B", &["D", "E"])]);

    // DFS pre-order: A, B, D, E, C
    assert_eq!(frames.len(), 5);

    let mut buf = Vec::new();
    send_sync(&mut buf, frames).await.unwrap();

    let mut received_data: Vec<Vec<u8>> = Vec::new();
    recv_sync(&mut buf.as_slice(), |frame| {
        if let ReceivedFrame::Data { data, .. } = frame {
            received_data.push(data);
        }
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(received_data.len(), 5);
    assert_eq!(received_data[0], b"A");
    assert_eq!(received_data[1], b"B");
    assert_eq!(received_data[2], b"D");
    assert_eq!(received_data[3], b"E");
    assert_eq!(received_data[4], b"C");
}

/// Diamond DAG — shared node D is sent only once.
///
/// ```text
///     A
///    / \
///   B   C
///    \ /
///     D
/// ```
#[tokio::test]
async fn full_sync_diamond_dedup() {
    let frames = build_sync_frames("A", &[("A", &["B", "C"]), ("B", &["D"]), ("C", &["D"])]);

    // D should appear only once: A, B, D, C
    assert_eq!(frames.len(), 4);

    let mut buf = Vec::new();
    send_sync(&mut buf, frames).await.unwrap();

    let mut hashes: Vec<[u8; 32]> = Vec::new();
    recv_sync(&mut buf.as_slice(), |frame| {
        match frame {
            ReceivedFrame::Data { hash, .. } => hashes.push(hash),
            ReceivedFrame::HashOnly { hash } => hashes.push(hash),
        }
        Ok(())
    })
    .await
    .unwrap();

    // No duplicate hashes
    let unique: HashSet<[u8; 32]> = hashes.iter().copied().collect();
    assert_eq!(hashes.len(), unique.len(), "duplicate hashes in sync stream");
}

/// Mixed inline and hash-only frames.
/// Simulates stem/leaf split: inlines A, B (metadata), hash-only for C (large blob).
#[tokio::test]
async fn sync_mixed_inline_and_hash_only() {
    let a = hash("A");
    let b = hash("B");
    let c = hash("C");

    let frames = vec![
        (a, Some(b"A".to_vec())), // Commit — inline
        (b, Some(b"B".to_vec())), // Tree — inline
        (c, None),                // Blob — hash only, fetch later
    ];

    let mut buf = Vec::new();
    let stats = send_sync(&mut buf, frames).await.unwrap();
    assert_eq!(stats.data_frames, 2);
    assert_eq!(stats.hash_only_frames, 1);

    let mut data_count = 0u32;
    let mut hash_only_count = 0u32;
    recv_sync(&mut buf.as_slice(), |frame| {
        match frame {
            ReceivedFrame::Data { .. } => data_count += 1,
            ReceivedFrame::HashOnly { .. } => hash_only_count += 1,
        }
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(data_count, 2);
    assert_eq!(hash_only_count, 1);
}

// ============================================================================
// Request Serialization Tests
// ============================================================================

#[test]
fn request_full_with_filter_serialize() {
    let root = hash("root");
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::Exclude(BTreeSet::from([2])), // Exclude blobs
        }),
        inline: InlinePolicy::SizeThreshold(16384),
    };

    let bytes = request.to_bytes().unwrap();
    let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
    assert_eq!(request, decoded);
}

#[test]
fn request_sequence_serialize() {
    let hashes = vec![hash("h1"), hash("h2"), hash("h3")];
    let request = DagSyncRequest {
        traversal: TraversalOpts::Sequence(hashes),
        inline: InlinePolicy::All,
    };

    let bytes = request.to_bytes().unwrap();
    assert!(bytes.len() < 200, "sequence request unexpectedly large");
    let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
    assert_eq!(request, decoded);
}

#[test]
fn request_with_known_heads_serialize() {
    let root = hash("root");
    let known = BTreeSet::from([hash("head1"), hash("head2"), hash("head3")]);
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: known,
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::MetadataOnly,
    };

    let bytes = request.to_bytes().unwrap();
    let decoded = DagSyncRequest::from_bytes(&bytes).unwrap();
    assert_eq!(request, decoded);
}

/// Verify the sync stats track bytes correctly.
#[tokio::test]
async fn sync_stats_byte_tracking() {
    let data = b"some test data for byte tracking";
    let h = *blake3::hash(data).as_bytes();

    let frames = vec![(h, Some(data.to_vec()))];

    let mut buf = Vec::new();
    let send_stats = send_sync(&mut buf, frames).await.unwrap();

    // 33 (header) + 8 (size u64) + data.len()
    let expected = 33 + 8 + data.len() as u64;
    assert_eq!(send_stats.bytes_transferred, expected);
    assert_eq!(buf.len() as u64, expected);
}
