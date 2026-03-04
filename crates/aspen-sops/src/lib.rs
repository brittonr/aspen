//! SOPS backend using Aspen Transit for key management.
//!
//! This crate provides two modes of operation:
//!
//! ## Native Mode
//!
//! Full Rust implementation of SOPS encrypt/decrypt that talks directly to
//! Aspen Transit via Iroh QUIC. No dependency on the Go `sops` binary.
//! Supports TOML, JSON, and YAML file formats.
//!
//! ```rust,ignore
//! use aspen_sops::{encrypt_file, EncryptConfig};
//!
//! // Works with .toml, .json, .yaml, and .yml files
//! let config = EncryptConfig {
//!     input_path: "secrets.yaml".into(),
//!     cluster_ticket: "aspen1q...".into(),
//!     transit_key: "sops-data-key".into(),
//!     ..Default::default()
//! };
//! let encrypted = encrypt_file(&config).await?;
//! ```
//!
//! ## Bridge Mode
//!
//! gRPC key service that bridges SOPS ↔ Aspen Transit. The Go `sops` binary
//! connects via `--keyservice unix:///path/to/socket`.
//!
//! ```bash
//! aspen-sops keyservice --cluster-ticket aspen1q... --transit-key sops-data-key
//! sops --keyservice unix:///tmp/aspen-sops.sock decrypt secrets.sops.yaml
//! ```

pub mod client;
pub mod constants;
pub mod decrypt;
pub mod edit;
pub mod encrypt;
pub mod error;
pub mod format;
pub mod mac;
pub mod metadata;
pub mod rotate;
pub mod updatekeys;
pub mod verified;

#[cfg(feature = "keyservice")]
pub mod keyservice;

// Re-export main types
pub use client::TransitClient;
pub use constants::*;
pub use decrypt::DecryptConfig;
pub use decrypt::decrypt_file;
pub use encrypt::EncryptConfig;
pub use encrypt::encrypt_file;
pub use error::Result;
pub use error::SopsError;
pub use metadata::AspenTransitRecipient;
pub use metadata::SopsFileMetadata;
