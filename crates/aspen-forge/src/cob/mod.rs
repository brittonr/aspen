//! Collaborative Objects (COBs) for Forge.
//!
//! COBs are Radicle-style collaborative data structures that enable decentralized
//! issue tracking, patch review, and other collaboration features. Each COB is
//! modeled as an immutable DAG of changes, similar to Git's commit history.
//!
//! ## Design
//!
//! - **Immutable Changes**: Each modification to a COB creates a new change object that references
//!   its parent(s) by hash.
//!
//! - **DAG Structure**: Multiple concurrent changes create branches in the DAG, which are resolved
//!   when reading the current state.
//!
//! - **Content Addressing**: Changes are stored in iroh-blobs, addressed by BLAKE3 hash.
//!
//! - **Eventual Consistency**: COB state is eventually consistent across nodes; all nodes that have
//!   the same set of changes will compute the same state.
//!
//! ## COB Types
//!
//! - **Issue**: Bug reports, feature requests, discussions
//! - **Patch**: Code change proposals (similar to PRs)
//! - **Review**: Code review on patches
//! - **Discussion**: General threaded discussions

mod change;
mod issue;
mod patch;
mod review;
mod store;

pub use change::CobChange;
pub use change::CobOperation;
pub use change::CobType;
pub use change::FieldResolution;
pub use change::MergeStrategy;
pub use change::ReviewComment;
pub use change::ReviewSide;
pub use issue::Issue;
pub use issue::IssueScalarFieldValue;
pub use issue::IssueState;
pub use patch::Patch;
pub use patch::PatchRevision;
pub use patch::PatchState;
pub use patch::ScalarFieldValue;
pub use review::GeneralComment;
pub use review::InlineComment;
pub use review::Review;
pub use review::ReviewVerdict;
pub use store::CobStore;
pub use store::CobUpdateEvent;
pub use store::ConflictReport;
pub use store::ConflictingValue;
pub use store::FieldConflict;
