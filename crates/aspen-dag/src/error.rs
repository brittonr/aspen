//! Error types for DAG traversal and sync operations.

use snafu::Snafu;

/// Errors from DAG traversal operations.
#[derive(Debug, Snafu)]
pub enum TraversalError {
    /// Traversal depth exceeded the configured maximum.
    #[snafu(display("traversal depth {depth} exceeds max {max}"))]
    DepthExceeded { depth: u32, max: u32 },

    /// Visited set exceeded the configured maximum.
    #[snafu(display("visited set size {size} exceeds max {max}"))]
    VisitedSetExceeded { size: u32, max: u32 },

    /// Node count exceeded the configured maximum.
    #[snafu(display("traversal yielded {count} nodes, exceeding max {max}"))]
    NodeCountExceeded { count: u32, max: u32 },

    /// Link extraction failed for a node.
    #[snafu(display("failed to extract links from node: {message}"))]
    LinkExtraction { message: String },

    /// Node data not found in storage.
    #[snafu(display("node not found: {hash}"))]
    NodeNotFound { hash: String },

    /// Too many children returned by link extraction.
    #[snafu(display("node has {count} children, exceeding max {max}"))]
    TooManyChildren { count: u32, max: u32 },
}

/// Result type alias for traversal operations.
pub type TraversalResult<T> = std::result::Result<T, TraversalError>;
