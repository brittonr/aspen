//! Outside-in scenarios: real node processes over the real Iroh transport and
//! real local Redb stores, driven through public entry points.
//!
//! The consensus path is excluded; the ChaosControl and live consensus
//! campaigns own it. Results carry the no-production, no-simulation-replacement,
//! and no-performance non-claims.

#[path = "../src/test/support.rs"]
mod test_support;

mod client;
mod negative;
mod node;
mod profiles;
mod shell;
