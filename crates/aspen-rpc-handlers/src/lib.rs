//! RPC handler implementations for Aspen distributed cluster.
//!
//! This crate contains domain-specific RPC handler implementations that process
//! client requests and coordinate with the Aspen cluster services.

#![allow(
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::iter_cloned_collect,
    clippy::too_many_arguments
)]
//!
//! # Architecture
//!
//! ```text
//! ClientProtocolHandler
//!        │
//!        ▼
//!   handle_client_request()
//!        │
//!        ▼
//!   HandlerRegistry::dispatch()
//!        │
//!        ├──► [WASM] KvPlugin (ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite)
//!        ├──► [WASM] CoordinationPlugin (Lock, Counter, Sequence, RateLimiter, etc.)
//!        ├──► [WASM] HooksPlugin (HookList, HookGetMetrics, HookTrigger)
//!        ├──► [WASM] SqlPlugin (ExecuteSql)
//!        ├──► [WASM] DnsPlugin (DnsSetRecord, DnsGetRecord, etc.)
//!        ├──► BlobHandler (AddBlob, GetBlob, DownloadBlob, etc.)
//!        ├──► DocsHandler (DocsSet, DocsGet, PeerCluster ops)
//!        ├──► ForgeHandler (federation + git bridge only)
//!        ├──► ClusterHandler (Init, AddLearner, Membership, etc.)
//!        └──► CoreHandler (Ping, Health, Metrics, NodeInfo)
//! ```
//!
//! # Tiger Style
//!
//! - Each handler has a single responsibility
//! - Handlers are stateless; all state comes from `ClientProtocolContext`
//! - Bounded request processing with explicit error handling
//! - Sanitized error messages for security

pub mod client;
pub mod context;
pub mod error_sanitization;
pub mod handlers;
pub mod proxy;
pub mod registry;
/// Verified pure functions for RPC handler logic.
pub mod verified;

#[cfg(any(test, feature = "testing"))]
pub mod test_mocks;

// Re-export key types for convenience
// Re-export aspen-rpc-core types for handler extraction
pub use aspen_rpc_core;
pub use client::CLIENT_ALPN;
pub use client::ClientProtocolHandler;
// Re-export from aspen-rpc-core for backward compatibility
// Note: context and registry modules still exist locally but re-export core types
pub use context::ClientProtocolContext;
// Re-export all handlers
pub use handlers::*;
pub use proxy::ProxyService;
pub use registry::HandlerFactory;
pub use registry::HandlerRegistry;
pub use registry::RequestHandler;
pub use registry::collect_handler_factories;
