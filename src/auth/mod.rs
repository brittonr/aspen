//! Capability-based authorization for Aspen.
//!
//! This module implements UCAN-inspired capability tokens that enable
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
//! # Usage
//!
//! ```rust,ignore
//! use aspen::auth::{TokenBuilder, TokenVerifier, Capability, Audience};
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
mod capability;
mod error;
mod token;
mod verifier;

pub use builder::{TokenBuilder, generate_root_token};
pub use capability::{Capability, Operation};
pub use error::AuthError;
pub use token::{Audience, CapabilityToken};
pub use verifier::TokenVerifier;

#[cfg(test)]
mod tests;
