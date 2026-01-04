//! Capability-based authorization for Aspen.
//!
//! This module re-exports the `aspen-auth` crate for backward compatibility.
//! New code should depend on `aspen-auth` directly.
//!
//! See [`aspen_auth`] for full documentation.

// Re-export everything from aspen-auth
// Re-export constants module for direct access
pub use aspen_auth::constants;
pub use aspen_auth::*;

#[cfg(test)]
mod tests;
