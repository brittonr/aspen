//! Verus formal verification specifications for aspen-nostr-relay.
//!
//! These specs model small pure classifier/admission helpers while leaving
//! async networking, WebSocket handshakes, and JSON serialization in the
//! production Rust shell.

mod auth_policy_spec;
mod filter_spec;
mod http_request_spec;
mod iroh_frame_spec;
mod nip11_info_spec;
mod rate_limit_spec;
mod storage_spec;
mod subscription_spec;

fn main() {}
