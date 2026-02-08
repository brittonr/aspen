//! Pure functions for Aspen distributed systems.
//!
//! This crate provides pure, side-effect-free functions for:
//!
//! - **Tuple encoding**: FoundationDB-compatible order-preserving serialization
//! - **Subspace isolation**: Namespace partitioning for multi-tenant workloads
//! - **Scan pipelines**: Pagination, filtering, and continuation token handling
//!
//! All functions in this crate are deterministic and suitable for:
//! - Formal verification (Verus)
//! - Property-based testing
//! - Constrained environments (WASM, no-std with alloc)
//!
//! # Example
//!
//! ```
//! use aspen_pure::{Tuple, Subspace, Element};
//!
//! // Create a namespace for user data
//! let users = Subspace::new(Tuple::new().push("users"));
//!
//! // Create a key: ("users", "alice", "profile")
//! let key = users.pack(&Tuple::new()
//!     .push("alice")
//!     .push("profile"));
//!
//! // Get range bounds for all of alice's data
//! let alice = users.subspace(&Tuple::new().push("alice"));
//! let (start, end) = alice.range();
//! ```

pub mod constants;
pub mod scan;
pub mod subspace;
pub mod tuple;

// Re-export all public types at crate root
pub use constants::DEFAULT_SCAN_LIMIT;
pub use constants::MAX_SCAN_RESULTS;
pub use scan::build_scan_metadata;
pub use scan::decode_continuation_token;
pub use scan::encode_continuation_token;
pub use scan::execute_scan;
pub use scan::filter_scan_entries;
pub use scan::normalize_scan_limit;
pub use scan::paginate_entries;
pub use subspace::Subspace;
pub use subspace::SubspaceError;
pub use tuple::Element;
pub use tuple::Tuple;
pub use tuple::TupleError;
