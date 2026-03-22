//! Benchmarks for DAG sync throughput and traversal strategies.
//!
//! Measures:
//! - DAG sync throughput vs per-object fetch for a 5,000-object repo
//! - Stem/leaf split with multiple peers vs single-peer full sync

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_dag::DagTraversal;
use aspen_dag::FnLinkExtractor;
use aspen_dag::FullTraversal;
use aspen_dag::SequenceTraversal;
use aspen_dag::TraversalResult;
use aspen_dag::sync::send_sync;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;

// ============================================================================
// Helpers
// ============================================================================

#[derive(Clone)]
struct BenchDag {
    edges: HashMap<blake3::Hash, Vec<blake3::Hash>>,
    data: HashMap<blake3::Hash, Vec<u8>>,
    all_hashes: Vec<blake3::Hash>,
    root: blake3::Hash,
}

/// Build a synthetic repo-like DAG with the given number of objects.
///
/// Structure: N/10 commits in a linear chain, each pointing to a tree
/// with 9 blob children. Total objects ≈ N.
fn build_repo_dag(object_count: u32) -> BenchDag {
    let commits = (object_count / 10).max(1);
    let blobs_per_tree = 9u32;

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
    let mut data: HashMap<blake3::Hash, Vec<u8>> = HashMap::new();
    let mut all_hashes = Vec::new();
    let mut prev_commit: Option<blake3::Hash> = None;
    let mut root = blake3::hash(b"genesis");

    for i in 0..commits {
        // Create blobs
        let mut blob_hashes = Vec::new();
        for j in 0..blobs_per_tree {
            let blob_data = format!("blob-{}-{}", i, j).into_bytes();
            let blob_hash = blake3::hash(&blob_data);
            data.insert(blob_hash, blob_data);
            edges.entry(blob_hash).or_default();
            all_hashes.push(blob_hash);
            blob_hashes.push(blob_hash);
        }

        // Create tree pointing to blobs
        let tree_data = format!("tree-{}", i).into_bytes();
        let tree_hash = blake3::hash(&tree_data);
        data.insert(tree_hash, tree_data);
        edges.insert(tree_hash, blob_hashes);
        all_hashes.push(tree_hash);

        // Create commit pointing to tree + parent
        let commit_data = format!("commit-{}", i).into_bytes();
        let commit_hash = blake3::hash(&commit_data);
        data.insert(commit_hash, commit_data);
        let mut commit_children = vec![tree_hash];
        if let Some(parent) = prev_commit {
            commit_children.push(parent);
        }
        edges.insert(commit_hash, commit_children);
        all_hashes.push(commit_hash);

        prev_commit = Some(commit_hash);
        root = commit_hash;
    }

    BenchDag {
        edges,
        data,
        all_hashes,
        root,
    }
}

fn link_extractor(
    dag: &BenchDag,
) -> FnLinkExtractor<blake3::Hash, impl Fn(&blake3::Hash) -> TraversalResult<Vec<blake3::Hash>>> {
    let edges = dag.edges.clone();
    FnLinkExtractor::new(move |h: &blake3::Hash| Ok(edges.get(h).cloned().unwrap_or_default()))
}

fn tokio_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

// ============================================================================
// Benchmark: DAG sync throughput vs per-object fetch
// ============================================================================

fn bench_dag_sync_vs_per_object(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_sync_throughput");

    for size in [500, 1000, 5000] {
        let dag = build_repo_dag(size);
        let rt = tokio_rt();

        // DAG sync: single traversal + framed stream write
        group.bench_with_input(BenchmarkId::new("dag_sync", size), &dag, |b, dag| {
            b.iter(|| {
                rt.block_on(async {
                    let links = link_extractor(dag);
                    let mut trav = FullTraversal::new(dag.root, (), links);

                    let mut frames = Vec::new();
                    while let Some(h) = trav.next().await.unwrap() {
                        let data = dag.data.get(&h).cloned();
                        frames.push((*h.as_bytes(), data));
                    }

                    let mut buf = Vec::with_capacity(frames.len() * 128);
                    send_sync(&mut buf, frames).await.unwrap();
                    buf.len()
                })
            });
        });

        // Per-object fetch: iterate all hashes and "fetch" each one individually
        group.bench_with_input(BenchmarkId::new("per_object", size), &dag, |b, dag| {
            b.iter(|| {
                rt.block_on(async {
                    let mut total_bytes = 0u64;
                    for hash in &dag.all_hashes {
                        // Simulate individual fetch: serialize one frame at a time
                        let data = dag.data.get(hash).cloned();
                        let frames = vec![(*hash.as_bytes(), data)];
                        let mut buf = Vec::new();
                        send_sync(&mut buf, frames).await.unwrap();
                        total_bytes = total_bytes.saturating_add(buf.len() as u64);
                    }
                    total_bytes
                })
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Stem/leaf split vs single-peer full sync
// ============================================================================

fn bench_stem_leaf_split(c: &mut Criterion) {
    let mut group = c.benchmark_group("stem_leaf_split");

    for size in [500, 1000, 5000] {
        let dag = build_repo_dag(size);
        let rt = tokio_rt();

        // Classify nodes: commits and trees are "stem", blobs are "leaf"
        let leaf_hashes: HashSet<blake3::Hash> = dag
            .data
            .iter()
            .filter(|(_, v)| {
                let s = String::from_utf8_lossy(v);
                s.starts_with("blob-")
            })
            .map(|(h, _)| *h)
            .collect();

        // Single-peer full sync
        group.bench_with_input(BenchmarkId::new("full_sync", size), &dag, |b, dag| {
            b.iter(|| {
                rt.block_on(async {
                    let links = link_extractor(dag);
                    let mut trav = FullTraversal::new(dag.root, (), links);

                    let mut frames = Vec::new();
                    while let Some(h) = trav.next().await.unwrap() {
                        let data = dag.data.get(&h).cloned();
                        frames.push((*h.as_bytes(), data));
                    }

                    let mut buf = Vec::with_capacity(frames.len() * 128);
                    send_sync(&mut buf, frames).await.unwrap();
                    buf.len()
                })
            });
        });

        // Stem phase: traverse full DAG but only inline non-leaf (stem) data
        let leaf_set = leaf_hashes.clone();
        group.bench_with_input(BenchmarkId::new("stem_phase", size), &dag, |b, dag| {
            b.iter(|| {
                rt.block_on(async {
                    let links = link_extractor(dag);
                    let mut trav = FullTraversal::new(dag.root, (), links);

                    let mut frames = Vec::new();
                    while let Some(h) = trav.next().await.unwrap() {
                        if leaf_set.contains(&h) {
                            // Hash-only for leaves
                            frames.push((*h.as_bytes(), None));
                        } else {
                            // Inline for stem nodes
                            let data = dag.data.get(&h).cloned();
                            frames.push((*h.as_bytes(), data));
                        }
                    }

                    let mut buf = Vec::with_capacity(frames.len() * 64);
                    send_sync(&mut buf, frames).await.unwrap();
                    buf.len()
                })
            });
        });

        // Leaf phase: parallel fetch of blob hashes (simulated with 3 peers)
        let peer_count = 3u32;
        let leaf_list: Vec<blake3::Hash> = leaf_hashes.iter().copied().collect();
        group.bench_with_input(BenchmarkId::new("leaf_phase_3peers", size), &dag, |b, dag| {
            b.iter(|| {
                rt.block_on(async {
                    // Partition leaves across peers by hash modulo
                    let mut peer_batches: Vec<Vec<blake3::Hash>> = vec![Vec::new(); peer_count as usize];
                    for (i, h) in leaf_list.iter().enumerate() {
                        peer_batches[i % peer_count as usize].push(*h);
                    }

                    // Each peer sends its batch as a SequenceTraversal
                    let mut total = 0usize;
                    for batch in &peer_batches {
                        let frames: Vec<([u8; 32], Option<Vec<u8>>)> = batch
                            .iter()
                            .map(|h| {
                                let data = dag.data.get(h).cloned();
                                (*h.as_bytes(), data)
                            })
                            .collect();

                        let mut buf = Vec::new();
                        send_sync(&mut buf, frames).await.unwrap();
                        total += buf.len();
                    }
                    total
                })
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_dag_sync_vs_per_object, bench_stem_leaf_split);
criterion_main!(benches);
