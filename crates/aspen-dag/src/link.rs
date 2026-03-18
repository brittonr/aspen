//! Link extraction trait for discovering DAG children.
//!
//! Each DAG type (Git objects, COB changes, snix directories) has its own
//! format for encoding links to child nodes. The [`LinkExtractor`] trait
//! abstracts over these formats so the traversal engine doesn't need to
//! know about specific data layouts.
//!
//! # Implementations
//!
//! - [`NoLinks`]: For flat hash sets with no child relationships.
//!
//! Forge provides `GitLinkExtractor` and `CobLinkExtractor`.
//! Snix provides `DirectoryLinkExtractor`.

use crate::error::TraversalResult;

/// Extracts child hashes from a node in a content-addressed DAG.
///
/// Given a node's hash and a database to read from, returns the hashes
/// of all direct children. The traversal calls this after yielding a
/// node to discover what to visit next.
///
/// # Type Parameters
///
/// - `Hash`: The hash type used to identify nodes (e.g., `blake3::Hash`).
pub trait LinkExtractor: Send {
    /// Hash type for nodes in the DAG.
    type Hash: Copy + Eq + std::hash::Hash + std::fmt::Debug;

    /// Extract child hashes from a node.
    ///
    /// Reads the node's data from `db` and parses it to find child references.
    ///
    /// # Returns
    ///
    /// - `Ok(vec![])` for leaf nodes (no children).
    /// - `Ok(vec![child1, child2, ...])` for interior nodes.
    /// - `Err(...)` if the node can't be read or parsed.
    fn extract_links<D>(&self, hash: &Self::Hash, db: &D) -> TraversalResult<Vec<Self::Hash>>;
}

/// A no-op link extractor for flat hash sequences.
///
/// Always returns an empty child list. Useful for `SequenceTraversal`
/// or any traversal where children aren't relevant.
pub struct NoLinks;

impl LinkExtractor for NoLinks {
    type Hash = blake3::Hash;

    fn extract_links<D>(&self, _hash: &blake3::Hash, _db: &D) -> TraversalResult<Vec<blake3::Hash>> {
        Ok(vec![])
    }
}

/// A link extractor backed by a closure.
///
/// Convenience wrapper for cases where a full trait impl isn't needed.
pub struct FnLinkExtractor<H, F> {
    f: F,
    _marker: std::marker::PhantomData<H>,
}

impl<H, F> FnLinkExtractor<H, F>
where
    H: Copy + Eq + std::hash::Hash + std::fmt::Debug,
    F: Fn(&H) -> TraversalResult<Vec<H>> + Send,
{
    /// Create a link extractor from a closure.
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<H, F> LinkExtractor for FnLinkExtractor<H, F>
where
    H: Copy + Eq + std::hash::Hash + std::fmt::Debug + Send,
    F: Fn(&H) -> TraversalResult<Vec<H>> + Send,
{
    type Hash = H;

    fn extract_links<D>(&self, hash: &H, _db: &D) -> TraversalResult<Vec<H>> {
        (self.f)(hash)
    }
}
