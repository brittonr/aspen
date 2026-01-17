//! FoundationDB-style layer abstractions for ordered key-value storage.
//!
//! This crate provides order-preserving key encoding and namespace isolation patterns
//! inspired by FoundationDB's layer architecture. These primitives enable:
//!
//! - **Tuple encoding**: Order-preserving serialization of composite keys
//! - **Subspace isolation**: Namespace partitioning for multi-tenant workloads
//! - **Range queries**: Efficient prefix-based scans using lexicographic ordering
//!
//! # Architecture
//!
//! ```text
//! Application Layer (SQL, Indexes, etc.)
//!          ↓
//! ┌─────────────────────────────────────┐
//! │         Subspace Layer              │  Namespace isolation
//! │  ┌─────────────────────────────┐   │
//! │  │       Tuple Layer           │   │  Ordered key encoding
//! │  └─────────────────────────────┘   │
//! └─────────────────────────────────────┘
//!          ↓
//!    Key-Value Storage (raw bytes)
//! ```
//!
//! # FoundationDB Compatibility
//!
//! The tuple encoding follows the [FoundationDB Tuple Layer specification](
//! https://github.com/apple/foundationdb/blob/main/design/tuple.md),
//! ensuring binary compatibility with existing FDB tooling and enabling
//! future interoperability.
//!
//! # Example
//!
//! ```
//! use aspen_layer::{Tuple, Subspace, Element};
//!
//! // Create a namespace for user data
//! let users = Subspace::new(Tuple::new().push("users"));
//!
//! // Create a key: ("users", "alice", "profile")
//! let key = users.pack(&Tuple::new()
//!     .push("alice")
//!     .push("profile"));
//!
//! // Get range for all of alice's data
//! let (start, end) = users.subspace(&Tuple::new().push("alice")).range();
//! ```
//!
//! # References
//!
//! - [FoundationDB Tuple Layer](https://github.com/apple/foundationdb/blob/main/design/tuple.md)
//! - [FoundationDB Data Modeling](https://apple.github.io/foundationdb/data-modeling.html)
//! - [Subspace Pattern](https://forums.foundationdb.org/t/application-design-using-subspace-and-tuple/452)

#![warn(missing_docs)]

pub mod index;
mod subspace;
mod tuple;

#[cfg(test)]
mod proptest;

// Re-export all public types at crate root
// Re-export index types
pub use index::INDEX_METADATA_PREFIX;
pub use index::IndexDefinition;
pub use index::IndexError;
pub use index::IndexFieldType;
pub use index::IndexOptions;
pub use index::IndexQueryExecutor;
pub use index::IndexRegistry;
pub use index::IndexResult;
pub use index::IndexScanResult;
pub use index::IndexUpdate;
pub use index::IndexableEntry;
pub use index::KeyExtractor;
pub use index::MAX_INDEX_SCAN_RESULTS;
pub use index::MAX_INDEXES;
pub use index::SecondaryIndex;
pub use index::extract_primary_key_from_tuple;
pub use subspace::Subspace;
pub use subspace::SubspaceError;
pub use tuple::Element;
pub use tuple::Tuple;
pub use tuple::TupleError;
