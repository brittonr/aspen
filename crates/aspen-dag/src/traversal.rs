//! Composable DAG traversal traits and implementations.
//!
//! A [`DagTraversal`] produces a deterministic sequence of hashes by walking
//! a content-addressed DAG. Traversals are lazy — they yield one node at a
//! time and give mutable access to the database between steps so the caller
//! can insert newly received data before the next step.
//!
//! # Implementations
//!
//! - [`FullTraversal`]: Depth-first pre-order walk from a root, with a visited set.
//! - [`SequenceTraversal`]: Fixed sequence of hashes (for individual/unrelated fetches).
//!
//! # Combinators
//!
//! - [`Filtered`]: Skip nodes matching a predicate.
//! - [`Bounded`]: Stop after a depth or count limit.

use std::collections::HashSet;
use std::future::Future;

use crate::constants::MAX_CHILDREN_PER_NODE;
use crate::constants::MAX_DAG_TRAVERSAL_DEPTH;
use crate::constants::MAX_VISITED_SET_SIZE;
use crate::error::TraversalError;
use crate::error::TraversalResult;
use crate::link::LinkExtractor;
use crate::verified::traversal::TraversalBound;
use crate::verified::traversal::compute_child_depth;
use crate::verified::traversal::is_within_count_bound;
use crate::verified::traversal::is_within_depth_bound;
use crate::verified::traversal::should_visit;

// ============================================================================
// Core Trait
// ============================================================================

/// A deterministic traversal over a content-addressed DAG.
///
/// Produces hashes one at a time. The caller gets mutable access to the
/// database between steps via [`db_mut`](DagTraversal::db_mut), which is
/// required for the receiver side of DAG sync — newly arrived data must
/// be inserted before the traversal can discover its children.
pub trait DagTraversal {
    /// Hash type for nodes in the DAG.
    type Hash: Copy + Eq + std::hash::Hash + std::fmt::Debug;

    /// Database/storage type the traversal reads from.
    type Db;

    /// Advance to the next node in the traversal.
    ///
    /// Returns `Ok(None)` when the traversal is complete.
    fn next(&mut self) -> impl Future<Output = TraversalResult<Option<Self::Hash>>> + Send;

    /// Mutable access to the underlying database.
    ///
    /// Called between `next()` steps to insert received data on the
    /// receiver side of a sync.
    fn db_mut(&mut self) -> &mut Self::Db;

    /// The root hashes of this traversal.
    fn roots(&self) -> Vec<Self::Hash>;

    /// Wrap this traversal in a filter that skips nodes matching a predicate.
    fn filtered<F: Fn(&Self::Hash) -> bool>(self, predicate: F) -> Filtered<Self, F>
    where Self: Sized {
        Filtered { inner: self, predicate }
    }

    /// Wrap this traversal with a count bound.
    ///
    /// Stops the traversal after `max_count` nodes have been yielded.
    /// Depth bounding is handled by `FullTraversal` via `MAX_DAG_TRAVERSAL_DEPTH`.
    fn bounded(self, max_count: u32) -> Bounded<Self>
    where Self: Sized {
        Bounded {
            inner: self,
            max_count,
            yielded: 0,
        }
    }
}

// ============================================================================
// FullTraversal
// ============================================================================

/// Depth-first pre-order traversal of a DAG from a root hash.
///
/// Maintains a stack for DFS and a visited set to avoid re-traversing
/// nodes in a DAG with shared sub-structures. After yielding a node,
/// the next call to `next()` extracts that node's children (via
/// [`LinkExtractor`]) and pushes them onto the stack.
///
/// This two-step yield-then-expand pattern is critical for sync:
/// the receiver doesn't have the node data until it's inserted between
/// calls to `next()`.
pub struct FullTraversal<H, D, L> {
    /// DFS stack. Each entry is (hash, depth).
    stack: Vec<(H, u32)>,
    /// Nodes already visited (by hash).
    visited: HashSet<H>,
    /// The node most recently yielded, pending child expansion.
    pending: Option<H>,
    /// Depth of the most recently yielded node.
    last_depth: u32,
    /// Known heads — traversal terminates when hitting these.
    known_heads: HashSet<H>,
    /// Database handle.
    db: D,
    /// Link extractor for discovering children.
    links: L,
}

impl<H, D, L> FullTraversal<H, D, L>
where
    H: Copy + Eq + std::hash::Hash + std::fmt::Debug,
    L: LinkExtractor<Hash = H>,
{
    /// Create a new full traversal from a single root.
    pub fn new(root: H, db: D, links: L) -> Self {
        let mut visited = HashSet::new();
        visited.reserve(256);
        Self {
            stack: vec![(root, 0)],
            visited,
            pending: None,
            last_depth: 0,
            known_heads: HashSet::new(),
            db,
            links,
        }
    }

    /// Create a full traversal with known heads for partial sync.
    ///
    /// The traversal will stop descending when it encounters a hash
    /// in `known_heads`, skipping the entire sub-DAG below it.
    pub fn with_known_heads(root: H, db: D, links: L, known_heads: HashSet<H>) -> Self {
        let mut t = Self::new(root, db, links);
        t.known_heads = known_heads;
        t
    }

    /// Create a full traversal seeded with an initial visited set.
    pub fn with_visited(root: H, db: D, links: L, visited: HashSet<H>) -> Self {
        let mut t = Self::new(root, db, links);
        t.visited = visited;
        t
    }
}

impl<H, D, L> DagTraversal for FullTraversal<H, D, L>
where
    H: Copy + Eq + std::hash::Hash + std::fmt::Debug + Send,
    D: Send,
    L: LinkExtractor<Hash = H> + Send,
{
    type Hash = H;
    type Db = D;

    async fn next(&mut self) -> TraversalResult<Option<H>> {
        // Expand children of the previously yielded node.
        if let Some(parent) = self.pending.take() {
            let children = self.links.extract_links(&parent, &self.db)?;
            let child_count = children.len() as u32;
            if child_count > MAX_CHILDREN_PER_NODE {
                return Err(TraversalError::TooManyChildren {
                    count: child_count,
                    max: MAX_CHILDREN_PER_NODE,
                });
            }

            // Push children in reverse order so left-to-right pops first.
            let child_depth = compute_child_depth(self.last_depth);
            self.stack.reserve(children.len());
            for child in children.into_iter().rev() {
                self.stack.push((child, child_depth));
            }
        }

        while let Some((hash, depth)) = self.stack.pop() {
            // Depth bound check (verified pure function).
            if !is_within_depth_bound(depth, TraversalBound::new(MAX_DAG_TRAVERSAL_DEPTH)) {
                return Err(TraversalError::DepthExceeded {
                    depth,
                    max: MAX_DAG_TRAVERSAL_DEPTH,
                });
            }

            // Visited set bound check.
            let max_visited_entries = match usize::try_from(MAX_VISITED_SET_SIZE) {
                Ok(max_visited_entries) => max_visited_entries,
                Err(_) => usize::MAX,
            };
            if self.visited.len() >= max_visited_entries {
                let visited_entries = match u32::try_from(self.visited.len()) {
                    Ok(visited_entries) => visited_entries,
                    Err(_) => u32::MAX,
                };
                return Err(TraversalError::VisitedSetExceeded {
                    size: visited_entries,
                    max: MAX_VISITED_SET_SIZE,
                });
            }

            // Skip if already visited or in known heads (verified pure function).
            if !should_visit(self.visited.contains(&hash), self.known_heads.contains(&hash)) {
                continue;
            }

            self.visited.insert(hash);
            self.pending = Some(hash);
            self.last_depth = depth;
            return Ok(Some(hash));
        }
        Ok(None)
    }

    fn db_mut(&mut self) -> &mut D {
        &mut self.db
    }

    fn roots(&self) -> Vec<H> {
        // The initial root is either still on the stack or already visited.
        // Return whatever is on the bottom of the stack plus pending.
        let root_count = self.stack.len().saturating_add(usize::from(self.pending.is_some()));
        let mut roots = Vec::with_capacity(root_count);
        if let Some(h) = self.pending {
            roots.push(h);
        }
        for (h, _) in &self.stack {
            roots.push(*h);
        }
        roots
    }
}

// ============================================================================
// SequenceTraversal
// ============================================================================

/// A traversal over a fixed sequence of hashes.
///
/// Used for fetching individual unrelated objects or a known list of hashes.
/// No link extraction — each hash in the sequence is yielded exactly once.
pub struct SequenceTraversal<H, D> {
    hashes: std::vec::IntoIter<H>,
    all: Vec<H>,
    db: D,
}

impl<H: Copy, D> SequenceTraversal<H, D> {
    /// Create a sequence traversal from a list of hashes.
    pub fn new(hashes: Vec<H>, db: D) -> Self {
        let all = hashes.clone();
        Self {
            hashes: hashes.into_iter(),
            all,
            db,
        }
    }
}

impl<H, D> DagTraversal for SequenceTraversal<H, D>
where
    H: Copy + Eq + std::hash::Hash + std::fmt::Debug + Send,
    D: Send,
{
    type Hash = H;
    type Db = D;

    async fn next(&mut self) -> TraversalResult<Option<H>> {
        Ok(self.hashes.next())
    }

    fn db_mut(&mut self) -> &mut D {
        &mut self.db
    }

    fn roots(&self) -> Vec<H> {
        self.all.clone()
    }
}

// ============================================================================
// Filtered Combinator
// ============================================================================

/// A traversal that skips nodes matching a predicate.
///
/// The predicate returns `true` for nodes that should be **excluded**.
pub struct Filtered<T, F> {
    inner: T,
    predicate: F,
}

impl<T, F> DagTraversal for Filtered<T, F>
where
    T: DagTraversal + Send,
    F: Fn(&T::Hash) -> bool + Send,
{
    type Hash = T::Hash;
    type Db = T::Db;

    async fn next(&mut self) -> TraversalResult<Option<T::Hash>> {
        let max_skips = match usize::try_from(MAX_VISITED_SET_SIZE) {
            Ok(max_skips) => max_skips,
            Err(_) => usize::MAX,
        };
        for _skip_index in 0..max_skips {
            match self.inner.next().await? {
                Some(hash) if (self.predicate)(&hash) => continue,
                other => return Ok(other),
            }
        }
        Err(TraversalError::VisitedSetExceeded {
            size: MAX_VISITED_SET_SIZE,
            max: MAX_VISITED_SET_SIZE,
        })
    }

    fn db_mut(&mut self) -> &mut T::Db {
        self.inner.db_mut()
    }

    fn roots(&self) -> Vec<T::Hash> {
        self.inner.roots()
    }
}

// ============================================================================
// Bounded Combinator
// ============================================================================

/// A traversal bounded by maximum depth and/or node count.
///
/// Wraps an inner traversal and stops early if either limit is reached.
/// Unlike hitting the hard `MAX_*` constants (which are errors), these
/// are soft limits — the traversal returns `Ok(None)` at the boundary.
pub struct Bounded<T> {
    inner: T,
    max_count: u32,
    yielded: u32,
}

impl<T: DagTraversal> DagTraversal for Bounded<T>
where
    T: DagTraversal + Send,
    T::Hash: Send,
{
    type Hash = T::Hash;
    type Db = T::Db;

    async fn next(&mut self) -> TraversalResult<Option<T::Hash>> {
        // Count bound (verified pure function).
        if !is_within_count_bound(self.yielded, TraversalBound::new(self.max_count)) {
            return Ok(None);
        }

        match self.inner.next().await? {
            Some(hash) => {
                self.yielded = self.yielded.saturating_add(1);
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    fn db_mut(&mut self) -> &mut T::Db {
        self.inner.db_mut()
    }

    fn roots(&self) -> Vec<T::Hash> {
        self.inner.roots()
    }
}
