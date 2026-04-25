//! State machine apply helpers.

mod batch;
mod cas;
mod delete;
mod dispatch;
mod lease_ops;
pub(in crate::raft_storage) mod set;
mod transaction;
