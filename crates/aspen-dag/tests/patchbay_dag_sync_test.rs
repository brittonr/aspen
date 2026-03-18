//! DAG sync integration tests over patchbay simulated networks.
//!
//! Uses patchbay network namespaces for real iroh QUIC connections without
//! relay servers. Tests run in <1s (vs ~9s for relay-dependent tests).
//!
//! Requirements: Linux with unprivileged user namespaces, nft, tc in PATH.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use aspen_dag::handler::DagSyncProtocolHandler;
use aspen_dag::handler::connect_dag_sync;
use aspen_dag::protocol::*;
use aspen_dag::sync::ReceivedFrame;
use aspen_dag::sync::recv_sync;
use aspen_dag::sync::send_sync;
use aspen_testing_patchbay::skip_unless_patchbay;
use patchbay::Lab;
use patchbay::RouterPreset;

/// Hash → (data, children)
type TestDag = HashMap<[u8; 32], (Vec<u8>, Vec<[u8; 32]>)>;

/// Initialize user namespace before tokio runtime starts.
#[ctor::ctor]
fn init_userns() {
    if aspen_testing_patchbay::skip::patchbay_available() {
        unsafe { patchbay::init_userns_for_ctor() };
    }
}

fn hash_of(s: &str) -> [u8; 32] {
    *blake3::hash(s.as_bytes()).as_bytes()
}

fn build_test_dag(edges: &[(&str, &[&str])]) -> TestDag {
    let mut dag: TestDag = HashMap::new();
    for (parent, kids) in edges {
        let ph = hash_of(parent);
        let child_hashes: Vec<[u8; 32]> = kids.iter().map(|k| hash_of(k)).collect();
        dag.entry(ph).or_insert_with(|| (parent.as_bytes().to_vec(), vec![])).1 = child_hashes;
        for kid in *kids {
            let kh = hash_of(kid);
            dag.entry(kh).or_insert_with(|| (kid.as_bytes().to_vec(), vec![]));
        }
    }
    dag
}

fn dfs_frames(dag: &TestDag, root: [u8; 32], known_heads: &HashSet<[u8; 32]>) -> Vec<([u8; 32], Option<Vec<u8>>)> {
    let mut stack = vec![root];
    let mut visited = HashSet::new();
    let mut frames = Vec::new();
    while let Some(h) = stack.pop() {
        if !visited.insert(h) || known_heads.contains(&h) {
            continue;
        }
        if let Some((data, children)) = dag.get(&h) {
            frames.push((h, Some(data.clone())));
            for child in children.iter().rev() {
                stack.push(*child);
            }
        }
    }
    frames
}

/// Spin up a sender endpoint + handler inside a patchbay device namespace.
/// Returns the sender's EndpointAddr via a channel once ready.
/// Keeps running until the shutdown signal fires.
async fn spawn_sender(
    sender_dev: patchbay::Device,
    dag: Arc<TestDag>,
    root: [u8; 32],
) -> (iroh::EndpointAddr, tokio::sync::oneshot::Sender<()>) {
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    sender_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![DAG_SYNC_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            let handler = DagSyncProtocolHandler::from_fn(move |request, mut send| {
                let dag = Arc::clone(&dag);
                async move {
                    let known: HashSet<[u8; 32]> = match &request.traversal {
                        TraversalOpts::Full(opts) => opts.known_heads.iter().copied().collect(),
                        _ => HashSet::new(),
                    };

                    let all_frames = dfs_frames(&dag, root, &known);

                    // Apply filter
                    let filter = match &request.traversal {
                        TraversalOpts::Full(opts) => &opts.filter,
                        _ => &TraversalFilter::All,
                    };
                    let frames: Vec<_> = all_frames
                        .into_iter()
                        .filter(|(h, _)| match filter {
                            TraversalFilter::All => true,
                            TraversalFilter::Exclude(tags) => {
                                // Convention: tag 2 = blob hashes (those with no children in dag)
                                let is_blob = dag.get(h).map(|(_, c)| c.is_empty()).unwrap_or(false);
                                !(is_blob && tags.contains(&2))
                            }
                            TraversalFilter::Only(tags) => {
                                let is_blob = dag.get(h).map(|(_, c)| c.is_empty()).unwrap_or(false);
                                is_blob && tags.contains(&2)
                            }
                        })
                        .collect();

                    let stats = send_sync(&mut send, frames).await?;
                    send.finish().map_err(|e| aspen_dag::ProtocolError::Io {
                        source: std::io::Error::other(e.to_string()),
                    })?;
                    Ok(stats)
                }
            });

            let _router = iroh::protocol::Router::builder(ep.clone()).accept(DAG_SYNC_ALPN, handler).spawn();

            let addr = ep.addr();
            let _ = addr_tx.send(addr);

            // Keep alive until shutdown signal
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let addr = addr_rx.await.unwrap();
    (addr, shutdown_tx)
}

/// Run the receiver inside a patchbay device namespace.
/// Connects to the sender, sends the request, and collects received data.
async fn run_receiver(
    receiver_dev: patchbay::Device,
    sender_addr: iroh::EndpointAddr,
    request: DagSyncRequest,
) -> Vec<Vec<u8>> {
    let jh = receiver_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();

            let mut conn = connect_dag_sync(&ep, sender_addr, &request).await.unwrap();

            let data = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let data_clone = std::sync::Arc::clone(&data);
            recv_sync(&mut conn.recv, |frame| {
                if let ReceivedFrame::Data { data: d, .. } = frame {
                    data_clone.lock().unwrap().push(d);
                }
                Ok(())
            })
            .await
            .unwrap();

            data.lock().unwrap().clone()
        })
        .unwrap();

    jh.await.unwrap()
}

// ============================================================================
// Tests
// ============================================================================

/// Full sync: A → B → C → D, all objects transferred.
#[tokio::test]
async fn patchbay_full_sync_linear_chain() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();

    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    let dag = Arc::new(build_test_dag(&[("A", &["B"]), ("B", &["C"]), ("C", &["D"])]));
    let root = hash_of("A");

    let (sender_addr, shutdown) = spawn_sender(sender_dev, Arc::clone(&dag), root).await;

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let received = run_receiver(receiver_dev, sender_addr, request).await;

    assert_eq!(received.len(), 4);
    assert_eq!(received[0], b"A");
    assert_eq!(received[1], b"B");
    assert_eq!(received[2], b"C");
    assert_eq!(received[3], b"D");

    let _ = shutdown.send(());
}

/// Partial sync: receiver has C,D — only A,B should be transferred.
#[tokio::test]
async fn patchbay_partial_sync_known_heads() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();

    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    let dag = Arc::new(build_test_dag(&[("A", &["B"]), ("B", &["C"]), ("C", &["D"])]));
    let root = hash_of("A");

    let (sender_addr, shutdown) = spawn_sender(sender_dev, Arc::clone(&dag), root).await;

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: BTreeSet::from([hash_of("C")]),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let received = run_receiver(receiver_dev, sender_addr, request).await;

    assert_eq!(received.len(), 2);
    assert_eq!(received[0], b"A");
    assert_eq!(received[1], b"B");

    let _ = shutdown.send(());
}

/// Stem/leaf split: two-phase sync.
/// Phase 1: structure only (commits, trees). Phase 2: blobs only.
#[tokio::test]
async fn patchbay_stem_leaf_split() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();

    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    // root_commit → tree → [blob1, blob2]
    // blob1 and blob2 are leaf nodes (no children)
    let dag = Arc::new(build_test_dag(&[("root_commit", &["tree"]), ("tree", &["blob1", "blob2"])]));
    let root = hash_of("root_commit");

    let (sender_addr, shutdown) = spawn_sender(sender_dev, Arc::clone(&dag), root).await;

    // Phase 1: stem sync — exclude blobs (type tag 2)
    let stem_request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::Exclude(BTreeSet::from([2])),
        }),
        inline: InlinePolicy::All,
    };

    let stem = run_receiver(receiver_dev.clone(), sender_addr.clone(), stem_request).await;
    assert_eq!(stem.len(), 2);
    assert_eq!(stem[0], b"root_commit");
    assert_eq!(stem[1], b"tree");

    // Phase 2: leaf sync — only blobs (type tag 2)
    let leaf_request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::Only(BTreeSet::from([2])),
        }),
        inline: InlinePolicy::All,
    };

    let leaves = run_receiver(receiver_dev, sender_addr, leaf_request).await;
    assert_eq!(leaves.len(), 2);
    let leaf_strs: Vec<&str> = leaves.iter().map(|d| std::str::from_utf8(d).unwrap()).collect();
    assert!(leaf_strs.contains(&"blob1"));
    assert!(leaf_strs.contains(&"blob2"));

    let _ = shutdown.send(());
}

/// Diamond DAG deduplication: shared nodes only transferred once.
///
/// ```text
///     A
///    / \
///   B   C
///    \ /
///     D
/// ```
#[tokio::test]
async fn patchbay_diamond_dedup() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();

    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    let dag = Arc::new(build_test_dag(&[("A", &["B", "C"]), ("B", &["D"]), ("C", &["D"])]));
    let root = hash_of("A");

    let (sender_addr, shutdown) = spawn_sender(sender_dev, Arc::clone(&dag), root).await;

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root,
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let received = run_receiver(receiver_dev, sender_addr, request).await;

    // A, B, D, C — D only once despite being referenced by both B and C
    assert_eq!(received.len(), 4);
    assert_eq!(received[0], b"A");
    assert_eq!(received[1], b"B");
    assert_eq!(received[2], b"D");
    assert_eq!(received[3], b"C");

    let _ = shutdown.send(());
}
