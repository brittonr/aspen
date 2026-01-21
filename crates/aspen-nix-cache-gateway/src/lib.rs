//! HTTP/3 Nix binary cache gateway for Aspen.
//!
//! This crate provides a Nix binary cache server using HTTP/3 over Iroh QUIC,
//! implemented with the h3-iroh library. It satisfies Aspen's "Iroh-only networking"
//! architecture while enabling standard Nix clients to use HTTP semantics.
//!
//! # Architecture
//!
//! The gateway integrates with Aspen's existing infrastructure:
//!
//! - **Storage**: Uses [`aspen_cache::CacheIndex`] for metadata and [`aspen_blob::BlobStore`]
//!   for NAR content
//! - **Networking**: Implements [`iroh::protocol::ProtocolHandler`] for Iroh Router integration
//! - **Protocol**: HTTP/3 via h3-iroh with ALPN `iroh+h3`
//!
//! # Nix Binary Cache Protocol
//!
//! Implements the standard Nix binary cache HTTP endpoints:
//!
//! - `GET /nix-cache-info` - Cache metadata (StoreDir, Priority, WantMassQuery)
//! - `GET /{hash}.narinfo` - Store path metadata with optional signature
//! - `GET /nar/{hash}.nar` - NAR archive download with streaming and range support
//!
//! # Key Design Decisions
//!
//! 1. **Custom h3 handlers instead of axum integration**: The h3-iroh axum integration
//!    buffers entire request/response bodies in memory, which is unacceptable for
//!    NAR files that can be gigabytes. We use raw h3 handlers with streaming.
//!
//! 2. **Signing via SOPS**: Narinfo signatures use Ed25519 keys stored in Aspen's
//!    SOPS-encrypted secrets, loaded via [`aspen_secrets::SecretsManager`].
//!
//! 3. **Tiger Style**: All operations have bounded resources (chunk sizes, timeouts,
//!    connection limits) and use snafu for structured error handling.
//!
//! # Usage
//!
//! ```ignore
//! use aspen_nix_cache_gateway::{NixCacheGateway, NixCacheGatewayConfig};
//!
//! let config = NixCacheGatewayConfig::default();
//! let gateway = NixCacheGateway::new(config, blob_store, cache_index);
//! let handler = gateway.protocol_handler();
//!
//! // Register with Iroh Router
//! router_builder.nix_cache(handler);
//! ```

pub mod config;
pub mod constants;
pub mod endpoints;
pub mod error;
pub mod handler;
pub mod signing;
pub mod streaming;

pub use config::NixCacheGatewayConfig;
pub use constants::*;
pub use error::{NixCacheError, Result};
pub use handler::NixCacheProtocolHandler;
pub use signing::NarinfoSigner;
pub use streaming::NarStreamingHandler;

// Re-export the ALPN constant from aspen-transport
pub use aspen_transport::constants::NIX_CACHE_H3_ALPN;
