//! Ref storage for Forge.
//!
//! Refs (branches, tags) are stored in Raft for strong consistency.
//! Updates go through consensus to ensure all nodes agree on the canonical state.

mod store;

pub use store::RefStore;
