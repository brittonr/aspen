//! Core RPC abstractions for Aspen distributed cluster.
//!
//! This crate provides the foundational types and traits for building
//! modular RPC handlers in the Aspen cluster. It is designed to break
//! circular dependencies between handler implementations and the
//! central handler registry.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    aspen-rpc-handlers                            │
//! │  (HandlerRegistry, dispatches requests to domain handlers)       │
//! └─────────────────────────────────────────────────────────────────┘
//!          │                │                │
//!          ▼                ▼                ▼
//! ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
//! │ ForgeHandler │ │ BlobHandler  │ │ CiHandler    │
//! │  (optional)  │ │  (optional)  │ │  (optional)  │
//! └──────────────┘ └──────────────┘ └──────────────┘
//!          │                │                │
//!          └────────────────┼────────────────┘
//!                           ▼
//!              ┌─────────────────────────┐
//!              │     aspen-rpc-core      │
//!              │  - RequestHandler trait │
//!              │  - ClientProtocolContext│
//!              └─────────────────────────┘
//! ```
//!
//! # Handler Extraction Pattern
//!
//! Domain handlers can be extracted to separate crates by:
//! 1. Depend on `aspen-rpc-core` for `RequestHandler` and `ClientProtocolContext`
//! 2. Depend on the domain crate (e.g., `aspen-blob`, `aspen-forge`)
//! 3. Implement `RequestHandler` for the domain handler struct
//!
//! The central `aspen-rpc-handlers` crate then depends on all handler crates
//! and registers them in the `HandlerRegistry`.

mod context;
mod handler;
pub mod proxy;
pub mod service;

pub use context::ClientProtocolContext;
pub use context::NetworkMetricsProvider;
pub use context::PoolMetricsAdapter;
pub use context::SpanForwarder;
#[cfg(feature = "testing")]
pub use context::test_support;
pub use handler::HandlerFactory;
pub use handler::RequestHandler;
pub use proxy::ProxyConfig;
pub use service::ServiceExecutor;
pub use service::ServiceHandler;
