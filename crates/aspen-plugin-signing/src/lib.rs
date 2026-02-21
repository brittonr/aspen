//! Ed25519 signing and verification for Aspen WASM plugins.
//!
//! Plugins are signed by computing `BLAKE3(wasm_bytes)` and signing the hash
//! with an Ed25519 secret key. The [`PluginSignature`] struct carries the
//! hex-encoded public key, signature, hash, and timestamp.
//!
//! # Signing
//!
//! ```
//! use aspen_plugin_signing::{keys, signer, verifier, PluginSignature};
//!
//! let key = keys::generate_keypair();
//! let wasm = b"(module)";
//! let sig = signer::sign_plugin(wasm, &key);
//! assert!(verifier::verify_plugin(wasm, &sig).is_ok());
//! ```
//!
//! # Trust Management
//!
//! The [`keys::TrustedKeys`] type manages an allowlist of author public keys
//! stored at `~/.config/aspen/plugin-keys.json`.

pub mod error;
pub mod keys;
pub mod signature;
pub mod signer;
pub mod verifier;

pub use error::SigningError;
pub use signature::PluginSignature;
