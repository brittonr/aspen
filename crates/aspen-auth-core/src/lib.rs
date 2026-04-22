//! Alloc-focused authorization types and pure logic for Aspen.
//!
//! This crate keeps portable capability models and deterministic helpers out of
//! the runtime-oriented `aspen-auth` shell crate.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod capability;
pub mod constants;
pub mod error;
pub mod token;
pub mod verified_auth;
pub mod verified_credential;

pub use capability::Capability;
pub use capability::Operation;
pub use error::AuthError;
pub use token::Audience;
pub use token::CapabilityToken;
