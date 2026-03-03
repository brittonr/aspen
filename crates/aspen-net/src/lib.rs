//! Service mesh control plane for Aspen.
//!
//! Provides named service discovery, SOCKS5 proxying, port forwarding,
//! and DNS integration over iroh's P2P QUIC network. Built as a thin
//! control plane on top of `iroh-proxy-utils`' existing data plane.
//!
//! # Architecture
//!
//! - **Registry**: Service CRUD in Raft KV (`/_sys/net/svc/` prefix)
//! - **Resolver**: Name → (endpoint_id, port) with local cache
//! - **SOCKS5**: RFC 1928 proxy resolving `*.aspen` names
//! - **Forward**: TCP port forwarding via `DownstreamProxy`
//! - **Auth**: UCAN capability token verification wrapper
//! - **Daemon**: Orchestrates all components

pub mod auth;
pub mod constants;
pub mod daemon;
pub mod forward;
pub mod handler;
pub mod registry;
pub mod resolver;
pub mod socks5;
pub mod types;
pub mod verified;
