//! State machine apply helpers: `apply_*_in_txn` methods that handle applying
//! Raft log entries to the state.
//!
//! # Module Organization
//!
//! - `set` - Set and SetMulti operations
//! - `delete` - Delete and DeleteMulti operations
//! - `cas` - CompareAndSwap and CompareAndDelete operations
//! - `batch` - Batch and ConditionalBatch operations
//! - `lease_ops` - Lease grant, revoke, and keepalive operations
//! - `transaction` - Transaction (etcd-style) and OptimisticTransaction (FoundationDB-style)
//! - `dispatch` - Main `apply_request_in_txn` dispatch

mod batch;
mod cas;
mod delete;
mod dispatch;
mod lease_ops;
mod set;
mod transaction;
