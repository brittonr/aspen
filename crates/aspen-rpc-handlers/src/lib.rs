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
//!        ├──► KvHandler (ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite)
//!        ├──► CoordinationHandler (Lock, Counter, Sequence, RateLimiter, Barrier, Semaphore, RWLock, Queue)
//!        ├──► BlobHandler (AddBlob, GetBlob, DownloadBlob, etc.)
//!        ├──► DnsHandler (DnsSetRecord, DnsGetRecord, etc.)
//!        ├──► DocsHandler (DocsSet, DocsGet, PeerCluster ops)
//!        ├──► ForgeHandler (all Forge* operations)
//!        ├──► PijulHandler (all Pijul* operations)
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
pub mod registry;
/// Verified pure functions for RPC handler logic.
pub mod verified;

#[cfg(any(test, feature = "testing"))]
pub mod test_mocks;

// Re-export key types for convenience
pub use client::CLIENT_ALPN;
pub use client::ClientProtocolHandler;
pub use context::ClientProtocolContext;
// Re-export all handlers
pub use handlers::*;
pub use registry::HandlerRegistry;
pub use registry::RequestHandler;
