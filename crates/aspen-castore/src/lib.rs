//! Nix content-addressed store over irpc.
//!
//! Implements snix-castore's [`BlobService`] and [`DirectoryService`] traits
//! with iroh-blobs as the blob backend and irpc (QUIC RPC) as the transport.
//!
//! # Architecture
//!
//! ```text
//! Client (nix, snix tooling)        Server (aspen node)
//! ┌──────────────────────┐          ┌──────────────────────────┐
//! │ IrpcBlobService      │──irpc──► │ CastoreServer            │
//! │ IrpcDirectoryService │  (QUIC)  │  ├─ IrohBlobService<S>   │
//! └──────────────────────┘          │  └─ MemoryDirectoryStore  │
//!                                   └──────────────────────────┘
//! ```
//!
//! The server wraps any [`aspen_blob::BlobStore`] implementation (typically
//! backed by iroh-blobs) and a local directory store, exposing them over
//! irpc with a custom ALPN.
//!
//! The client implements the snix-castore traits by making irpc calls,
//! so upstream snix tooling can use an Aspen cluster as a castore backend
//! transparently.

pub mod client;
pub mod protocol;
pub mod server;

#[cfg(test)]
mod tests;

/// ALPN protocol identifier for the castore irpc service.
pub const CASTORE_ALPN: &[u8] = b"aspen-castore/0";

/// Re-export key types.
pub use client::IrpcBlobService;
pub use client::IrpcDirectoryService;
pub use server::CastoreServer;
