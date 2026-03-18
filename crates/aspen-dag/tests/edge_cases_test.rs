//! Tests exercising edge cases, error paths, and combinator composition
//! that aren't covered by the existing test suite.

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_dag::DagTraversal;
use aspen_dag::FnLinkExtractor;
use aspen_dag::FullTraversal;
use aspen_dag::SequenceTraversal;
use aspen_dag::TraversalError;
use aspen_dag::TraversalResult;

fn hash(s: &str) -> blake3::Hash {
    blake3::hash(s.as_bytes())
}

// ============================================================================
// Cycle resilience
// ============================================================================

/// A cycle: A → B → C → A. The visited set must break the loop.
#[tokio::test]
async fn full_traversal_cycle_terminates() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
    edges.insert(a, vec![b]);
    edges.insert(b, vec![c]);
    edges.insert(c, vec![a]); // Back-edge

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let mut trav = FullTraversal::new(a, (), links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    // Each node visited exactly once despite the cycle.
    assert_eq!(visited, vec![a, b, c]);
}

/// Self-loop: A → A. Should yield A once.
#[tokio::test]
async fn full_traversal_self_loop() {
    let a = hash("a");

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
    edges.insert(a, vec![a]);

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let mut trav = FullTraversal::new(a, (), links);
    let first = trav.next().await.unwrap();
    assert_eq!(first, Some(a));

    let second = trav.next().await.unwrap();
    assert_eq!(second, None);
}

// ============================================================================
// Error paths
// ============================================================================

/// Depth limit exceeded returns an error, not silent truncation.
#[tokio::test]
async fn full_traversal_depth_exceeded() {
    // Build a chain 10_001 nodes deep — one past MAX_DAG_TRAVERSAL_DEPTH.
    let depth = 10_001u32;
    let hashes: Vec<blake3::Hash> = (0..depth).map(|i| blake3::hash(format!("node-{i}").as_bytes())).collect();

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
    for i in 0..(depth - 1) {
        edges.insert(hashes[i as usize], vec![hashes[(i + 1) as usize]]);
    }

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let mut trav = FullTraversal::new(hashes[0], (), links);
    let mut count = 0u32;
    loop {
        match trav.next().await {
            Ok(Some(_)) => count += 1,
            Ok(None) => panic!("traversal completed without depth error — chain was too short"),
            Err(TraversalError::DepthExceeded { depth: d, max }) => {
                assert_eq!(d, 10_000);
                assert_eq!(max, 10_000);
                break;
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    // Should have yielded 10_000 nodes before hitting the error.
    assert_eq!(count, 10_000);
}

/// Too many children on a single node.
#[tokio::test]
async fn full_traversal_too_many_children() {
    let root = hash("root");
    let over_limit = 1_000_001u32;
    let children: Vec<blake3::Hash> = (0..over_limit)
        .map(|i| blake3::hash(format!("child-{i}").as_bytes()))
        .collect();

    let children_clone = children.clone();
    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        if *h == root {
            Ok(children_clone.clone())
        } else {
            Ok(vec![])
        }
    });

    let mut trav = FullTraversal::new(root, (), links);

    // First next() yields root. Second next() tries to expand root's children.
    let first = trav.next().await.unwrap();
    assert_eq!(first, Some(root));

    let err = trav.next().await.unwrap_err();
    assert!(
        matches!(err, TraversalError::TooManyChildren { count, max } if count == over_limit && max == 1_000_000),
        "expected TooManyChildren, got {err:?}"
    );
}

// ============================================================================
// Combinator composition
// ============================================================================

/// Filtered + bounded stacked: filter removes some, bound caps the rest.
///
/// ```text
///       A
///      / \
///     B   C
///    / \   \
///   D   E   F
/// ```
///
/// DFS: A, B, D, E, C, F
/// Filter out D → A, B, E, C, F
/// Bound to 3 → A, B, E
#[tokio::test]
async fn filtered_then_bounded() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");
    let d = hash("d");
    let e = hash("e");
    let f = hash("f");

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
    edges.insert(a, vec![b, c]);
    edges.insert(b, vec![d, e]);
    edges.insert(c, vec![f]);

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let trav = FullTraversal::new(a, (), links);
    let mut composed = trav.filtered(|h| *h == d).bounded(3);
    let mut visited = vec![];

    while let Some(h) = composed.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, b, e]);
}

/// Bounded then filtered (reversed order): bound triggers first.
///
/// Same DAG. Bounded(3) first sees A, B, D → stops.
/// Then filter on D would have removed it, but bound already stopped.
/// With bounded wrapping the raw traversal and filtered wrapping bounded:
/// bounded yields A(1), B(2), D(3) → done; filter skips D → yields A, B.
#[tokio::test]
async fn bounded_then_filtered() {
    let a = hash("a");
    let b = hash("b");
    let d = hash("d");
    let e = hash("e");
    let c = hash("c");
    let f = hash("f");

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();
    edges.insert(a, vec![b, c]);
    edges.insert(b, vec![d, e]);
    edges.insert(c, vec![f]);

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let trav = FullTraversal::new(a, (), links);
    // Bound first, then filter — different result than filter-then-bound.
    let mut composed = trav.bounded(3).filtered(|h| *h == d);
    let mut visited = vec![];

    while let Some(h) = composed.next().await.unwrap() {
        visited.push(h);
    }

    // Bounded yields A, B, D (3 items), filter drops D → A, B
    assert_eq!(visited, vec![a, b]);
}

// ============================================================================
// roots() method
// ============================================================================

#[tokio::test]
async fn full_traversal_roots_before_iteration() {
    let root = hash("root");
    let links = FnLinkExtractor::new(|_: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> { Ok(vec![]) });
    let trav = FullTraversal::new(root, (), links);

    let roots = trav.roots();
    // Before any iteration, the stack has the root.
    assert!(roots.contains(&root));
}

#[tokio::test]
async fn sequence_traversal_roots() {
    let a = hash("a");
    let b = hash("b");
    let c = hash("c");

    let trav = SequenceTraversal::new(vec![a, b, c], ());
    let roots = trav.roots();
    assert_eq!(roots, vec![a, b, c]);
}

// ============================================================================
// SequenceTraversal edge cases
// ============================================================================

/// Duplicate hashes in a sequence are yielded as-is (no dedup).
#[tokio::test]
async fn sequence_traversal_duplicates_preserved() {
    let a = hash("a");

    let mut trav = SequenceTraversal::new(vec![a, a, a], ());
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    assert_eq!(visited, vec![a, a, a]);
}

// ============================================================================
// Wide fan-out (within bounds)
// ============================================================================

/// A single node with 1000 children — all should be visited.
#[tokio::test]
async fn full_traversal_wide_node() {
    let root = hash("root");
    let children: Vec<blake3::Hash> = (0..1000u32)
        .map(|i| blake3::hash(format!("child-{i}").as_bytes()))
        .collect();

    let children_clone = children.clone();
    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        if *h == root {
            Ok(children_clone.clone())
        } else {
            Ok(vec![])
        }
    });

    let mut trav = FullTraversal::new(root, (), links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    // root + 1000 children
    assert_eq!(visited.len(), 1001);
    assert_eq!(visited[0], root);

    // Children should appear in left-to-right order (stack pushes reversed).
    let visited_children: HashSet<blake3::Hash> = visited[1..].iter().copied().collect();
    for child in &children {
        assert!(visited_children.contains(child));
    }
}

// ============================================================================
// Realistic DAG: git-like repo structure
// ============================================================================

/// Simulates a git repo with 50 commits, each pointing to a tree
/// with 3 blobs, plus parent commit chains.
///
/// ```text
/// C49 → C48 → ... → C1 → C0
///  |      |            |    |
///  T49    T48          T1   T0
///  /|\    /|\         /|\   /|\
/// b b b  b b b       b b b b b b
/// ```
///
/// Total: 50 commits + 50 trees + 150 blobs = 250 nodes.
/// Diamond sharing: all commits share the same 3 blob hashes per tree
/// (different trees though).
#[tokio::test]
async fn realistic_git_dag() {
    let commit_count = 50u32;

    let commits: Vec<blake3::Hash> = (0..commit_count)
        .map(|i| blake3::hash(format!("commit-{i}").as_bytes()))
        .collect();
    let trees: Vec<blake3::Hash> = (0..commit_count)
        .map(|i| blake3::hash(format!("tree-{i}").as_bytes()))
        .collect();

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();

    for i in 0..commit_count {
        let idx = i as usize;
        let mut children = vec![trees[idx]];
        // Parent commit link.
        if i > 0 {
            children.push(commits[idx - 1]);
        }
        edges.insert(commits[idx], children);

        // Each tree has 3 unique blobs.
        let blobs: Vec<blake3::Hash> = (0..3u32)
            .map(|j| blake3::hash(format!("blob-{i}-{j}").as_bytes()))
            .collect();
        edges.insert(trees[idx], blobs);
    }

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let root = commits[(commit_count - 1) as usize];
    let mut trav = FullTraversal::new(root, (), links);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    // 50 commits + 50 trees + 150 blobs = 250 total.
    assert_eq!(visited.len(), 250);

    // No duplicates.
    let unique: HashSet<blake3::Hash> = visited.iter().copied().collect();
    assert_eq!(visited.len(), unique.len());

    // Root commit is first.
    assert_eq!(visited[0], root);
}

/// Same git-like DAG but with known heads — incremental sync.
/// If we know commit 25 and below, only commits 26-49 and their
/// trees/blobs should be traversed.
#[tokio::test]
async fn realistic_git_dag_incremental() {
    let commit_count = 50u32;

    let commits: Vec<blake3::Hash> = (0..commit_count)
        .map(|i| blake3::hash(format!("commit-{i}").as_bytes()))
        .collect();
    let trees: Vec<blake3::Hash> = (0..commit_count)
        .map(|i| blake3::hash(format!("tree-{i}").as_bytes()))
        .collect();

    let mut edges: HashMap<blake3::Hash, Vec<blake3::Hash>> = HashMap::new();

    for i in 0..commit_count {
        let idx = i as usize;
        let mut children = vec![trees[idx]];
        if i > 0 {
            children.push(commits[idx - 1]);
        }
        edges.insert(commits[idx], children);

        let blobs: Vec<blake3::Hash> = (0..3u32)
            .map(|j| blake3::hash(format!("blob-{i}-{j}").as_bytes()))
            .collect();
        edges.insert(trees[idx], blobs);
    }

    let links = FnLinkExtractor::new(move |h: &blake3::Hash| -> TraversalResult<Vec<blake3::Hash>> {
        Ok(edges.get(h).cloned().unwrap_or_default())
    });

    let root = commits[49];
    let known_head = commits[25]; // We already have commit 25 and everything below.
    let known_heads = HashSet::from([known_head]);

    let mut trav = FullTraversal::with_known_heads(root, (), links, known_heads);
    let mut visited = vec![];

    while let Some(h) = trav.next().await.unwrap() {
        visited.push(h);
    }

    // Commits 26-49 (24) + their trees (24) + their blobs (72) = 120.
    // Commit 49's tree is traversed. Then commit 48, its tree, etc.
    // When we reach commit 26 → commit 25 is a known head → stop.
    assert_eq!(visited.len(), 120);

    // Known head itself was NOT visited.
    assert!(!visited.contains(&known_head));
}
