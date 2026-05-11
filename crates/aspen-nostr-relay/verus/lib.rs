//! Verus formal verification specifications for aspen-nostr-relay.
//!
//! These specs model small pure classifier/admission helpers while leaving
//! async networking, WebSocket handshakes, and JSON serialization in the
//! production Rust shell.

mod http_request_spec;
mod rate_limit_spec;

fn main() {}
