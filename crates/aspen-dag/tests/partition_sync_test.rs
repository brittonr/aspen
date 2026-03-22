//! Deterministic tests for DAG sync under adverse network conditions.
//!
//! These tests simulate network partitions and connection drops using
//! in-memory channels and controlled failure injection, without requiring
//! real network or madsim. They validate that:
//!
//! - Partial sync state is recoverable after interruption
//! - DAG traversal produces consistent results across retries
//! - Connection drops don't corrupt receiver state

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_dag::DagTraversal;
use aspen_dag::FnLinkExtractor;
use aspen_dag::FullTraversal;
use aspen_dag::TraversalResult;
use aspen_dag::protocol::DagSyncRequest;
use aspen_dag::protocol::InlinePolicy;
use aspen_dag::protocol::TraversalOpts;
use aspen_dag::sync::ReceivedFrame;
use aspen_dag::sync::SyncStats;
use aspen_dag::sync::recv_sync;
use aspen_dag::sync::send_sync;

// ============================================================================
// Helpers
// ============================================================================

#[derive(Clone, Default)]
struct TestDag {
    edges: HashMap<blake3::Hash, Vec<blake3::Hash>>,
    data: HashMap<blake3::Hash, Vec<u8>>,
}

impl TestDag {
    fn new() -> Self {
        Self::default()
    }

    fn add_node(&mut self, name: &str) -> blake3::Hash {
        let data = name.as_bytes().to_vec();
        let hash = blake3::hash(&data);
        self.data.insert(hash, data);
        self.edges.entry(hash).or_default();
        hash
    }

    fn add_edge(&mut self, parent: blake3::Hash, child: blake3::Hash) {
        self.edges.entry(parent).or_default().push(child);
    }

    fn link_extractor(
        &self,
    ) -> FnLinkExtractor<blake3::Hash, impl Fn(&blake3::Hash) -> TraversalResult<Vec<blake3::Hash>>> {
        let edges = self.edges.clone();
        FnLinkExtractor::new(move |h: &blake3::Hash| Ok(edges.get(h).cloned().unwrap_or_default()))
    }
}

/// Collect full traversal order from a DAG.
async fn traverse_all(dag: &TestDag, root: blake3::Hash) -> Vec<blake3::Hash> {
    let links = dag.link_extractor();
    let mut trav = FullTraversal::new(root, (), links);
    let mut visited = vec![];
    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }
    visited
}

/// Build frames from a traversal (sender side).
fn build_frames(dag: &TestDag, traversal_order: &[blake3::Hash]) -> Vec<([u8; 32], Option<Vec<u8>>)> {
    traversal_order
        .iter()
        .map(|h| {
            let data = dag.data.get(h).cloned();
            (*h.as_bytes(), data)
        })
        .collect()
}

// ============================================================================
// Test: Partial sync resumption after simulated connection drop
// ============================================================================

/// Simulates a connection drop mid-stream by truncating the sender's output.
/// The receiver processes whatever frames arrived before the drop, then
/// requests a second sync from the sender. The sender re-traverses from
/// root with the receiver's known set as `known_heads`, which prunes
/// already-synced subtrees. The root itself is re-sent (cheap duplicate)
/// because the sender must start from a root, and the receiver deduplicates.
#[tokio::test]
async fn partial_sync_resumption_after_drop() {
    let mut dag = TestDag::new();
    let a = dag.add_node("a");
    let b = dag.add_node("b");
    let c = dag.add_node("c");
    let d = dag.add_node("d");
    let e = dag.add_node("e");

    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_edge(b, d);
    dag.add_edge(c, e);

    // Full traversal order: a, b, d, c, e
    let full_order = traverse_all(&dag, a).await;
    assert_eq!(full_order.len(), 5);

    // Build all frames
    let all_frames = build_frames(&dag, &full_order);

    // Simulate sending only the first 3 frames before "connection drop"
    let partial_frames = &all_frames[..3];

    let mut buf = Vec::new();
    let stats = send_sync(&mut buf, partial_frames.iter().cloned()).await.unwrap();
    assert_eq!(stats.data_frames, 3);

    // Receiver processes partial stream
    let mut received_hashes: HashSet<blake3::Hash> = HashSet::new();
    recv_sync(&mut buf.as_slice(), |frame| {
        match frame {
            ReceivedFrame::Data { hash, .. } => {
                received_hashes.insert(blake3::Hash::from_bytes(hash));
            }
            ReceivedFrame::HashOnly { hash } => {
                received_hashes.insert(blake3::Hash::from_bytes(hash));
            }
        }
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(received_hashes.len(), 3);

    // Resume: sender re-traverses from root, using the receiver's known
    // hashes as the visited set (not known_heads, since known_heads would
    // prevent root expansion). The sender uses `with_visited` so
    // already-received nodes are skipped but their children are still
    // discovered via the root's child expansion.
    //
    // Note: the root `a` is already visited, so the traversal skips it.
    // But its children were pushed onto the stack during the FIRST traversal's
    // `next()` call. In the resumed traversal, the root is already visited
    // so we need to pre-seed the stack by starting fresh and letting the
    // visited set handle dedup.
    //
    // The practical pattern: sender traverses from root with known_heads
    // set to the received internal nodes (not the root). The root is
    // re-sent as a cheap duplicate.
    let mut known = received_hashes.clone();
    known.remove(&a); // Don't mark root as known — sender must re-expand it

    let links = dag.link_extractor();
    let mut trav = FullTraversal::with_known_heads(a, (), links, known);

    let mut resumed = vec![];
    while let Some(h) = trav.next().await.unwrap() {
        resumed.push(h);
    }

    // Root is re-sent (cheap dup), plus the missing nodes c and e
    assert!(resumed.contains(&a), "root is re-sent on resumption");
    assert!(resumed.contains(&c), "missing node c should be in resumed set");
    assert!(resumed.contains(&e), "missing node e should be in resumed set");

    // b and d should NOT appear (they were in known_heads)
    assert!(!resumed.contains(&b));
    assert!(!resumed.contains(&d));

    // Together: received + resumed covers the full DAG
    let mut all: HashSet<blake3::Hash> = received_hashes;
    all.extend(resumed.iter());
    let full_set: HashSet<blake3::Hash> = full_order.iter().copied().collect();
    assert_eq!(all, full_set, "first + resumed should cover the entire DAG");
}

// ============================================================================
// Test: Network partition — sender and receiver diverge, then converge
// ============================================================================

/// Two nodes have partial overlapping DAGs. After a "partition heals",
/// they sync using known_heads to transfer only the delta.
#[tokio::test]
async fn sync_after_partition_transfers_delta_only() {
    // Node A's DAG: root → {x, y}
    // Node B's DAG: root → {x, z}  (B has x but not y; B has z which A lacks)
    // After partition heals, A syncs from B: B sends z (A sends y separately)

    let mut shared_dag = TestDag::new();
    let root = shared_dag.add_node("root");
    let x = shared_dag.add_node("x");
    let y = shared_dag.add_node("y");
    let z = shared_dag.add_node("z");

    // Full DAG (what B has)
    shared_dag.add_edge(root, x);
    shared_dag.add_edge(root, z);

    // A knows: root, x, y. But sender always starts from root, so root
    // is excluded from known_heads (sender must re-expand it).
    let a_known_children: HashSet<blake3::Hash> = HashSet::from([x, y]);

    // B traverses from root with A's known *children* as termination points.
    // Root is re-sent (cheap dup), z is the new data.
    let links = shared_dag.link_extractor();
    let mut trav = FullTraversal::with_known_heads(root, (), links, a_known_children);

    let mut b_sends = vec![];
    while let Some(h) = trav.next().await.unwrap() {
        b_sends.push(h);
    }

    // B sends root (re-expanded) + z. x is skipped (known head).
    assert!(b_sends.contains(&root), "root is re-sent for child expansion");
    assert!(b_sends.contains(&z), "z is the new node");
    assert!(!b_sends.contains(&x), "x was known to A");
    assert_eq!(b_sends.len(), 2);
}

// ============================================================================
// Test: Determinism across retries after failures
// ============================================================================

/// Verifies that traversal order is identical across multiple retries,
/// even when partial progress was made before each failure.
#[tokio::test]
async fn traversal_deterministic_across_retries() {
    let mut dag = TestDag::new();
    let r = dag.add_node("root");
    let a = dag.add_node("a");
    let b = dag.add_node("b");
    let c = dag.add_node("c");
    let d = dag.add_node("d");
    let e = dag.add_node("e");
    let f = dag.add_node("f");

    dag.add_edge(r, a);
    dag.add_edge(r, b);
    dag.add_edge(a, c);
    dag.add_edge(a, d);
    dag.add_edge(b, e);
    dag.add_edge(b, f);

    let reference = traverse_all(&dag, r).await;
    assert_eq!(reference.len(), 7);

    // Retry 20 times — each time traversal must produce the same sequence
    for _ in 0..20 {
        let result = traverse_all(&dag, r).await;
        assert_eq!(result, reference, "traversal must be deterministic on retry");
    }
}

// ============================================================================
// Test: Interrupted transfer preserves receiver state
// ============================================================================

/// Receiver gets partial data, then the stream ends abruptly.
/// Verify that the data received so far is correct and usable.
#[tokio::test]
async fn interrupted_transfer_preserves_partial_state() {
    let mut dag = TestDag::new();
    let a = dag.add_node("alpha");
    let b = dag.add_node("beta");
    let c = dag.add_node("gamma");

    dag.add_edge(a, b);
    dag.add_edge(a, c);

    let order = traverse_all(&dag, a).await;
    let frames = build_frames(&dag, &order);

    // Send only the first 2 frames
    let mut buf = Vec::new();
    send_sync(&mut buf, frames[..2].iter().cloned()).await.unwrap();

    // Receiver collects what it can
    let mut store: HashMap<blake3::Hash, Vec<u8>> = HashMap::new();
    recv_sync(&mut buf.as_slice(), |frame| {
        if let ReceivedFrame::Data { hash, data } = frame {
            store.insert(blake3::Hash::from_bytes(hash), data);
        }
        Ok(())
    })
    .await
    .unwrap();

    // Exactly 2 items received with correct content
    assert_eq!(store.len(), 2);

    for (hash, data) in &store {
        // Verify BLAKE3 integrity of each received item
        assert_eq!(&blake3::hash(data), hash);
    }
}

// ============================================================================
// Test: Empty known_heads = full sync (no partition assumed)
// ============================================================================

/// When known_heads is empty, the entire DAG is transferred.
#[tokio::test]
async fn empty_known_heads_means_full_sync() {
    let mut dag = TestDag::new();
    let a = dag.add_node("a");
    let b = dag.add_node("b");
    let c = dag.add_node("c");
    dag.add_edge(a, b);
    dag.add_edge(a, c);

    let full = traverse_all(&dag, a).await;

    let links = dag.link_extractor();
    let mut trav = FullTraversal::with_known_heads(a, (), links, HashSet::new());
    let mut result = vec![];
    while let Some(h) = trav.next().await.unwrap() {
        result.push(h);
    }

    assert_eq!(result, full, "empty known_heads should produce full traversal");
}
