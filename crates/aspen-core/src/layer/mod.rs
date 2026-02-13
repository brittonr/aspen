//! FoundationDB-style layer abstractions for ordered key-value storage.
//!
//! This module re-exports types from `aspen-layer` and provides additional
//! storage-dependent abstractions (directory layer, high-contention allocator).
//!
//! # Architecture
//!
//! ```text
//! Application Layer (SQL, Indexes, etc.)
//!          |
//! +-------------------------------------+
//! |    Directory Layer (aspen-core)    |  Hierarchical namespaces
//! |  +-----------------------------+   |
//! |  |  Subspace Layer (aspen-layer)|  |  Namespace isolation
//! |  |  +----------------------+   |   |
//! |  |  | Tuple Layer          |   |   |  Ordered key encoding
//! |  |  +----------------------+   |   |
//! |  +-----------------------------+   |
//! +-------------------------------------+
//!          |
//!    Key-Value Storage (raw bytes)
//! ```
//!
//! # Example
//!
//! ```
//! use aspen_core::layer::{Tuple, Subspace, Element};
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

// Re-export from aspen-layer for backward compatibility
pub use aspen_layer::*;

// Storage-dependent modules (depend on KeyValueStore trait)
mod allocator;
mod directory;

pub use allocator::AllocationError;
pub use allocator::HighContentionAllocator;
pub use directory::DirectoryError;
pub use directory::DirectoryLayer;
pub use directory::DirectorySubspace;
