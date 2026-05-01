//! Capability-based authorization for Aspen.
//!
//! This crate implements UCAN-inspired capability tokens that enable
//! decentralized authorization without a central user database.
//!
//! # Design Principles
//!
//! 1. **Reuse Iroh's identity**: `NodeId` is already an Ed25519 public key
//! 2. **Self-contained tokens**: No database lookup needed for authorization
//! 3. **Delegation by default**: Tokens can create child tokens with fewer permissions
//! 4. **Offline verification**: Works without contacting the cluster
//! 5. **Tiger Style**: Fixed bounds on token size, delegation depth, capability count
//!
//! # Compatibility re-exports
//!
//! `aspen-auth` intentionally re-exports portable `aspen-auth-core` types
//! (`Capability`, `Operation`, `CapabilityToken`, `Audience`, and `AuthError`) for
//! existing runtime consumers. New portable crates should depend on
//! `aspen-auth-core` directly. These re-exports are owned by the auth/ticket
//! extraction track and remain until root/runtime consumers have been migrated
//! or documented as shell-only.
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen_auth::{TokenBuilder, TokenVerifier, Capability, Audience};
//!
//! // Create a root token
//! let token = TokenBuilder::new(secret_key)
//!     .with_capability(Capability::Full { prefix: "myapp:".into() })
//!     .with_capability(Capability::Delegate)
//!     .with_lifetime(Duration::from_secs(3600))
//!     .build()?;
//!
//! // Verify and authorize
//! let verifier = TokenVerifier::new();
//! verifier.authorize(&token, &Operation::Write { key: "myapp:data".into(), value: vec![] }, None)?;
//! ```

mod builder;
mod credential;
pub mod hmac_auth;
mod revocation;
mod utils;
mod verifier;

pub use aspen_auth_core::AuthError;
pub use aspen_auth_core::Audience;
pub use aspen_auth_core::Capability;
pub use aspen_auth_core::CapabilityToken;
pub use aspen_auth_core::Operation;
pub use aspen_auth_core::constants;
pub use aspen_auth_core::token;
pub use aspen_auth_core::verified_auth;
pub use aspen_auth_core::verified_credential;
pub use builder::TokenBuilder;
pub use builder::generate_root_token;
pub use credential::Credential;
pub use revocation::KeyValueRevocationStore;
pub use revocation::RevocationStore;
pub use verifier::TokenVerifier;

#[cfg(test)]
mod tests;
