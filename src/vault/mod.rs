//! Vault enforcement and rate limiting.
//!
//! This module provides quota enforcement and rate limiting for vault operations.
//! It complements the vault types in `crate::api::vault` with runtime enforcement.
//!
//! # Architecture
//!
//! ```text
//! Client Write Request
//!         |
//!         v
//! validate_client_key() --> Reject if _system: prefix or invalid vault name
//!         |
//!         v
//! VaultEnforcer::check_quota() --> Reject if quota exceeded
//!         |
//!         v
//! VaultRateLimiter::check() --> Reject if rate limited
//!         |
//!         v
//! KeyValueStore::write() --> Perform actual write
//!         |
//!         v
//! VaultEnforcer::update_metadata() --> Update key count (via SetMulti)
//! ```
//!
//! # Tiger Style
//!
//! - All limits defined as constants in `crate::api::vault`
//! - Synchronous metadata updates via SetMulti (atomic with key write)
//! - Rate limiter uses bounded LRU cache

pub mod enforcement;
pub mod rate_limit;

pub use enforcement::VaultEnforcer;
pub use rate_limit::VaultRateLimiter;
