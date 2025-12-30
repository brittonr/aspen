//! FoundationDB-style layer abstractions for Aspen.
//!
//! This module provides ordered key encoding and namespace isolation patterns
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
//!    SharedRedbStorage (raw KV)
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
//! ```ignore
//! use aspen::layer::{Tuple, Subspace, Element};
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

// Allow dead code in this module - it's planned for future integration
// with FoundationDB-style layer patterns but not yet used in production.
#[allow(dead_code)]
mod subspace;
#[allow(dead_code)]
mod tuple;

// These types are exported for future integration of FoundationDB-style layer patterns.
// Currently unused but maintained for planned subspace/tuple layer functionality.
#[allow(unused_imports)]
pub use subspace::Subspace;
#[allow(unused_imports)]
pub use tuple::Element;
#[allow(unused_imports)]
pub use tuple::Tuple;
#[allow(unused_imports)]
pub use tuple::TupleError;

#[cfg(test)]
mod proptest;
