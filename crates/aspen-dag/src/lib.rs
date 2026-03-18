//! Composable DAG traversal and sync protocol for content-addressed graphs.
//!
//! This crate provides a generic framework for traversing and synchronizing
//! Merkle DAGs over iroh QUIC. It is used by:
//!
//! - **Forge**: Git object and COB change sync
//! - **Snix**: Nix store closure sync (PathInfo → Directory → Blob)
//! - **Blob replication**: DAG-aware repair scans
//!
//! # Architecture
//!
//! ```text
//! DagTraversal trait
//!     |
//!     +-> FullTraversal (depth-first from root, with visited set)
//!     +-> SequenceTraversal (fixed hash list)
//!     |
//!     +-> Combinators
//!         +-> Filtered (skip by predicate)
//!         +-> Bounded (depth/count limits)
//!
//! LinkExtractor trait
//!     |
//!     +-> NoLinks (flat sequences)
//!     +-> FnLinkExtractor (closure-based)
//!     +-> GitLinkExtractor (in aspen-forge)
//!     +-> DirectoryLinkExtractor (in aspen-snix)
//! ```
//!
//! # Quick Start
//!
//! ```ignore
//! use aspen_dag::{DagTraversal, FullTraversal, FnLinkExtractor};
//!
//! // Define how to find children
//! let links = FnLinkExtractor::new(|hash: &blake3::Hash| {
//!     Ok(my_db.get_children(hash))
//! });
//!
//! // Traverse from root
//! let mut traversal = FullTraversal::new(root_hash, db, links);
//! while let Some(hash) = traversal.next().await? {
//!     println!("visiting: {hash}");
//! }
//! ```
//!
//! # Tiger Style
//!
//! All operations are bounded by constants in [`constants`]:
//! - `MAX_DAG_TRAVERSAL_DEPTH`: 10,000
//! - `MAX_VISITED_SET_SIZE`: 1,000,000
//! - `MAX_TRAVERSAL_NODES`: 5,000,000
//! - `MAX_DAG_SYNC_REQUEST_SIZE`: 16 MiB
//! - `MAX_DAG_SYNC_TRANSFER_SIZE`: 10 GiB

pub mod constants;
pub mod error;
pub mod handler;
pub mod link;
pub mod protocol;
pub mod sync;
pub mod traversal;
pub mod verified;

// Re-export core types
pub use error::TraversalError;
pub use error::TraversalResult;
pub use handler::DagSyncConnection;
pub use handler::DagSyncProtocolHandler;
pub use handler::connect_dag_sync;
pub use link::FnLinkExtractor;
pub use link::LinkExtractor;
pub use link::NoLinks;
pub use protocol::DAG_SYNC_ALPN;
pub use protocol::DagSyncRequest;
pub use protocol::FullTraversalOpts;
pub use protocol::InlinePolicy;
pub use protocol::ProtocolError;
pub use protocol::ResponseFrameHeader;
pub use protocol::TraversalFilter;
pub use protocol::TraversalOpts;
pub use protocol::TraversalOrder;
pub use sync::InlineContext;
pub use sync::ReceivedFrame;
pub use sync::SyncStats;
pub use sync::read_frame;
pub use sync::recv_sync;
pub use sync::send_sync;
pub use sync::write_data_inline;
pub use sync::write_hash_only;
pub use traversal::Bounded;
pub use traversal::DagTraversal;
pub use traversal::Filtered;
pub use traversal::FullTraversal;
pub use traversal::SequenceTraversal;
