//! Library component of aspen-snix-bridge.
//!
//! Re-exports the nix-daemon module for integration testing.

#[cfg(feature = "snix-daemon")]
pub mod daemon;
