//! Key-value operation types.
//!
//! This module re-exports the key-value types from `aspen-kv-types` for backward compatibility.
//! New code should prefer importing from `aspen_kv_types` directly for lighter dependencies.
//!
//! ## Module Organization
//!
//! - [`batch`]: Batch operation types for atomic multi-key writes
//! - [`read`]: Read operation types for querying key-value state
//! - [`scan`]: Scan operation types for prefix-based key enumeration
//! - [`transaction`]: Transaction types for atomic multi-key operations with conditions
//! - [`validation`]: Validation functions for write commands
//! - [`mod@write`]: Write operation types for modifying key-value state

// Re-export submodules from aspen-kv-types
// Re-export all public types at the module root for convenience
pub use aspen_kv_types::BatchCondition;
pub use aspen_kv_types::BatchOperation;
pub use aspen_kv_types::CompareOp;
pub use aspen_kv_types::CompareTarget;
pub use aspen_kv_types::DeleteRequest;
pub use aspen_kv_types::DeleteResult;
pub use aspen_kv_types::KeyValueWithRevision;
pub use aspen_kv_types::ReadConsistency;
pub use aspen_kv_types::ReadRequest;
pub use aspen_kv_types::ReadResult;
pub use aspen_kv_types::ScanRequest;
pub use aspen_kv_types::ScanResult;
pub use aspen_kv_types::TxnCompare;
pub use aspen_kv_types::TxnOp;
pub use aspen_kv_types::TxnOpResult;
pub use aspen_kv_types::WriteCommand;
pub use aspen_kv_types::WriteOp;
pub use aspen_kv_types::WriteRequest;
pub use aspen_kv_types::WriteResult;
pub use aspen_kv_types::batch;
pub use aspen_kv_types::read;
pub use aspen_kv_types::scan;
pub use aspen_kv_types::transaction;
pub use aspen_kv_types::validate_write_command;
pub use aspen_kv_types::validation;
pub use aspen_kv_types::write;
