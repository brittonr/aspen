//! Property-based tests for DAG traversal determinism.

use std::collections::HashMap;

use aspen_dag::DagTraversal;
use aspen_dag::FnLinkExtractor;
use aspen_dag::FullTraversal;
use aspen_dag::TraversalResult;
use proptest::prelude::*;

/// Generate a random DAG with N nodes and random edges.
/// Edges only go from lower-numbered nodes to higher-numbered nodes
/// to guarantee acyclicity.
fn arb_dag(max_nodes: usize) -> impl Strategy<Value = (Vec<blake3::Hash>, HashMap<blake3::Hash, Vec<blake3::Hash>>)> {
    (2..=max_nodes).prop_flat_map(|n| {
        // Generate n distinct "labels" and then edge sets.
        // For each node i (0..n-1), pick a random subset of nodes j where j > i.
        let edge_strat = (0..n)
            .map(move |i| {
                proptest::collection::vec(
                    (i + 1)..n.max(i + 2),  // at least one possible target
                    0..=(n - i - 1).min(5), // up to 5 children per node
                )
            })
            .collect::<Vec<_>>();

        edge_strat.prop_map(move |edges_per_node| {
            // Create deterministic hashes from node index.
            let hashes: Vec<blake3::Hash> = (0..n).map(|i| blake3::hash(format!("node-{i}").as_bytes())).collect();

            let mut edge_map: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
            for (i, targets) in edges_per_node.iter().enumerate() {
                let children: Vec<blake3::Hash> = targets.iter().filter(|&&j| j < n).map(|&j| hashes[j]).collect();
                edge_map.insert(hashes[i], children);
            }

            (hashes, edge_map)
        })
    })
}

proptest! {
    /// Same DAG + same root = same traversal sequence, always.
    #[test]
    fn traversal_determinism((hashes, edges) in arb_dag(20)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let root = hashes[0];

            // Run traversal twice.
            let mut run1 = vec![];
            let mut run2 = vec![];

            for output in [&mut run1, &mut run2] {
                let edges_clone = edges.clone();
                let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
                    Ok(edges_clone.get(h).cloned().unwrap_or_default())
                });
                let mut trav = FullTraversal::new(root, (), links);

                while let Some(h) = trav.next().await.unwrap() {
                    output.push(h);
                }
            }

            prop_assert_eq!(run1, run2);
            Ok(())
        })?;
    }

    /// Every node in the traversal output appears exactly once (no duplicates).
    #[test]
    fn traversal_no_duplicates((hashes, edges) in arb_dag(20)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let root = hashes[0];
            let edges_clone = edges.clone();
            let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
                Ok(edges_clone.get(h).cloned().unwrap_or_default())
            });
            let mut trav = FullTraversal::new(root, (), links);

            let mut seen = std::collections::HashSet::new();
            while let Some(h) = trav.next().await.unwrap() {
                prop_assert!(seen.insert(h), "duplicate hash in traversal: {h}");
            }
            Ok(())
        })?;
    }

    /// The root is always the first node yielded.
    #[test]
    fn traversal_root_first((hashes, edges) in arb_dag(20)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let root = hashes[0];
            let edges_clone = edges.clone();
            let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
                Ok(edges_clone.get(h).cloned().unwrap_or_default())
            });
            let mut trav = FullTraversal::new(root, (), links);

            let first = trav.next().await.unwrap();
            prop_assert_eq!(first, Some(root));
            Ok(())
        })?;
    }
}
