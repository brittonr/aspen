//! Core essentials RPC handlers for Aspen.
//!
//! This crate consolidates four closely related handler crates:
//! - **Core**: Ping, GetHealth, GetRaftMetrics, GetNodeInfo, GetLeader, GetMetrics, CheckpointWal,
//!   ListVaults, GetVaultKeys
//! - **KV**: ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite, ConditionalBatchWrite,
//!   CompareAndSwapKey, CompareAndDeleteKey, WriteKeyWithLease
//! - **Lease**: LeaseGrant, LeaseRevoke, LeaseKeepalive, LeaseTimeToLive, LeaseList,
//!   WriteKeyWithLease
//! - **Watch**: WatchCreate, WatchCancel, WatchStatus

mod core;
pub(crate) mod error_utils;
mod kv;
mod lease;
mod watch;

pub use core::CoreHandler;
use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use kv::KvHandler;
pub use lease::LeaseHandler;
pub use watch::WatchHandler;

// =============================================================================
// Handler Factories (Plugin Registration)
// =============================================================================

/// Factory for creating `CoreHandler` instances.
///
/// Priority 100 (core handler range: 100-199).
pub struct CoreHandlerFactory;

impl CoreHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CoreHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for CoreHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(CoreHandler))
    }

    fn name(&self) -> &'static str {
        "CoreHandler"
    }

    fn priority(&self) -> u32 {
        100
    }
}

/// Factory for creating `LeaseHandler` instances.
///
/// Priority 200 (essential handler range: 200-299).
pub struct LeaseHandlerFactory;

impl LeaseHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LeaseHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for LeaseHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(LeaseHandler))
    }

    fn name(&self) -> &'static str {
        "LeaseHandler"
    }

    fn priority(&self) -> u32 {
        200
    }
}

/// Factory for creating `WatchHandler` instances.
///
/// Priority 210 (essential handler range: 200-299).
pub struct WatchHandlerFactory;

impl WatchHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for WatchHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for WatchHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(WatchHandler))
    }

    fn name(&self) -> &'static str {
        "WatchHandler"
    }

    fn priority(&self) -> u32 {
        210
    }
}

/// Factory for creating `KvHandler` instances.
///
/// Priority 110 (core handler range: 100-199).
/// KV is foundational — plugin install, reload, and other system ops
/// depend on KV being available before any WASM plugins load.
pub struct KvHandlerFactory;

impl KvHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for KvHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for KvHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(KvHandler))
    }

    fn name(&self) -> &'static str {
        "KvHandler"
    }

    fn priority(&self) -> u32 {
        110
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(CoreHandlerFactory);
aspen_rpc_core::submit_handler_factory!(KvHandlerFactory);
aspen_rpc_core::submit_handler_factory!(LeaseHandlerFactory);
aspen_rpc_core::submit_handler_factory!(WatchHandlerFactory);
