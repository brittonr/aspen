//! DAG sync integration for Forge.
//!
//! Provides [`GitLinkExtractor`] and [`CobLinkExtractor`] for walking
//! Git object graphs and COB change DAGs using the `aspen-dag` traversal
//! framework. Also defines stem/leaf filter constants for two-phase sync.
//!
//! # Architecture
//!
//! ```text
//! DagTraversal + GitLinkExtractor
//!     → walks commit → tree → blob graph
//!     → single-stream sync replaces per-object fetch
//!
//! DagTraversal + CobLinkExtractor
//!     → walks change → parent change graph
//!     → single-stream sync for COB replication
//! ```

mod extractors;
mod filters;

pub use extractors::CobLinkExtractor;
pub use extractors::GitLinkExtractor;
pub use filters::ForgeNodeType;
pub use filters::is_blob_node;
pub use filters::is_stem_node;
