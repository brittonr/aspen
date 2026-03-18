//! Integration tests for DAG traversal.

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_dag::DagTraversal;
use aspen_dag::FnLinkExtractor;
use aspen_dag::FullTraversal;
use aspen_dag::LinkExtractor;
use aspen_dag::NoLinks;
use aspen_dag::SequenceTraversal;
use aspen_dag::TraversalResult;

// ============================================================================
// Test Helpers
// ============================================================================

/// A simple in-memory DAG for testing.
/// Maps hash → list of child hashes.
#[derive(Clone, Default)]
struct TestDag {
    edges: HashMap<blake3::Hash, Vec<blake3::Hash>>,
}

impl TestDag {
    fn new() -> Self {
        Self::default()
    }

    fn add_edge(&mut self, parent: blake3::Hash, child: blake3::Hash) {
        self.edges.entry(parent).or_default().push(child);
    }

    fn add_node(&mut self, hash: blake3::Hash) {
        self.edges.entry(hash).or_default();
    }
}

fn hash(s: &str) -> blake3::Hash {
    blake3::hash(s.as_bytes())
}

fn make_link_extractor(
    dag: TestDag,
) -> FnLinkExtractor<blake3::Hash, impl Fn(&blake3::Hash) -> TraversalResult<Vec<blake3::Hash>>> {
    FnLinkExtractor::new(move |h: &blake3::Hash| Ok(dag.edges.get(h).cloned().unwrap_or_default()))
}

// ============================================================================
// FullTraversal Tests
// ============================================================================

/// Single node, no children.
///
/// ```text
/// A
/// ```
#[tokio::test]
async fn full_traversal_single_node() {
    let a = hash("a");
    let dag = TestDag::new(); // No edges = leaf
    let links = make_link_extractor(dag.clone());

    let mut trav = FullTraversal::new(a, dag, links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a]);
}

/// Linear chain: A → B → C → D.
///
/// Depth-first pre-order should yield A, B, C, D.
#[tokio::test]
async fn full_traversal_linear_chain() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(b, c);
    dag.add_edge(c, d);
    dag.add_node(d);

    let links = make_link_extractor(dag.clone());
    let mut trav = FullTraversal::new(a, dag, links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, b, c, d]);
}

/// Binary tree:
///
/// ```text
///       A
///      / \
///     B   C
///    / \
///   D   E
/// ```
///
/// DFS pre-order left-to-right: A, B, D, E, C
#[tokio::test]
async fn full_traversal_binary_tree() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");
    let e = hash("e");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_edge(b, d);
    dag.add_edge(b, e);
    dag.add_node(c);
    dag.add_node(d);
    dag.add_node(e);

    let links = make_link_extractor(dag.clone());
    let mut trav = FullTraversal::new(a, dag, links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, b, d, e, c]);
}

/// Diamond DAG (shared child):
///
/// ```text
///     A
///    / \
///   B   C
///    \ /
///     D
/// ```
///
/// D should be visited exactly once.
#[tokio::test]
async fn full_traversal_diamond_dedup() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_edge(b, d);
    dag.add_edge(c, d);
    dag.add_node(d);

    let links = make_link_extractor(dag.clone());
    let mut trav = FullTraversal::new(a, dag, links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    // DFS pre-order: A, B, D, C. D already visited when C tries to expand it.
    assert_eq!(visited, vec![a, b, d, c]);
}

/// Known heads terminate traversal at boundaries.
///
/// ```text
///     A → B → C → D
/// ```
///
/// If D is a known head, traversal should yield A, B, C (skip D and below).
#[tokio::test]
async fn full_traversal_known_heads() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(b, c);
    dag.add_edge(c, d);
    dag.add_node(d);

    let links = make_link_extractor(dag.clone());
    let known_heads = HashSet::from([d]);
    let mut trav = FullTraversal::with_known_heads(a, dag, links, known_heads);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, b, c]);
}

/// Known heads in the middle cut off an entire subtree.
///
/// ```text
///       A
///      / \
///     B   C
///    / \
///   D   E
/// ```
///
/// If B is a known head, only A and C should be visited.
#[tokio::test]
async fn full_traversal_known_heads_subtree() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");
    let e = hash("e");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_edge(b, d);
    dag.add_edge(b, e);
    dag.add_node(c);

    let links = make_link_extractor(dag.clone());
    let known_heads = HashSet::from([b]);
    let mut trav = FullTraversal::with_known_heads(a, dag, links, known_heads);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, c]);
}

/// Initial visited set skips already-synced nodes.
#[tokio::test]
async fn full_traversal_with_visited() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_node(b);
    dag.add_node(c);

    let links = make_link_extractor(dag.clone());
    let visited = HashSet::from([b]);
    let mut trav = FullTraversal::with_visited(a, dag, links, visited);
    let mut result = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        result.push(h);
    }

    // B already visited, so only A and C.
    assert_eq!(result, vec![a, c]);
}

// ============================================================================
// SequenceTraversal Tests
// ============================================================================

#[tokio::test]
async fn sequence_traversal_yields_all() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");

    let mut trav = SequenceTraversal::new(vec![a, b, c], ());
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, b, c]);
}

#[tokio::test]
async fn sequence_traversal_empty() {
    let mut trav: SequenceTraversal<blake3::Hash, ()> = SequenceTraversal::new(vec![], ());
    assert!(trav.next().await.unwrap().is_none());
}

// ============================================================================
// Filtered Combinator Tests
// ============================================================================

/// Filter out specific nodes from a traversal.
#[tokio::test]
async fn filtered_skips_matching() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_node(b);
    dag.add_node(c);

    let links = make_link_extractor(dag.clone());
    let trav = FullTraversal::new(a, dag, links);

    // Filter out B.
    let mut filtered = trav.filtered(|h| *h == b);
    let mut visited = vec![];

    while let Some(h) = filtered.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, c]);
}

// ============================================================================
// Bounded Combinator Tests
// ============================================================================

/// Bounded stops after max_count nodes.
#[tokio::test]
async fn bounded_stops_at_count() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(b, c);
    dag.add_edge(c, d);
    dag.add_node(d);

    let links = make_link_extractor(dag.clone());
    let trav = FullTraversal::new(a, dag, links);
    let mut bounded = trav.bounded(2);
    let mut visited = vec![];

    while let Some(h) = bounded.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, b]);
}

/// Bounded with max_count=0 yields nothing.
#[tokio::test]
async fn bounded_zero_yields_nothing() {
    let a = hash("a");
    let dag = TestDag::new();
    let links = make_link_extractor(dag.clone());
    let trav = FullTraversal::new(a, dag, links);
    let mut bounded = trav.bounded(0);

    assert!(bounded.next().await.unwrap().is_none());
}

// ============================================================================
// db_mut access
// ============================================================================

/// Verify db_mut provides mutable access between traversal steps.
#[tokio::test]
async fn db_mut_is_accessible() {
    let a = hash("a");
    let mut dag = TestDag::new();
    dag.add_node(a);

    let links = make_link_extractor(dag.clone());
    let mut trav = FullTraversal::new(a, dag, links);

    // Yield the root.
    let _ = trav.next().await.unwrap();

    // Access db_mut — should compile and not panic.
    let db = trav.db_mut();
    db.add_node(hash("new"));
}

// ============================================================================
// Determinism Test
// ============================================================================

/// Same DAG + same root = same traversal order, every time.
#[tokio::test]
async fn traversal_is_deterministic() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");
    let e = hash("e");

    let mut dag = TestDag::new();
    dag.add_edge(a, b);
    dag.add_edge(a, c);
    dag.add_edge(b, d);
    dag.add_edge(b, e);
    dag.add_node(c);
    dag.add_node(d);
    dag.add_node(e);

    // Run the traversal 10 times — must produce the same sequence each time.
    let mut reference: Option<Vec<blake3::Hash>> = None;

    for _ in 0..10 {
        let links = make_link_extractor(dag.clone());
        let mut trav = FullTraversal::new(a, dag.clone(), links);
        let mut visited = vec![];

        while let Some(h) = trav.next().await.unwrap() {
            visited.push(h);
        }

        match &reference {
            None => reference = Some(visited),
            Some(expected) => assert_eq!(&visited, expected, "traversal order diverged"),
        }
    }
}

/// NoLinks extractor always returns empty children.
#[tokio::test]
async fn no_links_extractor() {
    let a = hash("a");
    let links = NoLinks;
    let result = links.extract_links::<()>(&a, &()).unwrap();
    assert!(result.is_empty());
}
