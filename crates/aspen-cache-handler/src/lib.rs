//! Nix binary cache RPC handlers for Aspen.
//!
//! This crate provides the cache handlers extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles cache operations:
//! - CacheQuery, CacheStats, CacheDownload: Cache queries
//! - CacheMigrationStart/Status/Cancel/Validate: Migration management

mod cache_handler;
mod migration_handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use cache_handler::CacheHandler;
pub use migration_handler::CacheMigrationHandler;
