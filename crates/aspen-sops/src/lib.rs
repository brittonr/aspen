//! SOPS CLI using Aspen Transit for key management.
//!
//! All SOPS library code (encrypt, decrypt, edit, rotate, MAC, format handling)
//! now lives in `aspen_secrets::sops`. This crate provides:
//!
//! 1. The `aspen-sops` CLI binary
//! 2. The gRPC key service bridge (feature-gated behind `keyservice`)
//!
//! ## Usage
//!
//! ```bash
//! aspen-sops encrypt secrets.toml --cluster-ticket aspen1q...
//! aspen-sops decrypt secrets.sops.toml
//! aspen-sops edit secrets.sops.toml --cluster-ticket aspen1q...
//! ```

// Re-export everything from aspen-secrets::sops for backward compatibility
// Re-export main types
pub use aspen_secrets::sops::DecryptConfig;
pub use aspen_secrets::sops::EncryptConfig;
pub use aspen_secrets::sops::SopsError;
pub use aspen_secrets::sops::SopsFileMetadata;
pub use aspen_secrets::sops::SopsResult as Result;
pub use aspen_secrets::sops::TransitClient;
pub use aspen_secrets::sops::client;
pub use aspen_secrets::sops::decrypt;
pub use aspen_secrets::sops::edit;
pub use aspen_secrets::sops::encrypt;
pub use aspen_secrets::sops::format;
pub use aspen_secrets::sops::mac;
pub use aspen_secrets::sops::metadata;
pub use aspen_secrets::sops::rotate;
pub use aspen_secrets::sops::sops_constants as constants;
pub use aspen_secrets::sops::sops_error as error;
pub use aspen_secrets::sops::updatekeys;
pub use aspen_secrets::verified;

#[cfg(feature = "keyservice")]
pub mod keyservice;
