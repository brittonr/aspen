//! Stem/leaf traversal filters for Forge DAG sync.
//!
//! These filters enable two-phase sync:
//! 1. **Stem sync**: Transfer structural nodes (commits, trees, tags) first — small, fast.
//! 2. **Leaf sync**: Transfer data nodes (blobs) second — large, parallelizable across peers.
//!
//! # Usage with `DagTraversal::filtered`
//!
//! ```ignore
//! // Phase 1: sync only structural nodes
//! let stem = traversal.filtered(|h| is_blob_node(h, &store));
//!
//! // Phase 2: sync only leaf data nodes
//! let leaves = traversal.filtered(|h| is_stem_node(h, &store));
//! ```

use serde::Deserialize;
use serde::Serialize;

/// Type classification for Forge DAG nodes.
///
/// Used by traversal filters to distinguish structural (stem) from data (leaf) nodes.
/// Type tags correspond to `TraversalFilter::Exclude` / `TraversalFilter::Only` values
/// in the wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum ForgeNodeType {
    /// A commit object (structural).
    Commit = 0,
    /// A tree object (structural).
    Tree = 1,
    /// A blob object (leaf data).
    Blob = 2,
    /// A tag object (structural).
    Tag = 3,
    /// A COB change (structural).
    CobChange = 4,
}

impl ForgeNodeType {
    /// Type tag as u32, matching the wire protocol's `TraversalFilter` values.
    pub fn as_tag(self) -> u32 {
        self as u32
    }

    /// Whether this node type is structural (stem).
    pub fn is_stem(self) -> bool {
        !matches!(self, ForgeNodeType::Blob)
    }

    /// Whether this node type is a leaf (data).
    pub fn is_leaf(self) -> bool {
        matches!(self, ForgeNodeType::Blob)
    }
}

/// Filter predicate: returns true for blob nodes (use with `filtered` to exclude blobs).
///
/// When used as a filter predicate, this excludes blobs from the traversal,
/// producing a "stem only" sync.
///
/// # Arguments
///
/// * `node_type` - The type of the node being considered
pub fn is_blob_node(node_type: ForgeNodeType) -> bool {
    node_type.is_leaf()
}

/// Filter predicate: returns true for stem nodes (use with `filtered` to exclude stems).
///
/// When used as a filter predicate, this excludes structural nodes from the
/// traversal, producing a "leaf only" sync.
///
/// # Arguments
///
/// * `node_type` - The type of the node being considered
pub fn is_stem_node(node_type: ForgeNodeType) -> bool {
    node_type.is_stem()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_type_tags() {
        assert_eq!(ForgeNodeType::Commit.as_tag(), 0);
        assert_eq!(ForgeNodeType::Tree.as_tag(), 1);
        assert_eq!(ForgeNodeType::Blob.as_tag(), 2);
        assert_eq!(ForgeNodeType::Tag.as_tag(), 3);
        assert_eq!(ForgeNodeType::CobChange.as_tag(), 4);
    }

    #[test]
    fn stem_leaf_classification() {
        assert!(ForgeNodeType::Commit.is_stem());
        assert!(ForgeNodeType::Tree.is_stem());
        assert!(ForgeNodeType::Tag.is_stem());
        assert!(ForgeNodeType::CobChange.is_stem());

        assert!(!ForgeNodeType::Blob.is_stem());
        assert!(ForgeNodeType::Blob.is_leaf());
    }

    #[test]
    fn filter_predicates() {
        assert!(is_blob_node(ForgeNodeType::Blob));
        assert!(!is_blob_node(ForgeNodeType::Commit));
        assert!(!is_blob_node(ForgeNodeType::Tree));

        assert!(is_stem_node(ForgeNodeType::Commit));
        assert!(is_stem_node(ForgeNodeType::Tree));
        assert!(!is_stem_node(ForgeNodeType::Blob));
    }
}
