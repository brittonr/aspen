//! Ref storage for Forge.
//!
//! Refs (branches, tags) are stored in Raft for strong consistency.
//! Updates go through consensus to ensure all nodes agree on the canonical state.
//!
//! ## Delegate Authorization
//!
//! Canonical refs (like the default branch and tags) require authorization
//! from repository delegates. Contributor refs (feature branches) can be
//! updated by anyone with a valid signature.

mod delegate;
mod store;

pub use delegate::DelegateVerifier;
pub use delegate::MultiSigCollector;
pub use delegate::SignedRefUpdate;
pub use store::RefStore;
pub use store::RefUpdateEvent;
