//! Raft RPC authentication using HMAC-SHA256.
//!
//! This module re-exports the authentication implementation from `aspen-auth`.

pub use aspen_auth::hmac_auth::*;
